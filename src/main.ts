import { createApp } from 'vue';
import './style.css';
import App from './App.vue';
import ElementPlus from 'element-plus';
import 'element-plus/dist/index.css';
import 'element-plus/theme-chalk/dark/css-vars.css';
import { createRouter, createWebHistory } from 'vue-router';
import TextbookDownloadPage from '@/pages/TextbookDownloadPage.vue';
import SettingsPage from '@/pages/SettingsPage.vue';
import DisclaimerPage from '@/pages/DisclaimerPage.vue';
import HelpPage from '@/pages/HelpPage.vue'

const routes = [
  { path: '/', component: TextbookDownloadPage },
  { path: '/textbook-download', component: TextbookDownloadPage },
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

