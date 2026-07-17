import { createApp } from 'vue';
import ElementPlus from 'element-plus';
import 'element-plus/dist/index.css';
import 'element-plus/theme-chalk/dark/css-vars.css';
// style.css 必须在 Element Plus 样式之后引入，其中的主题变量覆写才能生效
import './style.css';
import App from './App.vue';
import { createRouter, createWebHistory } from 'vue-router';
import TextbookDownloadPage from '@/pages/TextbookDownloadPage.vue';
import CourseDownloadPage from '@/pages/CourseDownloadPage.vue';
import DownloadManagerPage from '@/pages/DownloadManagerPage.vue';
import WelcomePage from '@/pages/WelcomePage.vue';
import SettingsPage from '@/pages/SettingsPage.vue';
import DisclaimerPage from '@/pages/DisclaimerPage.vue';
import HelpPage from '@/pages/HelpPage.vue'

const routes = [
  { path: '/', component: WelcomePage },
  { path: '/textbook-download', component: TextbookDownloadPage },
  { path: '/course-download', component: CourseDownloadPage },
  { path: '/downloads', component: DownloadManagerPage },
  { path: '/settings', component: SettingsPage },
  { path: '/disclaimer', component: DisclaimerPage },
  { path: '/help', component: HelpPage },
];

const router = createRouter({
  history: createWebHistory(),
  routes,
});

const app = createApp(App);
app.use(ElementPlus);
app.use(router);
app.mount('#app');

