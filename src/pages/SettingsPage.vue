<template>
  <div class="max-h-[calc(100vh-100px)] text-left"
    :style="{ backgroundColor: 'var(--bg-color)', color: 'var(--text-color)' }">
    <div class="text-3xl font-bold mb-6">设置</div>

    <div class="p-6 rounded-lg shadow mb-6">
      <div class="text-xl font-semibold mb-4">下载设置</div>
      <el-form label-position="left" label-width="120px">
        <el-form-item label="Access Token">
          <div class="w-full">
            <el-input v-model="apiToken" placeholder="请输入登录凭据 Access Token（下载教材必需）" show-password
              clearable></el-input>
            <div class="mt-1 text-xs" style="color: var(--text-color); opacity: 0.6;">
              平台已要求登录后才能下载教材，令牌仅保存在本机。
              <el-button link type="primary" size="small" @click="showTokenHelp = !showTokenHelp">
                {{ showTokenHelp ? '收起' : '如何获取？' }}
              </el-button>
            </div>
            <div v-if="showTokenHelp" class="mt-2 p-3 rounded border border-gray-300 text-xs leading-relaxed w-full">
              <ol class="list-decimal list-inside space-y-1">
                <li>浏览器访问 https://basic.smartedu.cn 并登录（没有账号需先注册）</li>
                <li>按 F12（或右键 → 检查）打开开发者工具，切换到「控制台 / Console」</li>
                <li>粘贴以下代码并回车，复制输出的 Access Token 填入上方输入框</li>
              </ol>
              <pre class="mt-2 p-2 rounded overflow-x-auto select-text"
                style="background-color: rgba(127,127,127,0.15);">{{ tokenScript }}</pre>
              <div class="mt-2 flex items-center space-x-2">
                <el-button size="small" @click="copyTokenScript">复制代码</el-button>
                <span style="opacity: 0.6;">提示：令牌约一周过期，下载提示 Token 失效时请重新获取。</span>
              </div>
            </div>
          </div>
        </el-form-item>
        <el-form-item label="下载路径">
          <div class="flex items-center w-full space-x-2">
            <el-input v-model="downloadPath" placeholder="请选择下载路径" :readonly="true" class="flex-1"></el-input>
            <el-button @click="selectDownloadPath">选择目录</el-button>
          </div>
        </el-form-item>
        <el-form-item label="多线程下载数">
          <el-slider v-model="threadCount" show-input :min="1" :max="16" />
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
          <el-switch :model-value="isDarkMode" active-text="暗色模式" inactive-text="亮色模式" @change="toggleTheme" />
        </el-form-item>
      </el-form>
    </div>

    <div class="p-6 rounded-lg shadow mb-6">
      <div class="text-xl font-semibold mb-4">系统设置</div>
      <el-form label-position="left" label-width="120px">
        <el-form-item label="缓存管理">
          <div class="flex items-center space-x-2">
            <el-button type="warning" @click="clearCache" :loading="clearingCache">
              清理标签缓存
            </el-button>
            <span class="text-sm text-gray-500">清理教材标签缓存，下次请求时将重新获取</span>
          </div>
        </el-form-item>
      </el-form>
    </div>

    <div class="mt-6 flex justify-center">
      <el-button type="primary" @click="saveSettings">保存设置</el-button>
    </div>
    <div class="pt-4"></div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, inject, type Ref } from 'vue';
import { ElInput, ElButton, ElMessage, ElForm, ElFormItem, ElSwitch, ElSlider, ElMessageBox } from 'element-plus';
import { open } from '@tauri-apps/plugin-dialog';
import { invoke } from '@tauri-apps/api/core';

const isDarkMode = inject('isDarkMode') as Ref<boolean>;
const toggleTheme = inject('toggleTheme') as () => void;

const apiToken = ref('');
const downloadPath = ref('');
const threadCount = ref(4);
const saveByCategory = ref(false);
const clearingCache = ref(false);
const showTokenHelp = ref(false);

// Reads the login token from smartedu.cn localStorage (the "...&token" key holds
// a JSON string whose access_token field is what the download API needs).
const tokenScript = `(function () {
  const key = Object.keys(localStorage).find(
    (k) => k.startsWith("ND_UC_AUTH") && k.endsWith("&token")
  );
  if (!key) { console.error("未找到 Access Token，请确认已登录！"); return; }
  const token = JSON.parse(JSON.parse(localStorage.getItem(key)).value).access_token;
  console.log("%cAccess Token:", "color: green; font-weight: bold", token);
})();`;

const copyTokenScript = async () => {
  try {
    await navigator.clipboard.writeText(tokenScript);
    ElMessage.success('代码已复制，请粘贴到平台页面的浏览器控制台');
  } catch {
    ElMessage.error('复制失败，请手动选择代码复制');
  }
};

const LOCAL_STORAGE_TOKEN_KEY = 'api_token';
const LOCAL_STORAGE_DOWNLOAD_PATH_KEY = 'download_path';
const LOCAL_STORAGE_THREAD_COUNT_KEY = 'thread_count';
const LOCAL_STORAGE_SAVE_BY_CATEGORY_KEY = 'save_by_category';

onMounted(() => {
  apiToken.value = localStorage.getItem(LOCAL_STORAGE_TOKEN_KEY) || '';
  downloadPath.value = localStorage.getItem(LOCAL_STORAGE_DOWNLOAD_PATH_KEY) || '';
  threadCount.value = parseInt(localStorage.getItem(LOCAL_STORAGE_THREAD_COUNT_KEY) || '4', 10);
  saveByCategory.value = localStorage.getItem(LOCAL_STORAGE_SAVE_BY_CATEGORY_KEY) === 'true';
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

const clearCache = async () => {
  try {
    await ElMessageBox.confirm(
      '清理缓存后，下次获取教材标签数据时将重新从服务器加载。确定要清理缓存吗？',
      '确认清理缓存',
      {
        confirmButtonText: '确定',
        cancelButtonText: '取消',
        type: 'warning',
      }
    );

    clearingCache.value = true;
    await invoke('clear_tch_material_tag_cache');
    ElMessage.success('标签缓存已清理');
  } catch (error) {
    if (error !== 'cancel') {
      console.error('清理缓存失败:', error);
      ElMessage.error('清理缓存失败: ' + error);
    }
  } finally {
    clearingCache.value = false;
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

<style scoped></style>
