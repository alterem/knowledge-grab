<script setup lang="ts">
import { ref, watch, computed, type Ref } from 'vue';
import { ElSelect, ElOption, ElMessage, ElMessageBox, ElBreadcrumb, ElBreadcrumbItem } from 'element-plus';
import TextbookItem from '../components/TextbookItem.vue';
import { useRoute } from 'vue-router';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { onMounted, onUnmounted } from 'vue';

const route = useRoute();
const textbookCategories = ref<DropdownOption[]>([]);
const breadcrumbItems = computed(() => {
  const items: { path?: string; title: string }[] = [];
  let textbookDownloadPath = '/textbook-download';
  if (textbookCategories.value.length > 0) {
    textbookDownloadPath = `/textbook-download?category=${encodeURIComponent(textbookCategories.value[0].value)}`;
  }
  items.push({ path: textbookDownloadPath, title: '课本下载' });
  if (route.query.category) {
    const selectedCategory = textbookCategories.value.find(
      (cat) => cat.value === route.query.category
    );
    items.push({ title: selectedCategory ? selectedCategory.label : (route.query.category as string) });
  }
  return items;
});


let unlistenStatus: (() => void) | null = null;
let unlistenProgress: (() => void) | null = null;
let unlistenBatchCompleted: (() => void) | null = null;

const subject = ref('');
const version = ref('');
const grade = ref('');
const year = ref('');

const subjects = ref<DropdownOption[]>([]);
const versions = ref<DropdownOption[]>([]);
const grades = ref<DropdownOption[]>([]);
const years = ref<DropdownOption[]>([]);

interface Textbook {
  id: number;
  cover_url: string;
  title: string;
  total_uv: number;
  like_count: number;
  download_url: string;
}

interface DropdownOption {
  value: string;
  label: string;
}

const textbooks = ref<Textbook[]>([]);

const isLoading = ref(false);

const handleBatchDownload = () => {
  console.log('Batch Download clicked');
  const apiToken = localStorage.getItem('api_token');
  const downloadPath = localStorage.getItem('download_path');
  const threadCount = localStorage.getItem('thread_count') ? parseInt(localStorage.getItem('thread_count')!, 10) : 4;
  const saveByCategorySetting = localStorage.getItem('save_by_category') === 'true';

  if (!downloadPath) {
    console.error('Download path is not set.');
    ElMessage.warning('下载路径没有设置，前往设置页面设置下载路径后继续')
    return;
  }

  const selectedCategoryLabel = textbookCategories.value.find(cat => cat.value === route.query.category)?.label || '';
  const selectedSubjectLabel = subjects.value.find(sub => sub.value === subject.value)?.label || '';
  const selectedVersionLabel = versions.value.find(ver => ver.value === version.value)?.label || '';
  const selectedGradeLabel = grades.value.find(gra => gra.value === grade.value)?.label || '';
  const selectedYearLabel = years.value.find(ye => ye.value === year.value)?.label || '';

  const textbooksToDownload = textbooks.value.map(textbook => ({
    url: textbook.download_url,
    title: textbook.title,
    category_label: selectedCategoryLabel,
    subject_label: selectedSubjectLabel,
    version_label: selectedVersionLabel,
    grade_label: selectedGradeLabel,
    year_label: selectedYearLabel,
    save_by_category: saveByCategorySetting,
  }));

  invoke('batch_download_textbooks', {
    textbooksToDownload: textbooksToDownload,
    token: apiToken,
    downloadPath: downloadPath,
    threadCount: threadCount,
  }).then(() => {
    console.log('Batch download command sent to backend.');
  }).catch(error => {
    console.error('Failed to start batch download:', error);
    ElMessage.error('下载错误' + error)
  });
};

