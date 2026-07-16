<script lang="ts">
export default { name: 'CourseDownloadPage' };
</script>

<script setup lang="ts">
import { ref, reactive } from 'vue';
import { ElInput, ElButton, ElMessage, ElIcon, ElImage, ElTag } from 'element-plus';
import { Search, Download, VideoPlay, Document, Loading, Check, Close, Refresh, FolderOpened } from '@element-plus/icons-vue';
import { invoke } from '@tauri-apps/api/core';
import { useDownload } from '@/composables/useDownloadManager';
import { readDownloadSettings } from '@/utils/settings';
import type { CourseParseResult, CourseResource } from '@/types';

const url = ref('');
const parsing = ref(false);
const result = ref<CourseParseResult | null>(null);

// 每个资源的下载状态从共享仓库取，key 为其 download_url（与后端取消令牌一致）
const stateOf = (resource: CourseResource) => useDownload(resource.download_url);

// 封面走后端代理转 base64（webview 直连外部图片不稳定，同教材封面做法），按原始 URL 缓存。
// 值为 'loading' 表示请求中（显示 loading），'' 表示失败/无封面（显示占位图），data URL 表示成功
const coverCache = reactive(new Map<string, string>());
const loadCover = (coverUrl: string) => {
  if (!coverUrl || coverCache.has(coverUrl)) return;
  coverCache.set(coverUrl, 'loading');
  invoke<string>('fetch_image', { url: coverUrl })
    .then((base64) => coverCache.set(coverUrl, `data:image/jpeg;base64,${base64}`))
    .catch(() => coverCache.set(coverUrl, ''));
};

// 判断缓存值是否为已加载好的图片（区别于 'loading' / '' 占位态）
const isDataUrl = (v: string | undefined) => !!v && v.startsWith('data:');

// 打开下载完成的文件（视频用系统默认播放器播放）
const playResource = (resource: CourseResource) => {
  const path = stateOf(resource).filePath;
  if (!path) {
    ElMessage.warning('未找到已下载的文件');
    return;
  }
  invoke('open_file', { path }).catch((error) => {
    ElMessage.error('打开失败: ' + error);
  });
};

const handleParse = async () => {
  const trimmed = url.value.trim();
  if (!trimmed) {
    ElMessage.warning('请粘贴课程或视频页面的链接');
    return;
  }
  parsing.value = true;
  result.value = null;
  try {
    result.value = await invoke<CourseParseResult>('parse_course_url', { url: trimmed });
    if (!result.value.resources.length) {
      ElMessage.info('该链接下没有找到可下载的资源');
    } else {
      for (const r of result.value.resources) loadCover(r.cover_url);
    }
  } catch (error) {
    ElMessage.error('解析失败: ' + error);
  } finally {
    parsing.value = false;
  }
};

const startDownload = (resource: CourseResource) => {
  const settings = readDownloadSettings();
  if (!settings.downloadPath) {
    ElMessage.warning('下载路径未设置，请前往设置页面配置');
    return;
  }

  const state = stateOf(resource);
  state.status = 'downloading';
  state.progress = 0;
  state.error = '';

  invoke('download_course_resource', {
    resource: {
      download_url: resource.download_url,
      title: resource.title,
      format: resource.format,
      is_video: resource.is_video,
      course_title: result.value?.title ?? null,
      save_by_category: settings.saveByCategory,
      category_path: result.value?.category_path ?? [],
    },
    token: settings.token,
    downloadPath: settings.downloadPath,
    ffmpegPath: settings.ffmpegPath,
  }).catch((error) => {
    ElMessage.error('下载启动失败: ' + error);
  });
};

const cancelDownload = (resource: CourseResource) => {
  invoke('cancel_download', { url: resource.download_url }).catch((error) => {
    console.error('取消下载失败:', error);
  });
};

// 在系统文件管理器中定位已下载的文件
const revealResource = (resource: CourseResource) => {
  const path = stateOf(resource).filePath;
  if (!path) {
    ElMessage.warning('未找到已下载的文件');
    return;
  }
  invoke('reveal_file', { path }).catch((error) => {
    ElMessage.error('打开文件夹失败: ' + error);
  });
};

const handleBatchDownload = () => {
  const settings = readDownloadSettings();
  if (!settings.downloadPath) {
    ElMessage.warning('下载路径未设置，请前往设置页面配置');
    return;
  }
  if (!result.value?.resources.length) return;

  for (const resource of result.value.resources) {
    const state = stateOf(resource);
    if (state.status !== 'completed') {
      state.status = 'downloading';
      state.progress = 0;
      state.error = '';
    }
  }

  invoke('batch_download_course_resources', {
    resources: result.value.resources.map((r) => ({
      download_url: r.download_url,
      title: r.title,
      format: r.format,
      is_video: r.is_video,
      course_title: result.value?.title ?? null,
      save_by_category: settings.saveByCategory,
      category_path: result.value?.category_path ?? [],
    })),
    token: settings.token,
    downloadPath: settings.downloadPath,
    ffmpegPath: settings.ffmpegPath,
    threadCount: settings.threadCount,
  }).catch((error) => {
    ElMessage.error('批量下载启动失败: ' + error);
  });
};
// 批量下载完成/失败提示由 App.vue 全局统一处理，避免 keep-alive 下重复弹窗
</script>

