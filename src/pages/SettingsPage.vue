<template>
  <div class="max-h-[calc(100vh-100px)] text-left" :style="{ backgroundColor: 'var(--bg-color)', color: 'var(--text-color)' }">
    <div class="text-3xl font-bold mb-6">设置</div>

    <div class="p-6 rounded-lg shadow mb-6">
      <div class="text-xl font-semibold mb-4">下载设置</div>
      <el-form label-position="left" label-width="120px">
        <el-form-item label="API Token">
          <el-input
              v-model="apiToken"
              placeholder="请输入 API Token"
          ></el-input>
        </el-form-item>
        <el-form-item label="下载路径">
          <div class="flex items-center w-full space-x-2">
            <el-input
                v-model="downloadPath"
                placeholder="请选择下载路径"
                :readonly="true"
                class="flex-1"
            ></el-input>
            <el-button @click="selectDownloadPath">选择目录</el-button>
          </div>
        </el-form-item>
        <el-form-item label="多线程下载数">
          <el-slider v-model="threadCount" show-input :min="1" :max="16"/>
        </el-form-item>
        <el-form-item label="按分类保存">
          <el-switch v-model="saveByCategory" />
        </el-form-item>
      </el-form>
    </div>
    <div class="p-6 rounded-lg shadow mb-6">
      <div class="text-xl font-semibold mb-4">主题设置</div>
      <el-form label-position="left" label-width="120px">
        <el-form-item label="主题模式">
          <el-switch
              :model-value="isDarkMode"
              active-text="暗色模式"
              inactive-text="亮色模式"
              @change="toggleTheme"
          />
        </el-form-item>
      </el-form>
    </div>

    <div class="mt-6 flex justify-center">
      <el-button type="primary" @click="saveSettings">保存设置</el-button>
    </div>
  </div>
</template>

<script setup lang="ts">
import {ref, onMounted, inject, type Ref} from 'vue';
import {ElInput, ElButton, ElMessage, ElForm, ElFormItem, ElSwitch} from 'element-plus';
import {open} from '@tauri-apps/plugin-dialog';

const isDarkMode = inject('isDarkMode') as Ref<boolean>;
const toggleTheme = inject('toggleTheme') as () => void;

const apiToken = ref('');
const downloadPath = ref('');
const threadCount = ref(4);
const saveByCategory = ref(false);

const LOCAL_STORAGE_TOKEN_KEY = 'api_token';
const LOCAL_STORAGE_DOWNLOAD_PATH_KEY = 'download_path';
const LOCAL_STORAGE_THREAD_COUNT_KEY = 'thread_count';
const LOCAL_STORAGE_SAVE_BY_CATEGORY_KEY = 'save_by_category';

onMounted(() => {
  const savedToken = localStorage.getItem(LOCAL_STORAGE_TOKEN_KEY);
  if (savedToken) {
    apiToken.value = savedToken;
  }

  const savedDownloadPath = localStorage.getItem(LOCAL_STORAGE_DOWNLOAD_PATH_KEY);
  if (savedDownloadPath) {
    downloadPath.value = savedDownloadPath;
  }

  const savedThreadCount = localStorage.getItem(LOCAL_STORAGE_THREAD_COUNT_KEY);
  if (savedThreadCount) {
    threadCount.value = parseInt(savedThreadCount, 10);
  }

  const savedSaveByCategory = localStorage.getItem(LOCAL_STORAGE_SAVE_BY_CATEGORY_KEY);
  if (savedSaveByCategory !== null) {
    saveByCategory.value = savedSaveByCategory === 'true';
  }
});

const selectDownloadPath = async () => {
  const selected = await open({
    directory: true,
    multiple: false,
    defaultPath: downloadPath.value || undefined,
  });

  if (selected !== null && typeof selected === 'string') {
    downloadPath.value = selected;
  }
};

const saveSettings = () => {
  localStorage.setItem(LOCAL_STORAGE_TOKEN_KEY, apiToken.value);
  localStorage.setItem(LOCAL_STORAGE_THREAD_COUNT_KEY, threadCount.value.toString());
  localStorage.setItem(LOCAL_STORAGE_SAVE_BY_CATEGORY_KEY, saveByCategory.value.toString());

  if (downloadPath.value) {
    localStorage.setItem(LOCAL_STORAGE_DOWNLOAD_PATH_KEY, downloadPath.value);
    ElMessage.success('设置已保存');
  } else {
    ElMessage.warning('请先选择下载路径');
  }
};
</script>

<style scoped>
</style>
