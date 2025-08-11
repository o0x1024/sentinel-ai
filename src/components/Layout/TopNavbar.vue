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
      <div class="form-control">
        <div class="input-group">
          <input 
            type="text" 
            placeholder="搜索任务、漏洞、工具..." 
            class="input input-bordered input-sm w-64" 
            v-model="searchQuery"
            @keyup.enter="performSearch"
          />
          <button class="btn btn-square btn-sm" @click="performSearch">
            <i class="fas fa-search"></i>
          </button>
        </div>
      </div>
    </div>

    <!-- 右侧快捷操作区 -->
    <div class="navbar-end flex-shrink-0 gap-2">
      <!-- 通知按钮 -->
      <div class="dropdown dropdown-end">
        <div tabindex="0" role="button" class="btn btn-ghost btn-circle btn-sm sm:btn-md indicator">
          <i class="fas fa-bell text-lg sm:text-xl"></i>
          <span v-if="unreadNotifications > 0" class="badge badge-xs badge-primary indicator-item">{{ unreadNotifications }}</span>
        </div>
        <div tabindex="0" class="dropdown-content z-[60] card card-compact w-80 p-2 shadow bg-base-100">
          <div class="card-body">
            <h3 class="card-title text-sm">通知中心</h3>
            <div class="space-y-2 max-h-64 overflow-y-auto">
              <div v-for="notification in notifications" :key="notification.id" class="alert alert-info py-2">
                <i :class="notification.icon"></i>
                <div>
                  <div class="font-bold text-xs">{{ notification.title }}</div>
                  <div class="text-xs opacity-70">{{ notification.message }}</div>
                </div>
              </div>
              <div v-if="notifications.length === 0" class="text-center text-sm opacity-70 py-4">
                暂无新通知
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- AI助手按钮组 -->
      <div class="dropdown dropdown-end">
        <div tabindex="0" role="button" class="btn btn-ghost btn-circle btn-sm sm:btn-md tooltip tooltip-bottom" data-tip="AI助手">
          <i class="fas fa-robot text-lg sm:text-xl" :class="{ 'text-primary': showAIChat }"></i>
        </div>
        <ul tabindex="0" class="dropdown-content z-[60] menu p-2 shadow bg-base-100 rounded-box w-48">
          <li>
            <a @click="toggleAIChat">
              <i class="fas fa-comment mr-2"></i>{{ t('aiChat.toggleInApp', '应用内聊天') }}
            </a>
          </li>
          <li>
            <a @click="openAIChatWindow">
              <i class="fas fa-external-link-alt mr-2"></i>{{ t('aiChat.openWindow', '独立窗口') }}
            </a>
          </li>
        </ul>
      </div>

      <!-- 语言切换器 -->
      <div class="dropdown dropdown-end">
        <div tabindex="0" role="button" class="btn btn-ghost btn-circle btn-sm sm:btn-md tooltip tooltip-bottom" data-tip="语言切换">
          <i class="fas fa-language text-lg sm:text-xl"></i>
        </div>
        <ul tabindex="0" class="dropdown-content z-[60] menu p-2 shadow bg-base-100 rounded-box w-36">
          <li v-for="lang in availableLanguages" :key="lang.code">
            <a @click="switchLanguage(lang.code)" :class="{ 'active': locale === lang.code }">
              <i :class="`fas ${lang.icon} mr-2`"></i>{{ lang.name }}
            </a>
          </li>
        </ul>
      </div>

      <!-- 主题切换器 -->
      <div class="dropdown dropdown-end">
        <div tabindex="0" role="button" class="btn btn-ghost btn-circle btn-sm sm:btn-md tooltip tooltip-bottom" data-tip="主题切换">
          <i class="fas fa-palette text-lg sm:text-xl"></i>
        </div>
        <ul tabindex="0" class="dropdown-content z-[60] menu p-2 shadow bg-base-100 rounded-box w-52">
          <li v-for="theme in availableThemes" :key="theme.code">
            <a @click="setTheme(theme.code)">
              <i :class="`fas ${theme.icon} mr-2`"></i>{{ theme.name }}
            </a>
          </li>
        </ul>
      </div>

    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'

// Props
const props = defineProps<{
  showAIChat: boolean
}>()

// Emits
const emit = defineEmits<{
  toggleSidebar: []
  toggleAIChat: []
  setTheme: [theme: string]
  switchLanguage: [lang: string]
}>()

// Composables
const { t, locale } = useI18n()
const router = useRouter()

// 搜索相关
const searchQuery = ref('')

// 通知相关
const unreadNotifications = ref(3)
const notifications = ref([
  {
    id: 1,
    title: '扫描完成',
    message: '目标 example.com 的扫描已完成',
    icon: 'fas fa-check-circle text-success',
    time: '2分钟前'
  },
  {
    id: 2,
    title: '发现漏洞',
    message: '在 target.com 发现高危漏洞',
    icon: 'fas fa-exclamation-triangle text-warning',
    time: '5分钟前'
  },
  {
    id: 3,
    title: 'Agent状态',
    message: 'Agent-001 已上线',
    icon: 'fas fa-robot text-info',
    time: '10分钟前'
  }
])

// 可用语言
const availableLanguages = [
  { code: 'zh', name: '中文', icon: 'fa-language' },
  { code: 'en', name: 'English', icon: 'fa-globe' }
]

// 可用主题
const availableThemes = computed(() => [
  { code: 'light', name: t('settings.themes.light', '浅色'), icon: 'fa-sun' },
  { code: 'dark', name: t('settings.themes.dark', '深色'), icon: 'fa-moon' },
  { code: 'corporate', name: t('settings.themes.corporate', '企业'), icon: 'fa-building' }
])

// 方法
const toggleSidebar = () => {
  emit('toggleSidebar')
}

const toggleAIChat = () => {
  emit('toggleAIChat')
}

// 打开AI助手独立窗口
const openAIChatWindow = async () => {
  try {
    await invoke('toggle_ai_chat_window')
  } catch (error) {
    console.error('Failed to open AI chat window:', error)
  }
}

const setTheme = (theme: string) => {
  emit('setTheme', theme)
}

const switchLanguage = (lang: string) => {
  emit('switchLanguage', lang)
}

const performSearch = () => {
  if (searchQuery.value.trim()) {
    // 执行搜索逻辑
    console.log('搜索:', searchQuery.value)
    // 可以导航到搜索结果页面
    router.push({ path: '/search', query: { q: searchQuery.value } })
  }
}

const logout = () => {
  // 退出登录逻辑
  console.log('退出登录')
  // 清除用户数据，导航到登录页面等
}
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
  0%, 100% {
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