async function setupTauriListeners() {
  console.log('Attempting to set up Tauri listeners...');
  try {
    console.log('Setting up initial listeners...');
    console.log('Setting up download-status listener...');
    unlistenStatus = await listen('download-status', (event: { payload: any }) => {
      console.log('Download status event received:', event.payload);
      const payload = event.payload;
      const relatedTextbook = textbooks.value.find(tb => tb.download_url === payload.url);
      if (relatedTextbook) {
        if (payload.status === 'completed' || payload.status === 'failed') {
          console.log(`Download status for ${payload.url}: ${payload.status}`);
        }
      }
    });
    console.log('Download status listener registered.');
    console.log('Setting up download-progress listener...');
    unlistenProgress = await listen('download-progress', (event: { payload: any }) => {
      console.log('Download progress event received:', event.payload);
      const payload = event.payload;
      const relatedTextbook = textbooks.value.find(tb => tb.download_url === payload.url);
      if (relatedTextbook) {
        console.log(`Download progress for ${payload.url}: ${payload.progress}%`);
      }
    });
    console.log('Download progress listener registered.');
    console.log('Setting up batch-download-completed listener...');
    unlistenBatchCompleted = await listen('batch-download-completed', (event: {
      payload: { downloadPath: string }
    }) => {
      console.log('RECEIVED batch-download-completed event:', event);
      console.log('Batch download completed event payload:', event.payload);
      const downloadPath = event.payload.downloadPath;
      console.log('Batch download completed. Download path:', downloadPath);
      ElMessage.success('批量下载完成！');
      ElMessageBox.confirm(
        '下载已完成，是否打开下载文件夹？',
        '下载完成',
        {
          confirmButtonText: '打开文件夹',
          cancelButtonText: '关闭',
          type: 'success',
        }
      )
        .then(() => {
          console.log('User confirmed to open download folder');
          invoke('open_download_folder_prompt', { downloadPath })
            .then(() => console.log('Open download folder command sent'))
            .catch(err => console.error('Failed to open download folder:', err));
        })
        .catch(() => {
          console.log('User chose not to open download folder');
        });
    });
    console.log('Batch download completed listener registered successfully.');
    console.log('Setting up batch-download-failed listener...');
    await listen('batch-download-failed', (event: { payload: { downloadPath: string } }) => {
      console.log('Batch download failed event received:', event.payload);
      const downloadPath = event.payload.downloadPath;
      console.log('Some batch downloads failed. Download path:', downloadPath);
      ElMessage.error('部分文件下载失败，请检查网络连接后重试。');
    });
    console.log('Batch download failed listener registered.');

  } catch (error) {
    console.error('Error setting up Tauri listeners:', error);
    console.warn('Tauri API might not be available or there was an error during setup.');
    ElMessage.error('初始化应用程序时出错，请重启应用。');
  }
}

const isHighSchoolCategory = computed(() => {
  const selectedCategoryOption = textbookCategories.value.find(cat => cat.value === route.query.category);
  return selectedCategoryOption?.label === '高中';
});
const specialEducationCategoryValue = ref('');
const isSpecialEducationCategory = computed(() => {
  return route.query.category === specialEducationCategoryValue.value;
});
const subjectLabel = computed(() => isSpecialEducationCategory.value ? '类别' : '学科');
const versionLabel = computed(() => isSpecialEducationCategory.value ? '学段' : '版本');
const gradeLabel = computed(() => isSpecialEducationCategory.value ? '学科' : '年级');
const yearLabel = computed(() => '年级');

const isGradeDropdownVisible = computed(() => {
  const isVisible = !!subject.value && !!version.value && (isSpecialEducationCategory.value || !isHighSchoolCategory.value);
  console.log('isGradeDropdownVisible changed:', isVisible);
  return isVisible;
});

const isYearDropdownVisible = computed(() => {
  const isVisible = isSpecialEducationCategory.value && !!subject.value && !!version.value && !!grade.value;
  console.log('isYearDropdownVisible changed:', isVisible);
  return isVisible;
});

const fetchFilterOptions = async (args: any, targetRef: Ref<DropdownOption[]>) => {
  try {
    console.log('Fetching filter options with args:', args);
    const options = await invoke<DropdownOption[]>('fetch_filter_options', { args });
    const filteredOptions = options ? options.filter(cat => !(cat.value === "267a11ad-0a45-4d3e-a95b-423fc3e959af" && cat.label === "培智学校")) : [];
    targetRef.value = filteredOptions.sort((a, b) => a.label.localeCompare(b.label));
    console.log('Fetched options:', targetRef.value);
  } catch (error) {
    console.error('Failed to fetch filter options:', error);
    ElMessage.error('获取筛选信息出错');
  }
};

const fetchFirstFilterOptions = async (categoryId: string) => {
  await fetchFilterOptions({ category_id: categoryId }, subjects);
  subject.value = '';
  version.value = '';
  grade.value = '';
  year.value = '';
  versions.value = [];
  grades.value = [];
  years.value = [];
};

