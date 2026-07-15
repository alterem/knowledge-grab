<script setup lang="ts">
import { computed, watch } from 'vue';
import { ElButton, ElProgress, ElMessage } from 'element-plus';
import { Download, Picture, Check, Close, StarFilled, View, Loading } from '@element-plus/icons-vue';
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

const likeCountText = computed(() => formatCount(props.textbook.like_count));
const totalUvText = computed(() => formatCount(props.textbook.total_uv));
</script>

<template>
  <div class="flex items-center p-4 rounded-lg shadow-sm mb-4"
    :style="{ border: '1px solid var(--border-color)', backgroundColor: 'var(--secondary-bg-color)' }">
    <div class="cover-wrapper mr-6">
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
      <el-image v-else :src="coverSrc" :alt="textbook.title" fit="cover" class="cover-image"
        :preview-src-list="[coverSrc]">
        <template #error>
          <div class="cover-placeholder">
            <el-icon class="cover-fallback-icon">
              <Picture />
            </el-icon>
          </div>
        </template>
      </el-image>
    </div>
    <div class="flex-1 flex flex-col justify-between">
      <h3 class="text-lg font-semibold mb-2" :style="{ color: 'var(--text-color)' }">{{ textbook.title }}</h3>
      <div class="text-sm mb-4" :style="{ color: 'var(--text-color)' }">
        <div class="flex items-center justify-center">
          <div class="flex items-center mr-4">
            <el-icon class="mr-1">
              <StarFilled />
            </el-icon>
            <span :title="textbook.like_count.toLocaleString('en-US')">{{ likeCountText }}</span>
          </div>
          <div class="flex items-center">
            <el-icon class="mr-1">
              <View />
            </el-icon>
            <span :title="textbook.total_uv.toLocaleString('en-US')">{{ totalUvText }}</span>
          </div>
        </div>
      </div>

      <div v-if="download.status === 'downloading'" class="flex flex-col space-y-2 mb-2">
        <div class="flex items-center space-x-2">
          <el-progress :percentage="download.progress" :stroke-width="20" :text-inside="true" class="flex-1"
            :status="download.progress === 100 ? 'success' : ''"></el-progress>
          <span class="text-sm" :style="{ color: 'var(--text-color)' }">{{ download.progress }}%</span>
        </div>
        <span class="text-xs text-gray-500">正在下载中，请稍候...</span>
      </div>

      <div v-else-if="download.status === 'completed'" class="text-sm text-green-600 mb-2 flex items-center">
        <el-icon class="mr-1">
          <Check />
        </el-icon>
        下载完成！
      </div>

      <div v-else-if="download.status === 'failed'" class="text-sm text-red-600 mb-2 flex items-center">
        <el-icon class="mr-1">
          <Close />
        </el-icon>
        下载失败: {{ download.error }}
      </div>

      <div class="flex space-x-3 justify-end">
        <el-button @click="handleDownload" size="default"
          :type="download.status === 'completed' ? 'success' : 'primary'"
          :disabled="download.status === 'downloading' || download.status === 'completed'"
          :loading="download.status === 'downloading'">
          <el-icon class="mr-1">
            <Download v-if="download.status !== 'completed'" />
            <Check v-else />
          </el-icon>
          {{
            download.status === 'downloading' ? '下载中...' :
              download.status === 'completed' ? '已下载' :
                download.status === 'failed' ? '重试下载' : '下载'
          }}
        </el-button>
        <el-button v-if="download.status === 'downloading'" @click="handleCancel" size="default" type="danger">
          取消
        </el-button>
      </div>
    </div>
  </div>
</template>

<style scoped>
/* 封面比例不一（竖版单封面/横版跨页），固定容器 + object-fit 保证行高一致 */
.cover-wrapper {
  width: 6rem;
  height: 8rem;
  flex-shrink: 0;
}

.cover-image {
  width: 100%;
  height: 100%;
  border-radius: 0.375rem;
  display: block;
}

.cover-placeholder {
  width: 100%;
  height: 100%;
  border-radius: 0.375rem;
  display: flex;
  align-items: center;
  justify-content: center;
  background-color: var(--secondary-bg-color);
  border: 1px solid var(--border-color);
}

.cover-spinner {
  font-size: 1.5rem;
  color: var(--el-color-primary);
}

.cover-fallback-icon {
  font-size: 1.75rem;
  color: var(--text-color);
  opacity: 0.35;
}
</style>
