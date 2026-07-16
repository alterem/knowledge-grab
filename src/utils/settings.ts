// localStorage 键集中管理，避免散落各处的字符串字面量
export const STORAGE_KEYS = {
  token: 'api_token',
  // mac_key 与 access_token 同源，供视频课件的 doc-center 签名下载使用
  macKey: 'api_mac_key',
  downloadPath: 'download_path',
  threadCount: 'thread_count',
  saveByCategory: 'save_by_category',
  ffmpegPath: 'ffmpeg_path',
  theme: 'theme',
} as const;

export interface DownloadSettings {
  token: string | null;
  macKey: string | null;
  downloadPath: string | null;
  threadCount: number;
  saveByCategory: boolean;
  ffmpegPath: string | null;
}

export function readDownloadSettings(): DownloadSettings {
  const threadCount = parseInt(localStorage.getItem(STORAGE_KEYS.threadCount) || '4', 10);
  return {
    token: localStorage.getItem(STORAGE_KEYS.token),
    macKey: localStorage.getItem(STORAGE_KEYS.macKey),
    downloadPath: localStorage.getItem(STORAGE_KEYS.downloadPath),
    threadCount: Number.isFinite(threadCount) && threadCount > 0 ? threadCount : 4,
    saveByCategory: localStorage.getItem(STORAGE_KEYS.saveByCategory) === 'true',
    ffmpegPath: localStorage.getItem(STORAGE_KEYS.ffmpegPath),
  };
}
