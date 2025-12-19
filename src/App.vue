<script setup lang="ts">
import { onMounted, ref, computed, watch, onUnmounted, nextTick } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { listen } from '@tauri-apps/api/event'
import { invoke } from '@tauri-apps/api/core'
import TopNavbar from './components/Layout/TopNavbar.vue'
import Sidebar from './components/Layout/Sidebar.vue'
import LicenseActivation from './components/LicenseActivation.vue'

import Toast from './components/Toast.vue'
import { setLanguage } from './i18n'

const router = useRouter()

// License activation state
const isLicensed = ref(true) // Default to true, check on mount

// 初始化i18n
const { t, locale } = useI18n()




// 侧边栏控制
const sidebarCollapsed = ref(false)
const toggleSidebar = () => {
  sidebarCollapsed.value = !sidebarCollapsed.value
}

// 移动端菜单控制
const showMobileMenu = ref(false)
const toggleMobileMenu = () => {
  showMobileMenu.value = !showMobileMenu.value
}

// 关闭移动端菜单
const closeMobileMenu = () => {
  showMobileMenu.value = false
}

// 注册AI助手快捷键 (Alt+A)
const handleKeyDown = (e: KeyboardEvent) => {
  if (e.key === 'Escape' && showMobileMenu.value) {
    closeMobileMenu()
  }

  if (e.key === 'Backspace') {
    const target = e.target as HTMLElement
    const tagName = target.tagName.toLowerCase()
    const isEditable = target.isContentEditable
    const isInput = tagName === 'input' || tagName === 'textarea'

    if (!isInput && !isEditable) {
      e.preventDefault()
    }
  }
}

const handleClickOutside = (e: MouseEvent) => {
  if (showMobileMenu.value) {
    const target = e.target as Element
    const dropdown = document.querySelector('.dropdown.lg\\:hidden')
    if (dropdown && !dropdown.contains(target)) {
      closeMobileMenu()
    }
  }
}

const setupAIChatShortcut = () => {
  window.addEventListener('keydown', handleKeyDown)
  document.addEventListener('click', handleClickOutside)
}



// Check license status
async function checkLicenseStatus() {
  try {
    const info = await invoke<{ is_licensed: boolean; needs_activation: boolean }>('get_license_info')
    isLicensed.value = info.is_licensed
  } catch (e) {
    console.error('Failed to check license:', e)
    isLicensed.value = true // Fallback to licensed on error
  }
}

function onLicenseActivated() {
  isLicensed.value = true
}

// Shell Permission Handling is now done inline in ShellToolResult component

// 在组件挂载时导航到Dashboard (如果当前在根路径)
onMounted(async () => {
  // Check license first
  await checkLicenseStatus()

  // 只有在根路径时才重定向，避免路由冲突
  if (router.currentRoute.value.path === '/') {
    router.replace('/dashboard')
  }

  // 设置AI助手快捷键
  setupAIChatShortcut()

  // Initialize shell permission handler so backend can send permission requests
  try {
    await invoke('init_shell_permission_handler')
    console.log('Shell permission handler initialized')
  } catch (e) {
    console.error('Failed to init shell permission handler:', e)
  }
})

// 组件卸载时清理事件监听器
onUnmounted(() => {
  window.removeEventListener('keydown', handleKeyDown)
  document.removeEventListener('click', handleClickOutside)
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
  <div id="app" class="h-screen bg-base-100 overflow-hidden">
    <!-- License Activation Dialog -->
    <LicenseActivation v-if="!isLicensed" @activated="onLicenseActivated" />

    <!-- 主应用窗口 -->
    <!-- 顶部导航栏 -->
    <TopNavbar @toggle-sidebar="toggleSidebar" @set-theme="setTheme" @switch-language="switchLanguage" />

    <!-- 主要内容区域 - 受缩放影响 -->
    <div :class="appClasses" :style="appStyles" class="flex h-[calc(100vh-4rem)]">
      <!-- 侧边栏 -->
      <Sidebar :collapsed="sidebarCollapsed"
        class="fixed left-0 top-16 h-[calc(100vh-4rem)] transition-all duration-300 z-1000 " :class="{
          'w-16': sidebarCollapsed,
          'w-64': !sidebarCollapsed
        }" />

      <!-- 主内容区 -->
      <main class="flex-1 transition-all duration-300 overflow-y-auto mt-16" :class="{
        'ml-16': sidebarCollapsed,
        'ml-64': !sidebarCollapsed
      }">
        <!-- 使用 keep-alive 保持组件活跃，确保事件监听器不会丢失 -->
        <router-view v-slot="{ Component }">
          <keep-alive :include="['Passive', 'AIAssistant', 'Vulnerabilities','settings']">
            <component :is="Component" class="min-h-full" />
          </keep-alive>
        </router-view>
      </main>
    </div>

    <Toast />
  </div>
