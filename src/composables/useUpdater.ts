import { reactive } from 'vue';
import { check, type Update } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';
import { invoke } from '@tauri-apps/api/core';
import { ElMessage, ElMessageBox, ElNotification } from 'element-plus';
import { STORAGE_KEYS } from '@/utils/settings';

export const RELEASES_URL = 'https://github.com/alterem/knowledge-grab/releases/latest';

export type UpdaterStatus = 'idle' | 'checking' | 'downloading' | 'installing';

// 模块级单例：设置页与启动静默检查共享同一份状态，避免并发触发两次更新流程
const state = reactive({
  status: 'idle' as UpdaterStatus,
  downloaded: 0,
  // 总字节数可能未知（服务器未返回 Content-Length），为 0 时进度条显示不确定态
  total: 0,
});

export function useUpdaterState() {
  return state;
}

export function isAutoCheckEnabled(): boolean {
  return localStorage.getItem(STORAGE_KEYS.autoCheckUpdate) !== 'false';
}

export function setAutoCheckEnabled(enabled: boolean): void {
  localStorage.setItem(STORAGE_KEYS.autoCheckUpdate, String(enabled));
}

async function openReleasesPage(): Promise<void> {
  try {
    await invoke('open_url', { url: RELEASES_URL });
  } catch (error) {
    console.error('打开发布页失败:', error);
  }
}

// 下载并安装更新。Windows 上安装器会自动退出应用，relaunch 实际不会执行到；
// macOS/Linux(AppImage) 替换完成后由 relaunch 重启。
async function downloadAndInstall(update: Update): Promise<void> {
  state.status = 'downloading';
  state.downloaded = 0;
  state.total = 0;
  try {
    await update.downloadAndInstall((event) => {
      switch (event.event) {
        case 'Started':
          state.total = event.data.contentLength ?? 0;
          break;
        case 'Progress':
          state.downloaded += event.data.chunkLength;
          break;
        case 'Finished':
          state.status = 'installing';
          break;
      }
    });
    await relaunch();
  } catch (error) {
    state.status = 'idle';
    console.error('自动更新失败:', error);
    try {
      await ElMessageBox.confirm(`自动更新失败：${error}`, '更新失败', {
        confirmButtonText: '前往发布页手动下载',
        cancelButtonText: '关闭',
        type: 'error',
      });
      await openReleasesPage();
    } catch {
      // 用户选择关闭
    }
  }
}

async function promptInstall(update: Update): Promise<void> {
  const notes = (update.body || '').trim();
  const message = `发现新版本 v${update.version}，是否立即下载并安装？${
    notes ? `\n\n更新内容：\n${notes.slice(0, 800)}` : ''
  }`;
  try {
    await ElMessageBox.confirm(message, '发现新版本', {
      confirmButtonText: '立即更新',
      cancelButtonText: '暂不更新',
      type: 'info',
      // 更新说明是多行文本，保留换行
      customStyle: { whiteSpace: 'pre-line' },
    });
  } catch {
    return; // 暂不更新
  }
  await downloadAndInstall(update);
}

/**
 * 检查更新。silent 为 true 时（启动时自动检查）：无更新与失败都不打扰用户，
 * 有更新只弹一条可点击的通知；手动检查则全程有反馈，失败可跳转发布页。
 */
export async function checkForUpdates(options: { silent?: boolean } = {}): Promise<void> {
  const { silent = false } = options;
  if (state.status !== 'idle') return;

  state.status = 'checking';
  let update: Update | null = null;
  try {
    update = await check();
  } catch (error) {
    state.status = 'idle';
    // 常见失败：网络不通、GitHub 访问受限、deb/rpm 安装方式不支持自动更新
    if (silent) {
      console.warn('检查更新失败:', error);
      return;
    }
    try {
      await ElMessageBox.confirm(
        `检查更新失败：${error}\n\n可以前往 GitHub 发布页手动查看最新版本。`,
        '检查更新',
        {
          confirmButtonText: '前往发布页',
          cancelButtonText: '关闭',
          type: 'warning',
          customStyle: { whiteSpace: 'pre-line' },
        }
      );
      await openReleasesPage();
    } catch {
      // 用户选择关闭
    }
    return;
  }

  state.status = 'idle';
  if (!update) {
    if (!silent) ElMessage.success('当前已是最新版本');
    return;
  }

  if (silent) {
    const notification = ElNotification({
      title: '发现新版本',
      message: `v${update.version} 已发布，点击查看并更新`,
      type: 'info',
      duration: 0,
      onClick: () => {
        notification.close();
        void promptInstall(update);
      },
    });
  } else {
    await promptInstall(update);
  }
}
