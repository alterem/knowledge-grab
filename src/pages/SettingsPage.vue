<template>
  <div class="page-shell">
    <div class="page-header">
      <div class="page-title">设置</div>
      <div class="page-desc">配置下载参数与应用偏好，修改后请点击底部保存</div>
    </div>

    <div class="page-body settings-body">

      <section class="settings-card">
        <div class="card-title">账号凭据</div>
        <el-form label-position="left" label-width="120px">
          <el-form-item label="Access Token">
            <div class="w-full">
              <div class="flex items-center w-full space-x-2">
                <el-input v-model="apiToken" placeholder="请输入登录凭据 Access Token（下载教材必需）" show-password clearable
                  class="flex-1"></el-input>
                <el-button type="primary" @click="loginAndFetchToken">登录平台自动获取</el-button>
              </div>
              <div class="mt-1 text-xs form-hint">
                平台已要求登录后才能下载教材，令牌仅保存在本机。点击上方按钮登录官方页面自动获取，或
                <el-button link type="primary" size="small" @click="showTokenHelp = !showTokenHelp">
                  {{ showTokenHelp ? '收起手动获取步骤' : '查看手动获取步骤' }}
                </el-button>
              </div>
              <div v-if="showTokenHelp" class="token-help">
                <ol class="token-steps">
                  <li>浏览器访问 <code>https://basic.smartedu.cn</code> 并登录（没有账号需先注册）</li>
                  <li>按 <kbd>F12</kbd>（或右键 → 检查）打开开发者工具，切换到「控制台 / Console」</li>
                  <li>粘贴以下代码并回车，复制输出的 Access Token 填入上方输入框</li>
                </ol>
                <pre class="token-script select-text">{{ tokenScript }}</pre>
                <div class="token-help-footer">
                  <el-button size="small" @click="copyTokenScript">
                    <el-icon class="mr-1">
                      <CopyDocument />
                    </el-icon>
                    复制代码
                  </el-button>
                  <span class="form-hint">令牌约一周过期，下载提示 Token 失效时请重新获取</span>
                </div>
              </div>
            </div>
          </el-form-item>
          <el-form-item label="MAC Key">
            <div class="w-full">
              <div class="flex items-center w-full space-x-2">
                <el-input v-model="macKey" placeholder="可选：部分需鉴权的课件下载才用到，「登录平台自动获取」会一并抓取"
                  show-password clearable class="flex-1"></el-input>
              </div>
              <div class="mt-1 text-xs form-hint">
                与 Access Token 同源，仅少数需要二次鉴权的课件下载会用到。上方「登录平台自动获取」会连同令牌一起抓取，或
                <el-button link type="primary" size="small" @click="showMacKeyHelp = !showMacKeyHelp">
                  {{ showMacKeyHelp ? '收起手动获取步骤' : '查看手动获取步骤' }}
                </el-button>
              </div>
              <div v-if="showMacKeyHelp" class="token-help">
                <ol class="token-steps">
                  <li>浏览器访问 <code>https://basic.smartedu.cn</code> 并登录（没有账号需先注册）</li>
                  <li>按 <kbd>F12</kbd>（或右键 → 检查）打开开发者工具，切换到「控制台 / Console」</li>
                  <li>粘贴以下代码并回车，复制输出的 MAC Key 填入上方输入框</li>
                </ol>
                <pre class="token-script select-text">{{ tokenScript }}</pre>
                <div class="token-help-footer">
                  <el-button size="small" @click="copyTokenScript">
                    <el-icon class="mr-1">
                      <CopyDocument />
                    </el-icon>
                    复制代码
                  </el-button>
                  <span class="form-hint">与令牌同步过期，失效后重新获取即可</span>
                </div>
              </div>
            </div>
          </el-form-item>
        </el-form>
      </section>

      <section class="settings-card">
        <div class="card-title">下载设置</div>
        <el-form label-position="left" label-width="120px">
          <el-form-item label="下载路径">
            <div class="flex items-center w-full space-x-2">
              <el-input v-model="downloadPath" placeholder="请选择下载路径" :readonly="true" class="flex-1"></el-input>
              <el-button @click="selectDownloadPath">选择目录</el-button>
            </div>
          </el-form-item>
          <el-form-item label="多线程下载数">
            <el-slider v-model="threadCount" show-input :min="1" :max="16" class="thread-slider" />
          </el-form-item>
          <el-form-item label="按分类保存">
            <el-switch v-model="saveByCategory" />
          </el-form-item>
          <el-form-item label="ffmpeg 路径">
            <div class="w-full">
              <div class="flex items-center w-full space-x-2">
                <el-input v-model="ffmpegPath" placeholder="可选：配置后视频将合成为 .mp4，留空则保存为 .ts"
                  clearable class="flex-1"></el-input>
                <el-button @click="selectFfmpegPath">选择文件</el-button>
                <el-button @click="checkFfmpeg" :loading="checkingFfmpeg">检测</el-button>
              </div>
              <div class="mt-1 text-xs form-hint">
                下载的课程视频是加密的 TS 切片，本工具会自动解密。配置 ffmpeg 后可无损合成为标准 .mp4；
                未配置时保存为可直接播放的 .ts 文件。
                <el-button link type="primary" size="small" @click="showFfmpegHelp = !showFfmpegHelp">
                  {{ showFfmpegHelp ? '收起下载指引' : '如何获取 ffmpeg？' }}
                </el-button>
              </div>
              <div v-if="showFfmpegHelp" class="token-help">
                <ol class="token-steps">
                  <li>
                    访问 ffmpeg 官网下载页
                    <el-button link type="primary" size="small" @click="openFfmpegSite">
                      ffmpeg.org/download
                    </el-button>
                    ，选择对应系统的构建版本
                  </li>
                  <li>macOS 可用 <code>brew install ffmpeg</code>，Windows 下载后解压得到 <code>ffmpeg.exe</code></li>
                  <li>点击上方「选择文件」定位到 ffmpeg 可执行文件，或直接填入其完整路径</li>
                  <li>点击「检测」确认可用后保存</li>
                </ol>
              </div>
            </div>
          </el-form-item>
        </el-form>
      </section>

      <section class="settings-card">
        <div class="card-title">主题设置</div>
        <el-form label-position="left" label-width="120px">
          <el-form-item label="主题模式">
            <el-switch :model-value="isDarkMode" active-text="暗色模式" inactive-text="亮色模式" @change="toggleTheme" />
          </el-form-item>
        </el-form>
      </section>

      <section class="settings-card">
        <div class="card-title">系统设置</div>
        <el-form label-position="left" label-width="120px">
          <el-form-item label="缓存管理">
            <div class="flex items-center space-x-2">
              <el-button type="warning" plain @click="clearCache" :loading="clearingCache">
                清理数据缓存
              </el-button>
              <span class="text-sm form-hint">清理教材标签与书目缓存，下次请求时将重新获取</span>
            </div>
          </el-form-item>
        </el-form>
      </section>

      <section class="settings-card">
        <div class="card-title">关于与更新</div>
        <el-form label-position="left" label-width="120px">
          <el-form-item label="当前版本">
            <div class="flex items-center space-x-2">
              <el-tag effect="plain">v{{ appVersion || '…' }}</el-tag>
              <el-button type="primary" plain @click="handleCheckUpdate"
                :loading="updater.status === 'checking'" :disabled="updater.status === 'downloading' || updater.status === 'installing'">
                检查更新
              </el-button>
              <el-button link type="primary" @click="openReleases">前往发布页</el-button>
            </div>
          </el-form-item>
          <el-form-item v-if="updater.status === 'downloading' || updater.status === 'installing'" label="更新进度">
            <div class="w-full">
              <el-progress :percentage="updateProgress" :indeterminate="!updater.total" :stroke-width="8" />
              <div class="mt-1 text-xs form-hint">
                {{
                  updater.status === 'installing'
                    ? '下载完成，正在安装，应用稍后将自动重启…'
                    : `正在下载更新 ${formatBytes(updater.downloaded)}${updater.total ? ' / ' + formatBytes(updater.total) : ''}`
                }}
              </div>
            </div>
          </el-form-item>
          <el-form-item label="自动检查更新">
            <div class="flex items-center space-x-2">
              <el-switch v-model="autoCheckUpdate" @change="onAutoCheckChange" />
              <span class="text-sm form-hint">启动时静默检查新版本，发现更新时提醒（不会自动安装）</span>
            </div>
          </el-form-item>
          <el-form-item label="自定义更新源">
            <div class="w-full">
              <el-input v-model="updateEndpoint" clearable
                placeholder="可选：镜像的 latest.json 完整地址，留空使用 GitHub 官方源" />
              <div class="mt-1 text-xs form-hint">
                GitHub 访问受限时可填加速镜像地址（修改后点击底部保存），例如：
              </div>
              <pre class="token-script select-text whitespace-nowrap">{{ updateEndpointExample }}</pre>
              <div class="token-help-footer">
                <el-button size="small" @click="copyUpdateEndpointExample">
                  <el-icon class="mr-1">
                    <CopyDocument />
                  </el-icon>
                  复制地址
                </el-button>
              </div>
            </div>
          </el-form-item>
        </el-form>
      </section>
    </div>

    <div class="settings-footer">
      <span class="text-sm form-hint">修改设置后请点击保存</span>
      <el-button type="primary" @click="saveSettings">
        <el-icon class="mr-1">
          <Check />
        </el-icon>
        保存设置
      </el-button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, inject, type Ref } from 'vue';
