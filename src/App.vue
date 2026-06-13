<script setup lang="ts">
import { onMounted, ref, computed, watch, onUnmounted, nextTick, provide } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import TopNavbar from './components/Layout/TopNavbar.vue'
import GlobalSearchPalette from './components/Layout/GlobalSearchPalette.vue'
import ImmersiveDrillDock from './components/Layout/ImmersiveDrillDock.vue'
import ImmersiveMinimizedToolTray from './components/Layout/ImmersiveMinimizedToolTray.vue'
import ImmersiveSecurityCenterOverlay from './components/Layout/ImmersiveSecurityCenterOverlay.vue'
import Sidebar from './components/Layout/Sidebar.vue'
import AppDialog from './components/AppDialog.vue'
import GlobalPluginEditor from './components/PluginManagement/GlobalPluginEditor.vue'
import AskUserQuestionModal from './components/Agent/AskUserQuestionModal.vue'

import LicenseActivation from './components/LicenseActivation.vue'
import Toast from './components/Toast.vue'
import { setLanguage } from './i18n'
import { isGlobalSearchShortcut, requestGlobalSearchOpen } from './services/globalSearchFocus'
import { refreshFeatureEntitlements } from './services/featureEntitlements'
import {
  immersiveDrillModeEnabled,
  toggleImmersiveDrillMode,
} from './services/immersiveDrillMode'
import { closeTrafficAssistant } from './services/trafficAssistantWorkspace'
import { applyTheme } from './views/settingsUiSupport'
import { isEditableKeyboardEvent, isEditableKeyboardTarget } from './utils/editableKeyboardTarget'

const router = useRouter()
const route = useRoute()
const isStandaloneRoute = computed(() => Boolean(route.meta?.standalone))
const licenseActivationRef = ref<InstanceType<typeof LicenseActivation> | null>(null)
provide('openLicenseDialog', () => licenseActivationRef.value?.openDialog())
const routeKeepAliveIncludes = [
  'CyberChefView',
  'BotConsole',
  'WorkflowStudio',
  'AgentManagement',
  'PluginManagement',
  'BugBountyView',
  'AIAssistant',
]
const mainContentRef = ref<HTMLElement | null>(null)
const SECURITY_DISCLAIMER_STORAGE_KEY = 'sentinel:security-disclaimer:accepted:v1'
const routeScrollPositions = new Map<string, number>()
const NAVBAR_VISIBILITY_STORAGE_KEY = 'sentinel:navbar:visible:v1'

// 初始化i18n
const { t, locale } = useI18n()




// 顶栏与侧边栏控制
const sidebarCollapsed = ref(false)
const navbarVisible = ref(
  window.localStorage.getItem(NAVBAR_VISIBILITY_STORAGE_KEY) !== 'false',
)
const securityDisclaimerOpen = ref(false)
const toggleSidebar = () => {
  sidebarCollapsed.value = !sidebarCollapsed.value
}

const toggleNavbarVisibility = () => {
  navbarVisible.value = !navbarVisible.value
  window.localStorage.setItem(NAVBAR_VISIBILITY_STORAGE_KEY, String(navbarVisible.value))
}

const showNavbar = () => {
  navbarVisible.value = true
  window.localStorage.setItem(NAVBAR_VISIBILITY_STORAGE_KEY, 'true')
}

const showStartupSecurityDisclaimer = () => {
  securityDisclaimerOpen.value =
    window.localStorage.getItem(SECURITY_DISCLAIMER_STORAGE_KEY) !== 'true'
}

