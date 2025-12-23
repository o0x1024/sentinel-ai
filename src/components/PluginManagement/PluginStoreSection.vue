<template>
  <div>
    <!-- Store Header -->
    <div class="flex justify-between items-center mb-4">
      <div class="flex items-center gap-2">
        <i class="fas fa-store text-primary text-xl"></i>
        <span class="text-lg font-semibold">{{ $t('plugins.store.title', '插件商店') }}</span>
        <span v-if="lastCacheTime > 0" class="text-xs text-base-content/60">
          ({{ formatCacheTime(lastCacheTime) }})
        </span>
      </div>
      <div class="flex items-center gap-2">
        <!-- View Mode Toggle -->
        <div class="btn-group">
          <button 
            class="btn btn-sm"
            :class="{ 'btn-active': viewMode === 'list' }"
            @click="handleViewModeChange('list')"
            :title="$t('plugins.store.listView', '列表视图')">
            <i class="fas fa-list"></i>
          </button>
          <button 
            class="btn btn-sm"
            :class="{ 'btn-active': viewMode === 'card' }"
            @click="handleViewModeChange('card')"
            :title="$t('plugins.store.cardView', '卡片视图')">
            <i class="fas fa-th"></i>
          </button>
        </div>
        <button class="btn btn-sm btn-primary" :disabled="loading" @click="refreshStore(true)">
          <span v-if="loading" class="loading loading-spinner loading-xs"></span>
          <i v-else class="fas fa-sync-alt"></i>
          <span class="ml-1">{{ $t('common.refresh', '刷新') }}</span>
        </button>
      </div>
    </div>

    <!-- Error Alert -->
    <div v-if="error" class="alert alert-error mb-4">
      <i class="fas fa-exclamation-circle"></i>
      <span>{{ error }}</span>
      <button class="btn btn-sm btn-ghost" @click="error = ''">
        <i class="fas fa-times"></i>
      </button>
    </div>

    <!-- Loading State -->
    <div v-if="loading && storePlugins.length === 0" class="flex justify-center py-12">
      <span class="loading loading-spinner loading-lg text-primary"></span>
    </div>

    <!-- Search and Filter -->
    <div v-else class="flex gap-2 mb-4 flex-wrap items-center">
      <input
        v-model="searchText"
        type="text"
        :placeholder="$t('plugins.store.searchPlaceholder')"
        class="input input-bordered input-sm flex-1 min-w-48" />
      
      <select v-model="categoryFilter" class="select select-bordered select-sm">
        <option value="">{{ $t('plugins.store.allCategories') }}</option>
        <option value="passive">{{ $t('plugins.categories.passive') }}</option>
        <option value="agent">{{ $t('plugins.categories.agents') }}</option>
      </select>
    </div>

    <!-- Plugin Display -->
    <div v-if="!hasFetched && !loading" class="alert alert-info">
      <i class="fas fa-info-circle"></i>
      <span>{{ $t('plugins.store.refreshToLoad') }}</span>
    </div>

    <div v-else-if="filteredPlugins.length === 0 && !loading" class="alert alert-info">
      <i class="fas fa-info-circle"></i>
      <span>{{ $t('plugins.store.noPlugins') }}</span>
    </div>

    <!-- Card View -->
    <div v-else-if="viewMode === 'card'" class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
      <div v-for="plugin in filteredPlugins" :key="plugin.id"
        class="card bg-base-200 shadow-md hover:shadow-lg transition-shadow">
        <div class="card-body p-4">
          <!-- Plugin Header -->
          <div class="flex items-start justify-between">
            <div class="flex-1">
              <h3 class="card-title text-base">
                {{ plugin.name }}
                <span v-if="isInstalled(plugin.id)" class="badge badge-success badge-sm ml-1">
                  {{ $t('plugins.store.installed') }}
                </span>
              </h3>
              <p class="text-xs text-base-content/60">{{ plugin.id }}</p>
            </div>
            <div class="badge" :class="getCategoryBadgeClass(plugin.main_category)">
              {{ getCategoryLabel(plugin.main_category) }}
            </div>
          </div>

          <!-- Plugin Description -->
          <p class="text-sm text-base-content/80 line-clamp-2 mt-2">
            {{ plugin.description || $t('plugins.store.noDescription') }}
          </p>

          <!-- Plugin Meta -->
          <div class="flex flex-wrap gap-2 mt-2">
            <span class="badge badge-outline badge-sm">v{{ plugin.version }}</span>
            <span v-if="plugin.author" class="badge badge-ghost badge-sm">
              <i class="fas fa-user mr-1"></i>{{ plugin.author }}
            </span>
            <span v-if="plugin.default_severity" class="badge badge-sm" :class="getSeverityClass(plugin.default_severity)">
              {{ plugin.default_severity }}
            </span>
          </div>

          <!-- Tags -->
          <div v-if="plugin.tags && plugin.tags.length > 0" class="flex flex-wrap gap-1 mt-2">
            <span v-for="tag in plugin.tags.slice(0, 3)" :key="tag" class="badge badge-xs badge-outline">
              {{ tag }}
            </span>
            <span v-if="plugin.tags.length > 3" class="badge badge-xs badge-outline">
              +{{ plugin.tags.length - 3 }}
            </span>
          </div>

          <!-- Actions -->
          <div class="card-actions justify-end mt-3">
            <button v-if="!isInstalled(plugin.id)" class="btn btn-sm btn-primary"
              :disabled="installing === plugin.id" @click="installPlugin(plugin)">
              <span v-if="installing === plugin.id" class="loading loading-spinner loading-xs"></span>
              <i v-else class="fas fa-download mr-1"></i>
              {{ $t('plugins.store.install') }}
            </button>
            <button v-else class="btn btn-sm btn-outline" disabled>
              <i class="fas fa-check mr-1"></i>
              {{ $t('plugins.store.installed') }}
            </button>
            <button class="btn btn-sm btn-ghost" @click="viewPluginDetail(plugin)">
              <i class="fas fa-eye"></i>
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- List View -->
    <div v-else class="space-y-2">
      <div v-for="plugin in filteredPlugins" :key="plugin.id"
        class="bg-base-200 rounded-lg p-4 hover:shadow-md transition-shadow">
        <div class="flex items-center gap-4">
          <!-- Plugin Info -->
          <div class="flex-1 min-w-0">
            <div class="flex items-center gap-2 mb-1">
              <h3 class="font-semibold text-base truncate">{{ plugin.name }}</h3>
              <span v-if="isInstalled(plugin.id)" class="badge badge-success badge-sm">
                {{ $t('plugins.store.installed') }}
              </span>
              <div class="badge badge-sm" :class="getCategoryBadgeClass(plugin.main_category)">
                {{ getCategoryLabel(plugin.main_category) }}
              </div>
            </div>
            <p class="text-xs text-base-content/60 mb-2">{{ plugin.id }}</p>
            <p class="text-sm text-base-content/80 line-clamp-1 mb-2">
              {{ plugin.description || $t('plugins.store.noDescription') }}
            </p>
            <div class="flex flex-wrap gap-2 items-center">
              <span class="badge badge-outline badge-sm">v{{ plugin.version }}</span>
              <span v-if="plugin.author" class="badge badge-ghost badge-sm">
                <i class="fas fa-user mr-1"></i>{{ plugin.author }}
              </span>
              <span v-if="plugin.default_severity" class="badge badge-sm" :class="getSeverityClass(plugin.default_severity)">
                {{ plugin.default_severity }}
              </span>
              <div v-if="plugin.tags && plugin.tags.length > 0" class="flex flex-wrap gap-1">
                <span v-for="tag in plugin.tags.slice(0, 3)" :key="tag" class="badge badge-xs badge-outline">
                  {{ tag }}
                </span>
                <span v-if="plugin.tags.length > 3" class="badge badge-xs badge-outline">
                  +{{ plugin.tags.length - 3 }}
                </span>
              </div>
            </div>
          </div>

          <!-- Actions -->
          <div class="flex items-center gap-2 flex-shrink-0">
            <button v-if="!isInstalled(plugin.id)" class="btn btn-sm btn-primary"
              :disabled="installing === plugin.id" @click="installPlugin(plugin)">
              <span v-if="installing === plugin.id" class="loading loading-spinner loading-xs"></span>
              <i v-else class="fas fa-download"></i>
              <span class="hidden sm:inline ml-1">{{ $t('plugins.store.install') }}</span>
            </button>
            <button v-else class="btn btn-sm btn-outline" disabled>
              <i class="fas fa-check"></i>
              <span class="hidden sm:inline ml-1">{{ $t('plugins.store.installed') }}</span>
            </button>
            <button class="btn btn-sm btn-ghost" @click="viewPluginDetail(plugin)">
              <i class="fas fa-eye"></i>
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- Plugin Detail Modal -->
    <dialog ref="detailDialogRef" class="modal">
      <div class="modal-box max-w-3xl">
        <h3 class="font-bold text-lg">{{ selectedPlugin?.name }}</h3>
        
        <div v-if="selectedPlugin" class="py-4 space-y-4">
          <!-- Basic Info -->
          <div class="grid grid-cols-2 gap-4">
            <div>
              <span class="text-sm text-base-content/60">{{ $t('plugins.pluginId') }}:</span>
              <span class="ml-2 font-mono text-sm">{{ selectedPlugin.id }}</span>
            </div>
            <div>
              <span class="text-sm text-base-content/60">{{ $t('plugins.version') }}:</span>
              <span class="ml-2">{{ selectedPlugin.version }}</span>
            </div>
            <div>
              <span class="text-sm text-base-content/60">{{ $t('plugins.author') }}:</span>
              <span class="ml-2">{{ selectedPlugin.author || '-' }}</span>
            </div>
            <div>
              <span class="text-sm text-base-content/60">{{ $t('plugins.category') }}:</span>
              <span class="ml-2 badge" :class="getCategoryBadgeClass(selectedPlugin.main_category)">
                {{ getCategoryLabel(selectedPlugin.main_category) }}
              </span>
            </div>
          </div>

          <!-- Description -->
          <div>
            <span class="text-sm text-base-content/60">{{ $t('plugins.pluginDescription') }}:</span>
            <p class="mt-1 text-sm">{{ selectedPlugin.description || $t('plugins.store.noDescription') }}</p>
          </div>

          <!-- Tags -->
          <div v-if="selectedPlugin.tags && selectedPlugin.tags.length > 0">
            <span class="text-sm text-base-content/60">{{ $t('plugins.tags') }}:</span>
            <div class="flex flex-wrap gap-1 mt-1">
              <span v-for="tag in selectedPlugin.tags" :key="tag" class="badge badge-sm badge-outline">
                {{ tag }}
              </span>
            </div>
          </div>

          <!-- Code Preview (if available) -->
          <div v-if="selectedPluginCode" class="mt-4">
            <div class="flex justify-between items-center mb-2">
              <span class="text-sm text-base-content/60">{{ $t('plugins.pluginCode') }}:</span>
              <button class="btn btn-xs btn-ghost" @click="copyCode">
                <i class="fas fa-copy mr-1"></i>{{ $t('plugins.copy') }}
              </button>
            </div>
            <pre class="bg-base-300 p-3 rounded-lg text-xs overflow-auto max-h-64"><code>{{ selectedPluginCode }}</code></pre>
          </div>
        </div>

        <div class="modal-action">
          <button v-if="selectedPlugin && !isInstalled(selectedPlugin.id)" class="btn btn-primary"
            :disabled="installing === selectedPlugin.id" @click="installPlugin(selectedPlugin)">
            <span v-if="installing === selectedPlugin.id" class="loading loading-spinner loading-xs"></span>
            <i v-else class="fas fa-download mr-1"></i>
            {{ $t('plugins.store.install') }}
          </button>
          <button class="btn" @click="closeDetailDialog">{{ $t('common.close') }}</button>
        </div>
      </div>
      <form method="dialog" class="modal-backdrop">
        <button>close</button>
      </form>
    </dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useI18n } from 'vue-i18n'