const fetchSecondFilterOptions = async (categoryId: string, firstFilterValue: string) => {
  await fetchFilterOptions({ category_id: categoryId, subject_id: firstFilterValue }, versions);
  version.value = '';
  grade.value = '';
  year.value = '';
  grades.value = [];
  years.value = [];
};

const fetchThirdFilterOptions = async (categoryId: string, firstFilterValue: string, secondFilterValue: string) => {
  await fetchFilterOptions({ category_id: categoryId, subject_id: firstFilterValue, version_id: secondFilterValue }, grades);
  grade.value = '';
  year.value = '';
  years.value = [];
};

const fetchFourthFilterOptions = async (categoryId: string, firstFilterValue: string, secondFilterValue: string, thirdFilterValue: string) => {
  await fetchFilterOptions({ category_id: categoryId, subject_id: firstFilterValue, version_id: secondFilterValue, grade_id: thirdFilterValue }, years);
  year.value = '';
};

const isBreadcrumbSingleLevel = computed(() => {
  return breadcrumbItems.value.length <= 1;
});

watch(() => route.query.category, (newCategory) => {
  console.log('Category query changed:', newCategory);
  textbooks.value = [];
  if (newCategory) {
    fetchFirstFilterOptions(newCategory as string);
  } else {
    subject.value = '';
    version.value = '';
    grade.value = '';
    year.value = '';
    subjects.value = [];
    versions.value = [];
    grades.value = [];
    years.value = [];
  }
});

watch(() => subject.value, (newSubject) => {
  console.log('First filter (subject) changed:', newSubject);
  if (newSubject && route.query.category) {
    fetchSecondFilterOptions(route.query.category as string, newSubject);
  } else {
    version.value = '';
    grade.value = '';
    year.value = '';
    versions.value = [];
    grades.value = [];
    years.value = [];
  }
});

watch(() => version.value, (newVersion) => {
  console.log('Second filter (version) changed:', newVersion);
  if (newVersion && route.query.category && subject.value) {
    fetchThirdFilterOptions(route.query.category as string, subject.value, newVersion);
  } else {
    grade.value = '';
    year.value = '';
    grades.value = [];
    years.value = [];
  }
});

watch(() => grade.value, (newGrade) => {
  console.log('Third filter (grade) changed:', newGrade);
  if (isSpecialEducationCategory.value && newGrade && route.query.category && subject.value && version.value) {
    fetchFourthFilterOptions(route.query.category as string, subject.value, version.value, newGrade);
  } else {
    year.value = '';
    years.value = [];
  }
});


onMounted(async () => {
  console.log('TextbookDownloadPage mounted, setting up listeners...');
  setupTauriListeners();

  try {
    textbookCategories.value = await invoke('fetch_textbook_categories') as DropdownOption[];
    console.log('Fetched textbook categories for breadcrumb:', textbookCategories.value);
    const specialEducationCategory = textbookCategories.value.find(cat => cat.label === '特殊教育');
    if (specialEducationCategory) {
      specialEducationCategoryValue.value = specialEducationCategory.value;
      console.log('Special Education Category Value:', specialEducationCategoryValue.value);
    }
  } catch (error) {
    console.error('Error fetching textbook categories for breadcrumb:', error);
    ElMessage.error('获取课本分类出错');
  }

  if (route.query.category) {
    fetchFirstFilterOptions(route.query.category as string);
  }
});

onUnmounted(() => {
  console.log('TextbookDownloadPage unmounting, cleaning up listeners...');
  if (unlistenStatus) {
    unlistenStatus();
    console.log('Download status listener cleaned up');
  }
  if (unlistenProgress) {
    unlistenProgress();
    console.log('Download progress listener cleaned up');
  }
  if (unlistenBatchCompleted) {
    unlistenBatchCompleted();
    console.log('Batch download completed listener cleaned up');
  }
});

