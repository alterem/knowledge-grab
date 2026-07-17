<template>
  <div class="page-shell">
    <div class="page-header">
      <div class="page-title">使用帮助</div>
      <div class="page-desc">工具介绍、使用指南与常见问题</div>
    </div>

    <div class="page-body help-body">
      <div class="help-inner">
        <section class="help-card">
          <h2 class="section-title">项目概述</h2>
          <p class="leading-7">
            国家中小学智慧教育平台资源下载工具是一款专为教育工作者、学生和家长设计的桌面应用程序，旨在简化国家中小学智慧教育平台上教育资源的获取和管理过程。该工具提供了直观的用户界面，支持按学科、版本和年级筛选教材，并提供单本下载和批量下载功能，让用户能够轻松获取所需的教育资源。
          </p>
        </section>

        <section class="help-card">
          <h2 class="section-title">主要功能</h2>

          <div class="mb-4">
            <h3 class="block-title">1. 教材资源浏览与筛选</h3>
            <ul class="help-list">
              <li>多维度筛选：支持按学科、教材版本和年级进行精确筛选</li>
              <li>资源预览：显示教材封面、标题和相关统计信息</li>
              <li>搜索功能：支持关键词搜索，快速定位特定教材</li>
            </ul>
          </div>

          <div class="mb-4">
            <h3 class="block-title">2. 下载管理与断点续传</h3>
            <ul class="help-list">
              <li>下载池调度：点击下载即加入队列，按设置的并发数自动调度，切换页面不影响下载</li>
              <li>「下载管理」页：集中查看所有任务（进行中/已完成/失败），支持暂停、继续、重试、删除与清空记录</li>
              <li>断点续传：课件按字节续传、视频按已下载切片续传；暂停或中断后继续下载不会从头再来</li>
              <li>重启恢复：应用重启后，未完成任务标记为「已中断」，一键继续即可接着下</li>
              <li>迷你悬浮条：任意页面右下角显示总进度与速度，点击直达下载管理</li>
            </ul>
          </div>

          <div>
            <h3 class="block-title">3. 系统设置与自动更新</h3>
            <ul class="help-list">
              <li>下载路径设置：允许用户自定义下载文件的保存位置</li>
              <li>并发下载设置：可调整同时下载的任务数量，根据网络状况优化下载速度</li>
              <li>API 令牌管理：支持设置和更新 API 访问令牌</li>
              <li>主题切换：支持浅色/深色主题切换，提供舒适的视觉体验</li>
              <li>应用内更新：启动时自动检查新版本（可关闭），也可在「设置 → 关于与更新」手动检查并一键升级；GitHub 访问受限时可配置更新源镜像</li>
            </ul>
          </div>
        </section>

        <section class="help-card">
          <h2 class="section-title">使用指南</h2>

          <div class="mb-4">
            <h3 class="block-title">第一步：设置应用</h3>
            <p class="mb-2">首次使用前，请先进入"设置"页面：</p>
            <ol class="help-list list-decimal">
              <li>设置下载路径：选择一个有足够空间的文件夹存储下载的教材</li>
              <li>设置并发下载数：根据您的网络状况，设置合适的并发下载数量</li>
              <li>输入 Access Token：平台已要求登录后才能下载教材，请按「设置」页中的「如何获取？」指引，从智慧教育平台获取登录凭据并填入（令牌约一周过期，失效后重新获取即可）</li>
            </ol>
          </div>

          <div class="mb-4">
            <h3 class="block-title">第二步：浏览和筛选教材</h3>
            <ol class="help-list list-decimal">
              <li>在主页面选择学科、版本和年级</li>
              <li>点击"搜索"按钮查看符合条件的教材</li>
              <li>浏览教材列表，查看封面和详细信息</li>
            </ol>
          </div>

          <div>
            <h3 class="block-title">第三步：下载教材</h3>
            <ol class="help-list list-decimal">
              <li>单本下载：点击教材卡片上的"下载"按钮，任务会加入下载队列自动开始</li>
              <li>批量下载：点击页面上方的"批量下载"按钮，全部加入队列按并发数依次下载</li>
              <li>查看进度：教材卡片、侧边栏「下载管理」徽标和右下角悬浮条都会显示进度</li>
              <li>暂停/继续：卡片或「下载管理」页中可随时暂停；继续时自动断点续传</li>
            </ol>
          </div>
        </section>

        <section class="help-card">
          <h2 class="section-title">示例链接</h2>
          <p class="mb-3 page-desc">
            在「课程&视频下载」页粘贴课程页链接即可解析下载。下面是两个可直接试用的示例，点右侧按钮复制：
          </p>
          <div class="sample-list">
            <div v-for="sample in sampleLinks" :key="sample.url" class="sample-item">
              <div class="sample-info">
                <div class="sample-name">{{ sample.name }}</div>
                <div class="sample-url" :title="sample.url">{{ sample.url }}</div>
              </div>
              <el-button size="small" plain @click="copyLink(sample.url)">
                <el-icon class="mr-1">
                  <DocumentCopy />
                </el-icon>
                复制
              </el-button>
            </div>
          </div>
        </section>

        <section class="help-card">
          <h2 class="section-title">常见问题</h2>

          <div class="mb-4">
            <h3 class="block-title">下载速度慢怎么办？</h3>
            <p class="mb-1">可以尝试以下方法：</p>
            <ul class="help-list">
              <li>检查网络连接是否稳定</li>
              <li>减少并发下载数量</li>
              <li>选择网络较好的时间段下载</li>
            </ul>
          </div>

          <div class="mb-4">
            <h3 class="block-title">下载失败如何处理？</h3>
            <p class="mb-1">下载失败时，可以：</p>
            <ul class="help-list">
              <li>提示 Token 失效或需要登录：Access Token 已过期（约一周有效），请到「设置」页重新获取并填写。令牌失效时剩余任务会自动暂停，更新令牌后到「下载管理」继续即可</li>
              <li>检查网络连接</li>
              <li>确认下载路径是否有足够空间</li>
              <li>在「下载管理」页点击"重试"：会从已下载的部分断点续传，不会从头再来</li>
            </ul>
          </div>

          <div class="mb-4">
            <h3 class="block-title">如何升级应用？</h3>
            <p class="mb-1">应用支持在线升级：</p>
            <ul class="help-list">
              <li>启动时会自动检查新版本（可在设置中关闭），发现更新会弹出提醒</li>
              <li>也可以到「设置 → 关于与更新」手动点击"检查更新"，确认后自动下载并安装</li>
              <li>检查或下载更新失败（多为 GitHub 访问受限）：在设置中配置「自定义更新源」镜像地址，或前往 GitHub Releases 手动下载</li>
            </ul>
          </div>

          <div>
            <h3 class="block-title">找不到特定教材怎么办？</h3>
            <p class="mb-1">可以尝试：</p>
            <ul class="help-list">
              <li>调整筛选条件，使用更广泛的筛选范围</li>
              <li>确认所需教材是否在平台上提供</li>
              <li>检查 API 令牌是否有效（如适用）</li>
            </ul>
          </div>
        </section>

        <section class="help-card">
          <h2 class="section-title">免责声明</h2>
          <div class="leading-7">
            <p class="mb-3">本应用提供的所有信息仅供参考。虽然我们努力确保信息的准确性和及时性，但不对任何信息的完整性、准确性、可靠性、适用性或可用性作出任何明示或暗示的保证。</p>
            <p class="mb-3">用户应自行判断和承担使用本应用信息的风险。对于因使用或无法使用本应用所提供的信息而导致的任何直接、间接、附带、特殊或后果性损害，我们不承担任何责任。</p>
            <p class="mb-3">本应用可能包含指向第三方网站的链接。这些链接仅为方便用户而提供，并不表示我们认可这些网站的内容。我们对任何链接网站的内容或其准确性不承担任何责任。</p>
            <p class="mb-3">本应用的内容可能会随时更新或修改，恕不另行通知。</p>
            <p>使用本应用即表示您同意本免责声明的所有条款和条件。</p>
          </div>
        </section>

        <section class="help-card">
          <h2 class="section-title">联系与支持</h2>
          <p class="mb-3">如果您在使用过程中遇到任何问题，或有任何建议和反馈，欢迎前往 GitHub 与我们交流：</p>
          <div class="support-actions">
            <el-button type="primary" @click="openGitHub('/issues')">
              <el-icon class="mr-1">
                <ChatDotRound />
              </el-icon>
              提交 Issue 反馈
            </el-button>
            <el-button @click="openGitHub()">
              <svg class="mr-1" viewBox="0 0 16 16" width="14" height="14" fill="currentColor" aria-hidden="true">
                <path
                  d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27s1.36.09 2 .27c1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.01 8.01 0 0 0 16 8c0-4.42-3.58-8-8-8" />
              </svg>
              访问项目主页
            </el-button>
          </div>
          <p class="mt-3 page-desc">觉得好用的话，欢迎顺手点个 Star ⭐</p>
        </section>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ElButton, ElIcon, ElMessage } from 'element-plus';
