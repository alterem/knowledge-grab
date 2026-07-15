<script setup lang="ts">
import { ref, onUnmounted, onMounted, provide } from 'vue';
import { ElContainer, ElHeader, ElAside, ElMain, ElMessage } from 'element-plus';
import { Sunny, Moon, QuestionFilled } from '@element-plus/icons-vue';
import Sidebar from './components/Sidebar.vue';
import { useRouter } from 'vue-router';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { STORAGE_KEYS } from '@/utils/settings';

const router = useRouter();

const onHelp = () => {
  router.push('/help');
}

const openGitHub = async () => {
  try {
    await invoke('open_url', { url: 'https://github.com/alterem/knowledge-grab' });
  } catch (error) {
    console.error('无法打开 GitHub 链接:', error);
  }
}

const asideWidth = ref(200);
const isResizing = ref(false);

let startX = 0;
let startWidth = 0;

const startDragging = (event: MouseEvent) => {
  isResizing.value = true;
  startX = event.clientX;
  startWidth = asideWidth.value;
  document.addEventListener('mousemove', dragMove);
  document.addEventListener('mouseup', stopDragging);
};

const dragMove = (event: MouseEvent) => {
  if (!isResizing.value) return;
  const newWidth = startWidth + (event.clientX - startX);
  asideWidth.value = Math.max(180, Math.min(240, newWidth));
};

const stopDragging = () => {
  isResizing.value = false;
  document.removeEventListener('mousemove', dragMove);
  document.removeEventListener('mouseup', stopDragging);
};

const isDarkMode = ref(false);

// 同步页面主题类与原生窗口主题（macOS/Windows 标题栏跟随应用而非系统）
const applyTheme = (dark: boolean) => {
  document.documentElement.classList.toggle('dark', dark);
  getCurrentWindow()
    .setTheme(dark ? 'dark' : 'light')
    .catch((error) => console.error('设置窗口主题失败:', error));
};

const toggleTheme = () => {
  isDarkMode.value = !isDarkMode.value;
  applyTheme(isDarkMode.value);
  localStorage.setItem(STORAGE_KEYS.theme, isDarkMode.value ? 'dark' : 'light');
};

provide('isDarkMode', isDarkMode);
provide('toggleTheme', toggleTheme);

// 登录窗口捕获的令牌在应用层持久化，避免用户离开设置页后丢失
let unlistenTokenCaptured: UnlistenFn | null = null;

onMounted(async () => {
  isDarkMode.value = localStorage.getItem(STORAGE_KEYS.theme) === 'dark';
  applyTheme(isDarkMode.value);

  document.addEventListener('contextmenu', handleRightClick);
  window.addEventListener('keydown', handleKeydown);

  unlistenTokenCaptured = await listen<{ token: string }>('access-token-captured', (event) => {
    const token = event.payload?.token;
    if (!token) return;
    localStorage.setItem(STORAGE_KEYS.token, token);
    ElMessage.success('已自动获取 Access Token 并保存');
  });
});

const handleRightClick = (event: MouseEvent) => {
  event.preventDefault();
};

// macOS 惯例：Command + , 打开设置页面
const handleKeydown = (event: KeyboardEvent) => {
  if (event.metaKey && event.key === ',') {
    event.preventDefault();
    if (router.currentRoute.value.path !== '/settings') {
      router.push('/settings');
    }
  }
};

onUnmounted(() => {
  stopDragging();
  document.removeEventListener('contextmenu', handleRightClick);
  window.removeEventListener('keydown', handleKeydown);
  unlistenTokenCaptured?.();
  unlistenTokenCaptured = null;
});

</script>

<template>
  <el-config-provider>
    <el-container class="h-screen w-screen">

      <el-header height="56px" class="app-header">
        <div class="flex items-center min-w-0">
          <img src="/icon.png" alt="Logo" class="h-8 w-8 mr-3 shrink-0" @contextmenu.prevent>
          <div class="min-w-0">
            <div class="app-title">国家中小学智慧教育平台</div>
            <div class="app-subtitle">教材资源下载工具</div>
          </div>
        </div>

        <div class="flex items-center gap-1">
          <el-tooltip :content="isDarkMode ? '切换到亮色模式' : '切换到暗色模式'" placement="bottom">
            <button class="header-icon-btn" type="button" @click="toggleTheme">
              <el-icon :size="18">
                <component :is="isDarkMode ? Moon : Sunny" />
              </el-icon>
            </button>
          </el-tooltip>
          <el-tooltip content="帮助" placement="bottom">
            <button class="header-icon-btn" type="button" @click="onHelp">
              <el-icon :size="18">
                <QuestionFilled />
              </el-icon>
            </button>
          </el-tooltip>
          <el-tooltip content="GitHub 仓库" placement="bottom">
            <button class="header-icon-btn" type="button" @click="openGitHub">
              <svg viewBox="0 0 16 16" width="17" height="17" fill="currentColor" aria-hidden="true">
                <path
                  d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27s1.36.09 2 .27c1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.01 8.01 0 0 0 16 8c0-4.42-3.58-8-8-8" />
              </svg>
            </button>
          </el-tooltip>
        </div>
      </el-header>

      <el-container>
        <el-aside :width="asideWidth + 'px'" class="app-aside">
          <Sidebar />
        </el-aside>
        <div class="resize-handle" :class="{ 'is-resizing': isResizing }" @mousedown="startDragging"></div>

        <el-main class="app-main">
          <router-view></router-view>
        </el-main>
      </el-container>

    </el-container>
  </el-config-provider>
</template>

<style scoped>
.app-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 16px;
  background-color: var(--secondary-bg-color);
  border-bottom: 1px solid var(--border-color);
  transition: background-color 0.2s, border-color 0.2s;
}

.app-title {
  font-size: 15px;
  font-weight: 600;
  line-height: 1.35;
  color: var(--text-color);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.app-subtitle {
  font-size: 11px;
  line-height: 1.3;
  color: var(--text-muted);
  white-space: nowrap;
}

.header-icon-btn {
  display: grid;
  place-items: center;
  width: 34px;
  height: 34px;
  padding: 0;
  border: none;
  border-radius: 8px;
  background: transparent;
  color: var(--text-muted);
  cursor: pointer;
  transition: background-color 0.15s, color 0.15s;
}

.header-icon-btn:hover {
  background-color: var(--hover-bg);
  color: var(--el-color-primary);
}

.app-aside {
  background-color: var(--secondary-bg-color);
  border-right: 1px solid var(--border-color);
  overflow-y: auto;
  overflow-x: hidden;
  transition: background-color 0.2s, border-color 0.2s;
}

/* 拖拽条平时透明、悬停/拖动时显示主色，并向左覆盖侧栏边框 */
.resize-handle {
  position: relative;
  z-index: 10;
  flex-shrink: 0;
  width: 3px;
  margin-left: -3px;
  cursor: ew-resize;
  background-color: transparent;
  transition: background-color 0.15s;
}

.resize-handle:hover,
.resize-handle.is-resizing {
  background-color: var(--el-color-primary);
}

.app-main {
  position: relative;
  padding: 0;
  overflow-y: auto;
  overflow-x: hidden;
  background-color: var(--bg-color);
  transition: background-color 0.2s;
}
</style>
