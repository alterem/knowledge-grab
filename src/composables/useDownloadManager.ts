import { computed, reactive } from 'vue';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { ElMessage, ElMessageBox } from 'element-plus';
import { readDownloadSettings } from '@/utils/settings';

// ---------------------------------------------------------------------------
// 全局下载池：所有下载入口只负责 enqueue，真正的启动由池按并发上限调度。
// 任务与页面解耦（切页/换搜索结果都不影响），记录持久化到 localStorage，
// 重启后未完成任务标记为「已中断」，续传由 Rust 侧的 .part/.parts 半成品实现。
// ---------------------------------------------------------------------------

export type DownloadTaskKind = 'textbook' | 'course-video' | 'course-doc';

/**
 * idle 仅是"卡片曾查询过、尚未入队"的占位态，不算真实任务；
 * interrupted 表示应用重启前处于排队/下载中，可续传。
 */
export type DownloadStatus =
  | 'idle'
  | 'queued'
  | 'downloading'
  | 'paused'
  | 'completed'
  | 'failed'
  | 'interrupted';

// 与 Rust TextbookDownloadInfo / CourseDownloadInfo 对应（snake_case 原样传给 invoke）
export interface TextbookDownloadPayload {
  url: string;
  title: string;
  category_label: string;
  subject_label: string;
  version_label: string;
  grade_label: string;
  year_label: string;
  save_by_category: boolean;
}

export interface CourseDownloadPayload {
  download_url: string;
  title: string;
  format: string;
  is_video: boolean;
  course_title: string | null;
  save_by_category: boolean;
  category_path: string[];
}

export interface DownloadTask {
  url: string;
  kind: DownloadTaskKind;
  title: string;
  subtitle: string;
  payload: TextbookDownloadPayload | CourseDownloadPayload | null;
  status: DownloadStatus;
  progress: number;
  downloadedBytes: number;
  /** 0 表示未知（m3u8 视频按切片计进度） */
  totalBytes: number;
  /** 字节/秒，易变值不持久化 */
  speed: number;
  error: string;
  /** 任务开始时 Rust 上报的目标路径；完成后为实际文件路径 */
  filePath: string;
  createdAt: number;
  completedAt: number | null;
}

export interface EnqueueInput {
  url: string;
  kind: DownloadTaskKind;
  title: string;
  subtitle?: string;
  payload: TextbookDownloadPayload | CourseDownloadPayload;
}

interface StatusPayload {
  url: string;
  status: 'downloading' | 'failed' | 'completed' | 'cancelled';
  progress?: number;
  error?: string;
  filePath?: string;
}

interface ProgressPayload {
  url: string;
  progress: number;
  downloadedBytes?: number;
  totalBytes?: number | null;
}

const STORAGE_KEY = 'download_tasks_v1';
const MAX_RECORDS = 300;
const SAVE_THROTTLE_MS = 1000;

// 以下载 URL 为键的全局任务表。所有卡片/管理页共享一对事件监听，
// 避免 N 个可见条目各注册 N 对监听、每个事件被重复检查 N 次
const tasks = reactive(new Map<string, DownloadTask>());

// 正在执行 invoke 的任务（并发占用），与 status 分开记：事件与 invoke 完成时序不保证一致
const inflight = new Set<string>();
// 各任务 invoke 的完成信号，删除任务时等它结束再清理磁盘半成品
const inflightPromises = new Map<string, Promise<void>>();

// 速度计算的上次采样（非响应式即可）
const speedSamples = new Map<string, { bytes: number; time: number }>();

// 一轮连续下载的统计，队列排空时用于汇总提示（替代原 batch-download-* 事件）
let burstStarted = 0;
let burstCompleted = 0;
let burstFailed = 0;

function createTask(url: string): DownloadTask {
  return {
    url,
    kind: 'textbook',
    title: '',
    subtitle: '',
    payload: null,
    status: 'idle',
    progress: 0,
    downloadedBytes: 0,
    totalBytes: 0,
    speed: 0,
    error: '',
    filePath: '',
    createdAt: 0,
    completedAt: null,
  };
}

function ensureTask(url: string): DownloadTask {
  let task = tasks.get(url);
  if (!task) {
    tasks.set(url, createTask(url));
    // 重新 get 拿到 Map 返回的响应式代理，直接用字面量对象的话修改不会被跟踪
    task = tasks.get(url)!;
  }
  return task;
}

// ---------------------------------------------------------------------------
// 持久化
// ---------------------------------------------------------------------------

let saveTimer: number | null = null;