import type { PluginRecord, CommandResponse } from './types'
import { PluginStoreCache, ViewModeStorage } from '@/services/storage'

// Store plugin interface
interface StorePlugin {
  id: string
  name: string
  version: string
  author?: string
  main_category: string
  category: string
  description?: string
  default_severity?: string
  tags: string[]
  download_url: string
  code?: string
}

interface StorePluginListResponse {
  success: boolean
  plugins: StorePlugin[]
  error?: string
}

interface CacheData {
  plugins: StorePlugin[]
  timestamp: number
}

const { t } = useI18n()

const props = defineProps<{
  installedPluginIds: string[]
}>()

const emit = defineEmits<{
  pluginInstalled: [pluginId: string]
}>()

// Constants (保留用于向后兼容，但实际使用 storage service)
const CACHE_KEY = 'plugin_store_cache'
const VIEW_MODE_KEY = 'plugin_store_view_mode'
const CACHE_DURATION = 5 * 60 * 1000 // 5分钟缓存

// State
const loading = ref(false)
const error = ref('')
const storePlugins = ref<StorePlugin[]>([])
const searchText = ref('')
const categoryFilter = ref('')
const installing = ref<string | null>(null)
const selectedPlugin = ref<StorePlugin | null>(null)
const selectedPluginCode = ref('')
const detailDialogRef = ref<HTMLDialogElement | null>(null)
const hasFetched = ref(false)
const lastCacheTime = ref<number>(0)
const viewMode = ref<'list' | 'card'>('list')

