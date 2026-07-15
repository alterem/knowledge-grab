import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import type { DropdownOption } from '@/types';

// 分类列表全局只请求一次，Sidebar 与下载页共用
const categories = ref<DropdownOption[]>([]);
let loadPromise: Promise<DropdownOption[]> | null = null;

export function useCategories() {
  const load = (): Promise<DropdownOption[]> => {
    if (!loadPromise) {
      loadPromise = invoke<DropdownOption[]>('fetch_textbook_categories')
        .then((list) => {
          categories.value = list ?? [];
          return categories.value;
        })
        .catch((error) => {
          loadPromise = null; // 失败后允许重试
          throw error;
        });
    }
    return loadPromise;
  };

  return { categories, load };
}
