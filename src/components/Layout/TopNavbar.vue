<template>
  <div class="navbar bg-base-200 shadow-md z-50 fixed top-0 left-0 right-0 min-h-16 h-auto">
    <div class="navbar-start">
      <!-- 侧边栏切换按钮 -->
      <button @click="toggleSidebar" class="btn btn-ghost btn-circle">
        <i class="fas fa-bars"></i>
      </button>

      <!-- Logo -->
      <router-link to="/" class="btn btn-ghost normal-case text-lg sm:text-xl flex-shrink-0 ml-2">
        <i class="fas fa-shield-alt text-primary mr-1 sm:mr-2"></i>
        <span class="hidden sm:inline">Sentinel AI</span>
        <span class="sm:hidden">Sentinel</span>
      </router-link>
    </div>

    <!-- 中间区域 - 可以放置搜索框或其他功能 -->
    <div class="navbar-center hidden lg:flex">
      <!-- 全局搜索框 -->
      <div ref="searchContainerRef" class="form-control relative">
        <div class="input-group">
          <input
            ref="searchInputRef"
            type="text"
            placeholder="搜索页面、功能、消息..."
            class="input input-bordered input-sm w-64"
            v-model="searchQuery"
            @focus="handleSearchFocus"
            @keydown="handleSearchKeydown"
          />
          <button class="btn btn-square btn-sm" @click="performSearch">
            <i class="fas fa-search"></i>
          </button>
        </div>
        <TopNavbarSearchResults
          :visible="showSearchResults"
          :query="trimmedSearchQuery"
          :results="searchResults"
          :highlighted-index="highlightedSearchIndex"
          :recent-searches="recentSearches"
          @select="openSearchResult"
          @highlight="highlightedSearchIndex = $event"
          @recent-search="applyRecentSearch"
          @clear-recent-searches="clearNavbarRecentSearches"
        />
      </div>
    </div>

    <!-- 右侧快捷操作区 -->
    <div class="navbar-end flex-shrink-0 gap-2">
      <TopNavbarActivityDropdown
        :title="t('notifications.center.messagesTitle')"
        icon-class="fas fa-inbox"
        :empty-text="t('notifications.center.emptyMessages')"
        :items="messageItems"
        :unread-count="unreadMessageCount"
        @open="openActivity"
        @remove="removeNotification"
        @mark-all-read="markAllAsRead('message')"
        @clear-all="clearCategory('message')"
        @view-all="openNotificationCenter('message')"
      />

      <TopNavbarActivityDropdown
        :title="t('notifications.center.notificationsTitle')"
        icon-class="fas fa-bell"
        :empty-text="t('notifications.center.emptyNotifications')"
        :items="notificationItems"
        :unread-count="unreadNotificationCount"
        @open="openActivity"
        @remove="removeNotification"
        @mark-all-read="markAllAsRead('notification')"
        @clear-all="clearCategory('notification')"
        @view-all="openNotificationCenter('notification')"
      />

      <!-- 语言切换器 -->
      <div class="dropdown dropdown-end">
        <div
          tabindex="0"
          role="button"
          class="btn btn-ghost btn-circle btn-sm sm:btn-md tooltip tooltip-bottom"
          data-tip="语言切换"
        >
          <i class="fas fa-language text-lg sm:text-xl"></i>
        </div>
        <ul
          tabindex="0"
          class="dropdown-content z-[60] menu p-2 shadow bg-base-100 rounded-box w-36"
        >
          <li v-for="lang in availableLanguages" :key="lang.code">
            <a @click="switchLanguage(lang.code)" :class="{ active: locale === lang.code }">
              <i :class="`fas ${lang.icon} mr-2`"></i>{{ lang.name }}
            </a>
          </li>
        </ul>
      </div>

      <!-- 主题切换器 -->
      <div class="dropdown dropdown-end">
        <div
          tabindex="0"
          role="button"
          class="btn btn-ghost btn-circle btn-sm sm:btn-md tooltip tooltip-bottom"
          data-tip="主题切换"
        >
          <i class="fas fa-palette text-lg sm:text-xl"></i>
        </div>
        <ul
          tabindex="0"
          class="dropdown-content z-[60] menu p-2 shadow bg-base-100 rounded-box w-52"
        >
          <li v-for="theme in availableThemes" :key="theme.code">
            <a @click="setTheme(theme.code)">
              <i :class="`fas ${theme.icon} mr-2`"></i>{{ theme.name }}
            </a>
          </li>
        </ul>
      </div>

      <!-- 帮助按钮 -->
      <div class="dropdown dropdown-end">
        <div
          tabindex="0"
          role="button"
          class="btn btn-ghost btn-circle btn-sm sm:btn-md tooltip tooltip-bottom"
          :data-tip="t('common.tour.help')"
        >
          <i class="fas fa-question-circle text-lg sm:text-xl"></i>
        </div>
        <ul
          tabindex="0"
          class="dropdown-content z-[60] menu p-2 shadow bg-base-100 rounded-box w-52"
        >
          <li>
            <a @click="startPageTour">
              <i class="fas fa-route mr-2"></i>{{ t('common.tour.guide') }}
            </a>
          </li>
          <li>
            <a @click="openHelpDocumentation">
              <i class="fas fa-book mr-2"></i>{{ t('common.tour.documentation') }}
            </a>
          </li>
        </ul>
      </div>

      <button
        type="button"
        class="btn btn-ghost btn-circle btn-sm sm:btn-md tooltip tooltip-bottom"
        :data-tip="t('common.immersiveDrillMode', '沉浸式挖洞模式')"
        :title="t('common.immersiveDrillMode', '沉浸式挖洞模式')"
        :aria-label="t('common.immersiveDrillMode', '沉浸式挖洞模式')"
        @click="toggleImmersiveDrillMode"
      >
        <i class="fas fa-crosshairs text-lg sm:text-xl"></i>
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue'
import { WebviewWindow } from '@tauri-apps/api/webviewWindow'
import { useI18n } from 'vue-i18n'
import { useRouter, useRoute } from 'vue-router'
import { usePageTour, type TourStep } from '@/composables/usePageTour'
import { useGlobalSearch } from '@/composables/useGlobalSearch'
import { useSearchFindings } from '@/composables/useSearchFindings'
import { useNotificationCenter } from '@/composables/useNotificationCenter'
import { buildHelpCenterWindowUrl, HELP_CENTER_WINDOW_LABEL } from '@/router/standalone'
import { GLOBAL_SEARCH_FOCUS_EVENT } from '@/services/globalSearchFocus'
import { isStrongSearchMatch, type GlobalSearchResult } from '@/services/globalSearch'
import { addRecentSearch, clearRecentSearches, loadRecentSearches } from '@/services/searchHistory'
import type { AppNotificationItem, NotificationCategory } from '@/types/notification'
import TopNavbarActivityDropdown from './TopNavbarActivityDropdown.vue'
import TopNavbarSearchResults from './TopNavbarSearchResults.vue'

