<script setup lang="ts">
import { onMounted, ref, computed, watch, onUnmounted, nextTick } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import Sidebar from './components/Layout/Sidebar.vue'
import AIChat from './components/AIChat.vue'
import { setLanguage } from './i18n'

const router = useRouter()

// 初始化i18n
const { t, locale } = useI18n()

// AI助手控制
const showAIChat = ref(false)

// 切换AI助手显示状态
const toggleAIChat = () => {
  showAIChat.value = !showAIChat.value
}

// 侧边栏控制
const sidebarCollapsed = ref(false)
const toggleSidebar = () => {
  sidebarCollapsed.value = !sidebarCollapsed.value
}

// 注册AI助手快捷键 (Alt+A)
const setupAIChatShortcut = () => {
  const handleKeyDown = (e: KeyboardEvent) => {
    if (e.altKey && e.key === 'a') {
      toggleAIChat()
    }
  }
  
  window.addEventListener('keydown', handleKeyDown)
  
  // 监听自定义事件，用于响应侧边栏的AI助手按钮点击
  window.addEventListener('toggle-ai-chat', () => {
    toggleAIChat()
  })
  
  onUnmounted(() => {
    window.removeEventListener('keydown', handleKeyDown)
    window.removeEventListener('toggle-ai-chat', toggleAIChat)
  })
}

// 在组件挂载时导航到Dashboard (如果当前在根路径)
onMounted(() => {
  // 只有在根路径时才重定向，避免路由冲突
  if (router.currentRoute.value.path === '/') {
    router.replace('/dashboard')
  }
  
  // 设置AI助手快捷键
  setupAIChatShortcut()
})

// 主题管理
const setTheme = (theme: string) => {
  document.documentElement.setAttribute('data-theme', theme);
  localStorage.setItem('theme', theme);
};

// 语言管理
const switchLanguage = (lang: string) => {
  setLanguage(lang as 'zh' | 'en');
};

// 可用语言
const availableLanguages = [
  { code: 'zh', name: '中文', icon: 'fa-language' },
  { code: 'en', name: 'English', icon: 'fa-globe' }
];

// 可用主题
const availableThemes = [
  { code: 'light', name: t('settings.themes.light'), icon: 'fa-sun' },
  { code: 'dark', name: t('settings.themes.dark'), icon: 'fa-moon' },
  { code: 'corporate', name: t('settings.themes.corporate'), icon: 'fa-building' }
];

onMounted(() => {
  // 恢复保存的主题
  const savedTheme = localStorage.getItem('theme') || 'light';
  setTheme(savedTheme);
  
  // 加载FontAwesome
  const link = document.createElement('link');
  link.href = 'https://cdnjs.cloudflare.com/ajax/libs/font-awesome/6.4.0/css/all.min.css';
  link.rel = 'stylesheet';
  document.head.appendChild(link);
})

// 字体大小和界面缩放
const fontSize = ref('normal')
const uiScale = ref(100)

// 计算应用的样式类
const appClasses = computed(() => {
  const classes = []
  
  // 添加字体大小类
  classes.push(`font-size-${fontSize.value}`)
  
  return classes.join(' ')
})

// 计算应用的内联样式
const appStyles = computed(() => {
  return {
    transform: `scale(${uiScale.value / 100})`,
    transformOrigin: 'top left',
    width: uiScale.value !== 100 ? `${10000 / uiScale.value}%` : '100%',
    height: uiScale.value !== 100 ? `${10000 / uiScale.value}%` : '100%'
  }
})

// 从localStorage加载设置
onMounted(() => {
  const savedSettings = localStorage.getItem('sentinel-settings')
  if (savedSettings) {
    try {
      const settings: any = JSON.parse(savedSettings)
      fontSize.value = settings.system?.fontSize || 'normal'
      uiScale.value = settings.system?.uiScale || 100
    } catch (error) {
      console.error(t('settings.saveFailed'), error)
    }
  }
})

// 监听设置变化并保存
watch([fontSize, uiScale], () => {
  const savedSettings = localStorage.getItem('sentinel-settings')
  let settings: any = {}
  
  if (savedSettings) {
    try {
      settings = JSON.parse(savedSettings)
    } catch (error) {
      console.error(t('settings.saveFailed'), error)
    }
  }
  
  if (!settings.system) {
    settings.system = {}
  }
  
  settings.system.fontSize = fontSize.value
  settings.system.uiScale = uiScale.value
  
  localStorage.setItem('sentinel-settings', JSON.stringify(settings))
})

// 暴露给全局使用
declare global {
  interface Window {
    updateFontSize: (newSize: string) => void
    updateUIScale: (newScale: number) => void
  }
}

window.updateFontSize = (newSize: string) => {
  fontSize.value = newSize
}

window.updateUIScale = (newScale: number) => {
  uiScale.value = newScale
}
</script>

