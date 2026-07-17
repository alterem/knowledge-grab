<script setup lang="ts">
import { computed } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { ElIcon, ElProgress } from 'element-plus';
import { Download } from '@element-plus/icons-vue';
import { useDownloadPool } from '@/composables/useDownloadManager';
import { formatSpeed } from '@/utils/format';

// 全局迷你下载指示条：有活动任务且不在下载管理页时悬浮于右下角，点击跳转
const route = useRoute();
const router = useRouter();
const { activeCount, overallProgress, overallSpeed } = useDownloadPool();

const visible = computed(() => activeCount.value > 0 && route.path !== '/downloads');

const goDownloads = () => {
  router.push('/downloads');
};
</script>

<template>
  <transition name="minibar">
    <button v-if="visible" class="mini-bar" type="button" title="查看下载管理" @click="goDownloads">
      <el-progress
        type="circle"
        :percentage="overallProgress"
        :width="34"
        :stroke-width="4"
        :show-text="false"
        class="mini-progress"
      />
      <span class="mini-icon">
        <el-icon :size="14"><Download /></el-icon>
      </span>
      <span class="mini-info">
        <span class="mini-title">{{ activeCount }} 个任务下载中</span>
        <span class="mini-speed">{{ overallSpeed > 0 ? formatSpeed(overallSpeed) : `${overallProgress}%` }}</span>
      </span>
    </button>
  </transition>
</template>

<style scoped>
.mini-bar {
  position: fixed;
  right: 24px;
  bottom: 24px;
  z-index: 60;
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 16px 10px 10px;
  border: 1px solid var(--border-color);
  border-radius: 999px;
  background-color: var(--secondary-bg-color);
  box-shadow: 0 8px 24px rgba(15, 23, 42, 0.16);
  cursor: pointer;
  transition: transform 0.15s, box-shadow 0.15s, background-color 0.2s, border-color 0.2s;
}

html.dark .mini-bar {
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.5);
}

.mini-bar:hover {
  transform: translateY(-2px);
  border-color: var(--el-color-primary-light-5);
  box-shadow: 0 12px 28px rgba(15, 23, 42, 0.2);
}

/* 进度环中心叠一个下载图标 */
.mini-progress {
  position: relative;
}

.mini-icon {
  position: absolute;
  left: 10px;
  top: 50%;
  transform: translateY(-50%);
  width: 34px;
  display: grid;
  place-items: center;
  color: var(--el-color-primary);
  pointer-events: none;
}

.mini-info {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  line-height: 1.3;
}

.mini-title {
  font-size: 12px;
  font-weight: 600;
  color: var(--text-color);
  white-space: nowrap;
}

.mini-speed {
  font-size: 11px;
  color: var(--text-muted);
  font-variant-numeric: tabular-nums;
  white-space: nowrap;
}

.minibar-enter-active,
.minibar-leave-active {
  transition: opacity 0.2s, transform 0.2s;
}

.minibar-enter-from,
.minibar-leave-to {
  opacity: 0;
  transform: translateY(12px);
}
</style>