// Emits
const emit = defineEmits<{
  toggleSidebar: []
  toggleNavbarVisibility: []
  toggleImmersiveDrillMode: []
  setTheme: [theme: string]
  switchLanguage: [lang: string]
}>()

// Composables
const { t, locale } = useI18n()
const router = useRouter()
const route = useRoute()
const { manualStartTour } = usePageTour()
const {
  initializeNotificationCenter,
  items: notificationCenterItems,
  messageItems,
  notificationItems,
  unreadMessageCount,
  unreadNotificationCount,
  openNotification,
  removeNotification,
  markAllAsRead,
  clearCategory,
} = useNotificationCenter()
const { findings: searchFindings, initializeSearchFindings } = useSearchFindings()
const { search } = useGlobalSearch({
  notifications: notificationCenterItems,
  findings: searchFindings,
})

// 搜索相关
const searchQuery = ref('')
const recentSearches = ref<string[]>(loadRecentSearches())
const searchContainerRef = ref<HTMLElement | null>(null)
const searchInputRef = ref<HTMLInputElement | null>(null)
const isSearchFocused = ref(false)
const highlightedSearchIndex = ref(0)
const trimmedSearchQuery = computed(() => searchQuery.value.trim())
const searchResults = computed(() => search(trimmedSearchQuery.value, 6))
const showSearchResults = computed(
  () =>
    isSearchFocused.value &&
    (trimmedSearchQuery.value.length > 0 || recentSearches.value.length > 0)
)

watch(
  () => [route.path, route.query.q],
  ([path, query]) => {
    searchQuery.value = path === '/search' && typeof query === 'string' ? query : ''
  },
  { immediate: true }
)

watch(searchResults, results => {
  if (results.length === 0) {
    highlightedSearchIndex.value = 0
    return
  }

  if (highlightedSearchIndex.value >= results.length) {
    highlightedSearchIndex.value = 0
  }
})

watch(trimmedSearchQuery, () => {
  highlightedSearchIndex.value = 0
})

const closeSearchResults = () => {
  isSearchFocused.value = false
  highlightedSearchIndex.value = 0
}

const clearNavbarRecentSearches = () => {
  recentSearches.value = clearRecentSearches()
  highlightedSearchIndex.value = 0
}

const focusNavbarSearch = () => {
  if (route.path === '/search') {
    return
  }

  isSearchFocused.value = true
  nextTick(() => {
    searchInputRef.value?.focus()
    searchInputRef.value?.select()
  })
}