<template>
  <div class="page-shell">
    <div class="toolbar">
      <div class="page-title">课程 / 视频下载</div>
      <div class="page-desc">
        粘贴国家中小学智慧教育平台的课程、视频或课件页面链接，解析后即可下载。视频为加密流，下载后自动解密。
      </div>

      <div class="parse-row">
        <el-input
          v-model="url"
          placeholder="粘贴课程页链接，如 https://basic.smartedu.cn/syncClassroom/classActivity?activityId=..."
          clearable
          class="url-input"
          @keyup.enter="handleParse"
        />
        <el-button type="primary" :icon="Search" :loading="parsing" @click="handleParse">
          解析
        </el-button>
        <el-button
          v-if="result && result.resources.length > 1"
          :icon="Download"
          @click="handleBatchDownload"
        >
          全部下载
        </el-button>
      </div>
    </div>

    <div class="list-area">
      <div v-if="result && result.resources.length" class="course-block">
        <div class="course-title">
          {{ result.title }}
          <el-tag size="small" type="info" effect="plain" round>
            {{ result.resources.length }} 个资源
          </el-tag>
        </div>

        <div class="resource-grid">
          <div v-for="resource in result.resources" :key="resource.id || resource.download_url" class="res-card app-card">
            <div class="res-cover">
              <!-- 封面加载中 -->
              <div v-if="coverCache.get(resource.cover_url) === 'loading'" class="cover-fallback">
                <el-icon class="is-loading cover-spinner"><Loading /></el-icon>
              </div>
              <!-- 封面加载成功 -->
              <template v-else-if="isDataUrl(coverCache.get(resource.cover_url))">
                <el-image
                  :src="coverCache.get(resource.cover_url)"
                  fit="cover"
                  class="cover-img"
                  :class="{ 'cover-playable': resource.is_video && stateOf(resource).status === 'completed' }"
                  @click="resource.is_video && stateOf(resource).status === 'completed' && playResource(resource)"
                />
                <div
                  v-if="resource.is_video && stateOf(resource).status === 'completed'"
                  class="cover-play-overlay"
                  @click="playResource(resource)"
                >
                  <el-icon :size="28"><VideoPlay /></el-icon>
                </div>
              </template>
              <!-- 无封面/加载失败：类型图标占位 -->
              <div
                v-else
                class="cover-fallback"
                :class="{ 'cover-playable': resource.is_video && stateOf(resource).status === 'completed' }"
                @click="resource.is_video && stateOf(resource).status === 'completed' && playResource(resource)"
              >
                <el-icon :size="26">
                  <VideoPlay v-if="resource.is_video" />
                  <Document v-else />
                </el-icon>
              </div>
            </div>

            <div class="res-info">
              <h3 class="res-title" :title="resource.title">{{ resource.title }}</h3>
              <div class="res-meta">
                <el-tag size="small" :type="resource.is_video ? 'success' : 'warning'" effect="plain">
                  {{ resource.is_video ? '视频' : '课件' }}
                </el-tag>
                <span class="res-format">{{ resource.format.toUpperCase() }}</span>
                <el-tag
                  v-for="seg in result.category_path"
                  :key="seg"
                  size="small"
                  type="info"
                  effect="plain"
                >
                  {{ seg }}
                </el-tag>
              </div>

              <div v-if="stateOf(resource).status === 'downloading'" class="progress-block">
                <el-progress
                  :percentage="stateOf(resource).progress"
                  :stroke-width="8"
                  :show-text="false"
                  :status="stateOf(resource).progress === 100 ? 'success' : ''"
                />
                <div class="progress-meta">
                  <span>{{ resource.is_video ? '下载并解密中…' : '下载中…' }}</span>
                  <span>{{ stateOf(resource).progress }}%</span>
                </div>
              </div>

              <div v-else-if="stateOf(resource).status === 'completed'" class="status-line status-success">
                <el-icon :size="14"><Check /></el-icon>
                下载完成
              </div>

              <div
                v-else-if="stateOf(resource).status === 'failed'"
                class="status-line status-error"
                :title="stateOf(resource).error"
              >
                <el-icon :size="14"><Close /></el-icon>
                失败: {{ stateOf(resource).error }}
              </div>

              <div class="res-actions">
                <el-button
                  size="small"
                  :type="stateOf(resource).status === 'completed' ? 'success' : 'primary'"
                  :disabled="stateOf(resource).status === 'downloading'"
                  :loading="stateOf(resource).status === 'downloading'"
                  @click="startDownload(resource)"
                >
                  <el-icon class="mr-1" v-if="stateOf(resource).status !== 'downloading'">
                    <Refresh v-if="stateOf(resource).status === 'completed'" />
                    <Download v-else />
                  </el-icon>
                  {{
                    stateOf(resource).status === 'downloading' ? '下载中' :
                    stateOf(resource).status === 'completed' ? '重新下载' :
                    stateOf(resource).status === 'failed' ? '重试' : '下载'
                  }}
                </el-button>
                <el-button
                  v-if="stateOf(resource).status === 'downloading'"
                  size="small"
                  type="danger"
                  plain
                  @click="cancelDownload(resource)"
                >
                  取消
                </el-button>
                <el-button
                  v-if="stateOf(resource).status === 'completed'"
                  size="small"
                  type="primary"
                  plain
                  @click="playResource(resource)"
                >
                  <el-icon class="mr-1">
                    <VideoPlay v-if="resource.is_video" />
                    <Document v-else />
                  </el-icon>
                  {{ resource.is_video ? '播放' : '打开' }}
                </el-button>
                <el-button
                  v-if="stateOf(resource).status === 'completed'"
                  size="small"
                  plain
                  @click="revealResource(resource)"
                >
                  <el-icon class="mr-1"><FolderOpened /></el-icon>
                  打开文件夹
                </el-button>
              </div>
            </div>
          </div>
        </div>
      </div>

      <el-empty
        v-else-if="!parsing"
        description="粘贴链接后点击「解析」，即可看到该课程下的视频与课件"
        :image-size="110"
        class="empty-state"
      />
      <div v-else class="parsing-state">
        <el-icon class="is-loading" :size="28"><Loading /></el-icon>
        <span>正在解析…</span>
      </div>
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