import { ChatDotRound, DocumentCopy } from '@element-plus/icons-vue';
import { invoke } from '@tauri-apps/api/core';

const GITHUB_URL = 'https://github.com/alterem/knowledge-grab';

const openGitHub = async (path = '') => {
  try {
    await invoke('open_url', { url: GITHUB_URL + path });
  } catch (error) {
    console.error('无法打开 GitHub 链接:', error);
  }
};

const sampleLinks = [
  {
    name: '同步课堂 · 基础作业（课件文档）',
    url: 'https://basic.smartedu.cn/syncClassroom/basicWork/detail?contentType=assets_document&contentId=62044dd6-2ee9-454e-9db5-66693a302b70&catalogType=basicWork',
  },
  {
    name: '同步课堂 · 课程视频',
    url: 'https://basic.smartedu.cn/syncClassroom/classActivity?activityId=d1a57023-9b85-11ec-92ef-246e9675e50c&chapterId=4d1209f1-5a80-3f36-bde5-2c06685bf769&teachingmaterialId=cd5b8173-914b-40fd-b48c-d2bf77c00c4a&fromPrepare=0',
  },
];

const copyLink = async (link: string) => {
  try {
    await navigator.clipboard.writeText(link);
    ElMessage.success('链接已复制，粘贴到「课程 / 视频下载」页即可使用');
  } catch {
    ElMessage.error('复制失败，请重试');
  }
};
</script>

