<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue';
import { ElButton, ElProgress, ElMessage } from 'element-plus';
import { Download, Picture, Check, Close, StarFilled, View, Loading } from '@element-plus/icons-vue';
import { invoke } from '@tauri-apps/api/core';
import { useDownload } from '@/composables/useDownloadManager';

const props = defineProps<{
  textbook: {
    id: string;
    cover_url: string;
    title: string;
    total_uv: number;
    like_count: number;
    download_url: string;
  };
  categoryLabel: string;
  subjectLabel: string;
  versionLabel: string;
  gradeLabel: string;
  yearLabel: string;
}>();

// Shared, reactive download state for this book's URL (see useDownloadManager).
const download = useDownload(props.textbook.download_url);
const downloadStatus = computed(() => download.status);
const downloadProgress = computed(() => download.progress);
const downloadError = computed(() => download.error);
const coverImageSrc = ref('');
const coverLoading = ref(true);
const coverError = ref(false);

// The manager updates status centrally; surface the success toast here.
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
  const apiToken = localStorage.getItem('api_token');
  const downloadPath = localStorage.getItem('download_path');

  if (!downloadPath) {
    download.status = 'failed';
    download.error = '下载路径未设置';
    return;
  }

  download.status = 'downloading';
  download.progress = 0;
  download.error = '';

  const saveByCategorySetting = localStorage.getItem('save_by_category') === 'true';

  const textbookInfo = {
    url: props.textbook.download_url,
    title: props.textbook.title,
    category_label: props.categoryLabel,
    subject_label: props.subjectLabel,
    version_label: props.versionLabel,
    grade_label: props.gradeLabel,
    year_label: props.yearLabel,
    save_by_category: saveByCategorySetting,
  };

  invoke('download_textbook', {
    textbookInfo: textbookInfo,
    token: apiToken,
    downloadPath: downloadPath,
  }).catch(error => {
    console.error('Download failed:', error);
  });
};

const handleCancel = () => {
  invoke('cancel_download', { url: props.textbook.download_url })
    .then(() => {
      download.status = 'idle';
      download.progress = 0;
      download.error = '';
    })
    .catch(error => {
      console.error('Failed to cancel download:', error);
      download.error = '取消下载失败';
    });
};

// Compact display for large counts: 1.2万 / 3450 / 999. Keeps the row narrow
// and readable instead of showing raw seven-digit numbers.
const formatCount = (value: number): string => {
  if (!Number.isFinite(value) || value < 0) return '0';
  if (value < 10000) return value.toLocaleString('en-US');
  if (value < 100000000) {
    const wan = value / 10000;
    return `${wan >= 100 ? Math.round(wan) : parseFloat(wan.toFixed(1))}万`;
  }
  const yi = value / 100000000;
  return `${yi >= 100 ? Math.round(yi) : parseFloat(yi.toFixed(1))}亿`;
};

const likeCountText = computed(() => formatCount(props.textbook.like_count));
const totalUvText = computed(() => formatCount(props.textbook.total_uv));

const fetchCoverImage = async () => {
  if (!props.textbook.cover_url) {
    coverLoading.value = false;
    coverError.value = true;
    return;
  }
  coverLoading.value = true;
  coverError.value = false;
  try {
    const base64Image = await invoke('fetch_image', { url: props.textbook.cover_url });
    coverImageSrc.value = `data:image/jpeg;base64,${base64Image}`;
  } catch (error) {
    console.error('Failed to fetch cover image:', error);
    coverError.value = true;
  } finally {
    coverLoading.value = false;
  }
};

onMounted(() => {
  fetchCoverImage();
});
</script>

<template>
  <div class="flex items-center p-4 rounded-lg shadow-sm mb-4"
    :style="{ border: '1px solid var(--border-color)', backgroundColor: 'var(--secondary-bg-color)' }">
    <div class="cover-wrapper mr-6">
      <!-- While fetch_image resolves (base64 over IPC), show a spinner placeholder
           so the row doesn't sit with an empty gap. -->
      <div v-if="coverLoading" class="cover-placeholder">
        <el-icon class="is-loading cover-spinner">
          <Loading />
        </el-icon>
      </div>
      <div v-else-if="coverError || !coverImageSrc" class="cover-placeholder">
        <el-icon class="cover-fallback-icon">
          <Picture />
        </el-icon>
      </div>
      <el-image v-else :src="coverImageSrc" :alt="textbook.title" fit="cover"
        class="cover-image" :preview-src-list="[coverImageSrc]">
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
      <div v-if="downloadStatus === 'downloading'" class="flex flex-col space-y-2 mb-2">
        <div class="flex items-center space-x-2">
          <el-progress :percentage="downloadProgress" :stroke-width="20" :text-inside="true" class="flex-1"
            :status="downloadProgress === 100 ? 'success' : ''"></el-progress>
          <span class="text-sm" :style="{ color: 'var(--text-color)' }">{{ downloadProgress }}%</span>
        </div>
        <span class="text-xs text-gray-500">正在下载中，请稍候...</span>
      </div>

      <div v-else-if="downloadStatus === 'completed'" class="text-sm text-green-600 mb-2 flex items-center">
        <el-icon class="mr-1">
          <Check />
        </el-icon>
        下载完成！
      </div>

      <div v-else-if="downloadStatus === 'failed'" class="text-sm text-red-600 mb-2 flex items-center">
        <el-icon class="mr-1">
          <Close />
        </el-icon>
        下载失败: {{ downloadError }}
      </div>

      <div class="flex space-x-3 justify-end">
        <el-button @click="handleDownload" size="default" :type="downloadStatus === 'completed' ? 'success' : 'primary'"
          :disabled="downloadStatus === 'downloading' || downloadStatus === 'completed'"
          :loading="downloadStatus === 'downloading'">
          <el-icon class="mr-1">
            <Download v-if="downloadStatus !== 'completed'" />
            <Check v-else />
          </el-icon>
          {{
            downloadStatus === 'downloading' ? '下载中...' :
              downloadStatus === 'completed' ? '已下载' :
                downloadStatus === 'failed' ? '重试下载' : '下载'
          }}
        </el-button>
        <el-button v-if="downloadStatus === 'downloading'" @click="handleCancel" size="default" type="danger">
          取消
        </el-button>
      </div>
    </div>
  </div>
</template>

<style scoped>
/* Covers come in mixed aspect ratios (portrait single covers vs. landscape
   front+back spreads). A fixed box with object-fit: cover keeps every row the
   same height so the list stays tidy. */
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