function flushSave(): void {
  if (saveTimer !== null) {
    window.clearTimeout(saveTimer);
    saveTimer = null;
  }
  try {
    const records = [...tasks.values()]
      .filter((t) => t.status !== 'idle')
      .map(({ speed: _speed, ...rest }) => rest);
    localStorage.setItem(STORAGE_KEY, JSON.stringify(records));
  } catch (error) {
    console.warn('保存下载记录失败:', error);
  }
}

function scheduleSave(immediate = false): void {
  if (immediate) {
    flushSave();
    return;
  }
  if (saveTimer !== null) return;
  saveTimer = window.setTimeout(flushSave, SAVE_THROTTLE_MS);
}

// 记录数超限时淘汰最早的已结束任务，避免 localStorage 无限膨胀
function evictOldRecords(): void {
  const finished = [...tasks.values()]
    .filter((t) => t.status === 'completed' || t.status === 'failed')
    .sort((a, b) => (a.completedAt ?? a.createdAt) - (b.completedAt ?? b.createdAt));
  let overflow = [...tasks.values()].filter((t) => t.status !== 'idle').length - MAX_RECORDS;
  for (const task of finished) {
    if (overflow <= 0) break;
    tasks.delete(task.url);
    overflow -= 1;
  }
}

function loadPersisted(): void {
  let raw: string | null = null;
  try {
    raw = localStorage.getItem(STORAGE_KEY);
  } catch {
    return;
  }
  if (!raw) return;

  try {
    const records = JSON.parse(raw) as Array<Partial<DownloadTask>>;
    for (const record of records) {
      if (!record?.url) continue;
      const task = ensureTask(record.url);
      Object.assign(task, record);
      task.speed = 0;
      // 重启前还在排队/下载中的任务标记为「已中断」，用户可一键续传
      if (task.status === 'queued' || task.status === 'downloading') {
        task.status = 'interrupted';
        task.error = '';
      }
    }
  } catch (error) {
    console.warn('下载记录损坏，已忽略:', error);
  }
}

// ---------------------------------------------------------------------------
// 调度器：queued 任务按设置里的并发数依次启动
// ---------------------------------------------------------------------------

function maxConcurrent(): number {
  return Math.max(1, readDownloadSettings().threadCount);
}

// invoke 等待期间事件回调会并发改写 task.status；经函数读取拿到最新值，
// 同时绕过 TS 对「刚赋值过的属性」的流程收窄（否则下面的比较会被判为无交集）
function currentStatus(task: DownloadTask): DownloadStatus {
  return task.status;
}

async function startTask(task: DownloadTask): Promise<void> {
  const settings = readDownloadSettings();
  inflight.add(task.url);
  burstStarted += 1;
  task.status = 'downloading';
  task.error = '';
  task.speed = 0;
  speedSamples.delete(task.url);

  try {
    if (task.kind === 'textbook') {
      await invoke('download_textbook', {
        textbookInfo: task.payload,
        token: settings.token,
        downloadPath: settings.downloadPath,
      });
    } else {
      await invoke('download_course_resource', {
        resource: task.payload,
        token: settings.token,
        downloadPath: settings.downloadPath,
        ffmpegPath: settings.ffmpegPath,
      });
    }
    // completed 事件通常已先行处理；这里兜底校正
    if (currentStatus(task) !== 'completed') {
      task.status = 'completed';
      task.progress = 100;
      task.completedAt = Date.now();
    }
    burstCompleted += 1;
  } catch (error) {
    // 暂停触发的取消不算失败
    if (currentStatus(task) === 'paused') {
      // 保持 paused
    } else {
      task.status = 'failed';
      task.error = String(error);
      task.completedAt = Date.now();
      burstFailed += 1;
    }
  } finally {
    task.speed = 0;
    inflight.delete(task.url);
    scheduleSave(true);
    pump();
  }
}

function pump(): void {
  const limit = maxConcurrent();
  for (const task of tasks.values()) {
    if (inflight.size >= limit) break;
    if (task.status === 'queued' && !inflight.has(task.url) && task.payload) {
      const promise = startTask(task).finally(() => inflightPromises.delete(task.url));
      inflightPromises.set(task.url, promise);
    }
  }

  // 队列排空：汇总提示（多任务时），替代原 batch-download-completed/failed 事件
  const hasQueued = [...tasks.values()].some((t) => t.status === 'queued');
  if (!hasQueued && inflight.size === 0 && burstStarted > 0) {
    const completed = burstCompleted;
    const failed = burstFailed;
    const started = burstStarted;
    burstStarted = 0;
    burstCompleted = 0;
    burstFailed = 0;

    if (failed > 0) {
      ElMessage.error(`${failed} 个任务下载失败，可在「下载管理」中重试`);
    } else if (started > 1 && completed > 1) {
      const downloadPath = readDownloadSettings().downloadPath;
      ElMessageBox.confirm('全部下载完成，是否打开下载文件夹？', '下载完成', {
        confirmButtonText: '打开文件夹',
        cancelButtonText: '关闭',
        type: 'success',
      })
        .then(() => {
          if (downloadPath) {
            invoke('open_download_folder_prompt', { downloadPath }).catch((err) =>
              console.error('打开下载文件夹失败:', err)
            );
          }
        })
        .catch(() => {
          /* 用户选择不打开 */
        });
    }
  }
}