const handleSearch = async () => {
  console.log('Search clicked with filters:', subject.value, version.value, grade.value, year.value);
    isLoading.value = true;
    try {
      console.log('Fetching textbooks with filters:', {
        category_id: route.query.category as string,
        subject: subject.value,
        version: version.value,
        grade: grade.value,
        year: year.value,
      });
      const fetchedTextbooks = await invoke<Textbook[]>('fetch_textbooks', {
        categoryId: route.query.category as string,
        subjectId: subject.value,
        versionId: version.value,
        gradeId: grade.value,
        ...(isSpecialEducationCategory.value && year.value && { yearId: year.value }),
      }) as Textbook[];
      textbooks.value = fetchedTextbooks;
      console.log('Fetched textbooks:', textbooks.value);
      if (textbooks.value.length === 0) {
        ElMessage.info('获取到数据为空');
      }
    } catch (error) {
      console.error('Failed to fetch textbooks:', error);
      ElMessage.error('Failed to fetch textbooks:' + error)
    } finally {
      isLoading.value = false;
    }
};

</script>

<template>
  <div class="max-h-[calc(100vh-100px)]" :style="{ backgroundColor: 'var(--bg-color)', color: 'var(--text-color)' }"
    v-loading="isLoading">
    <el-breadcrumb class="mb-3" separator="/">
      <el-breadcrumb-item v-for="(item, index) in breadcrumbItems" :key="index" :to="item.path">
        {{ item.title }}
      </el-breadcrumb-item>
    </el-breadcrumb>

    <div class="flex items-center space-x-4 mb-6 flex-nowrap">
      <div class="w-50 flex items-center space-x-2">
        <label class="text-sm font-medium" :style="{ color: 'var(--text-color)' }">{{ subjectLabel }}</label>
        <el-select v-model="subject" :placeholder="'请选择' + subjectLabel" class="flex-1"
          :disabled="isBreadcrumbSingleLevel">
          <el-option v-for="item in subjects" :key="item.value" :label="item.label" :value="item.value" />
        </el-select>
      </div>

      <div class="w-50 flex items-center space-x-2">
        <label class="text-sm font-medium" :style="{ color: 'var(--text-color)' }">{{ versionLabel }}</label>
        <el-select v-model="version" :placeholder="'请选择' + versionLabel" class="flex-1"
          :disabled="isBreadcrumbSingleLevel">
          <el-option v-for="item in versions" :key="item.value" :label="item.label" :value="item.value" />
        </el-select>
      </div>

      <div class="w-50 flex items-center space-x-2" v-if="isGradeDropdownVisible">
        <label class="text-sm font-medium" :style="{ color: 'var(--text-color)' }">{{ gradeLabel }}</label>
        <el-select v-model="grade" :placeholder="'请选择' + gradeLabel" class="flex-1" :disabled="isBreadcrumbSingleLevel">
          <el-option v-for="item in grades" :key="item.value" :label="item.label" :value="item.value" />
        </el-select>
      </div>

      <div class="w-50 flex items-center space-x-2" v-if="isYearDropdownVisible">
        <label class="text-sm font-medium" :style="{ color: 'var(--text-color)' }">{{ yearLabel }}</label>
        <el-select v-model="year" :placeholder="'请选择' + yearLabel" class="flex-1" :disabled="isBreadcrumbSingleLevel">
          <el-option v-for="item in years" :key="item.value" :label="item.label" :value="item.value" />
        </el-select>
      </div>

      <el-button type="primary" @click="handleSearch" :disabled="isBreadcrumbSingleLevel">搜索</el-button>
      <el-button type="primary" @click="handleBatchDownload" v-if="textbooks.length > 0">批量下载</el-button>
    </div>

    <div class="textbook-list text-center">
      <TextbookItem
        v-for="textbook in textbooks"
        :key="textbook.id"
        :textbook="textbook"
        :categoryLabel="textbookCategories.find(cat => cat.value === route.query.category)?.label || ''"
        :subjectLabel="subjects.find(sub => sub.value === subject)?.label || ''"
        :versionLabel="versions.find(ver => ver.value === version)?.label || ''"
        :gradeLabel="grades.find(gra => gra.value === grade)?.label || ''"
        :yearLabel="years.find(ye => ye.value === year)?.label || ''"
      />
      <p v-if="textbooks.length === 0 && !isLoading" class="text-gray-500">{{ !route.query.category ? '请先选择一个课本分类' :
        '没有找到相关课本。' }}
      </p>
      <div v-if="textbooks.length > 0" class="mt-4 ml-2 text-sm text-gray-500">
        获取到课本数量：{{ textbooks.length }}
      </div>
    </div>
  </div>
</template>

<style scoped></style>