import { ElInput, ElButton, ElMessage, ElForm, ElFormItem, ElSwitch, ElSlider, ElMessageBox, ElIcon, ElTag, ElProgress } from 'element-plus';
import { Check, CopyDocument } from '@element-plus/icons-vue';
import { open } from '@tauri-apps/plugin-dialog';
import { invoke } from '@tauri-apps/api/core';
import { getVersion } from '@tauri-apps/api/app';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { STORAGE_KEYS } from '@/utils/settings';
import { formatBytes } from '@/utils/format';
import {
  RELEASES_URL,
  checkForUpdates,
  isAutoCheckEnabled,
  setAutoCheckEnabled,
  useUpdaterState,
} from '@/composables/useUpdater';

const isDarkMode = inject('isDarkMode') as Ref<boolean>;
const toggleTheme = inject('toggleTheme') as () => void;

const apiToken = ref('');
const macKey = ref('');
const downloadPath = ref('');
const threadCount = ref(4);
const saveByCategory = ref(false);
const clearingCache = ref(false);
const showTokenHelp = ref(false);
const showMacKeyHelp = ref(false);
const ffmpegPath = ref('');
const showFfmpegHelp = ref(false);
const checkingFfmpeg = ref(false);

// 关于与更新
const appVersion = ref('');
const updater = useUpdaterState();
const autoCheckUpdate = ref(isAutoCheckEnabled());
const updateEndpoint = ref('');
const updateEndpointExample = 'https://gh-proxy.com/https://github.com/alterem/knowledge-grab/releases/latest/download/latest.json';