<style scoped>
.help-body {
  padding: 20px 24px 32px;
}

.help-inner {
  max-width: 780px;
  margin: 0 auto;
}

.help-card {
  background-color: var(--secondary-bg-color);
  border: 1px solid var(--border-color);
  border-radius: 12px;
  padding: 20px 24px;
  margin-bottom: 16px;
  font-size: 14px;
  transition: background-color 0.2s, border-color 0.2s;
}

.section-title {
  font-size: 16px;
  font-weight: 600;
  margin: 0 0 12px;
  padding-left: 10px;
  border-left: 3px solid var(--el-color-primary);
  line-height: 1.4;
}

.block-title {
  font-size: 14px;
  font-weight: 600;
  margin: 0 0 6px;
}

.help-list {
  list-style: disc;
  padding-left: 1.4rem;
  margin: 0;
  color: var(--text-muted);
  line-height: 1.9;
}

.help-list.list-decimal {
  list-style: decimal;
}

.help-list li::marker {
  color: var(--el-color-primary);
}

.support-actions {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
}

.support-actions .el-button + .el-button {
  margin-left: 0;
}

.sample-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.sample-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 10px 12px;
  border: 1px solid var(--border-color);
  border-radius: 8px;
  background-color: var(--bg-color);
}

.sample-info {
  flex: 1;
  min-width: 0;
}

.sample-name {
  font-size: 13px;
  font-weight: 600;
  margin-bottom: 2px;
}

.sample-url {
  font-size: 12px;
  color: var(--text-muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
</style>