.parse-row {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-top: 12px;
}

.url-input {
  flex: 1;
}

.list-area {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 16px 20px 24px;
}

.course-title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 15px;
  font-weight: 600;
  color: var(--text-color);
  margin-bottom: 14px;
}

.resource-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(380px, 1fr));
  gap: 14px;
}

.res-card {
  display: flex;
  gap: 16px;
  height: 100%;
  padding: 14px;
  transition: background-color 0.2s, border-color 0.2s, box-shadow 0.2s, transform 0.2s;
}

.res-card:hover {
  border-color: var(--el-color-primary-light-7);
  box-shadow: 0 8px 24px rgba(15, 23, 42, 0.08);
  transform: translateY(-2px);
}

html.dark .res-card:hover {
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.45);
}

.res-cover {
  position: relative;
  width: 96px;
  height: 128px;
  flex-shrink: 0;
}

.res-cover-playable {
  cursor: pointer;
}

/* 已下载视频封面上的播放浮层，悬停显现 */
.cover-play-overlay {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 8px;
  background-color: rgba(0, 0, 0, 0.35);
  color: #fff;
  opacity: 0;
  transition: opacity 0.15s;
}

.res-cover-playable:hover .cover-play-overlay {
  opacity: 1;
}

.cover-img {
  width: 100%;
  height: 100%;
  border-radius: 8px;
  display: block;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.12);
}

.cover-fallback {
  width: 100%;
  height: 100%;
  border-radius: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  background-color: var(--hover-bg);
  border: 1px dashed var(--border-color);
  color: var(--text-muted);
}

.cover-spinner {
  font-size: 1.5rem;
  color: var(--el-color-primary);
}

.res-info {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.res-title {
  font-size: 15px;
  font-weight: 600;
  color: var(--text-color);
  margin: 2px 0 0;
  line-height: 1.45;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.res-meta {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 8px;
  row-gap: 4px;
}

.res-format {
  font-size: 11px;
  color: var(--text-muted);
}

.progress-block {
  margin-top: 2px;
}

.progress-meta {
  display: flex;
  justify-content: space-between;
  font-size: 11px;
  color: var(--text-muted);
  margin-top: 3px;
}

.status-line {
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: 12px;
}

.status-success {
  color: var(--el-color-success);
}

.status-error {
  color: var(--el-color-danger);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.res-actions {
  display: flex;
  flex-wrap: wrap;
  justify-content: flex-end;
  gap: 8px;
  margin-top: auto;
  padding-top: 12px;
}

.res-actions .el-button + .el-button {
  margin-left: 0;
}

.empty-state {
  margin-top: 8vh;
}

.parsing-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 10px;
  margin-top: 12vh;
  color: var(--text-muted);
}
</style>