const copyUpdateEndpointExample = async () => {
  try {
    await navigator.clipboard.writeText(updateEndpointExample);
    ElMessage.success('地址已复制');
  } catch {
    ElMessage.error('复制失败，请手动选择地址复制');
  }
};
const updateProgress = computed(() =>
  updater.total ? Math.min(100, Math.round((updater.downloaded / updater.total) * 100)) : 0
);

const handleCheckUpdate = () => {
  void checkForUpdates();
};

const onAutoCheckChange = (value: string | number | boolean) => {
  setAutoCheckEnabled(Boolean(value));
};

const openReleases = () => {
  invoke('open_url', { url: RELEASES_URL }).catch((err) => console.error('打开发布页失败:', err));
};

// 从 smartedu.cn 的 localStorage 读取登录凭据（"...&token" 键的 value 里含 access_token 与 mac_key）。
// access_token 用于所有下载鉴权；mac_key 供视频课件的 doc-center 签名下载使用。
const tokenScript = `(function () {
  const key = Object.keys(localStorage).find(
    (k) => k.startsWith("ND_UC_AUTH") && k.endsWith("&token")
  );
  if (!key) { console.error("未找到登录凭据，请确认已登录！"); return; }
  const value = JSON.parse(JSON.parse(localStorage.getItem(key)).value);
  console.log("%cAccess Token:", "color: green; font-weight: bold", value.access_token);
  console.log("%cMAC Key:", "color: green; font-weight: bold", value.mac_key || "(无)");
})();`;