// Computed
const filteredPlugins = computed(() => {
  let plugins = storePlugins.value

  if (searchText.value.trim()) {
    const query = searchText.value.toLowerCase()
    plugins = plugins.filter(p =>
      p.name.toLowerCase().includes(query) ||
      p.id.toLowerCase().includes(query) ||
      p.description?.toLowerCase().includes(query)
    )
  }

  if (categoryFilter.value) {
    plugins = plugins.filter(p => p.main_category === categoryFilter.value)
  }

  return plugins
})

// Methods
const isInstalled = (pluginId: string): boolean => {
  return props.installedPluginIds.includes(pluginId)
}

const getCategoryLabel = (category: string): string => {
  if (category === 'passive') return t('plugins.categories.passive')
  if (category === 'agent') return t('plugins.categories.agents')
  return category
}

const getCategoryBadgeClass = (category: string): string => {
  if (category === 'passive') return 'badge-info'
  if (category === 'agent') return 'badge-warning'
  return 'badge-ghost'
}

const getSeverityClass = (severity: string): string => {
  const classes: Record<string, string> = {
    critical: 'badge-error',
    high: 'badge-warning',
    medium: 'badge-info',
    low: 'badge-success',
    info: 'badge-ghost'
  }
  return classes[severity] || 'badge-ghost'
}

