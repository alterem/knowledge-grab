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

// One reactive store keyed by download URL, shared across every TextbookItem.
// A single pair of Tauri listeners dispatches into it, so N visible items no
// longer register N listener pairs that each re-check every event.
const states = reactive(new Map<string, DownloadState>());

function ensureState(url: string): DownloadState {
  let state = states.get(url);
  if (!state) {
    states.set(url, { status: 'idle', progress: 0, error: '', filePath: '' });
    // Re-read so callers get the reactive proxy the Map returns, not the raw
    // object literal — mutations on the raw object wouldn't be tracked.
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

/**
 * Returns the shared reactive download state for a URL, registering the global
 * listeners on first use. The returned object is stable for the URL's lifetime,
 * so components can read and mutate it directly.
 */
export function useDownload(url: string): DownloadState {
  const state = ensureState(url);
  // Fire-and-forget; idempotent across every caller.
  void init();
  return state;
}

/** Tears down the global listeners. Intended for a full app teardown only. */
export async function disposeDownloadManager(): Promise<void> {
  unlistenStatus?.();
  unlistenProgress?.();
  unlistenStatus = null;
  unlistenProgress = null;
  initPromise = null;
}
