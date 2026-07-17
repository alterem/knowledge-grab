<script lang="ts">
export default { name: 'WelcomePage' };
</script>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue';
import { ElButton, ElIcon } from 'element-plus';
import {
  Reading, VideoCamera,
  CircleCheckFilled, WarningFilled, ArrowRight, FolderChecked, Key, VideoPlay,
} from '@element-plus/icons-vue';
import { useRouter } from 'vue-router';
import { readDownloadSettings } from '@/utils/settings';

const router = useRouter();
const go = (path: string) => router.push(path);

// 读取当前配置，用于首页状态提示（每次进入首页刷新一次）
const settings = ref(readDownloadSettings());
onMounted(() => {
  settings.value = readDownloadSettings();
});

const hasDownloadPath = computed(() => !!settings.value.downloadPath);
const hasToken = computed(() => !!settings.value.token);
const hasFfmpeg = computed(() => !!settings.value.ffmpegPath);

// 三项配置状态，未就绪的排在前面提示用户
const configItems = computed(() => [
  {
    key: 'path',
    icon: FolderChecked,
    label: '下载路径',
    ok: hasDownloadPath.value,
    okText: settings.value.downloadPath || '',
    hint: '尚未设置，下载前需先选择保存目录',
  },
  {
    key: 'token',
    icon: Key,
    label: '登录令牌',
    ok: hasToken.value,
    okText: '已配置',
    hint: '未配置，下载教材/视频需要登录凭据',
  },
  {
    key: 'ffmpeg',
    icon: VideoPlay,
    label: 'ffmpeg',
    ok: hasFfmpeg.value,
    okText: '已配置，视频将合成 MP4',
    hint: '可选，未配置时视频保存为 .ts',
  },
]);

const allReady = computed(() => hasDownloadPath.value && hasToken.value);

const entries = [
  {
    icon: Reading,
    title: '课本下载',
    desc: '按学段、学科、版本、年级筛选电子教材，单本或批量下载 PDF。',
    path: '/textbook-download',
  },
  {
    icon: VideoCamera,
    title: '课程 / 视频下载',
    desc: '粘贴课程页链接解析视频与课件，加密视频下载后自动解密。',
    path: '/course-download',
  },
];
</script>

<template>
  <div class="page-shell">
    <div class="page-header">
      <div class="page-title">首页</div>
      <div class="page-desc">国家中小学智慧教育平台资源下载工具</div>
    </div>

    <div class="page-body welcome-body">
      <div class="welcome-inner">
        <!-- 配置状态条 -->
        <section class="config-strip app-card" :class="{ 'is-ready': allReady }">
          <div class="config-lead">
            <el-icon :size="18" :class="allReady ? 'lead-ok' : 'lead-warn'">
              <CircleCheckFilled v-if="allReady" />
              <WarningFilled v-else />
            </el-icon>
            <span>{{ allReady ? '配置就绪，可以开始下载' : '完善配置后开始使用' }}</span>
            <el-button class="config-action" text type="primary" @click="go('/settings')">
              前往设置
              <el-icon class="ml-1"><ArrowRight /></el-icon>
            </el-button>
          </div>

          <div class="config-items">
            <div v-for="item in configItems" :key="item.key" class="config-item">
              <el-icon class="ci-type" :size="15"><component :is="item.icon" /></el-icon>
              <span class="ci-label">{{ item.label }}</span>
              <el-icon class="ci-state" :class="item.ok ? 'ci-ok' : 'ci-warn'" :size="14">
                <CircleCheckFilled v-if="item.ok" />
                <WarningFilled v-else />
              </el-icon>
              <span class="ci-text" :class="{ 'ci-muted': !item.ok }" :title="item.ok ? item.okText : item.hint">
                {{ item.ok ? item.okText : item.hint }}
              </span>
            </div>
          </div>
        </section>

        <!-- 功能入口 -->
        <div class="entry-title">开始下载</div>
        <div class="entry-list">
          <button v-for="entry in entries" :key="entry.path" class="entry-row app-card" @click="go(entry.path)">
            <el-icon class="entry-icon" :size="22"><component :is="entry.icon" /></el-icon>
            <div class="entry-text">
              <div class="entry-name">{{ entry.title }}</div>
              <div class="entry-desc">{{ entry.desc }}</div>
            </div>
            <el-icon class="entry-arrow" :size="18"><ArrowRight /></el-icon>
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.welcome-body {
  padding: 20px 24px 32px;
}

.welcome-inner {
  max-width: 760px;
  margin: 0 auto;
}

/* 配置状态条 */
.config-strip {
  padding: 16px 18px;
  margin-bottom: 24px;
}

.config-lead {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 14px;
  font-weight: 600;
  color: var(--text-color);
}

.lead-ok {
  color: var(--el-color-success);
}

.lead-warn {
  color: var(--el-color-warning);
}

.config-action {
  margin-left: auto;
  font-weight: 500;
}

.config-items {
  display: grid;
  grid-template-columns: 1fr;
  gap: 8px;
  margin-top: 14px;
  padding-top: 14px;
  border-top: 1px solid var(--border-color);
}

@media (min-width: 620px) {
  .config-items {
    grid-template-columns: 1fr 1fr 1fr;
  }
}

.config-item {
  display: flex;
  align-items: center;
  gap: 6px;
  min-width: 0;
  font-size: 12px;
}

.ci-type {
  color: var(--text-muted);
  flex-shrink: 0;
}

.ci-label {
  color: var(--text-color);
  font-weight: 500;
  flex-shrink: 0;
}

.ci-state {
  flex-shrink: 0;
}

.ci-ok {
  color: var(--el-color-success);
}

.ci-warn {
  color: var(--el-color-warning);
}

.ci-text {
  color: var(--text-color);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.ci-muted {
  color: var(--text-muted);
}

/* 功能入口 */
.entry-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-muted);
  margin-bottom: 10px;
}

.entry-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.entry-row {
  display: flex;
  align-items: center;
  gap: 14px;
  width: 100%;
  padding: 16px 18px;
  text-align: left;
  cursor: pointer;
  transition: background-color 0.2s, border-color 0.2s, box-shadow 0.2s, transform 0.15s;
}

.entry-row:hover {
  border-color: var(--el-color-primary-light-7);
  box-shadow: 0 6px 18px rgba(15, 23, 42, 0.07);
  transform: translateY(-1px);
}

html.dark .entry-row:hover {
  box-shadow: 0 6px 18px rgba(0, 0, 0, 0.4);
}

.entry-icon {
  flex-shrink: 0;
  color: var(--el-color-primary);
}

.entry-text {
  flex: 1;
  min-width: 0;
}

.entry-name {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-color);
  margin-bottom: 3px;
}

.entry-desc {
  font-size: 12px;
  color: var(--text-muted);
  line-height: 1.5;
}

.entry-arrow {
  flex-shrink: 0;
  color: var(--text-muted);
  transition: transform 0.15s, color 0.15s;
}

.entry-row:hover .entry-arrow {
  color: var(--el-color-primary);
  transform: translateX(3px);
}
</style>
