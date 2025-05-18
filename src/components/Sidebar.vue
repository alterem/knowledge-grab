<script setup lang="ts">
import { ElMenu, ElMenuItem } from 'element-plus';
import { House, Tools, ChatRound, QuestionFilled } from '@element-plus/icons-vue';
import { invoke } from '@tauri-apps/api/core';
import { useRouter, useRoute } from 'vue-router';
import { computed, onMounted, ref, nextTick, markRaw } from 'vue';

const router = useRouter();
const route = useRoute();

interface DropdownOption {
  value: string;
  label: string;
}


const menuItems = ref([
  {
    index: '1',
    title: '课本下载',
    icon: markRaw(House),
    path: '/textbook-download',
    children: [] as any[]
  },
  { index: '2', title: '设置', icon: markRaw(Tools), path: '/settings' },
  { index: '3', title: '免责声明', icon: markRaw(ChatRound), path: '/disclaimer' },
  { index: '4', title: '关于/帮助', icon: markRaw(QuestionFilled), path: '/help' },
]);

const activeIndex = computed(() => {
  const currentPath = route.path;
  const currentQuery = route.query;
  console.log('Computing activeIndex for path:', currentPath, 'and query:', currentQuery);

  if (currentPath === '/textbook-download' && currentQuery.category) {
    const textbookItem = menuItems.value.find(item => item.index === '1');
    if (textbookItem && textbookItem.children) {
      const matchedChild = textbookItem.children.find((child: any) => child.value === currentQuery.category);
      if (matchedChild) {
        console.log('Matched nested item:', matchedChild.index);
        return matchedChild.index;
      }
    }
  }

  const matchedItem = menuItems.value.find(item => item.path === currentPath);
  if (matchedItem) {
    console.log('Matched top-level item:', matchedItem.index);
    return matchedItem.index;
  }

 return '1';
});

const handleSelect = (index: string) => {
  console.log('Selected menu item:', index);
  let selectedItem: any = null;

  selectedItem = menuItems.value.find(item => item.index === index);

  if (!selectedItem && index.startsWith('1-')) {
    const textbookItem = menuItems.value.find(item => item.index === '1');
    if (textbookItem && textbookItem.children) {
      selectedItem = textbookItem.children.find((child: any) => child.index === index);
    }
  }

  if (selectedItem && selectedItem.path) {
    nextTick(() => {
      router.push(selectedItem.path);
    });
  } else {
    console.log('Selected item without a path or unhandled index:', index);
  }
};

onMounted(async () => {
  try {
    const categories = await invoke('fetch_textbook_categories') as DropdownOption[];
    console.log('Fetched textbook categories:', categories);

    const textbookItem = menuItems.value.find(item => item.index === '1');

    if (textbookItem) {
      textbookItem.children = categories.map(category => ({
        index: `1-${category.value}`,
        title: category.label,
        value: category.value,
        path: `/textbook-download?category=${encodeURIComponent(category.value)}`
      }));
    }

  } catch (error) {
    console.error('Error fetching textbook categories:', error);
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

