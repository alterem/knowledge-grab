<script setup lang="ts">
import { ref, watch, computed, onMounted, onUnmounted } from 'vue';
import { ElSelect, ElOption, ElMessage, ElMessageBox, ElBreadcrumb, ElBreadcrumbItem } from 'element-plus';
import { Search, Download } from '@element-plus/icons-vue';
import { useRoute } from 'vue-router';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import TextbookItem from '@/components/TextbookItem.vue';
import { useCategories } from '@/composables/useCategories';
import { useTextbookFilters } from '@/composables/useTextbookFilters';
import { readDownloadSettings } from '@/utils/settings';
import type { Textbook, TextbookLabels } from '@/types';

const route = useRoute();
const { categories, load: loadCategories } = useCategories();

const categoryId = computed(() => (route.query.category as string) || '');
const currentCategory = computed(() =>
  categories.value.find((cat) => cat.value === categoryId.value)
);
const isHighSchool = computed(() => currentCategory.value?.label === '高中');
const isSpecialEducation = computed(() => currentCategory.value?.label === '特殊教育');

const { subject, version, grade, year, subjects, versions, grades, years, labelOf } =
  useTextbookFilters(categoryId, isSpecialEducation);

// 特殊教育的层级含义不同，界面文案随之变化
const subjectLabel = computed(() => (isSpecialEducation.value ? '类别' : '学科'));
const versionLabel = computed(() => (isSpecialEducation.value ? '学段' : '版本'));
const gradeLabel = computed(() => (isSpecialEducation.value ? '学科' : '年级'));
const yearLabel = '年级';

const isGradeDropdownVisible = computed(
  () => !!subject.value && !!version.value && (isSpecialEducation.value || !isHighSchool.value)
);
const isYearDropdownVisible = computed(
  () => isSpecialEducation.value && !!subject.value && !!version.value && !!grade.value
);
const noCategorySelected = computed(() => !categoryId.value);

const breadcrumbItems = computed(() => {
  const items: { path?: string; title: string }[] = [];
  let rootPath = '/textbook-download';
  if (categories.value.length > 0) {
    rootPath = `/textbook-download?category=${encodeURIComponent(categories.value[0].value)}`;
  }
  items.push({ path: rootPath, title: '课本下载' });
  if (categoryId.value) {
    items.push({ title: currentCategory.value?.label ?? categoryId.value });
  }
  return items;
});

const textbooks = ref<Textbook[]>([]);
const isLoading = ref(false);
const hasSearched = ref(false);

watch(categoryId, () => {
  textbooks.value = [];
  hasSearched.value = false;
});

const emptyDescription = computed(() => {
  if (noCategorySelected.value) return '请先在左侧选择一个课本分类';
  if (!hasSearched.value) return '选择筛选条件后点击「搜索」获取课本列表';
  return '没有找到相关课本，试试调整筛选条件';
});

const selectedLabels = computed<TextbookLabels>(() => ({
  category: currentCategory.value?.label || '',
  subject: labelOf(subjects, subject.value),
  version: labelOf(versions, version.value),
  grade: labelOf(grades, grade.value),
  year: labelOf(years, year.value),
}));

const handleSearch = async () => {
  isLoading.value = true;
  hasSearched.value = true;
  try {
    textbooks.value = await invoke<Textbook[]>('fetch_textbooks', {
      categoryId: categoryId.value,
      subjectId: subject.value,
      versionId: version.value,
      gradeId: grade.value,
      ...(isSpecialEducation.value && year.value && { yearId: year.value }),
    });
    if (textbooks.value.length === 0) {
      ElMessage.info('获取到数据为空');
    }
  } catch (error) {
    console.error('获取课本列表失败:', error);
    ElMessage.error('获取课本列表失败: ' + error);
  } finally {
    isLoading.value = false;
  }
};

const handleBatchDownload = () => {
  const settings = readDownloadSettings();
  if (!settings.downloadPath) {
    ElMessage.warning('下载路径没有设置，前往设置页面设置下载路径后继续');
    return;
  }

  const labels = selectedLabels.value;
  const textbooksToDownload = textbooks.value.map((textbook) => ({
    url: textbook.download_url,
    title: textbook.title,
    category_label: labels.category,
    subject_label: labels.subject,
    version_label: labels.version,
    grade_label: labels.grade,
    year_label: labels.year,
    save_by_category: settings.saveByCategory,
  }));

  invoke('batch_download_textbooks', {
    textbooksToDownload,
    token: settings.token,
    downloadPath: settings.downloadPath,
    threadCount: settings.threadCount,
  }).catch((error) => {
    console.error('批量下载启动失败:', error);
    ElMessage.error('下载错误' + error);
  });
};