const copyTokenScript = async () => {
  try {
    await navigator.clipboard.writeText(tokenScript);
    ElMessage.success('代码已复制，请粘贴到平台页面的浏览器控制台');
  } catch {
    ElMessage.error('复制失败，请手动选择代码复制');
  }
};

const loginAndFetchToken = async () => {
  try {
    // 传入当前主题，登录窗口的标题栏与加载背景色随应用主题
    await invoke('open_login_window', { isDark: isDarkMode.value });
    ElMessage.info('请在弹出的窗口中登录，成功后将自动获取 Access Token');
  } catch (error) {
    console.error('打开登录窗口失败:', error);
    ElMessage.error('打开登录窗口失败: ' + error);
  }
};

let unlistenTokenCaptured: UnlistenFn | null = null;

onMounted(async () => {
  apiToken.value = localStorage.getItem(STORAGE_KEYS.token) || '';
  macKey.value = localStorage.getItem(STORAGE_KEYS.macKey) || '';
  downloadPath.value = localStorage.getItem(STORAGE_KEYS.downloadPath) || '';
  threadCount.value = parseInt(localStorage.getItem(STORAGE_KEYS.threadCount) || '4', 10);
  saveByCategory.value = localStorage.getItem(STORAGE_KEYS.saveByCategory) === 'true';
  ffmpegPath.value = localStorage.getItem(STORAGE_KEYS.ffmpegPath) || '';
  updateEndpoint.value = localStorage.getItem(STORAGE_KEYS.updateEndpoint) || '';

  getVersion()
    .then((version) => (appVersion.value = version))
    .catch((err) => console.error('获取应用版本失败:', err));

  // 令牌的持久化由 App.vue 全局处理，这里只在设置页打开时同步输入框显示
  unlistenTokenCaptured = await listen<{ token: string; mac_key?: string }>(
    'access-token-captured',
    (event) => {
      const token = event.payload?.token;
      if (token) apiToken.value = token;
      if (event.payload?.mac_key) macKey.value = event.payload.mac_key;
    }
  );
});

onUnmounted(() => {
  unlistenTokenCaptured?.();
  unlistenTokenCaptured = null;
});

const selectDownloadPath = async () => {
  try {
    const selected = await open({
      directory: true,
      multiple: false,
      defaultPath: downloadPath.value || undefined,
    });

    if (selected !== null && typeof selected === 'string') {
      downloadPath.value = selected;
    }
  } catch (error) {
    // Windows 上系统对话框偶发失败：给出提示并留手动填写的活路，不再无声无息
    console.error('打开目录选择对话框失败:', error);
    ElMessage.error('打开目录选择窗口失败，请重试一次；若仍失败可重启应用后再试');
  }
};

const clearCache = async () => {
  try {
    await ElMessageBox.confirm(
      '清理缓存后，下次获取教材数据时将重新从服务器加载。确定要清理缓存吗？',
      '确认清理缓存',
      {
        confirmButtonText: '确定',
        cancelButtonText: '取消',
        type: 'warning',
      }
    );

    clearingCache.value = true;
    await invoke('clear_tch_material_tag_cache');
    ElMessage.success('数据缓存已清理');
  } catch (error) {
    if (error !== 'cancel') {
      console.error('清理缓存失败:', error);
      ElMessage.error('清理缓存失败: ' + error);
    }
  } finally {
    clearingCache.value = false;
  }
};

const selectFfmpegPath = async () => {
  try {
    const selected = await open({
      directory: false,
      multiple: false,
      defaultPath: ffmpegPath.value || undefined,
    });
    if (selected !== null && typeof selected === 'string') {
      ffmpegPath.value = selected;
    }
  } catch (error) {
    console.error('打开文件选择对话框失败:', error);
    ElMessage.error('打开文件选择窗口失败，可直接在输入框粘贴 ffmpeg 完整路径');
  }
};

