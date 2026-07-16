<script setup lang="ts">
import { computed, watch } from 'vue';
import { ElButton, ElProgress, ElMessage } from 'element-plus';
import { Download, Picture, Check, Close, StarFilled, View, Loading, Refresh, FolderOpened, Document } from '@element-plus/icons-vue';
import { invoke } from '@tauri-apps/api/core';
import { useDownload } from '@/composables/useDownloadManager';
import { useCoverImage } from '@/composables/useCoverImage';
import { formatCount } from '@/utils/format';
import { readDownloadSettings } from '@/utils/settings';
import type { Textbook, TextbookLabels } from '@/types';

const props = defineProps<{
  textbook: Textbook;
  labels: TextbookLabels;
}>();

// 所有条目共享一个下载状态仓库与一对事件监听（见 useDownloadManager）
const download = useDownload(props.textbook.download_url);
const { src: coverSrc, loading: coverLoading, failed: coverFailed } = useCoverImage(
  props.textbook.id,
  props.textbook.cover_url
);

watch(
  () => download.status,
  (status) => {
    if (status === 'completed') {
      ElMessage({
        message: `《${props.textbook.title}》下载成功`,
        type: 'success',
        duration: 3000,
      });
    }
  }
);

const handleDownload = () => {
  const settings = readDownloadSettings();
  if (!settings.downloadPath) {
    download.status = 'failed';
    download.error = '下载路径未设置';
    return;
  }

  download.status = 'downloading';
  download.progress = 0;
  download.error = '';

  invoke('download_textbook', {
    textbookInfo: {
      url: props.textbook.download_url,
      title: props.textbook.title,
      category_label: props.labels.category,
      subject_label: props.labels.subject,
      version_label: props.labels.version,
      grade_label: props.labels.grade,
      year_label: props.labels.year,
      save_by_category: settings.saveByCategory,
    },
    token: settings.token,
    downloadPath: settings.downloadPath,
  }).catch((error) => {
    console.error('下载失败:', error);
  });
};

const handleCancel = () => {
  invoke('cancel_download', { url: props.textbook.download_url })
    .then(() => {
      download.status = 'idle';
      download.progress = 0;
      download.error = '';
    })
    .catch((error) => {
      console.error('取消下载失败:', error);
      download.error = '取消下载失败';
    });
};

// 打开已下载的文件（用系统默认程序，如 PDF 阅读器）
const openFile = () => {
  if (!download.filePath) {
    ElMessage.warning('文件路径未知，无法打开');
    return;
  }
  invoke('open_file', { path: download.filePath }).catch((error) => {
    ElMessage.error('打开失败: ' + error);
  });
};

// 在文件管理器中定位到已下载的文件
const revealFile = () => {
  if (!download.filePath) {
    ElMessage.warning('文件路径未知，无法定位');
    return;
  }
  invoke('reveal_file', { path: download.filePath }).catch((error) => {
    ElMessage.error('打开文件夹失败: ' + error);
  });
};

const likeCountText = computed(() => formatCount(props.textbook.like_count));
const totalUvText = computed(() => formatCount(props.textbook.total_uv));
</script>