const showToast = (message: string, type: 'success' | 'error' | 'info' = 'success') => {
  const toast = document.createElement('div')
  toast.className = 'toast toast-top toast-end z-50'
  toast.style.top = '5rem'
  const alertClass = { success: 'alert-success', error: 'alert-error', info: 'alert-info' }[type]
  const icon = { success: 'fa-check-circle', error: 'fa-times-circle', info: 'fa-info-circle' }[type]
  toast.innerHTML = `<div class="alert ${alertClass} shadow-lg"><i class="fas ${icon}"></i><span>${message}</span></div>`
  document.body.appendChild(toast)
  setTimeout(() => toast.remove(), 3000)
}

const loadFromCache = async (): Promise<boolean> => {
  try {
    const cache = await PluginStoreCache.load()
    
    if (!cache) {
      return false
    }
    
    storePlugins.value = cache.plugins
    lastCacheTime.value = cache.timestamp
    hasFetched.value = true
    console.log('Loaded plugins from cache:', cache.plugins.length, 'plugins')
    return true
  } catch (e) {
    console.error('Failed to load cache:', e)
    return false
  }
}

const saveToCache = async (plugins: StorePlugin[]) => {
  try {
    await PluginStoreCache.save(plugins)
    lastCacheTime.value = Date.now()
    console.log('Saved plugins to cache:', plugins.length, 'plugins')
  } catch (e) {
    console.error('Failed to save cache:', e)
  }
}