// ---------------------------------------------------------------------------
// 事件监听（模块级注册一次）
// ---------------------------------------------------------------------------

let unlistenStatus: UnlistenFn | null = null;
let unlistenProgress: UnlistenFn | null = null;
let initPromise: Promise<void> | null = null;

function handleStatusEvent(payload: StatusPayload): void {
  if (!payload?.url) return;
  const task = ensureTask(payload.url);

  switch (payload.status) {
    case 'downloading':
      // 暂停指令已发但 Rust 尚未停下时，忽略迟到的 downloading 事件
      if (task.status !== 'paused') {
        task.status = 'downloading';
        if (typeof payload.progress === 'number' && payload.progress > 0) {
          task.progress = payload.progress;
        }
      }
      // 任务开始时上报的目标路径：续传/清理半成品的锚点
      if (payload.filePath) task.filePath = payload.filePath;
      break;
    case 'completed':
      task.status = 'completed';
      task.progress = 100;
      task.error = '';
      task.completedAt = Date.now();
      if (payload.filePath) task.filePath = payload.filePath;
      if (task.totalBytes > 0) task.downloadedBytes = task.totalBytes;
      break;
    case 'failed':
      // 进度保留在失败处，便于用户判断；错误详情展示在卡片上
      task.status = 'failed';
      task.error = payload.error || '未知错误';
      task.completedAt = Date.now();
      break;
    case 'cancelled':
      // 只有「暂停」会触发取消；若 Rust 侧自行停止也归入 paused，可继续
      if (task.status !== 'completed' && task.status !== 'failed') {
        task.status = 'paused';
      }
      break;
  }
  task.speed = 0;
  scheduleSave(true);
}

function handleProgressEvent(payload: ProgressPayload): void {
  if (!payload?.url) return;
  const task = ensureTask(payload.url);
  task.progress = Math.min(Math.max(0, payload.progress), 100);
  if (typeof payload.downloadedBytes === 'number') {
    const now = Date.now();
    const prev = speedSamples.get(task.url);
    if (prev && now > prev.time && payload.downloadedBytes >= prev.bytes) {
      const instant = ((payload.downloadedBytes - prev.bytes) / (now - prev.time)) * 1000;
      // 指数平滑，避免速度数字跳动
      task.speed = task.speed > 0 ? task.speed * 0.4 + instant * 0.6 : instant;
    }
    speedSamples.set(task.url, { bytes: payload.downloadedBytes, time: now });
    task.downloadedBytes = payload.downloadedBytes;
  }
  if (typeof payload.totalBytes === 'number' && payload.totalBytes > 0) {
    task.totalBytes = payload.totalBytes;
  }
  scheduleSave();
}

/** 初始化下载池：注册事件监听并载入历史记录。幂等，App 启动时调用一次即可 */
export function initDownloadManager(): Promise<void> {
  if (initPromise) return initPromise;

  loadPersisted();

  initPromise = (async () => {
    unlistenStatus = await listen<StatusPayload>('download-status', ({ payload }) =>
      handleStatusEvent(payload)
    );
    unlistenProgress = await listen<ProgressPayload>('download-progress', ({ payload }) =>
      handleProgressEvent(payload)
    );
  })();

  return initPromise;
}

// ---------------------------------------------------------------------------
// 对外 API
// ---------------------------------------------------------------------------

/**
 * 返回该 URL 的共享任务状态（对象在 URL 生命周期内稳定，可直接读取），
 * 首次调用时注册全局监听。卡片组件据此渲染进度，不要直接改写状态。
 */
export function useDownload(url: string): DownloadTask {
  const task = ensureTask(url);
  void initDownloadManager();
  return task;
}