<template>
  <div id="app" :class="appClasses" :style="appStyles" class="min-h-screen bg-base-100">
    <!-- 顶部导航栏 -->
    <div class="navbar bg-base-200 shadow-md z-50">
      <div class="navbar-start">
        <button @click="toggleSidebar" class="btn btn-ghost btn-circle">
          <i class="fas fa-bars"></i>
        </button>
        <router-link to="/" class="btn btn-ghost normal-case text-xl">
          <i class="fas fa-shield-alt text-primary mr-2"></i>Sentinel AI
        </router-link>
      </div>
      
      <div class="navbar-center hidden lg:flex">
        <ul class="menu menu-horizontal px-1 gap-1">
          <li>
            <router-link to="/dashboard" class="rounded-lg">
              <i class="fas fa-home mr-2"></i>{{ t('sidebar.dashboard') }}
            </router-link>
          </li>
          <li>
            <router-link to="/projects" class="rounded-lg">
              <i class="fas fa-trophy mr-2"></i>{{ t('sidebar.projects') }}
            </router-link>
          </li>
          <li>
            <router-link to="/scan-tasks" class="rounded-lg">
              <i class="fas fa-search mr-2"></i>{{ t('sidebar.scanTasks') }}
            </router-link>
          </li>
          <li>
            <router-link to="/vulnerabilities" class="rounded-lg">
              <i class="fas fa-bug mr-2"></i>{{ t('sidebar.vulnerabilities') }}
            </router-link>
          </li>
          <li>
            <router-link to="/submissions" class="nav-link">
              <i class="fas fa-paper-plane mr-2"></i>{{ t('sidebar.submissions') }}
            </router-link>
          </li>
          <li>
            <router-link to="/earnings" class="nav-link">
              <i class="fas fa-dollar-sign mr-2"></i>{{ t('sidebar.earnings') }}
            </router-link>
          </li>
          <li>
            <router-link to="/mcp-tools" class="nav-link">
              <i class="fas fa-tools mr-2"></i>{{ t('sidebar.mcpTools') }}
            </router-link>
          </li>
          <li>
            <router-link to="/settings" class="nav-link">
              <i class="fas fa-cog mr-2"></i>{{ t('sidebar.settings') }}
            </router-link>
          </li>

        </ul>
      </div>
      
      <div class="navbar-end">
        <!-- AI助手按钮 -->
        <button 
          class="btn btn-ghost btn-circle tooltip tooltip-bottom" 
          data-tip="AI助手 (Alt+A)"
          @click="toggleAIChat"
        >
          <i class="fas fa-robot text-xl" :class="{'text-primary': showAIChat}"></i>
        </button>
        
        <!-- 语言切换器 -->
        <div class="dropdown dropdown-end">
          <div tabindex="0" role="button" class="btn btn-ghost btn-circle">
            <i class="fas fa-language text-xl"></i>
          </div>
          <ul tabindex="0" class="dropdown-content z-[1] menu p-2 shadow bg-base-100 rounded-box w-36">
            <li v-for="lang in availableLanguages" :key="lang.code">
              <a @click="switchLanguage(lang.code)" :class="{ 'active': locale === lang.code }">
                <i :class="`fas ${lang.icon} mr-2`"></i>{{ lang.name }}
              </a>
            </li>
          </ul>
        </div>
        
        <!-- 主题切换器 -->
        <div class="dropdown dropdown-end">
          <div tabindex="0" role="button" class="btn btn-ghost btn-circle">
            <i class="fas fa-palette text-xl"></i>
          </div>
          <ul tabindex="0" class="dropdown-content z-[1] menu p-2 shadow bg-base-100 rounded-box w-52">
            <li v-for="theme in availableThemes" :key="theme.code">
              <a @click="setTheme(theme.code)">
                <i :class="`fas ${theme.icon} mr-2`"></i>{{ theme.name }}
              </a>
            </li>
          </ul>
        </div>
        
      </div>
    </div>

    <!-- 主要内容区域 -->
    <div class="flex">
      <!-- 侧边栏 -->
      <div :class="{'w-80': !sidebarCollapsed, 'w-20': sidebarCollapsed}" class="transition-all duration-300 ease-in-out">
        <Sidebar :collapsed="sidebarCollapsed" />
      </div>
      
      <!-- 主内容区 -->
      <div class="flex-1 p-4 overflow-auto">
        <router-view />
      </div>
    </div>
    
    <!-- AI助手聊天框 -->
    <AIChat v-if="showAIChat" @close="showAIChat = false" />
  </div>
</template>

<style>
/* 全局样式 */
html, body {
  height: 100%;
  margin: 0;
  padding: 0;
  font-family: 'Inter', system-ui, sans-serif;
}

/* 字体大小设置 */
.font-size-small {
  font-size: 0.875rem;
}

.font-size-normal {
  font-size: 1rem;
}

.font-size-large {
  font-size: 1.125rem;
}

.font-size-xlarge {
  font-size: 1.25rem;
}

/* 活动路由样式 */
.router-link-active {
  @apply bg-primary/10 text-primary;
}

/* 自定义滚动条 */
::-webkit-scrollbar {
  width: 6px;
  height: 6px;
}

::-webkit-scrollbar-track {
  background: var(--fallback-b3,oklch(var(--b3)/1));
  border-radius: 8px;
}

::-webkit-scrollbar-thumb {
  background: var(--fallback-b2,oklch(var(--b2)/1));
  border-radius: 8px;
}

::-webkit-scrollbar-thumb:hover {
  background: var(--fallback-n,oklch(var(--n)/1));
}

/* 过渡动画 */
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.3s ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}

.slide-enter-active,
.slide-leave-active {
  transition: transform 0.3s ease;
}

.slide-enter-from,
.slide-leave-to {
  transform: translateX(-100%);
}
</style>