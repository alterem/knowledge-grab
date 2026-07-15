import { reactive } from 'vue';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

export type DownloadStatus =
  | 'idle'
  | 'downloading'
  | 'completed'
  | 'failed'
  | 'cancelled';

export interface DownloadState {
  status: DownloadStatus;
  progress: number;
  error: string;
  filePath: string;
}

interface StatusPayload {
  url: string;
  status: DownloadStatus;
  progress?: number;
  error?: string;
  filePath?: string;
}

interface ProgressPayload {
  url: string;
  progress: number;
}

// 以下载 URL 为键的全局响应式状态仓库，所有 TextbookItem 共享一对事件监听，
// 避免 N 个可见条目各注册 N 对监听、每个事件被重复检查 N 次
const states = reactive(new Map<string, DownloadState>());

function ensureState(url: string): DownloadState {
  let state = states.get(url);
  if (!state) {
    states.set(url, { status: 'idle', progress: 0, error: '', filePath: '' });
    // 重新 get 拿到 Map 返回的响应式代理，直接用字面量对象的话修改不会被跟踪
    state = states.get(url)!;
  }
  return state;
}

let unlistenStatus: UnlistenFn | null = null;
let unlistenProgress: UnlistenFn | null = null;
let initPromise: Promise<void> | null = null;

function init(): Promise<void> {
  if (initPromise) return initPromise;

  initPromise = (async () => {
    unlistenStatus = await listen<StatusPayload>('download-status', ({ payload }) => {
      if (!payload?.url) return;
      const state = ensureState(payload.url);
      state.status = payload.status;
      if (typeof payload.progress === 'number') {
        state.progress = payload.progress;
      }

      switch (payload.status) {
        case 'failed':
          state.error = payload.error || '未知错误';
          break;
        case 'completed':
          state.progress = 100;
          if (payload.filePath) state.filePath = payload.filePath;
          break;
        case 'cancelled':
          state.status = 'idle';
          state.progress = 0;
          state.error = '下载已取消';
          break;
      }
    });

    unlistenProgress = await listen<ProgressPayload>('download-progress', ({ payload }) => {
      if (!payload?.url) return;
      const state = ensureState(payload.url);
      state.progress = Math.min(Math.max(0, payload.progress), 100);
    });
  })();

  return initPromise;
}

// 返回该 URL 的共享下载状态（对象在 URL 生命周期内稳定，可直接读写），首次调用时注册全局监听
export function useDownload(url: string): DownloadState {
  const state = ensureState(url);
  void init();
  return state;
}

// 仅用于整个应用的销毁清理
export async function disposeDownloadManager(): Promise<void> {
  unlistenStatus?.();
  unlistenProgress?.();
  unlistenStatus = null;
  unlistenProgress = null;
  initPromise = null;
}
