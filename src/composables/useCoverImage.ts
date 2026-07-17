import { onMounted, ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';

// 封面经后端代理获取（优先详情里与源 PDF 同批的转码图），
// 按书籍 id 做内存缓存，避免翻页/返回时重复请求
const cache = new Map<string, Promise<string>>();
const MAX_CACHE_ENTRIES = 300;

function fetchCover(bookId: string, fallbackUrl: string): Promise<string> {
  const cached = cache.get(bookId);
  if (cached) return cached;

  const promise = invoke<string>('fetch_cover', { bookId, fallbackUrl })
    .then((base64) => `data:image/jpeg;base64,${base64}`)
    .catch((error) => {
      cache.delete(bookId); // 失败不缓存，允许重试
      throw error;
    });

  cache.set(bookId, promise);
  if (cache.size > MAX_CACHE_ENTRIES) {
    const oldest = cache.keys().next().value;
    if (oldest) cache.delete(oldest);
  }
  return promise;
}

export function useCoverImage(bookId: string, fallbackUrl: string) {
  const src = ref('');
  const loading = ref(true);
  const failed = ref(false);

  onMounted(async () => {
    try {
      src.value = await fetchCover(bookId, fallbackUrl);
    } catch (error) {
      console.error('获取封面失败:', error);
      failed.value = true;
    } finally {
      loading.value = false;
    }
  });

  return { src, loading, failed };
}