const checkFfmpeg = async () => {
  if (!ffmpegPath.value) {
    ElMessage.warning('请先填写或选择 ffmpeg 路径');
    return;
  }
  checkingFfmpeg.value = true;
  try {
    const ok = await invoke<boolean>('check_ffmpeg', { path: ffmpegPath.value });
    if (ok) {
      ElMessage.success('ffmpeg 可用，视频将自动转封装为 MP4');
    } else {
      ElMessage.error('无法运行该路径的 ffmpeg，请检查是否填写正确');
    }
  } catch (error) {
    ElMessage.error('检测失败: ' + error);
  } finally {
    checkingFfmpeg.value = false;
  }
};

const openFfmpegSite = () => {
  invoke('open_url', { url: 'https://ffmpeg.org/download.html' }).catch((err) =>
    console.error('打开链接失败:', err)
  );
};

const saveSettings = () => {
  localStorage.setItem(STORAGE_KEYS.token, apiToken.value);
  localStorage.setItem(STORAGE_KEYS.macKey, macKey.value.trim());
  localStorage.setItem(STORAGE_KEYS.threadCount, threadCount.value.toString());
  localStorage.setItem(STORAGE_KEYS.saveByCategory, saveByCategory.value.toString());
  localStorage.setItem(STORAGE_KEYS.ffmpegPath, ffmpegPath.value.trim());
  localStorage.setItem(STORAGE_KEYS.updateEndpoint, updateEndpoint.value.trim());

  if (downloadPath.value) {
    localStorage.setItem(STORAGE_KEYS.downloadPath, downloadPath.value);
    ElMessage.success('设置已保存');
  } else {
    ElMessage.warning('请先选择下载路径');
  }
};
</script>

<style scoped>
/* 页面骨架由全局 .page-shell / .page-header / .page-body 提供：
   页头与底栏固定，仅 .settings-body 滚动，避免双滚动条。 */
.settings-body {
  padding: 20px 24px 8px;
}

.settings-card {
  background-color: var(--secondary-bg-color);
  border: 1px solid var(--border-color);
  border-radius: 12px;
  padding: 20px 24px 8px;
  margin-bottom: 16px;
  transition: background-color 0.2s, border-color 0.2s;
}

.card-title {
  font-size: 16px;
  font-weight: 600;
  margin-bottom: 16px;
  color: var(--text-color);
}

.form-hint {
  color: var(--text-muted);
}

/* Token 手动获取指引：内嵌浅一层的面板，步骤序号用主色 */
.token-help {
  margin-top: 10px;
  padding: 14px 16px;
  width: 100%;
  border: 1px solid var(--border-color);
  background-color: var(--bg-color);
  border-radius: 10px;
  font-size: 12px;
}

.token-steps {
  margin: 0;
  padding-left: 20px;
  list-style: decimal;
  line-height: 2;
  color: var(--text-color);
}

.token-steps li::marker {
  color: var(--el-color-primary);
  font-weight: 600;
}

.token-steps code {
  padding: 1px 6px;
  border: 1px solid var(--border-color);
  border-radius: 4px;
  background-color: var(--secondary-bg-color);
  color: var(--el-color-primary);
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 11px;
}

.token-steps kbd {
  display: inline-block;
  min-width: 20px;
  padding: 0 6px;
  border: 1px solid var(--border-color);
  border-bottom-width: 2px;
  border-radius: 5px;
  background-color: var(--secondary-bg-color);
  text-align: center;
  font-size: 11px;
  line-height: 18px;
}

/* 代码块跟随主题：亮色浅底深字，暗色深底浅字 */
.token-script {
  margin: 10px 0 0;
  padding: 12px 14px;
  border: 1px solid var(--border-color);
  border-radius: 8px;
  overflow-x: auto;
  background-color: var(--hover-bg);
  color: var(--text-color);
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 11px;
  line-height: 1.7;
}

.token-help-footer {
  margin-top: 10px;
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 10px;
}

.thread-slider {
  max-width: 480px;
}

.settings-footer {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 12px;
  padding: 10px 24px;
  background-color: var(--secondary-bg-color);
  border-top: 1px solid var(--border-color);
  transition: background-color 0.2s, border-color 0.2s;
}
</style>
