import { createApp } from "vue";
import { createPinia } from 'pinia';
import { createRouter, createWebHistory, RouteRecordRaw } from 'vue-router';
import App from "./App.vue";
import "./style.css";
import { performanceService } from './services/performance';
import i18n from './i18n'; // 导入i18n配置
import DialogPlugin from './composables/useDialog'; // 导入对话框插件
import ToastPlugin from './composables/useToast'; // 导入Toast插件
import { invoke } from '@tauri-apps/api/core'; // 导入Tauri API

// 启动时应用已保存的通用设置（主题/字体/语言）
const applyStartupSettings = () => {
  try {
    const saved = localStorage.getItem('sentinel-settings')
    if (!saved) return
    const parsed = JSON.parse(saved)
    const general = parsed?.general || {}

    // 主题
    if (general.theme) {
      let finalTheme = general.theme
      if (finalTheme === 'auto') {
        finalTheme = window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'
      }
      document.documentElement.setAttribute('data-theme', finalTheme)
      localStorage.setItem('theme', finalTheme)
    }

    // 字体大小
    if (typeof general.fontSize === 'number') {
      document.documentElement.style.fontSize = `${general.fontSize}px`
      document.documentElement.style.setProperty('--font-size-base', `${general.fontSize}px`)
    }

    // 语言
    if (general.language) {
      let finalLang = general.language as string
      if (finalLang === 'auto') {
        const browserLang = navigator.language.toLowerCase()
        if (browserLang.startsWith('zh')) {
          finalLang = browserLang.includes('tw') || browserLang.includes('hk') ? 'zh-TW' : 'zh-CN'
        } else if (browserLang.startsWith('en')) {
          finalLang = 'en-US'
        } else {
          finalLang = 'zh-CN'
        }
      }
      const langCode = finalLang.split('-')[0]
      // 使用 i18n 实例设置语言
      try {
        // i18n 在下方 app.use(i18n) 前，这里直接设置全局值可能无效；
        // 但我们先把本地值存起来，i18n 初始化会读取。
        localStorage.setItem('sentinel-language', langCode)
      } catch {}
    }
  } catch (e) {
    console.warn('applyStartupSettings failed', e)
  }
}

// 懒加载页面组件 - 性能优化
const Dashboard = () => import('./views/Dashboard.vue');
const ScanTasks = () => import('./views/ScanTasks.vue');
const ScanSessions = () => import('./views/ScanSessions.vue');
const Vulnerabilities = () => import('./views/Vulnerabilities.vue');
const AssetManagement = () => import('./views/AssetManagement.vue');
const McpTools = () => import('./views/McpTools.vue');
const DictionaryManagement = () => import('./views/DictionaryManagement.vue');

const AgentManagerView = () => import('./views/AgentManager.vue');
const WorkflowMonitor = () => import('./views/WorkflowMonitor.vue');
// const IntelligentSecurityTest = () => import('./components/IntelligentSecurityTest.vue');
// const PlanExecuteDemo = () => import('./components/PlanExecuteDemo.vue');
// const ReWOOTestPanel = () => import('./components/ReWOOTestPanel.vue');
// const LLMCompilerTest = () => import('./views/LLMCompilerTest.vue');
const PromptManagement = () => import('./views/PromptManagement.vue');
const AIAssistant = () => import('./views/AIAssistant.vue');
const RAGManagement = () => import('./views/RAGManagement.vue');

const Settings = () => import('./views/Settings.vue');
const PerformanceMonitor = () => import('./components/PerformanceMonitor.vue');

// 创建路由配置
const routes = [
  { 
    path: '/', 
    redirect: '/dashboard'
  },
  { 
    path: '/prompts', 
    name: 'PromptManagement', 
    component: PromptManagement,
    meta: { title: 'Prompt管理' }
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
    path: '/assets', 
    name: 'AssetManagement', 
    component: AssetManagement,
    meta: { title: '资产管理' }
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
    path: '/agent-manager', 
    name: 'AgentManager', 
    component: AgentManagerView,
    meta: { title: 'Agent管理' }
  },
  { 
    path: '/ai-assistant', 
    name: 'AIAssistant', 
    component: AIAssistant,
    meta: { title: 'AI助手' }
  },
  { 
    path: '/rag-management', 
    name: 'RAGManagement', 
    component: RAGManagement,
    meta: { title: 'RAG管理' }
  },
  { 
    path: '/workflow-monitor', 
    name: 'WorkflowMonitor', 
    component: WorkflowMonitor,
    meta: { title: '工作流监控' }
  },
  // { 
  //   path: '/plan-execute', 
  //   name: 'PlanExecuteDemo', 
  //   component: PlanExecuteDemo,
  //   meta: { title: 'Plan-and-Execute 演示' }
  // },
  // { 
  //   path: '/rewoo-test', 
  //   name: 'ReWOOTestPanel', 
  //   component: ReWOOTestPanel,
  //   meta: { title: 'ReWOO 架构测试' }
  // },
  // { 
  //   path: '/llm-compiler-test', 
  //   name: 'LLMCompilerTest', 
  //   component: LLMCompilerTest,
  //   meta: { title: 'LLMCompiler 引擎测试' }
  // },

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
  routes: routes as RouteRecordRaw[],
});

// 存储已创建的计时器
const activeTimers = new Set<string>();

// 修复路由守卫中的计时器重复问题
router.beforeEach((to, _from, next) => {
  // 设置页面标题
  if (to.meta?.title) {
    document.title = `${to.meta.title} - Sentinel AI`;
  }
  
  // 开始路由性能监控
  performanceService.markRouteStart(to.path);
  
  // 开发环境日志 - 检查计时器是否已存在
  if (import.meta.env.DEV) {
    const timerKey = `Route: ${to.path}`;
    if (!activeTimers.has(timerKey)) {
      console.time(timerKey);
      activeTimers.add(timerKey);
    }
  }
  
  next();
});

router.afterEach((to) => {
  // 结束路由性能监控
  performanceService.markRouteEnd(to.path);
  
  // 开发环境日志
  if (import.meta.env.DEV) {
    const timerKey = `Route: ${to.path}`;
    if (activeTimers.has(timerKey)) {
      try {
        console.timeEnd(timerKey);
        activeTimers.delete(timerKey);
      } catch (error) {
        // 忽略计时器不存在的错误
        console.log(`Route navigation to ${to.path} completed`);
        activeTimers.delete(timerKey);
      }
    }
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
app.use(ToastPlugin); // 注册Toast插件

// 初始化核心系统组件
const initializeCoreComponents = async () => {
  try {
    // 初始化Agent管理器
    await invoke('initialize_agent_manager');
  } catch (error) {
    console.warn('Agent manager initialization warning:', error);
  }
};

// 在应用挂载前应用本地持久化的通用设置
applyStartupSettings();

// 在应用挂载后初始化核心组件
app.mount("#app");

// 延迟初始化以确保应用已完全加载
setTimeout(() => {
  initializeCoreComponents();
}, 1000);
