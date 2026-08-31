<script lang="ts">
export default { name: 'CourseDownloadPage' };
</script>

<script setup lang="ts">
import { computed, onBeforeUnmount, reactive, ref } from 'vue';
import { ElInput, ElButton, ElMessage, ElMessageBox, ElIcon, ElImage, ElTag } from 'element-plus';
import { Search, Download, VideoPlay, VideoPause, Document, Loading, Check, Close, Refresh, FolderOpened } from '@element-plus/icons-vue';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import {
  enqueueDownload,
  pauseDownload,
  resumeDownload,
  useDownload,
  type DownloadStatus,
} from '@/composables/useDownloadManager';
import { readDownloadSettings } from '@/utils/settings';
import type {
  CatalogBook,
  CatalogLesson,
  CourseCatalog,
  CourseLink,
  CourseParseProgress,
  CourseParseResult,
  CourseResource,
} from '@/types';

const url = ref('');
const parsing = ref(false);
// 已解析出资源的课程。单个链接为 1 项，整册解析时每个课时一项
const courses = ref<CourseParseResult[]>([]);
// 目录页解析结果（整册/整个筛选条件下的课时清单），等用户挑选后再逐个解析
const catalog = ref<CourseCatalog | null>(null);
const selectedLessons = ref<string[]>([]);
const lessonParsing = ref(false);
const parseProgress = reactive({ done: 0, total: 0 });

// 一次入队超过这个数量时先确认，避免误点把上百个任务塞进下载池
const CONFIRM_BATCH_SIZE = 30;
// 封面要走后端代理逐张拉取，整册动辄几百个资源，只给靠前的加载，其余用占位图
const MAX_COVER_PREFETCH = 40;

// 每个资源的下载状态从共享任务池取，key 为其 download_url（与后端取消令牌一致）
const stateOf = (resource: CourseResource) => useDownload(resource.download_url);

const isActiveStatus = (status: DownloadStatus) => status === 'downloading' || status === 'queued';
const isResumableStatus = (status: DownloadStatus) =>
  status === 'paused' || status === 'interrupted' || status === 'failed';

const primaryText = (status: DownloadStatus) => {
  switch (status) {
    case 'downloading': return '下载中';
    case 'queued': return '排队中';
    case 'paused': return '继续';
    case 'interrupted': return '继续';
    case 'completed': return '重新下载';
    case 'failed': return '重试';
    default: return '下载';
  }
};

const allLessons = computed<CatalogLesson[]>(() =>
  catalog.value?.books.flatMap((book) => book.lessons) ?? []
);
const totalResources = computed(() =>
  courses.value.reduce((sum, course) => sum + course.resources.length, 0)
);

// 课时已按章节顺序排好，顺序聚合即可保持平台上的目录结构。
// 章节形如「第一单元 … / 秒的认识」，按顶层单元分组，避免碎成一堆单课时小组
const groupLessons = (book: CatalogBook) => {
  const groups: { chapter: string; lessons: CatalogLesson[] }[] = [];
  for (const lesson of book.lessons) {
    const chapter = lesson.chapter.split(' / ')[0] || '其他';
    const last = groups[groups.length - 1];
    if (last && last.chapter === chapter) {
      last.lessons.push(lesson);
    } else {
      groups.push({ chapter, lessons: [lesson] });
    }
  }
  return groups;
};

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

