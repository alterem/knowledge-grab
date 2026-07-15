import { ref, watch, type Ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { ElMessage } from 'element-plus';
import type { DropdownOption } from '@/types';

// 数据源里混入的异常节点，不展示
const EXCLUDED_OPTION = { value: '267a11ad-0a45-4d3e-a95b-423fc3e959af', label: '培智学校' };

// 分类 -> 学科 -> 版本 -> 年级 -> 年份 的级联筛选：
// 上级变化时重置下级并拉取直接下级的选项（选项保持后端返回的原始顺序）
export function useTextbookFilters(categoryId: Ref<string>, isSpecialEducation: Ref<boolean>) {
  const subject = ref('');
  const version = ref('');
  const grade = ref('');
  const year = ref('');

  const subjects = ref<DropdownOption[]>([]);
  const versions = ref<DropdownOption[]>([]);
  const grades = ref<DropdownOption[]>([]);
  const years = ref<DropdownOption[]>([]);

  const levels = [
    { value: subject, options: subjects },
    { value: version, options: versions },
    { value: grade, options: grades },
    { value: year, options: years },
  ];

  const resetFrom = (level: number) => {
    for (const { value, options } of levels.slice(level)) {
      value.value = '';
      options.value = [];
    }
  };

  const fetchOptions = async (args: Record<string, string>, target: Ref<DropdownOption[]>) => {
    try {
      const options = await invoke<DropdownOption[]>('fetch_filter_options', { args });
      target.value = (options ?? []).filter(
        (opt) => !(opt.value === EXCLUDED_OPTION.value && opt.label === EXCLUDED_OPTION.label)
      );
    } catch (error) {
      console.error('获取筛选选项失败:', error);
      ElMessage.error('获取筛选信息出错');
    }
  };

  watch(
    categoryId,
    (cat) => {
      resetFrom(0);
      if (cat) {
        fetchOptions({ category_id: cat }, subjects);
      }
    },
    { immediate: true }
  );

  watch(subject, (subj) => {
    resetFrom(1);
    if (subj && categoryId.value) {
      fetchOptions({ category_id: categoryId.value, subject_id: subj }, versions);
    }
  });

  watch(version, (ver) => {
    resetFrom(2);
    if (ver && categoryId.value && subject.value) {
      fetchOptions(
        { category_id: categoryId.value, subject_id: subject.value, version_id: ver },
        grades
      );
    }
  });

  // 仅特殊教育有第五级「年份」
  watch(grade, (gra) => {
    resetFrom(3);
    if (isSpecialEducation.value && gra && categoryId.value && subject.value && version.value) {
      fetchOptions(
        {
          category_id: categoryId.value,
          subject_id: subject.value,
          version_id: version.value,
          grade_id: gra,
        },
        years
      );
    }
  });

  const labelOf = (options: Ref<DropdownOption[]>, value: string) =>
    options.value.find((opt) => opt.value === value)?.label || '';

  return {
    subject,
    version,
    grade,
    year,
    subjects,
    versions,
    grades,
    years,
    labelOf,
  };
}
