<template>
  <div class="max-h-[calc(100vh-100px)] text-left"
    :style="{ backgroundColor: 'var(--bg-color)', color: 'var(--text-color)' }">
    <div class="text-3xl font-bold mb-6">使用帮助</div>

    <div class="mb-8">
      <div class="text-2xl font-semibold mb-3">项目概述</div>
      <p class="mb-4">
        国家中小学智慧教育平台资源下载工具是一款专为教育工作者、学生和家长设计的桌面应用程序，旨在简化国家中小学智慧教育平台上教育资源的获取和管理过程。该工具提供了直观的用户界面，支持按学科、版本和年级筛选教材，并提供单本下载和批量下载功能，让用户能够轻松获取所需的教育资源。
      </p>
    </div>

    <div class="mb-8">
      <div class="text-2xl font-semibold mb-3">主要功能</div>

      <div class="mb-4">
        <div class="text-lg font-medium mb-2">1. 教材资源浏览与筛选</div>
        <ul class="list-none pl-6 mb-3">
          <li>多维度筛选：支持按学科、教材版本和年级进行精确筛选</li>
          <li>资源预览：显示教材封面、标题和相关统计信息</li>
          <li>搜索功能：支持关键词搜索，快速定位特定教材</li>
        </ul>
      </div>

      <div class="mb-4">
        <div class="text-lg font-medium mb-2">2. 下载管理</div>
        <ul class="list-none pl-6 mb-3">
          <li>单本下载：支持单本教材的下载，并实时显示下载进度</li>
          <li>批量下载：支持选择多本教材进行批量下载，提高下载效率</li>
          <li>下载控制：提供下载取消功能</li>
          <li>下载状态追踪：清晰显示每本教材的下载状态</li>
        </ul>
      </div>

      <div class="mb-4">
        <div class="text-lg font-medium mb-2">3. 系统设置</div>
        <ul class="list-none pl-6 mb-3">
          <li>下载路径设置：允许用户自定义下载文件的保存位置</li>
          <li>并发下载设置：可调整同时下载的任务数量，根据网络状况优化下载速度</li>
          <li>API 令牌管理：支持设置和更新 API 访问令牌</li>
          <li>主题切换：支持浅色/深色主题切换，提供舒适的视觉体验</li>
        </ul>
      </div>
    </div>

    <div class="mb-8">
      <div class="text-2xl font-semibold mb-3">使用指南</div>

      <div class="mb-4">
        <div class="text-lg font-medium mb-2">第一步：设置应用</div>
        <p class="mb-2">首次使用前，请先进入"设置"页面：</p>
        <ol class="list-decimal pl-6 mb-3">
          <li>设置下载路径：选择一个有足够空间的文件夹存储下载的教材</li>
          <li>设置并发下载数：根据您的网络状况，设置合适的并发下载数量</li>
          <li>输入 API 令牌：如需访问特定资源，请输入有效的 API 令牌</li>
        </ol>
      </div>

      <div class="mb-4">
        <div class="text-lg font-medium mb-2">第二步：浏览和筛选教材</div>
        <ol class="list-decimal pl-6 mb-3">
          <li>在主页面选择学科、版本和年级</li>
          <li>点击"搜索"按钮查看符合条件的教材</li>
          <li>浏览教材列表，查看封面和详细信息</li>
        </ol>
      </div>

      <div class="mb-4">
        <div class="text-lg font-medium mb-2">第三步：下载教材</div>
        <ol class="list-decimal pl-6 mb-3">
          <li>单本下载：点击教材卡片上的"下载"按钮</li>
          <li>批量下载：选择多本教材后，点击页面上方的"批量下载"按钮</li>
          <li>查看下载进度：在教材卡片上可以看到实时下载进度</li>
          <li>取消下载：如需取消下载，点击"取消"按钮</li>
        </ol>
      </div>
    </div>

    <div class="mb-8">
      <div class="text-2xl font-semibold mb-3">常见问题</div>

      <div class="mb-3">
        <div class="text-lg font-medium mb-2">下载速度慢怎么办？</div>
        <p>可以尝试以下方法：</p>
        <ul class="list-none pl-6 mb-2">
          <li>检查网络连接是否稳定</li>
          <li>减少并发下载数量</li>
          <li>选择网络较好的时间段下载</li>
        </ul>
      </div>

      <div class="mb-3">
        <div class="text-lg font-medium mb-2">下载失败如何处理？</div>
        <p>下载失败时，可以：</p>
        <ul class="list-none pl-6 mb-2">
          <li>检查网络连接</li>
          <li>确认下载路径是否有足够空间</li>
          <li>重新点击下载按钮尝试下载</li>
        </ul>
      </div>

      <div class="mb-3">
        <div class="text-lg font-medium mb-2">找不到特定教材怎么办？</div>
        <p>可以尝试：</p>
        <ul class="list-none pl-6 mb-2">
          <li>调整筛选条件，使用更广泛的筛选范围</li>
          <li>确认所需教材是否在平台上提供</li>
          <li>检查 API 令牌是否有效（如适用）</li>
        </ul>
      </div>
    </div>

    <div class="mb-8">
      <div class="text-2xl font-semibold mb-3">免责声明</div>
      <div>
        <p class="mb-4">本应用提供的所有信息仅供参考。虽然我们努力确保信息的准确性和及时性，但不对任何信息的完整性、准确性、可靠性、适用性或可用性作出任何明示或暗示的保证。</p>
        <p class="mb-4">用户应自行判断和承担使用本应用信息的风险。对于因使用或无法使用本应用所提供的信息而导致的任何直接、间接、附带、特殊或后果性损害，我们不承担任何责任。</p>
        <p class="mb-4">本应用可能包含指向第三方网站的链接。这些链接仅为方便用户而提供，并不表示我们认可这些网站的内容。我们对任何链接网站的内容或其准确性不承担任何责任。</p>
        <p class="mb-4">本应用的内容可能会随时更新或修改，恕不另行通知。</p>
        <p>使用本应用即表示您同意本免责声明的所有条款和条件。</p>
      </div>
    </div>

    <div>
      <div class="text-2xl font-semibold mb-3">联系与支持</div>
      <p class="mb-4">如果您在使用过程中遇到任何问题，或有任何建议和反馈，请通过以下方式联系我们：</p>
      <ul class="list-none pl-6">
        <li>提交问题报告</li>
        <li>参与用户体验测试</li>
        <li>提供功能建议</li>
      </ul>
      <p class="mt-4">我们将持续改进应用，为您提供更好的教育资源获取体验。</p>
    </div>
    <div class="pt-4"></div>
  </div>
</template>

<style scoped></style>
<script setup lang="ts">
</script>