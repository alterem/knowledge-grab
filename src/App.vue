<script setup lang="ts">
import { ref, onUnmounted, onMounted, provide } from 'vue';
import { ElContainer, ElHeader, ElAside, ElMain } from 'element-plus';
import { Sunny, Moon, QuestionFilled } from '@element-plus/icons-vue';
import Sidebar from './components/Sidebar.vue';
import { useRouter } from 'vue-router';

const router = useRouter();

const onHelp = () => {
  router.push('/help');
}

const asideWidth = ref(200);

let isDragging = false;
let startX = 0;
let startWidth = 0;

const startDragging = (event: MouseEvent) => {
  isDragging = true;
  startX = event.clientX;
  startWidth = asideWidth.value;
  document.addEventListener('mousemove', dragMove);
  document.addEventListener('mouseup', stopDragging);
};

const dragMove = (event: MouseEvent) => {
  if (!isDragging) return;
  const newWidth = startWidth + (event.clientX - startX);
  asideWidth.value = Math.max(180, Math.min(240, newWidth));
};

const stopDragging = () => {
  isDragging = false;
  document.removeEventListener('mousemove', dragMove);
  document.removeEventListener('mouseup', stopDragging);
};

onUnmounted(() => {
  stopDragging();
});

const isDarkMode = ref(false);

const toggleTheme = () => {
  isDarkMode.value = !isDarkMode.value;
  if (isDarkMode.value) {
    document.documentElement.classList.add('dark');
    localStorage.setItem('theme', 'dark');
  } else {
    document.documentElement.classList.remove('dark');
    localStorage.setItem('theme', 'light');
  }
};

provide('isDarkMode', isDarkMode);
provide('toggleTheme', toggleTheme);

onMounted(() => {
  const savedTheme = localStorage.getItem('theme');
  if (savedTheme === 'dark') {
    isDarkMode.value = true;
    document.documentElement.classList.add('dark');
  } else {
    isDarkMode.value = false;
    document.documentElement.classList.remove('dark');
  }
  document.addEventListener('contextmenu', handleRightClick);
});

const handleRightClick = (event: MouseEvent) => {
  event.preventDefault();
};

onUnmounted(() => {
  stopDragging();
  document.removeEventListener('contextmenu', handleRightClick);
});

</script>

<template>
  <el-config-provider>
    <el-container class="h-screen w-screen">

      <el-header class="flex items-center justify-between border-b border-gray-200 px-6" :style="{ backgroundColor: 'var(--bg-color)', color: 'var(--text-color)' }">
        <div class="flex items-center">
          <img src="/icon.png" alt="Logo" class="h-8 w-auto mr-3" @contextmenu.prevent>
          <span class="text-xl font-semibold" :style="{ color: 'var(--text-color)' }">国家中小学智慧教育平台</span>
        </div>

        <div class="flex items-center space-x-4">
          <el-tooltip :content="isDarkMode ? '切换到亮色模式' : '切换到暗色模式'" placement="top">
            <el-icon :size="20" class="cursor-pointer text-gray-600 hover:text-blue-500" @click="toggleTheme">
              <component :is="isDarkMode ? Moon : Sunny" />
            </el-icon>
          </el-tooltip>
          <el-tooltip content="帮助" placement="top">
            <el-icon :size="20" class="cursor-pointer text-gray-600 hover:text-blue-500" @click="onHelp">
              <QuestionFilled />
            </el-icon>
          </el-tooltip>
        </div>
      </el-header>

      <el-container>
        <el-aside :width="asideWidth + 'px'" class="flex flex-col overflow-y-auto" :style="{ backgroundColor: 'var(--secondary-bg-color)' }">
          <Sidebar />
        </el-aside>
        <div
          class="w-1 cursor-ew-resize bg-gray-300 hover:bg-blue-500 transition-colors duration-200"
          @mousedown="startDragging"
        ></div>

        <el-main class="p-0 flex-1 overflow-y-auto" :style="{ backgroundColor: 'var(--bg-color)' }">
          <router-view></router-view>
        </el-main>
      </el-container>

    </el-container>
  </el-config-provider>
</template>

<style scoped>
</style>
