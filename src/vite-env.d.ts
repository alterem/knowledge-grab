/// <reference types="vite/client" />

interface Window {
  __TAURI_METADATA__?: object; // Add the type declaration for __TAURI_METADATA__
  __TAURI__?: { // Add the type declaration for __TAURI__
    window: {
      getCurrent: () => any; // Simplified type for getCurrent
    };
    // Add other Tauri global API types if needed
  };
}

declare module '@tauri-apps/api/window' {
  export const appWindow: any; // Add type declaration for appWindow
  export function getCurrent(): any; // Add type declaration for getCurrent
}
