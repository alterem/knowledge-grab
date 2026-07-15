// localStorage 键集中管理，避免散落各处的字符串字面量
export const STORAGE_KEYS = {
  token: 'api_token',
  downloadPath: 'download_path',
  threadCount: 'thread_count',
  saveByCategory: 'save_by_category',
  theme: 'theme',
} as const;

export interface DownloadSettings {
  token: string | null;
  downloadPath: string | null;
  threadCount: number;
  saveByCategory: boolean;
}

export function readDownloadSettings(): DownloadSettings {
  const threadCount = parseInt(localStorage.getItem(STORAGE_KEYS.threadCount) || '4', 10);
  return {
    token: localStorage.getItem(STORAGE_KEYS.token),
    downloadPath: localStorage.getItem(STORAGE_KEYS.downloadPath),
    threadCount: Number.isFinite(threadCount) && threadCount > 0 ? threadCount : 4,
    saveByCategory: localStorage.getItem(STORAGE_KEYS.saveByCategory) === 'true',
  };
}