<template>
  <div class="tb-card app-card">
    <div class="cover-wrapper">
      <div v-if="coverLoading" class="cover-placeholder">
        <el-icon class="is-loading cover-spinner">
          <Loading />
        </el-icon>
      </div>
      <div v-else-if="coverFailed || !coverSrc" class="cover-placeholder">
        <el-icon class="cover-fallback-icon">
          <Picture />
        </el-icon>
      </div>
      <!-- preview-teleported 必须开启：卡片 hover 有 transform，
           就地渲染的 fixed 预览层会以卡片为包含块被压进卡片内 -->
      <el-image v-else :src="coverSrc" :alt="textbook.title" fit="cover" class="cover-image"
        :preview-src-list="[coverSrc]" preview-teleported hide-on-click-modal>
        <template #error>
          <div class="cover-placeholder">
            <el-icon class="cover-fallback-icon">
              <Picture />
            </el-icon>
          </div>
        </template>
      </el-image>
    </div>

    <div class="info">
      <h3 class="title" :title="textbook.title">{{ textbook.title }}</h3>

      <div class="stats">
        <span class="stat" :title="textbook.like_count.toLocaleString('en-US')">
          <el-icon :size="14">
            <StarFilled />
          </el-icon>
          {{ likeCountText }}
        </span>
        <span class="stat" :title="textbook.total_uv.toLocaleString('en-US')">
          <el-icon :size="14">
            <View />
          </el-icon>
          {{ totalUvText }}
        </span>
      </div>

      <div v-if="download.status === 'downloading'" class="progress-block">
        <el-progress :percentage="download.progress" :stroke-width="8" :show-text="false"
          :status="download.progress === 100 ? 'success' : ''" />
        <div class="progress-meta">
          <span>正在下载，请稍候…</span>
          <span class="progress-percent">{{ download.progress }}%</span>
        </div>
      </div>

      <div v-else-if="download.status === 'completed'" class="status-line status-success">
        <el-icon :size="14">
          <Check />
        </el-icon>
        下载完成！
      </div>

      <div v-else-if="download.status === 'failed'" class="status-line status-error" :title="download.error">
        <el-icon :size="14">
          <Close />
        </el-icon>
        下载失败: {{ download.error }}
      </div>

      <div class="actions">
        <el-button @click="handleDownload" size="small"
          :type="download.status === 'completed' ? 'success' : 'primary'"
          :disabled="download.status === 'downloading'"
          :loading="download.status === 'downloading'">
          <el-icon class="mr-1" v-if="download.status !== 'downloading'">
            <Refresh v-if="download.status === 'completed'" />
            <Download v-else />
          </el-icon>
          {{
            download.status === 'downloading' ? '下载中...' :
              download.status === 'completed' ? '重新下载' :
                download.status === 'failed' ? '重试下载' : '下载'
          }}
        </el-button>
        <el-button v-if="download.status === 'downloading'" @click="handleCancel" size="small" type="danger" plain>
          取消
        </el-button>
        <el-button v-if="download.status === 'completed'" @click="openFile" size="small" type="primary" plain>
          <el-icon class="mr-1">
            <Document />
          </el-icon>
          预览
        </el-button>
        <el-button v-if="download.status === 'completed'" @click="revealFile" size="small" plain>
          <el-icon class="mr-1">
            <FolderOpened />
          </el-icon>
          打开文件夹
        </el-button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.tb-card {
  display: flex;
  gap: 16px;
  height: 100%;
  padding: 14px;
  text-align: left;
  transition: background-color 0.2s, border-color 0.2s, box-shadow 0.2s, transform 0.2s;
}

.tb-card:hover {
  border-color: var(--el-color-primary-light-7);
  box-shadow: 0 8px 24px rgba(15, 23, 42, 0.08);
  transform: translateY(-2px);
}

html.dark .tb-card:hover {
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.45);
}

/* 封面比例不一（竖版单封面/横版跨页），固定容器 + object-fit 保证行高一致 */
.cover-wrapper {
  width: 96px;
  height: 128px;
  flex-shrink: 0;
}

.cover-image {
  width: 100%;
  height: 100%;
  border-radius: 8px;
  display: block;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.12);
  cursor: zoom-in;
}

.cover-placeholder {
  width: 100%;
  height: 100%;
  border-radius: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  background-color: var(--hover-bg);
  border: 1px dashed var(--border-color);
}

.cover-spinner {
  font-size: 1.5rem;
  color: var(--el-color-primary);
}

.cover-fallback-icon {
  font-size: 1.75rem;
  color: var(--text-muted);
  opacity: 0.6;
}

.info {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
}

.title {
  margin: 2px 0 6px;
  font-size: 15px;
  font-weight: 600;
  line-height: 1.45;
  color: var(--text-color);
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.stats {
  display: flex;
  align-items: center;
  gap: 14px;
  font-size: 12px;
  color: var(--text-muted);
}

.stat {
  display: inline-flex;
  align-items: center;
  gap: 4px;
}

.progress-block {
  margin-top: 12px;
}

.progress-meta {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-top: 6px;
  font-size: 12px;
  color: var(--text-muted);
}

.progress-percent {
  font-variant-numeric: tabular-nums;
  color: var(--el-color-primary);
  font-weight: 600;
}

.status-line {
  display: flex;
  align-items: center;
  gap: 4px;
  margin-top: 12px;
  font-size: 13px;
  min-width: 0;
}

.status-success {
  color: var(--el-color-success);
}

/* 错误信息可能很长：改为块级 + 省略号，完整内容见 title 提示 */
.status-error {
  color: var(--el-color-danger);
  display: block;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.status-error .el-icon {
  vertical-align: -2px;
  margin-right: 4px;
}

.actions {
  margin-top: auto;
  padding-top: 12px;
  display: flex;
  flex-wrap: wrap;
  justify-content: flex-end;
  gap: 8px;
}

.actions .el-button + .el-button {
  margin-left: 0;
}
</style>
