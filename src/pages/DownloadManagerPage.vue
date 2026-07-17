<script lang="ts">
// keep-alive 依 include 匹配组件 name，单独声明
export default { name: 'DownloadManagerPage' };
</script>

<script setup lang="ts">
import { computed, ref } from 'vue';
import {
  ElButton,
  ElEmpty,
  ElIcon,
  ElMessage,
  ElMessageBox,
  ElProgress,
  ElRadioButton,
  ElRadioGroup,
  ElTag,
} from 'element-plus';
import {
  Delete,
  Document,
  FolderOpened,
  Reading,
  VideoCamera,
  VideoPause,
  VideoPlay,
} from '@element-plus/icons-vue';
import { invoke } from '@tauri-apps/api/core';
import {
  clearFinishedDownloads,
  pauseDownload,
  removeDownload,
  resumeDownload,
  useDownloadPool,
  type DownloadStatus,
  type DownloadTask,
} from '@/composables/useDownloadManager';
import { formatBytes, formatSpeed } from '@/utils/format';

const { taskList, activeCount } = useDownloadPool();

type Filter = 'all' | 'active' | 'completed' | 'failed';
const filter = ref<Filter>('all');

const ACTIVE_STATUSES: DownloadStatus[] = ['queued', 'downloading', 'paused', 'interrupted'];

const filtered = computed(() => {
  switch (filter.value) {
    case 'active':
      return taskList.value.filter((t) => ACTIVE_STATUSES.includes(t.status));
    case 'completed':
      return taskList.value.filter((t) => t.status === 'completed');
    case 'failed':
      return taskList.value.filter((t) => t.status === 'failed');
    default:
      return taskList.value;
  }
});

const completedCount = computed(() => taskList.value.filter((t) => t.status === 'completed').length);

const STATUS_TEXT: Record<string, string> = {
  queued: '排队中',
  downloading: '下载中',
  paused: '已暂停',
  completed: '已完成',
  failed: '失败',
  interrupted: '已中断',
};

const STATUS_TAG: Record<string, 'primary' | 'success' | 'warning' | 'danger' | 'info'> = {
  queued: 'warning',
  downloading: 'primary',
  paused: 'warning',
  completed: 'success',
  failed: 'danger',
  interrupted: 'info',
};

const kindIcon = (task: DownloadTask) =>
  task.kind === 'textbook' ? Reading : task.kind === 'course-video' ? VideoCamera : Document;

const kindText = (task: DownloadTask) =>
  task.kind === 'textbook' ? '教材' : task.kind === 'course-video' ? '视频' : '课件';

const showProgress = (task: DownloadTask) =>
  task.status === 'downloading' || task.status === 'queued' || task.status === 'paused' || task.status === 'interrupted';

// 进度行的说明文字：下载中显示速度与大小，其余显示状态说明
const progressMeta = (task: DownloadTask): string => {
  if (task.status === 'downloading') {
    const size = task.totalBytes
      ? `${formatBytes(task.downloadedBytes)} / ${formatBytes(task.totalBytes)}`
      : task.downloadedBytes
        ? formatBytes(task.downloadedBytes)
        : '';
    const speed = task.speed > 0 ? formatSpeed(task.speed) : '';
    return [size, speed].filter(Boolean).join(' · ') || '正在下载…';
  }
  if (task.status === 'queued') return '等待空闲线程…';
  if (task.status === 'paused') return '已暂停，可继续下载';
  return '上次未完成，可继续下载';
};

const canPause = (task: DownloadTask) => task.status === 'downloading' || task.status === 'queued';
const canResume = (task: DownloadTask) =>
  task.status === 'paused' || task.status === 'interrupted' || task.status === 'failed';

const handleResume = (task: DownloadTask) => resumeDownload(task.url);
const handlePause = (task: DownloadTask) => pauseDownload(task.url);