let unlistenBatchCompleted: (() => void) | null = null;
let unlistenBatchFailed: (() => void) | null = null;

onMounted(async () => {
  unlistenBatchCompleted = await listen<{ downloadPath: string }>(
    'batch-download-completed',
    (event) => {
      const downloadPath = event.payload.downloadPath;
      ElMessage.success('批量下载完成！');
      ElMessageBox.confirm('下载已完成，是否打开下载文件夹？', '下载完成', {
        confirmButtonText: '打开文件夹',
        cancelButtonText: '关闭',
        type: 'success',
      })
        .then(() => {
          invoke('open_download_folder_prompt', { downloadPath }).catch((err) =>
            console.error('打开下载文件夹失败:', err)
          );
        })
        .catch(() => {
          /* 用户选择不打开 */
        });
    }
  );

  unlistenBatchFailed = await listen('batch-download-failed', () => {
    ElMessage.error('部分文件下载失败，请检查网络连接后重试。');
  });

  loadCategories().catch((error) => {
    console.error('获取课本分类失败:', error);
    ElMessage.error('获取课本分类出错');
  });
});

onUnmounted(() => {
  unlistenBatchCompleted?.();
  unlistenBatchFailed?.();
});
</script>

<template>
  <div class="page-shell" v-loading="isLoading">
    <div class="toolbar">
      <el-breadcrumb class="crumbs" separator="/">
        <el-breadcrumb-item v-for="(item, index) in breadcrumbItems" :key="index" :to="item.path">
          {{ item.title }}
        </el-breadcrumb-item>
      </el-breadcrumb>

      <div class="filters">
        <el-select v-model="subject" class="filter-select" :placeholder="'选择' + subjectLabel"
          :disabled="noCategorySelected">
          <el-option v-for="item in subjects" :key="item.value" :label="item.label" :value="item.value" />
        </el-select>

        <el-select v-model="version" class="filter-select" :placeholder="'选择' + versionLabel"
          :disabled="noCategorySelected">
          <el-option v-for="item in versions" :key="item.value" :label="item.label" :value="item.value" />
        </el-select>

        <el-select v-if="isGradeDropdownVisible" v-model="grade" class="filter-select"
          :placeholder="'选择' + gradeLabel" :disabled="noCategorySelected">
          <el-option v-for="item in grades" :key="item.value" :label="item.label" :value="item.value" />
        </el-select>

        <el-select v-if="isYearDropdownVisible" v-model="year" class="filter-select"
          :placeholder="'选择' + yearLabel" :disabled="noCategorySelected">
          <el-option v-for="item in years" :key="item.value" :label="item.label" :value="item.value" />
        </el-select>

        <el-button type="primary" :icon="Search" @click="handleSearch" :disabled="noCategorySelected">
          搜索
        </el-button>
        <el-button v-if="textbooks.length > 0" :icon="Download" @click="handleBatchDownload">
          批量下载
        </el-button>

        <el-tag v-if="textbooks.length > 0" class="count-tag" type="info" effect="plain" round>
          共 {{ textbooks.length }} 本
        </el-tag>
      </div>
    </div>

    <div class="list-area">
      <div v-if="textbooks.length > 0" class="textbook-grid">
        <TextbookItem v-for="textbook in textbooks" :key="textbook.id" :textbook="textbook" :labels="selectedLabels" />
      </div>
      <el-empty v-else-if="!isLoading" :description="emptyDescription" :image-size="110" class="empty-state" />
    </div>
  </div>
</template>

<style scoped>
/* 页面骨架由全局 .page-shell 提供：工具栏固定，仅列表区滚动 */
.toolbar {
  flex-shrink: 0;
  padding: 12px 20px;
  background-color: var(--secondary-bg-color);
  border-bottom: 1px solid var(--border-color);
  transition: background-color 0.2s, border-color 0.2s;
}

.crumbs {
  margin-bottom: 10px;
  font-size: 12px;
}

.filters {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 10px;
}

.filters .el-button + .el-button {
  margin-left: 0;
}

.filter-select {
  width: 170px;
}

.count-tag {
  margin-left: auto;
}

.list-area {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 16px 20px 24px;
}

.textbook-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(380px, 1fr));
  gap: 14px;
}

.empty-state {
  margin-top: 8vh;
}
</style>