</template>

<style>
/* 页面布局样式 */
.page-content {
  min-height: 100%;
  overflow-y: auto;
}

.page-content-padded {
  min-height: 100%;
  overflow-y: auto;
  padding: 1rem;
}

/* 专门为AI助手等全屏组件设计，不允许滚动 */
.page-content-full {
  height: 100%;
  overflow: hidden;
  padding: 0;
}

/* 可滚动的内容区域 */
.page-content-scrollable {
  height: 100%;
  overflow-y: auto;
  padding: 1rem;
}

/* 防止内容被导航栏遮挡的安全区域 */
.safe-top {
  padding-top: 1rem;
}

.safe-top-lg {
  padding-top: 2rem;
}

/* 对于需要完整视口高度的组件，确保不被遮挡 */
.navbar-safe-area {
  min-height: calc(100% - 1rem);
  padding-top: 0.5rem;
}

/* 全局样式 */
html,
body {
  height: 100%;
  margin: 0;
  padding: 0;
  font-family: 'Inter', system-ui, sans-serif;
}

/* 字体大小设置 - 现在基于系统设置 */
.font-size-small {
  font-size: calc(var(--font-size-base, 14px) * 0.875);
}

.font-size-normal {
  font-size: var(--font-size-base, 14px);
}

.font-size-large {
  font-size: calc(var(--font-size-base, 14px) * 1.125);
}

.font-size-xlarge {
  font-size: calc(var(--font-size-base, 14px) * 1.25);
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
  background: var(--fallback-b3, oklch(var(--b3)/1));
  border-radius: 8px;
}

::-webkit-scrollbar-thumb {
  background: var(--fallback-b2, oklch(var(--b2)/1));
  border-radius: 8px;
}

::-webkit-scrollbar-thumb:hover {
  background: var(--fallback-n, oklch(var(--n)/1));
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

/* 导航栏响应式优化 */
.navbar {
  padding-left: 0.5rem;
  padding-right: 0.5rem;
}

@media (min-width: 640px) {
  .navbar {
    padding-left: 1rem;
    padding-right: 1rem;
  }
}

/* 移动端下拉菜单动画 */
.dropdown-content {
  animation: slideDown 0.2s ease-out;
  max-height: 70vh;
  overflow-y: auto;
}

@keyframes slideDown {
  from {
    opacity: 0;
    transform: translateY(-10px);
  }

  to {
    opacity: 1;
    transform: translateY(0);
  }
}

/* 确保导航栏在移动端有足够的空间 */
@media (max-width: 1023px) {
  .navbar {
    min-height: 4rem;
  }

  /* 当移动端菜单展开时，确保有足够的空间 */
  .dropdown.lg\:hidden .dropdown-content {
    position: absolute;
    top: 100%;
    left: 0;
    right: auto;
    margin-top: 0.5rem;
    z-index: 1000;
  }
}

/* 确保导航栏按钮在小屏幕上不会过小 */
@media (max-width: 639px) {
  .btn-circle {
    min-height: 2.5rem;
    min-width: 2.5rem;
  }
}

/* 导航栏文字在中等屏幕上的优化 */
@media (min-width: 1024px) and (max-width: 1279px) {
  .navbar-center .menu li a {
    padding-left: 0.5rem;
    padding-right: 0.5rem;
  }
}

/* 防止导航栏内容溢出 */
.navbar-start,
.navbar-center,
.navbar-end {
  min-width: 0;
}

.navbar-start {
  flex: 0 1 auto;
}

.navbar-center {
  flex: 1 1 auto;
  justify-content: center;
}

.navbar-end {
  flex: 0 1 auto;
}
</style>