const refreshStore = async (forceRefresh = false) => {
  // 如果不是强制刷新，先尝试从缓存加载
  if (!forceRefresh) {
    const loaded = await loadFromCache()
    if (loaded) {
      console.log('Using cached plugin data')
      return
    }
  }
  
  loading.value = true
  error.value = ''
  hasFetched.value = true
  
  try {
    console.log('Fetching plugins from store...')
    const response = await invoke<StorePluginListResponse>('fetch_store_plugins', {
      repoUrl: 'https://github.com/o0x1024/sentinel-plugin'
    })
    
    if (response.success) {
      storePlugins.value = response.plugins
      await saveToCache(response.plugins)
      console.log('Successfully fetched and cached plugins:', response.plugins.length)
    } else {
      error.value = response.error || t('plugins.store.fetchError')
    }
  } catch (e) {
    console.error('Failed to fetch store plugins:', e)
    error.value = e instanceof Error ? e.message : t('plugins.store.fetchError')
  } finally {
    loading.value = false
  }
}

const installPlugin = async (plugin: StorePlugin) => {
  installing.value = plugin.id
  
  try {
    const response = await invoke<CommandResponse<string>>('install_store_plugin', {
      plugin: {
        id: plugin.id,
        name: plugin.name,
        version: plugin.version,
        author: plugin.author || '',
        main_category: plugin.main_category,
        category: plugin.category,
        description: plugin.description || '',
        default_severity: plugin.default_severity || 'medium',
        tags: plugin.tags,
        download_url: plugin.download_url
      }
    })
    
    if (response.success) {
      showToast(t('plugins.store.installSuccess'), 'success')
      emit('pluginInstalled', plugin.id)
      closeDetailDialog()
    } else {
      showToast(response.error || t('plugins.store.installError'), 'error')
    }
  } catch (e) {
    console.error('Failed to install plugin:', e)
    showToast(e instanceof Error ? e.message : t('plugins.store.installError'), 'error')
  } finally {
    installing.value = null
  }
}

const viewPluginDetail = async (plugin: StorePlugin) => {
  selectedPlugin.value = plugin
  selectedPluginCode.value = ''
  detailDialogRef.value?.showModal()
  
  // Try to fetch plugin code if available
  if (plugin.download_url) {
    try {
      const response = await invoke<{ success: boolean; code: string; error?: string }>('fetch_plugin_code', {
        downloadUrl: plugin.download_url
      })
      if (response.success) {
        selectedPluginCode.value = response.code
      }
    } catch (e) {
      console.error('Failed to fetch plugin code:', e)
    }
  }
}

const closeDetailDialog = () => {
  detailDialogRef.value?.close()
  selectedPlugin.value = null
  selectedPluginCode.value = ''
}

const copyCode = async () => {
  if (selectedPluginCode.value) {
    await navigator.clipboard.writeText(selectedPluginCode.value)
    showToast(t('plugins.copySuccess'), 'success')
  }
}

const formatCacheTime = (timestamp: number): string => {
  const now = Date.now()
  const diff = now - timestamp
  const minutes = Math.floor(diff / 60000)
  
  if (minutes < 1) {
    return t('plugins.store.justNow', '刚刚更新') as string
  } else if (minutes < 60) {
    return t('plugins.store.minutesAgo', { minutes }, { default: '{minutes}分钟前' }) as string
  } else {
    const hours = Math.floor(minutes / 60)
    return t('plugins.store.hoursAgo', { hours }, { default: '{hours}小时前' }) as string
  }
}

// 加载视图模式
const loadViewMode = async () => {
  try {
    const mode = await ViewModeStorage.load()
    viewMode.value = mode
    console.log('Loaded view mode:', mode)
  } catch (e) {
    console.error('Failed to load view mode:', e)
  }
}

// 保存视图模式
const saveViewMode = async (mode: 'list' | 'card') => {
  try {
    await ViewModeStorage.save(mode)
    console.log('Saved view mode:', mode)
  } catch (e) {
    console.error('Failed to save view mode:', e)
  }
}

// 监听视图模式变化并保存
const handleViewModeChange = async (mode: 'list' | 'card') => {
  viewMode.value = mode
  await saveViewMode(mode)
}

// 组件挂载时自动从缓存加载
onMounted(async () => {
  console.log('PluginStoreSection mounted, loading cache and settings...')
  await loadFromCache()
  await loadViewMode()
})

// Expose methods
defineExpose({
  refreshStore
})
</script>

<style scoped>
.line-clamp-2 {
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
  line-clamp: 2;
}
</style>

