<script setup lang="ts">
import { ElMenu, ElMenuItem, ElSubMenu, ElIcon } from 'element-plus';
import { House, Tools, ChatRound, QuestionFilled } from '@element-plus/icons-vue';
import { useRouter, useRoute } from 'vue-router';
import { computed, onMounted, ref, markRaw, type Component } from 'vue';
import { useCategories } from '@/composables/useCategories';

const router = useRouter();
const route = useRoute();
const { load: loadCategories } = useCategories();

interface MenuChild {
  index: string;
  title: string;
  value: string;
  path: string;
}

interface MenuItem {
  index: string;
  title: string;
  icon: Component;
  path: string;
  children?: MenuChild[];
}

const menuItems = ref<MenuItem[]>([
  { index: '1', title: '课本下载', icon: markRaw(House), path: '/textbook-download', children: [] },
  { index: '2', title: '设置', icon: markRaw(Tools), path: '/settings' },
  { index: '3', title: '免责声明', icon: markRaw(ChatRound), path: '/disclaimer' },
  { index: '4', title: '关于/帮助', icon: markRaw(QuestionFilled), path: '/help' },
]);

const textbookChildren = computed(() => menuItems.value[0].children ?? []);

const activeIndex = computed(() => {
  if (route.path === '/textbook-download' && route.query.category) {
    const matched = textbookChildren.value.find((child) => child.value === route.query.category);
    if (matched) return matched.index;
  }
  return menuItems.value.find((item) => item.path === route.path)?.index ?? '1';
});

const handleSelect = (index: string) => {
  const selected =
    menuItems.value.find((item) => item.index === index) ??
    textbookChildren.value.find((child) => child.index === index);
  if (selected?.path) {
    router.push(selected.path);
  }
};

onMounted(async () => {
  try {
    const categories = await loadCategories();
    menuItems.value[0].children = categories.map((category) => ({
      index: `1-${category.value}`,
      title: category.label,
      value: category.value,
      path: `/textbook-download?category=${encodeURIComponent(category.value)}`,
    }));
  } catch (error) {
    console.error('获取课本分类失败:', error);
  }
});
</script>

<template>
  <el-menu :default-active="activeIndex" class="el-menu-vertical-demo h-full w-[200px]" @select="handleSelect"
    :collapse="false">
    <template v-for="item in menuItems" :key="item.index">
      <el-sub-menu v-if="item.children && item.children.length > 0" :index="item.index">
        <template #title>
          <el-icon v-if="item.icon">
            <component :is="item.icon" />
          </el-icon>
          <span>{{ item.title }}</span>
        </template>
        <el-menu-item v-for="child in item.children" :key="child.index" :index="child.index">
          {{ child.title }}
        </el-menu-item>
      </el-sub-menu>

      <el-menu-item v-else :index="item.index">
        <el-icon v-if="item.icon">
          <component :is="item.icon" />
        </el-icon>
        <span>{{ item.title }}</span>
      </el-menu-item>
    </template>
  </el-menu>
</template>

<style scoped>
.el-menu {
  border-right: none;
  background-color: var(--secondary-bg-color) !important;
}

.el-menu-item {
  border-bottom: 1px solid var(--border-color);
  background-color: var(--secondary-bg-color) !important;
  color: var(--text-color) !important;
}

.el-menu-item:last-child {
  border-bottom: none;
}

.el-menu-item:hover {
  background-color: var(--border-color) !important;
}

.el-menu-item.is-active {
  background-color: var(--primary-color) !important;
  color: #ffffff !important;
}
</style>