const openSearchTarget = async (target: Pick<GlobalSearchResult, 'path' | 'query'>) => {
  closeSearchResults()
  await router.push({
    path: target.path,
    query: target.query,
  })
}

const rememberSearchQuery = (query: string) => {
  recentSearches.value = addRecentSearch(query)
}

const openSearchResult = async (result: GlobalSearchResult) => {
  rememberSearchQuery(trimmedSearchQuery.value || result.title)
  await openSearchTarget(result)
}

const applyRecentSearch = async (query: string) => {
  searchQuery.value = query
  isSearchFocused.value = true
  highlightedSearchIndex.value = 0
  await performSearch()
}

const handleSearchFocus = () => {
  isSearchFocused.value = true
}

const moveSearchHighlight = (direction: 1 | -1) => {
  const count = searchResults.value.length
  if (count === 0) {
    highlightedSearchIndex.value = 0
    return
  }

  highlightedSearchIndex.value = (highlightedSearchIndex.value + direction + count) % count
}

const handleSearchKeydown = async (event: KeyboardEvent) => {
  if (event.key === 'ArrowDown') {
    event.preventDefault()
    isSearchFocused.value = true
    moveSearchHighlight(1)
    return
  }

  if (event.key === 'ArrowUp') {
    event.preventDefault()
    isSearchFocused.value = true
    moveSearchHighlight(-1)
    return
  }

  if (event.key === 'Escape') {
    event.preventDefault()
    closeSearchResults()
    searchInputRef.value?.blur()
    return
  }

  if (event.key === 'Enter') {
    event.preventDefault()
    const highlightedResult = searchResults.value[highlightedSearchIndex.value]
    if (showSearchResults.value && highlightedResult) {
      await openSearchResult(highlightedResult)
      return
    }

    await performSearch()
  }
}

const handleDocumentPointerDown = (event: MouseEvent) => {
  const target = event.target as Node | null
  if (!target) {
    return
  }

  if (searchContainerRef.value?.contains(target)) {
    return
  }

  closeSearchResults()
}

const handleGlobalSearchFocus = () => {
  focusNavbarSearch()
}

// 可用语言
const availableLanguages = [
  { code: 'zh', name: '中文', icon: 'fa-language' },
  { code: 'en', name: 'English', icon: 'fa-globe' },
]

// 可用主题
const availableThemes = computed(() => [
  { code: 'light', name: t('settings.themes.light', '浅色'), icon: 'fa-sun' },
  { code: 'dark', name: t('settings.themes.dark', '深色'), icon: 'fa-moon' },
  { code: 'corporate', name: t('settings.themes.corporate', '企业'), icon: 'fa-building' },
])

// 方法
const toggleSidebar = () => {
  emit('toggleSidebar')
}

const toggleImmersiveDrillMode = () => {
  emit('toggleImmersiveDrillMode')
}

const setTheme = (theme: string) => {
  emit('setTheme', theme)
}

const switchLanguage = (lang: string) => {
  emit('switchLanguage', lang)
}

const performSearch = async () => {
  const query = trimmedSearchQuery.value
  if (!query) {
    return
  }

  rememberSearchQuery(query)

  const [bestMatch, nextMatch] = search(query, 2)
  if (
    isStrongSearchMatch(bestMatch ?? null, query) &&
    !isStrongSearchMatch(nextMatch ?? null, query)
  ) {
    await openSearchTarget(bestMatch!)
    return
  }

  closeSearchResults()
  await router.push({ path: '/search', query: { q: query } })
}

const openActivity = async (item: AppNotificationItem) => {
  await openNotification(router, item)
}

const openNotificationCenter = async (category: NotificationCategory) => {
  await router.push({
    path: '/notification-center',
    query: { category },
  })
}

const canUseTauriWindow = () => typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

const openHelpDocumentation = async () => {
  const url = buildHelpCenterWindowUrl()

  if (!canUseTauriWindow()) {
    window.open(url, '_blank', 'noopener,noreferrer')
    return
  }

  try {
    const existingWindow = await WebviewWindow.getByLabel(HELP_CENTER_WINDOW_LABEL)
    if (existingWindow) {
      await existingWindow.show()
      await existingWindow.setFocus()
      return
    }

    const helpWindow = new WebviewWindow(HELP_CENTER_WINDOW_LABEL, {
      url,
      title: `Sentinel AI - ${t('common.tour.documentation')}`,
      width: 1180,
      height: 860,
      center: true,
      resizable: true,
    })

    await new Promise<void>((resolve, reject) => {
      void helpWindow.once('tauri://created', async () => {
        await helpWindow.setFocus()
        resolve()
      })
      void helpWindow.once('tauri://error', event => {
        reject(new Error(String(event.payload ?? 'Failed to open help center window')))
      })
    })
  } catch (error) {
    console.error('Failed to open help center window', error)
    window.open(url, '_blank', 'noopener,noreferrer')
  }
}