const handleRemove = async (task: DownloadTask) => {
  if (task.status !== 'completed') {
    try {
      await ElMessageBox.confirm(
        task.status === 'downloading' || task.status === 'queued'
          ? '任务正在下载，删除会停止下载并清理已下载的临时文件。确定删除？'
          : '删除任务会一并清理已下载的临时文件（半成品）。确定删除？',
        '删除任务',
        { confirmButtonText: '删除', cancelButtonText: '取消', type: 'warning' }
      );
    } catch {
      return;
    }
  }
  await removeDownload(task.url, true);
};

const handleClearFinished = () => {
  if (completedCount.value === 0) {
    ElMessage.info('没有已完成的记录');
    return;
  }
  clearFinishedDownloads();
  ElMessage.success('已清空已完成的下载记录');
};

const openFile = (task: DownloadTask) => {
  if (!task.filePath) {
    ElMessage.warning('文件路径未知，无法打开');
    return;
  }
  invoke('open_file', { path: task.filePath }).catch((error) => {
    ElMessage.error('打开失败: ' + error);
  });
};

const revealFile = (task: DownloadTask) => {
  if (!task.filePath) {
    ElMessage.warning('文件路径未知，无法定位');
    return;
  }
  invoke('reveal_file', { path: task.filePath }).catch((error) => {
    ElMessage.error('打开文件夹失败: ' + error);
  });
};

const emptyDescription = computed(() => {
  switch (filter.value) {
    case 'active': return '没有进行中的下载任务';
    case 'completed': return '还没有已完成的下载';
    case 'failed': return '没有失败的任务';
    default: return '下载任务会出现在这里，去「课本下载」或「课程&视频下载」添加吧';
  }
});
</script>

<template>
  <div class="page-shell">
    <div class="toolbar">
      <div class="page-title">下载管理</div>
      <div class="page-desc">
        所有下载任务集中在这里调度与展示，切换页面不影响下载；未完成的任务重启后可断点续传。
      </div>

      <div class="toolbar-row">
        <el-radio-group v-model="filter" size="small">
          <el-radio-button value="all">全部 ({{ taskList.length }})</el-radio-button>
          <el-radio-button value="active">进行中 ({{ activeCount }})</el-radio-button>
          <el-radio-button value="completed">已完成 ({{ completedCount }})</el-radio-button>
          <el-radio-button value="failed">失败</el-radio-button>
        </el-radio-group>

        <el-button size="small" plain @click="handleClearFinished">
          <el-icon class="mr-1"><Delete /></el-icon>
          清空已完成
        </el-button>
      </div>
    </div>

    <div class="list-area">
      <div v-if="filtered.length" class="task-list">
        <div v-for="task in filtered" :key="task.url" class="task-row app-card">
          <div class="task-icon" :class="`task-icon-${task.kind}`">
            <el-icon :size="20">
              <component :is="kindIcon(task)" />
            </el-icon>
          </div>

          <div class="task-main">
            <div class="task-head">
              <span class="task-title" :title="task.title">{{ task.title }}</span>
              <el-tag size="small" effect="plain" round>{{ kindText(task) }}</el-tag>
              <el-tag size="small" :type="STATUS_TAG[task.status]" effect="light" round>
                {{ STATUS_TEXT[task.status] }}
              </el-tag>
            </div>
            <div v-if="task.subtitle" class="task-subtitle" :title="task.subtitle">{{ task.subtitle }}</div>

            <div v-if="showProgress(task)" class="task-progress">
              <el-progress
                :percentage="task.progress"
                :stroke-width="7"
                :show-text="false"
                :status="task.status === 'downloading' ? '' : 'warning'"
              />
              <div class="task-progress-meta">
                <span>{{ progressMeta(task) }}</span>
                <span class="task-percent">{{ task.progress }}%</span>
              </div>
            </div>

            <div v-else-if="task.status === 'failed'" class="task-error" :title="task.error">
              {{ task.error || '未知错误' }}
            </div>
            <div v-else-if="task.status === 'completed' && task.filePath" class="task-path" :title="task.filePath">
              {{ task.filePath }}
            </div>
          </div>

          <div class="task-actions">
            <el-button v-if="canPause(task)" size="small" type="warning" plain @click="handlePause(task)">
              <el-icon class="mr-1"><VideoPause /></el-icon>
              暂停
            </el-button>
            <el-button v-if="canResume(task)" size="small" type="primary" @click="handleResume(task)">
              <el-icon class="mr-1"><VideoPlay /></el-icon>
              {{ task.status === 'failed' ? '重试' : '继续' }}
            </el-button>
            <el-button v-if="task.status === 'completed'" size="small" type="primary" plain @click="openFile(task)">
              <el-icon class="mr-1"><Document /></el-icon>
              打开
            </el-button>
            <el-button v-if="task.status === 'completed'" size="small" plain @click="revealFile(task)">
              <el-icon class="mr-1"><FolderOpened /></el-icon>
              所在目录
            </el-button>
            <el-button size="small" type="danger" plain @click="handleRemove(task)">
              <el-icon class="mr-1"><Delete /></el-icon>
              删除
            </el-button>
          </div>
        </div>
      </div>

      <el-empty v-else :description="emptyDescription" :image-size="110" class="empty-state" />
    </div>
  </div>
