import { createApp } from "vue";
import { createPinia } from 'pinia';
import { createRouter, createWebHistory } from 'vue-router';
import App from "./App.vue";
import "./style.css";
import { performanceService } from './services/performance';
import i18n from './i18n'; // 导入i18n配置
import DialogPlugin from './composables/useDialog'; // 导入对话框插件

// 懒加载页面组件 - 性能优化
const Dashboard = () => import('./views/Dashboard.vue');
const ScanTasks = () => import('./views/ScanTasks.vue');
const ScanSessions = () => import('./views/ScanSessions.vue');
const Vulnerabilities = () => import('./views/Vulnerabilities.vue');
const McpTools = () => import('./views/McpTools.vue');
const DictionaryManagement = () => import('./views/DictionaryManagement.vue');
const Settings = () => import('./views/Settings.vue');
const PerformanceMonitor = () => import('./components/PerformanceMonitor.vue');

// 创建路由配置
const routes = [
  { 
    path: '/', 
    redirect: '/dashboard'
  },
  { 
    path: '/dashboard', 
    name: 'DashboardAlias', 
    component: Dashboard,
    meta: { title: '总览' }
  },
  { 
    path: '/scan-tasks', 
    name: 'ScanTasks', 
    component: ScanTasks,
    meta: { title: '扫描任务' }
  },
  { 
    path: '/scan-sessions', 
    name: 'ScanSessions', 
    component: ScanSessions,
    meta: { title: '智能扫描会话' }
  },
  { 
    path: '/vulnerabilities', 
    name: 'Vulnerabilities', 
    component: Vulnerabilities,
    meta: { title: '漏洞管理' }
  },
  { 
    path: '/mcp-tools', 
    name: 'McpTools', 
    component: McpTools,
    meta: { title: 'MCP工具' }
  },
  { 
    path: '/dictionary', 
    name: 'DictionaryManagement', 
    component: DictionaryManagement,
    meta: { title: '字典管理' }
  },
  { 
    path: '/settings', 
    name: 'Settings', 
    component: Settings,
    meta: { title: '系统设置' }
  },
  { 
    path: '/performance', 
    name: 'PerformanceMonitor', 
    component: PerformanceMonitor,
    meta: { title: '性能监控' }
  },
];

const router = createRouter({
  history: createWebHistory(),
  routes,
});

// 路由守卫 - 性能监控和页面标题
router.beforeEach((to, _from, next) => {
  // 设置页面标题
  if (to.meta?.title) {
    document.title = `${to.meta.title} - Sentinel AI`;
  }
  
  // 开始路由性能监控
  performanceService.markRouteStart(to.path);
  
  // 开发环境日志
  if (import.meta.env.DEV) {
    console.time(`Route: ${to.path}`);
  }
  
  next();
});

router.afterEach((to) => {
  // 结束路由性能监控
  performanceService.markRouteEnd(to.path);
  
  // 开发环境日志
  if (import.meta.env.DEV) {
    console.timeEnd(`Route: ${to.path}`);
    console.log('Performance Metrics:', performanceService.getMetrics());
  }
});

// 创建Pinia store
const pinia = createPinia();

// 创建应用
const app = createApp(App);

app.use(pinia);
app.use(router);
app.use(i18n); // 使用i18n
app.use(DialogPlugin); // 注册对话框插件

app.mount("#app");