// 页面向导配置映射
const getPageTourSteps = (): TourStep[] => {
  const currentPath = route.path

  // 流量分析 - 历史记录
  if (currentPath === '/traffic') {
    return [
      {
        element: '.tabs.tabs-boxed',
        popover: {
          title: t('trafficAnalysis.tour.proxyHistory.filterBar.title'),
          description: t('trafficAnalysis.tour.proxyHistory.filterBar.description'),
          side: 'bottom' as const,
          align: 'start' as const,
        },
      },
      {
        element: 'table',
        popover: {
          title: t('trafficAnalysis.tour.proxyHistory.requestList.title'),
          description: t('trafficAnalysis.tour.proxyHistory.requestList.description'),
          side: 'top' as const,
          align: 'start' as const,
        },
      },
    ]
  }

  // AI 助手
  if (currentPath === '/ai-assistant') {
    return [
      {
        element: '.btn.btn-sm.btn-ghost[title*="会话"]',
        popover: {
          title: t('agent.tour.conversationList.title'),
          description: t('agent.tour.conversationList.description'),
          side: 'right' as const,
          align: 'start' as const,
        },
      },
      {
        element: '.conversation-header',
        popover: {
          title: t('agent.tour.newConversation.title'),
          description: t('agent.tour.newConversation.description'),
          side: 'bottom' as const,
          align: 'center' as const,
        },
      },
    ]
  }

  // 仪表板
  if (currentPath === '/dashboard') {
    return [
      {
        element: '.stats',
        popover: {
          title: '统计概览',
          description: '这里显示系统的关键指标和统计数据，包括扫描任务、漏洞数量等。',
          side: 'bottom' as const,
          align: 'start' as const,
        },
      },
    ]
  }

  // 安全中心
  if (currentPath === '/security-center') {
    return [
      {
        element: '.tabs',
        popover: {
          title: '功能标签',
          description: '切换不同的安全功能模块：漏洞管理、扫描任务、资产管理等。',
          side: 'bottom' as const,
          align: 'start' as const,
        },
      },
    ]
  }

  // 工作流工作室
  if (currentPath === '/workflow-studio') {
    return [
      {
        element: '.workflow-canvas',
        popover: {
          title: '工作流画布',
          description: '在这里拖拽节点创建自动化工作流，连接不同的安全测试步骤。',
          side: 'right' as const,
          align: 'start' as const,
        },
      },
    ]
  }

  // 知识库管理
  if (currentPath === '/rag-management') {
    return [
      {
        element: '.file-upload',
        popover: {
          title: '上传文档',
          description: '上传文档到知识库，AI 助手可以基于这些文档回答问题。',
          side: 'bottom' as const,
          align: 'start' as const,
        },
      },
    ]
  }

  // 默认通用向导
  return [
    {
      element: '.navbar-start',
      popover: {
        title: '导航栏',
        description: '点击左侧菜单按钮可以展开/收起侧边栏，访问不同的功能模块。',
        side: 'bottom' as const,
        align: 'start' as const,
      },
    },
    {
      element: '.navbar-end',
      popover: {
        title: '快捷操作',
        description: '这里提供通知、语言切换、主题切换等快捷功能。',
        side: 'bottom' as const,
        align: 'end' as const,
      },
    },
  ]
}

const startPageTour = () => {
  const steps = getPageTourSteps()
  if (steps.length > 0) {
    manualStartTour(steps)
  }
}

onMounted(async () => {
  document.addEventListener('mousedown', handleDocumentPointerDown)
  window.addEventListener(GLOBAL_SEARCH_FOCUS_EVENT, handleGlobalSearchFocus)
  await initializeSearchFindings()
  await initializeNotificationCenter(router)
})

onUnmounted(() => {
  document.removeEventListener('mousedown', handleDocumentPointerDown)
  window.removeEventListener(GLOBAL_SEARCH_FOCUS_EVENT, handleGlobalSearchFocus)
})
</script>

<style scoped>
/* 自定义样式 */
.navbar {
  backdrop-filter: blur(10px);
}

.dropdown-content {
  border: 1px solid hsl(var(--border-color, var(--b3)));
}

/* 搜索框样式 */
.input-group .input:focus {
  outline: none;
  border-color: hsl(var(--primary));
}

/* 通知徽章动画 */
.indicator-item {
  animation: pulse 2s infinite;
}

@keyframes pulse {
  0%,
  100% {
    opacity: 1;
  }
  50% {
    opacity: 0.5;
  }
}

/* 圆形按钮图标居中 */
.btn-circle {
  display: flex;
  align-items: center;
  justify-content: center;
}

.btn-circle i {
  display: flex;
  align-items: center;
  justify-content: center;
  line-height: 1;
}
</style>