</template>

<style scoped>
.toolbar {
  flex-shrink: 0;
  padding: 16px 20px;
  background-color: var(--secondary-bg-color);
  border-bottom: 1px solid var(--border-color);
  transition: background-color 0.2s, border-color 0.2s;
}

.page-title {
  font-size: 16px;
  font-weight: 600;
  color: var(--text-color);
}

.page-desc {
  margin-top: 4px;
  font-size: 12px;
  color: var(--text-muted);
  line-height: 1.5;
}

.toolbar-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  flex-wrap: wrap;
  gap: 10px;
  margin-top: 12px;
}

.list-area {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 16px 20px 24px;
}

.task-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.task-row {
  display: flex;
  align-items: flex-start;
  gap: 14px;
  padding: 14px 16px;
  transition: background-color 0.2s, border-color 0.2s, box-shadow 0.2s;
}

.task-row:hover {
  border-color: var(--el-color-primary-light-7);
  box-shadow: 0 4px 16px rgba(15, 23, 42, 0.06);
}

html.dark .task-row:hover {
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.4);
}

/* 类型图标底色区分教材/视频/课件 */
.task-icon {
  display: grid;
  place-items: center;
  width: 40px;
  height: 40px;
  border-radius: 10px;
  flex-shrink: 0;
  margin-top: 2px;
}

.task-icon-textbook {
  background-color: var(--el-color-primary-light-9);
  color: var(--el-color-primary);
}

.task-icon-course-video {
  background-color: var(--el-color-success-light-9);
  color: var(--el-color-success);
}

.task-icon-course-doc {
  background-color: var(--el-color-warning-light-9);
  color: var(--el-color-warning);
}

.task-main {
  flex: 1;
  min-width: 0;
}

.task-head {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
}

.task-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-color);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.task-subtitle {
  margin-top: 2px;
  font-size: 12px;
  color: var(--text-muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.task-progress {
  margin-top: 8px;
}

.task-progress-meta {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-top: 4px;
  font-size: 12px;
  color: var(--text-muted);
}

.task-percent {
  font-variant-numeric: tabular-nums;
  color: var(--el-color-primary);
  font-weight: 600;
}

.task-error {
  margin-top: 6px;
  font-size: 12px;
  color: var(--el-color-danger);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.task-path {
  margin-top: 6px;
  font-size: 12px;
  color: var(--text-muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.task-actions {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  justify-content: flex-end;
  gap: 6px;
  flex-shrink: 0;
  margin-top: 2px;
}

.task-actions .el-button + .el-button {
  margin-left: 0;
}

.empty-state {
  margin-top: 8vh;
}
</style>