/** 入队一个下载任务。已在排队/下载中的任务不会重复入队，返回是否入队成功 */
export function enqueueDownload(input: EnqueueInput): boolean {
  void initDownloadManager();
  const task = ensureTask(input.url);

  if (task.status === 'queued' || task.status === 'downloading') {
    return false;
  }

  task.kind = input.kind;
  task.title = input.title;
  task.subtitle = input.subtitle ?? '';
  task.payload = input.payload;
  task.status = 'queued';
  task.progress = 0;
  task.downloadedBytes = 0;
  task.totalBytes = 0;
  task.speed = 0;
  task.error = '';
  task.createdAt = Date.now();
  task.completedAt = null;
  speedSamples.delete(task.url);

  evictOldRecords();
  scheduleSave(true);
  pump();
  return true;
}

/** 暂停：排队任务原地挂起；下载中任务停止但保留半成品，继续时自动续传 */
export function pauseDownload(url: string): void {
  const task = tasks.get(url);
  if (!task) return;
  if (task.status !== 'queued' && task.status !== 'downloading') return;

  const wasRunning = inflight.has(url);
  task.status = 'paused';
  task.speed = 0;
  scheduleSave(true);

  if (wasRunning) {
    invoke('cancel_download', { url }).catch(() => {
      // invoke 刚发出、Rust 尚未注册取消令牌的空窗期：稍后重试一次
      window.setTimeout(() => {
        const current = tasks.get(url);
        if (current?.status === 'paused' && inflight.has(url)) {
          invoke('cancel_download', { url }).catch(() => {
            // Rust 侧确已结束（完成/失败事件随后到达并覆盖状态），忽略
          });
        }
      }, 500);
    });
  } else {
    pump();
  }
}

/** 继续/重试：paused、interrupted、failed 的任务重新排队（Rust 见半成品自动续传） */
export function resumeDownload(url: string): void {
  const task = tasks.get(url);
  if (!task?.payload) return;
  if (task.status !== 'paused' && task.status !== 'interrupted' && task.status !== 'failed') return;

  task.status = 'queued';
  task.error = '';
  task.completedAt = null;
  scheduleSave(true);
  pump();
}

/**
 * 删除任务记录。下载中的任务先停止；removeArtifacts 时连带清理磁盘上的
 * 半成品（<final>.part / <final>.parts），已完成的最终文件不会被删除。
 */
export async function removeDownload(url: string, removeArtifacts = true): Promise<void> {
  const task = tasks.get(url);
  if (!task) return;

  if (inflight.has(url)) {
    task.status = 'paused';
    await invoke('cancel_download', { url }).catch(() => {});
    // 等 Rust 任务真正停下（invoke 结束）再清理，避免半成品文件仍被占用
    await inflightPromises.get(url)?.catch(() => {});
  }

  const filePath = task.filePath;
  const finished = task.status === 'completed';
  tasks.delete(url);
  speedSamples.delete(url);
  scheduleSave(true);
  pump();

  if (removeArtifacts && !finished && filePath) {
    await invoke('remove_download_artifacts', { filePath }).catch((err) =>
      console.warn('清理半成品失败:', err)
    );
  }
}

/** 清空已完成的任务记录（只删记录，不动文件） */
export function clearFinishedDownloads(): void {
  for (const task of [...tasks.values()]) {
    if (task.status === 'completed') {
      tasks.delete(task.url);
      speedSamples.delete(task.url);
    }
  }
  scheduleSave(true);
}

const taskList = computed(() =>
  [...tasks.values()].filter((t) => t.status !== 'idle').sort((a, b) => b.createdAt - a.createdAt)
);

const activeCount = computed(
  () => taskList.value.filter((t) => t.status === 'queued' || t.status === 'downloading').length
);

// 活动任务的平均进度，迷你悬浮条用
const overallProgress = computed(() => {
  const active = taskList.value.filter(
    (t) => t.status === 'queued' || t.status === 'downloading'
  );
  if (active.length === 0) return 0;
  return Math.round(active.reduce((sum, t) => sum + t.progress, 0) / active.length);
});

const overallSpeed = computed(() =>
  taskList.value.reduce((sum, t) => (t.status === 'downloading' ? sum + t.speed : sum), 0)
);

/** 下载管理页 / 侧边栏徽标 / 迷你悬浮条共用的池视图 */
export function useDownloadPool() {
  void initDownloadManager();
  return { taskList, activeCount, overallProgress, overallSpeed };
}

// 仅用于整个应用的销毁清理
export async function disposeDownloadManager(): Promise<void> {
  flushSave();
  unlistenStatus?.();
  unlistenProgress?.();
  unlistenStatus = null;
  unlistenProgress = null;
  initPromise = null;
}
