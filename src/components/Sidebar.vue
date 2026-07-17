<script setup lang="ts">
import { ElMenu, ElMenuItem, ElIcon, ElBadge } from 'element-plus';
import { HomeFilled, Reading, Tools, ChatRound, QuestionFilled, VideoCamera, Download } from '@element-plus/icons-vue';
import { useRouter, useRoute } from 'vue-router';
import { computed, markRaw, type Component } from 'vue';
import { useDownloadPool } from '@/composables/useDownloadManager';

const router = useRouter();
const route = useRoute();
// 「下载管理」项的徽标：进行中 + 排队中的任务数
const { activeCount } = useDownloadPool();

interface MenuItem {
  index: string;
  title: string;
  icon: Component;
  path: string;
}

const menuItems: MenuItem[] = [
  { index: '0', title: '首页', icon: markRaw(HomeFilled), path: '/' },
  { index: '1', title: '课本下载', icon: markRaw(Reading), path: '/textbook-download' },
  { index: '2', title: '课程&视频下载', icon: markRaw(VideoCamera), path: '/course-download' },
  { index: '6', title: '下载管理', icon: markRaw(Download), path: '/downloads' },
  { index: '3', title: '设置', icon: markRaw(Tools), path: '/settings' },
  { index: '4', title: '免责声明', icon: markRaw(ChatRound), path: '/disclaimer' },
  { index: '5', title: '使用帮助', icon: markRaw(QuestionFilled), path: '/help' },
];

const activeIndex = computed(() => {
  return menuItems.find((item) => item.path === route.path)?.index ?? '0';
});

const handleSelect = (index: string) => {
  const selected = menuItems.find((item) => item.index === index);
  if (selected?.path && route.path !== selected.path) {
    router.push(selected.path);
  }
};
</script>

<template>
  <el-menu :default-active="activeIndex" class="sidebar-menu w-full" @select="handleSelect" :collapse="false">
    <el-menu-item v-for="item in menuItems" :key="item.index" :index="item.index">
      <el-icon>
        <component :is="item.icon" />
      </el-icon>
      <span>{{ item.title }}</span>
      <el-badge
        v-if="item.path === '/downloads' && activeCount > 0"
        :value="activeCount"
        :max="99"
        class="menu-badge"
      />
    </el-menu-item>
  </el-menu>
</template>

<style scoped>
.sidebar-menu {
  --el-menu-bg-color: transparent;
  --el-menu-text-color: var(--text-color);
  --el-menu-hover-text-color: var(--text-color);
  --el-menu-hover-bg-color: var(--hover-bg);
  --el-menu-active-color: var(--el-color-primary);
  --el-menu-item-height: 40px;
  --el-menu-sub-item-height: 36px;
  --el-menu-base-level-padding: 12px;
  --el-menu-level-padding: 16px;
  --el-menu-item-font-size: 14px;

  border-right: none;
  padding: 10px 8px;
}

/* 圆角菜单项，去掉旧的逐项分割线样式 */
.sidebar-menu .el-menu-item,
.sidebar-menu :deep(.el-sub-menu__title) {
  border-radius: 8px;
  margin-bottom: 2px;
  transition: background-color 0.15s, color 0.15s;
}

.sidebar-menu .el-menu-item.is-active {
  background-color: var(--el-color-primary-light-9);
  font-weight: 600;
}

.sidebar-menu :deep(.el-sub-menu.is-active > .el-sub-menu__title) {
  color: var(--el-color-primary);
}

/* 下载管理项的任务数徽标，靠右显示 */
.menu-badge {
  margin-left: auto;
  display: inline-flex;
  align-items: center;
}

.menu-badge :deep(.el-badge__content) {
  position: static;
  transform: none;
}
</style>
