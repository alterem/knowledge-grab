<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue';
import { ElButton, ElProgress, ElMessage } from 'element-plus';
import { Download, Picture, Check, Close, StarFilled, View } from '@element-plus/icons-vue';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

const props = defineProps<{
  textbook: {
    id: number;
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

const downloadStatus = ref('idle');
const downloadProgress = ref(0);
const downloadError = ref('');
const coverImageSrc = ref('');
let unlistenStatus: (() => void) | null = null;
let unlistenProgress: (() => void) | null = null;

const handleDownloadComplete = (filePath: string) => {
  console.log('Download completed successfully to:', filePath);
  const bookTitle = props.textbook.title;

  ElMessage({
    message: `《${bookTitle}》下载成功`,
    type: 'success',
    duration: 3000
  });
};

const handleDownload = () => {
  console.log('Download:', props.textbook.download_url);
  const apiToken = localStorage.getItem('api_token');
  const downloadPath = localStorage.getItem('download_path');

  if (!downloadPath) {
    console.error('Download path is not set.');
    downloadStatus.value = 'failed';
    downloadError.value = '下载路径未设置';
    return;
  }

  downloadStatus.value = 'downloading';
  downloadProgress.value = 0;
  downloadError.value = '';

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
  }).then(resp => {
    console.log(resp);
  }).catch(error => {
    console.error('Download failed:', error);
  });
};

const handleCancel = () => {
  console.log('Cancel download:', props.textbook.download_url);
  invoke('cancel_download', { url: props.textbook.download_url })
    .then(() => {
      downloadStatus.value = 'idle';
      downloadProgress.value = 0;
      downloadError.value = '';
      console.log('Download cancelled successfully.');
    })
    .catch(error => {
      console.error('Failed to cancel download:', error);
      downloadError.value = '取消下载失败';
    });
};

const fetchCoverImage = async () => {
  if (!props.textbook.cover_url) {
    return;
  }
  try {
    const base64Image = await invoke('fetch_image', { url: props.textbook.cover_url });
    coverImageSrc.value = `data:image/jpeg;base64,${base64Image}`;
  } catch (error) {
    console.error('Failed to fetch cover image:', error);
  }
};

onMounted(async () => {
  fetchCoverImage();
  try {
    unlistenStatus = await listen('download-status', (event: { payload: any }) => {
      console.log('TextbookItem download-status event received:', event.payload);
      const payload = event.payload;

      if (payload.url === props.textbook.download_url) {
        downloadStatus.value = payload.status;
        downloadProgress.value = payload.progress || 0;

        switch (payload.status) {
          case 'failed':
            downloadError.value = payload.error || '未知错误';
            break;
          case 'completed':
            handleDownloadComplete(payload.filePath);
            break;
          case 'cancelled':
            downloadStatus.value = 'idle';
            downloadProgress.value = 0;
            downloadError.value = '下载已取消';
            break;
        }
      }
    });

    unlistenProgress = await listen('download-progress', (event: { payload: any }) => {
      const payload = event.payload;

      if (payload.url === props.textbook.download_url) {
        downloadProgress.value = Math.min(Math.max(0, payload.progress), 100);
      }
    });
  } catch (error) {
    console.error('Error setting up event listeners:', error);
    downloadStatus.value = 'failed';
    downloadError.value = '无法设置下载监听器';
  }
});

onUnmounted(() => {
  if (unlistenStatus) {
    unlistenStatus();
  }
  if (unlistenProgress) {
    unlistenProgress();
  }
});
</script>

<template>
  <div class="flex items-center p-4 rounded-lg shadow-sm mb-4"
    :style="{ border: '1px solid var(--border-color)', backgroundColor: 'var(--secondary-bg-color)' }">
    <el-image :src="coverImageSrc" :alt="textbook.title" loading="lazy" fit="cover"
      class="w-24 h-auto object-contain mr-6 rounded" :preview-src-list="[coverImageSrc]">
      <template #error>
        <div class="image-slot">
          <el-icon>
            <Picture />
          </el-icon>
        </div>
      </template>
    </el-image>
    <div class="flex-1 flex flex-col justify-between">
      <h3 class="text-lg font-semibold mb-2" :style="{ color: 'var(--text-color)' }">{{ textbook.title }}</h3>
      <div class="text-sm mb-4" :style="{ color: 'var(--text-color)' }">
        <div class="flex items-center justify-center">
          <div class="flex items-center mr-4">
            <el-icon class="mr-1">
              <StarFilled />
            </el-icon>
            <span>{{ textbook.like_count }}</span>
          </div>
          <div class="flex items-center">
            <el-icon class="mr-1">
              <View />
            </el-icon>
            <span>{{ textbook.total_uv }}</span>
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

<style scoped></style>
