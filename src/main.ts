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
const Vulnerabilities = () => import('./views/Vulnerabilities.vue');
const Projects = () => import('./views/Projects.vue');
const McpTools = () => import('./views/McpTools.vue');
const Earnings = () => import('./views/Earnings.vue');
const Submissions = () => import('./views/Submissions.vue');
const Settings = () => import('./views/Settings.vue');

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
    path: '/vulnerabilities', 
    name: 'Vulnerabilities', 
    component: Vulnerabilities,
    meta: { title: '漏洞管理' }
  },
  { 
    path: '/projects', 
    name: 'Projects', 
    component: Projects,
    meta: { title: '赏金项目' }
  },
  { 
    path: '/mcp-tools', 
    name: 'McpTools', 
    component: McpTools,
    meta: { title: 'MCP工具' }
  },
  { 
    path: '/earnings', 
    name: 'Earnings', 
    component: Earnings,
    meta: { title: '收益统计' }
  },
  { 
    path: '/submissions', 
    name: 'Submissions', 
    component: Submissions,
    meta: { title: '提交记录' }
  },
  { 
    path: '/settings', 
    name: 'Settings', 
    component: Settings,
    meta: { title: '系统设置' }
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
