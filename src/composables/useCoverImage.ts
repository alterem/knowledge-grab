import { onMounted, ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';

// 封面经后端代理转 base64（直连有防盗链），内存缓存避免翻页/返回时重复请求
const cache = new Map<string, Promise<string>>();
const MAX_CACHE_ENTRIES = 300;

function fetchCover(url: string): Promise<string> {
  const cached = cache.get(url);
  if (cached) return cached;

  const promise = invoke<string>('fetch_image', { url })
    .then((base64) => `data:image/jpeg;base64,${base64}`)
    .catch((error) => {
      cache.delete(url); // 失败不缓存，允许重试
      throw error;
    });

  cache.set(url, promise);
  if (cache.size > MAX_CACHE_ENTRIES) {
    const oldest = cache.keys().next().value;
    if (oldest) cache.delete(oldest);
  }
  return promise;
}

export function useCoverImage(url: string) {
  const src = ref('');
  const loading = ref(true);
  const failed = ref(false);

  onMounted(async () => {
    if (!url) {
      loading.value = false;
      failed.value = true;
      return;
    }
    try {
      src.value = await fetchCover(url);
    } catch (error) {
      console.error('获取封面失败:', error);
      failed.value = true;
    } finally {
      loading.value = false;
    }
  });

  return { src, loading, failed };
}