const prefetchCovers = (list: CourseParseResult[]) => {
  let budget = MAX_COVER_PREFETCH;
  for (const course of list) {
    for (const resource of course.resources) {
      if (budget <= 0) return;
      if (!resource.cover_url) continue;
      loadCover(resource.cover_url);
      budget -= 1;
    }
  }
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

const resetResults = () => {
  courses.value = [];
  catalog.value = null;
  selectedLessons.value = [];
  parseProgress.done = 0;
  parseProgress.total = 0;
};

const handleParse = async () => {
  const trimmed = url.value.trim();
  if (!trimmed) {
    ElMessage.warning('请粘贴课程或视频页面的链接');
    return;
  }
  parsing.value = true;
  resetResults();
  try {
    const link = await invoke<CourseLink>('parse_course_link', { url: trimmed });
    if (link.kind === 'catalog') {
      catalog.value = link;
      // 默认全选：整册下载是这个入口最常见的诉求
      selectedLessons.value = link.books.flatMap((book) => book.lessons.map((l) => l.url));
      ElMessage.success(`已解析出 ${selectedLessons.value.length} 个课时，选好后点「解析选中课时」`);
      return;
    }
    courses.value = [link];
    if (!link.resources.length) {
      ElMessage.info('该链接下没有找到可下载的资源');
    } else {
      prefetchCovers(courses.value);
    }
  } catch (error) {
    ElMessage.error('解析失败: ' + error);
  } finally {
    parsing.value = false;
  }
};

// 目录页第二步：把选中的课时逐个解析成资源清单（后端并发请求并上报进度）
let unlistenProgress: UnlistenFn | null = null;

const parseSelectedLessons = async () => {
  const urls = [...selectedLessons.value];
  if (!urls.length) {
    ElMessage.warning('请先勾选要解析的课时');
    return;
  }

  lessonParsing.value = true;
  parseProgress.done = 0;
  parseProgress.total = urls.length;
  unlistenProgress?.();
  unlistenProgress = await listen<CourseParseProgress>('course-parse-progress', ({ payload }) => {
    parseProgress.done = payload.done;
    parseProgress.total = payload.total;
  });

  try {
    const results = await invoke<CourseParseResult[]>('parse_course_lessons', { urls });
    courses.value = results;
    prefetchCovers(results);
    const skipped = urls.length - results.length;
    ElMessage.success(
      skipped > 0
        ? `解析完成：${results.length} 个课时可下载，${skipped} 个没有公开资源已跳过`
        : `解析完成，共 ${results.length} 个课时`
    );
  } catch (error) {
    ElMessage.error('解析失败: ' + error);
  } finally {
    unlistenProgress?.();
    unlistenProgress = null;
    lessonParsing.value = false;
  }
};

onBeforeUnmount(() => {
  unlistenProgress?.();
  unlistenProgress = null;
});

const selectAllLessons = () => {
  selectedLessons.value = allLessons.value.map((lesson) => lesson.url);
};
const clearSelectedLessons = () => {
  selectedLessons.value = [];
};

// 入队单个资源；排队与并发由全局下载池调度
const enqueueResource = (resource: CourseResource, course: CourseParseResult): boolean => {
  const settings = readDownloadSettings();
  if (!settings.downloadPath) {
    ElMessage.warning('下载路径未设置，请前往设置页面配置');
    return false;
  }

  return enqueueDownload({
    url: resource.download_url,
    kind: resource.is_video ? 'course-video' : 'course-doc',
    title: resource.title,
    subtitle: course.title,
    payload: {
      download_url: resource.download_url,
      title: resource.title,
      format: resource.format,
      is_video: resource.is_video,
      // 同一课时的资源归到以课时名命名的子目录，整册下载时不会混在一起
      course_title: course.title,
      save_by_category: settings.saveByCategory,
      category_path: course.category_path,
    },
  });
};

// 主按钮：暂停/中断/失败走继续（续传），其余（重新）入队
const startDownload = (resource: CourseResource, course: CourseParseResult) => {
  const state = stateOf(resource);
  if (isResumableStatus(state.status)) {
    resumeDownload(resource.download_url);
    return;
  }
  enqueueResource(resource, course);
};

const pauseResource = (resource: CourseResource) => {
  pauseDownload(resource.download_url);
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

// 批量下载：把给定课程里未完成的资源全部入队
const enqueueCourses = (list: CourseParseResult[]) => {
  let queued = 0;
  for (const course of list) {
    for (const resource of course.resources) {
      const state = stateOf(resource);
      if (state.status === 'completed') continue;
      if (isResumableStatus(state.status)) {
        resumeDownload(resource.download_url);
        queued += 1;
      } else if (enqueueResource(resource, course)) {
        queued += 1;
      }
    }
  }

  if (queued > 0) {
    ElMessage.success(`已将 ${queued} 个资源加入下载队列`);
  } else {
    ElMessage.info('资源都已下载完成或在队列中');
  }
};

const handleBatchDownload = async (list: CourseParseResult[]) => {
  const settings = readDownloadSettings();
  if (!settings.downloadPath) {
    ElMessage.warning('下载路径未设置，请前往设置页面配置');
    return;
  }
  const pending = list.reduce((sum, course) => sum + course.resources.length, 0);
  if (!pending) return;

  if (pending > CONFIRM_BATCH_SIZE) {
    try {
      await ElMessageBox.confirm(
        `即将把 ${pending} 个资源加入下载队列，视频较多时会占用较长时间和磁盘空间，确定继续吗？`,
        '批量下载',
        { confirmButtonText: '开始下载', cancelButtonText: '再想想', type: 'warning' }
      );
    } catch {
      return;
    }
  }
  enqueueCourses(list);
};
// 下载完成/失败的汇总提示由全局下载池统一处理
</script>

<template>
  <div class="page-shell">
    <div class="toolbar">
      <div class="page-title">课程 / 视频下载</div>
      <div class="page-desc">
        粘贴国家中小学智慧教育平台的课程页链接即可下载；粘贴「课程教学」里选好学段/学科/版本/年级/册次后的目录页链接，可整册批量下载。视频为加密流，下载后自动解密。
      </div>

      <div class="parse-row">
        <el-input
          v-model="url"
          placeholder="课程页或整册目录页链接，如 https://basic.smartedu.cn/syncClassroom?defaultTag=..."
          clearable
          class="url-input"
          @keyup.enter="handleParse"
        />
        <el-button type="primary" :icon="Search" :loading="parsing" @click="handleParse">
          解析
        </el-button>
        <el-button
          v-if="totalResources > 1"
          :icon="Download"
          @click="handleBatchDownload(courses)"
        >
          全部下载（{{ totalResources }}）
        </el-button>
      </div>
    </div>

    <div class="list-area">
      <!-- 第一步：目录页解析出的课时清单，勾选后再解析资源 -->
      <div v-if="catalog" class="catalog-block app-card">
        <div class="catalog-head">
          <div class="course-title">
            {{ catalog.title }}
            <el-tag size="small" type="info" effect="plain" round>
              {{ allLessons.length }} 个课时
            </el-tag>
            <el-tag
              v-for="seg in catalog.category_path"
              :key="seg"
              size="small"
              type="info"
              effect="plain"
            >
              {{ seg }}
            </el-tag>
          </div>
          <div class="catalog-actions">
            <el-button size="small" plain @click="selectAllLessons">全选</el-button>
            <el-button size="small" plain @click="clearSelectedLessons">清空</el-button>
            <el-button
              type="primary"
              size="small"
              :loading="lessonParsing"
              :disabled="!selectedLessons.length"
              @click="parseSelectedLessons"
            >
              解析选中课时（{{ selectedLessons.length }}）
            </el-button>
          </div>
        </div>

        <div v-if="lessonParsing" class="catalog-progress">
          <el-progress
            :percentage="parseProgress.total ? Math.round((parseProgress.done / parseProgress.total) * 100) : 0"
            :stroke-width="8"
            :show-text="false"
          />
          <div class="progress-meta">
            <span>正在解析课时资源…</span>
            <span>{{ parseProgress.done }} / {{ parseProgress.total }}</span>
          </div>
        </div>

        <div v-for="book in catalog.books" :key="book.id" class="catalog-book">
          <div v-if="catalog.books.length > 1" class="book-title">
            {{ book.title }}
            <el-tag size="small" type="info" effect="plain" round>
              {{ book.lessons.length }} 课时
            </el-tag>
          </div>
          <el-checkbox-group v-model="selectedLessons">
            <div v-for="group in groupLessons(book)" :key="book.id + group.chapter" class="chapter-group">
              <div v-if="group.chapter" class="chapter-title">{{ group.chapter }}</div>
              <div class="lesson-list">
                <el-checkbox
                  v-for="lesson in group.lessons"
                  :key="lesson.url"
                  :value="lesson.url"
                  class="lesson-item"
                >
                  {{ lesson.title }}
                </el-checkbox>
              </div>
            </div>
          </el-checkbox-group>
        </div>
      </div>

      <!-- 第二步：解析出的资源清单（单个链接时直接到这一步） -->
      <div
        v-for="(course, courseIndex) in courses"
        :key="course.title + courseIndex"
        class="course-block"
      >
        <div class="course-title">
          {{ course.title }}
          <el-tag size="small" type="info" effect="plain" round>
            {{ course.resources.length }} 个资源
          </el-tag>
          <el-button
            v-if="courses.length > 1 && course.resources.length > 1"
            size="small"
            plain
            :icon="Download"
            @click="handleBatchDownload([course])"
          >
            下载本课时
          </el-button>
        </div>

        <div class="resource-grid">
          <div v-for="resource in course.resources" :key="resource.id || resource.download_url" class="res-card app-card">
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
                <el-tag size="small" type="info" effect="plain">
                  {{ resource.format.toUpperCase() }}
                </el-tag>
                <el-tag
                  v-for="seg in course.category_path"
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

              <div v-else-if="stateOf(resource).status === 'queued'" class="progress-block">
                <el-progress :percentage="stateOf(resource).progress" :stroke-width="8" :show-text="false" status="warning" />
                <div class="progress-meta">
                  <span>已加入下载队列，等待空闲线程…</span>
                </div>
              </div>

              <div v-else-if="stateOf(resource).status === 'paused' || stateOf(resource).status === 'interrupted'"
                class="progress-block">
                <el-progress :percentage="stateOf(resource).progress" :stroke-width="8" :show-text="false" status="warning" />
                <div class="progress-meta">
                  <span>{{ stateOf(resource).status === 'paused' ? '已暂停，可继续下载' : '上次未完成，可继续下载' }}</span>
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
                  :disabled="isActiveStatus(stateOf(resource).status)"
                  :loading="stateOf(resource).status === 'downloading'"
                  @click="startDownload(resource, course)"
                >
                  <el-icon class="mr-1" v-if="stateOf(resource).status !== 'downloading'">
                    <Refresh v-if="stateOf(resource).status === 'completed'" />
                    <Download v-else />
                  </el-icon>
                  {{ primaryText(stateOf(resource).status) }}
                </el-button>
                <el-button
                  v-if="isActiveStatus(stateOf(resource).status)"
                  size="small"
                  type="warning"
                  plain
                  @click="pauseResource(resource)"
                >
                  <el-icon class="mr-1"><VideoPause /></el-icon>
                  暂停
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
        v-if="!parsing && !catalog && !courses.length"
        description="粘贴链接后点击「解析」；整册目录页会先列出全部课时，勾选后再解析资源"
        :image-size="110"
        class="empty-state"
      />
      <div v-else-if="parsing" class="parsing-state">
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
  flex-wrap: wrap;
  gap: 8px;
  font-size: 15px;
  font-weight: 600;
  color: var(--text-color);
  margin-bottom: 14px;
}

/* 目录（课时清单）区 */
.catalog-block {
  padding: 14px 16px;
  margin-bottom: 18px;
}

.catalog-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
  flex-wrap: wrap;
}

.catalog-head .course-title {
  margin-bottom: 0;
}

.catalog-actions {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-left: auto;
}

.catalog-actions .el-button + .el-button {
  margin-left: 0;
}

.catalog-progress {
  margin-top: 12px;
}

.catalog-book + .catalog-book {
  margin-top: 16px;
  padding-top: 12px;
  border-top: 1px dashed var(--border-color);
}

.book-title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 14px;
  font-weight: 600;
  color: var(--text-color);
  margin: 12px 0 6px;
}

.chapter-group {
  margin-top: 10px;
}

.chapter-title {
  font-size: 12px;
  color: var(--text-muted);
  margin-bottom: 4px;
}

.lesson-list {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
  gap: 2px 12px;
}

.lesson-item {
  height: 26px;
  margin-right: 0;
  overflow: hidden;
}

.lesson-item :deep(.el-checkbox__label) {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 13px;
}

.course-block + .course-block {
  margin-top: 20px;
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
