import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import type { DropdownOption } from '@/types';

// 分类列表全局只请求一次，Sidebar 与下载页共用
const categories = ref<DropdownOption[]>([]);
// 请求进行中的标记，下拉框据此显示 loading
const loading = ref(false);
let loadPromise: Promise<DropdownOption[]> | null = null;

export function useCategories() {
  const load = (): Promise<DropdownOption[]> => {
    if (!loadPromise) {
      loading.value = true;
      loadPromise = invoke<DropdownOption[]>('fetch_textbook_categories')
        .then((list) => {
          categories.value = list ?? [];
          return categories.value;
        })
        .catch((error) => {
          loadPromise = null; // 失败后允许重试
          throw error;
        })
        .finally(() => {
          loading.value = false;
        });
    }
    return loadPromise;
  };

  return { categories, loading, load };
}