const acceptSecurityDisclaimer = () => {
  window.localStorage.setItem(SECURITY_DISCLAIMER_STORAGE_KEY, 'true')
  securityDisclaimerOpen.value = false
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

const saveMainScrollPosition = (routeKey: string) => {
  if (!routeKey || isStandaloneRoute.value) {
    return
  }

  const container = mainContentRef.value
  if (!container) {
    return
  }

  routeScrollPositions.set(routeKey, container.scrollTop)
}

const restoreMainScrollPosition = (routeKey: string) => {
  if (!routeKey || isStandaloneRoute.value) {
    return
  }

  const container = mainContentRef.value
  if (!container) {
    return
  }

  container.scrollTop = routeScrollPositions.get(routeKey) ?? 0
}

const openGlobalSearch = async () => {
  if (isStandaloneRoute.value) {
    return
  }

  requestGlobalSearchOpen()
}

// 注册AI助手快捷键 (Alt+A)
const handleKeyDown = (e: KeyboardEvent) => {
  if (e.key === 'Escape' && showMobileMenu.value) {
    closeMobileMenu()
  }

  if (isGlobalSearchShortcut(e) && !isEditableKeyboardTarget(e.target)) {
    e.preventDefault()
    void openGlobalSearch()
    return
  }

  if (e.key === 'Backspace') {
    if (!isEditableKeyboardEvent(e)) {
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



// Shell Permission Handling is now done inline in ShellToolResult component

// 在组件挂载时导航到Dashboard (如果当前在根路径)
onMounted(async () => {
  showStartupSecurityDisclaimer()
  void setLanguage((locale.value.startsWith('zh') ? 'zh' : 'en') as 'zh' | 'en')

  await refreshFeatureEntitlements()

  // 只有在根路径时才重定向，避免路由冲突
  if (router.currentRoute.value.path === '/' && !isStandaloneRoute.value) {
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

  requestAnimationFrame(() => {
    restoreMainScrollPosition(route.fullPath)
  })
})

// 组件卸载时清理事件监听器
onUnmounted(() => {
  saveMainScrollPosition(route.fullPath)
  window.removeEventListener('keydown', handleKeyDown)
  document.removeEventListener('click', handleClickOutside)
})

watch(
  () => route.fullPath,
  async (newRouteKey, oldRouteKey) => {
    if (oldRouteKey) {
      saveMainScrollPosition(oldRouteKey)
    }

    await nextTick()
    requestAnimationFrame(() => {
      restoreMainScrollPosition(newRouteKey)
    })
  },
)

const updateStoredGeneralSettings = (mutate: (general: Record<string, any>) => void) => {
  const savedSettings = localStorage.getItem('sentinel-settings')
  let settings: Record<string, any> = {}

  if (savedSettings) {
    try {
      settings = JSON.parse(savedSettings)
    } catch (error) {
      console.error(t('settings.saveFailed'), error)
    }
  }

  if (!settings.general || typeof settings.general !== 'object') {
    settings.general = {}
  }

  mutate(settings.general)
  localStorage.setItem('sentinel-settings', JSON.stringify(settings))
  return settings
}

const setTheme = (theme: string) => {
  const settings = updateStoredGeneralSettings(general => {
    general.theme = theme
  })
  applyTheme(theme, settings)
}

const switchLanguage = async (lang: string) => {
  const nextLang = (lang.startsWith('zh') ? 'zh' : 'en') as 'zh' | 'en'
  updateStoredGeneralSettings(general => {
    general.language = nextLang
  })
  await setLanguage(nextLang)
}

const availableLanguages = [
  { code: 'zh', name: '中文', icon: 'fa-language' },
  { code: 'en', name: 'English', icon: 'fa-globe' },
]

const availableThemes = [
  { code: 'light', name: t('settings.themes.light'), icon: 'fa-sun' },
  { code: 'dark', name: t('settings.themes.dark'), icon: 'fa-moon' },
  { code: 'corporate', name: t('settings.themes.corporate'), icon: 'fa-building' },
]

const shouldShowNavbar = computed(
  () => !isStandaloneRoute.value && !immersiveDrillModeEnabled.value && navbarVisible.value,
)

const shouldShowSidebar = computed(
  () => !isStandaloneRoute.value && !immersiveDrillModeEnabled.value,
)

const handleNavbarImmersiveDrillModeToggle = () => {
  if (!immersiveDrillModeEnabled.value) {
    closeTrafficAssistant()
  }

  toggleImmersiveDrillMode()
}

const appShellStyle = computed(() => ({
  '--app-navbar-height': shouldShowNavbar.value ? '4rem' : '0px',
}))

const appViewportStyle = computed(() => ({
  height: '100vh',
}))

const sidebarStyle = computed(() => ({
  top: 'var(--app-navbar-height, 4rem)',
  height: 'calc(100vh - var(--app-navbar-height, 4rem))',
}))

const mainContentStyle = computed(() => ({
  marginTop: 'var(--app-navbar-height, 4rem)',
}))

watch(
  () => immersiveDrillModeEnabled.value,
  enabled => {
    if (!enabled || isStandaloneRoute.value) {
      return
    }

    if (route.path !== '/traffic') {
      void router.push('/traffic')
    }
  },
  { immediate: true },
)
</script>

<template>
  <div id="app" class="h-screen bg-base-100 overflow-hidden" :style="appShellStyle">
    <AppDialog :open="securityDisclaimerOpen" @cancel.prevent>
      <div class="modal-box max-w-2xl">
        <div class="flex items-start gap-4">
          <div class="flex h-12 w-12 shrink-0 items-center justify-center rounded-xl bg-warning/10 text-warning">
            <i class="fas fa-shield-alt text-2xl"></i>
          </div>
          <div class="min-w-0">
            <h2 class="text-xl font-semibold text-base-content">授权安全测试免责声明</h2>
            <p class="mt-2 text-sm leading-6 text-base-content/70">
              本应用仅可用于已获得明确授权的安全测试、漏洞验证、资产自查、应急响应与防护能力评估。
            </p>
          </div>
        </div>

        <div class="mt-5 space-y-3 text-sm leading-6 text-base-content/75">
          <p>
            使用前请确认你对目标系统、网络、账号、数据和第三方服务拥有合法授权，并遵守适用法律法规、平台规则、客户授权范围和内部安全制度。
          </p>
          <p>
            请勿将本应用用于未授权扫描、入侵、绕过访问控制、破坏服务可用性、窃取或扩散敏感数据、规避审计、批量滥用接口，或任何可能损害他人系统与权益的行为。
          </p>
          <p>
            涉及漏洞利用、流量重放、自动化探测、Bot 接入和插件执行时，请优先使用低风险配置，控制请求频率与影响范围，并保留授权、测试计划和操作记录。
          </p>
        </div>

        <div class="mt-5 rounded-lg border border-warning/30 bg-warning/5 px-4 py-3 text-sm leading-6 text-base-content/75">
          继续使用即表示你已理解并承诺：所有操作均在授权范围内进行，测试结果仅用于合法的安全改进、验证和报告。
        </div>

        <div class="modal-action">
          <button type="button" class="btn btn-primary" @click="acceptSecurityDisclaimer">
            我已知悉并承诺合规使用
          </button>
        </div>
      </div>
    </AppDialog>
    <GlobalSearchPalette v-if="!isStandaloneRoute" />

    <template v-if="!isStandaloneRoute">
      <TopNavbar
        v-if="shouldShowNavbar"
        @toggle-sidebar="toggleSidebar"
        @toggle-navbar-visibility="toggleNavbarVisibility"
        @toggle-immersive-drill-mode="handleNavbarImmersiveDrillModeToggle"
        @set-theme="setTheme"
        @switch-language="switchLanguage"
      />

      <button
        v-if="!shouldShowNavbar && !isStandaloneRoute && !immersiveDrillModeEnabled"
        type="button"
        class="fixed right-4 top-4 z-[1100] btn btn-sm btn-primary shadow-lg"
        :title="t('common.showNavbar', '显示导航栏')"
        :aria-label="t('common.showNavbar', '显示导航栏')"
        @click="showNavbar"
      >
        <i class="fas fa-chevron-down mr-2"></i>
        {{ t('common.showNavbar', '显示导航栏') }}
      </button>

      <div :style="appViewportStyle" class="flex">
        <Sidebar
          v-if="shouldShowSidebar"
          :collapsed="sidebarCollapsed"
          class="fixed left-0 transition-all duration-300 z-1000 " :style="sidebarStyle" :class="{
            'w-16': sidebarCollapsed,
            'w-64': !sidebarCollapsed
          }" />

        <main ref="mainContentRef" class="flex-1 transition-all duration-300 overflow-y-auto" :style="mainContentStyle" :class="{
          'ml-0': !shouldShowSidebar,
          'ml-16': shouldShowSidebar && sidebarCollapsed,
          'ml-64': shouldShowSidebar && !sidebarCollapsed
        }">
          <router-view v-slot="{ Component }">
            <keep-alive :include="routeKeepAliveIncludes" :max="4">
              <component
                :is="Component"
                class="min-h-full"
              />
            </keep-alive>
          </router-view>
        </main>
      </div>

      <ImmersiveDrillDock v-if="immersiveDrillModeEnabled" />
      <ImmersiveMinimizedToolTray v-if="immersiveDrillModeEnabled" />
      <ImmersiveSecurityCenterOverlay v-if="immersiveDrillModeEnabled" />
      <GlobalPluginEditor />
    </template>

    <template v-else>
      <router-view />
    </template>

    <LicenseActivation ref="licenseActivationRef" />
    <Toast />
    <AskUserQuestionModal />
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
  font-family: var(--app-font-sans);
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
