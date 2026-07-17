<template>
  <div class="page-shell">
    <div class="page-header">
      <div class="page-title">免责声明</div>
      <div class="page-desc">使用本应用前请仔细阅读以下条款</div>
    </div>

    <div class="page-body disclaimer-body">
      <div class="disclaimer-inner">
        <section class="disclaimer-card">
          <div class="disclaimer-header">
            <el-icon :size="20" class="disclaimer-icon">
              <WarningFilled />
            </el-icon>
            <span>重要提示</span>
          </div>
          <div class="leading-7">
            <p class="mb-3">本应用提供的所有信息仅供参考。虽然我们努力确保信息的准确性和及时性，但不对任何信息的完整性、准确性、可靠性、适用性或可用性作出任何明示或暗示的保证。</p>
            <p class="mb-3">用户应自行判断和承担使用本应用信息的风险。对于因使用或无法使用本应用所提供的信息而导致的任何直接、间接、附带、特殊或后果性损害，我们不承担任何责任。</p>
            <p class="mb-3">本应用可能包含指向第三方网站的链接。这些链接仅为方便用户而提供，并不表示我们认可这些网站的内容。我们对任何链接网站的内容或其准确性不承担任何责任。</p>
            <p class="mb-3">本应用的内容可能会随时更新或修改，恕不另行通知。</p>
            <p>使用本应用即表示您同意本免责声明的所有条款和条件。</p>
          </div>
        </section>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ElIcon } from 'element-plus';
import { WarningFilled } from '@element-plus/icons-vue';
</script>

<style scoped>
.disclaimer-body {
  padding: 20px 24px 32px;
}

.disclaimer-inner {
  max-width: 780px;
  margin: 0 auto;
}

.disclaimer-card {
  background-color: var(--secondary-bg-color);
  border: 1px solid var(--border-color);
  border-radius: 12px;
  padding: 20px 24px;
  font-size: 14px;
  transition: background-color 0.2s, border-color 0.2s;
}

.disclaimer-header {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 16px;
  font-weight: 600;
  margin-bottom: 12px;
}

.disclaimer-icon {
  color: var(--el-color-warning);
}
</style>
