import { ref, computed, nextTick, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { useI18n } from 'vue-i18n'
import { EditorView, type ViewUpdate } from '@codemirror/view'
import { EditorState, Compartment } from '@codemirror/state'
import { basicSetup } from 'codemirror'
import { javascript } from '@codemirror/lang-javascript'
import { oneDark } from '@codemirror/theme-one-dark'
import type {
  PluginRecord, ReviewPlugin, TestResult, AdvancedTestResult,
  CommandResponse, BatchToggleResult, ReviewStats, NewPluginMetadata, AdvancedForm
} from './types'
import { trafficCategories, agentsCategories } from './types'

export function usePluginManagement() {
  const { t } = useI18n()

  // Component State
  const selectedCategory = ref('all')
  const plugins = ref<PluginRecord[]>([])
  
  // Dialog refs (will be set by parent component)
  const uploadDialog = ref<HTMLDialogElement>()
  const codeEditorDialog = ref<HTMLDialogElement>()
  const deleteDialog = ref<HTMLDialogElement>()
  const aiGenerateDialog = ref<HTMLDialogElement>()
  const testResultDialog = ref<HTMLDialogElement>()
  const advancedDialog = ref<HTMLDialogElement>()
  const reviewDetailDialog = ref<HTMLDialogElement>()
  const fileInput = ref<HTMLInputElement>()

  // CodeMirror Editor Refs
  const codeEditorContainer = ref<HTMLDivElement>()
  const reviewCodeEditorContainer = ref<HTMLDivElement>()
  const fullscreenCodeEditorContainer = ref<HTMLDivElement>()
  const codeEditorView: EditorView | null = null
  const reviewCodeEditorView: EditorView | null = null
  const fullscreenCodeEditorView: EditorView | null = null

  // CodeMirror Compartments
  const codeEditorReadOnly = new Compartment()
  const reviewCodeEditorReadOnly = new Compartment()

  // Review Plugin State
  const reviewPlugins = ref<ReviewPlugin[]>([])
  const selectedReviewPlugins = ref<ReviewPlugin[]>([])
  const selectedReviewPlugin = ref<ReviewPlugin | null>(null)
  const reviewSearchText = ref('')
  const reviewEditMode = ref(false)
  const editedReviewCode = ref('')
  const savingReview = ref(false)

  // Review Filter and Pagination State
  const reviewStatusFilter = ref<string>('all')
  const reviewCurrentPage = ref(1)
  const reviewPageSize = ref(10)
  const reviewTotalCount = ref(0)
  const reviewTotalPagesCount = ref(0)

  // Review Statistics
  const reviewStatsData = ref<ReviewStats>({
    total: 0, pending: 0, approved: 0, rejected: 0, failed: 0
  })

  // Plugin List Filter and Pagination State
  const pluginViewMode = ref<'favorited' | 'all'>('all')
  const pluginCurrentPage = ref(1)
  const pluginPageSize = ref(10)
  const pluginSearchText = ref('')
  const selectedSubCategory = ref('')
  const selectedTag = ref('')
  const batchToggling = ref(false)

  const selectedFile = ref<File | null>(null)
  const uploading = ref(false)
  const uploadError = ref('')

  const editingPlugin = ref<PluginRecord | null>(null)
  const pluginCode = ref('')
  const originalCode = ref('')
  const isEditing = ref(false)
  const saving = ref(false)
  const codeError = ref('')
  const isFullscreenEditor = ref(false)

  const deletingPlugin = ref<PluginRecord | null>(null)
  const deleting = ref(false)

  // New Plugin Metadata
  const newPluginMetadata = ref<NewPluginMetadata>({
    id: '', name: '', version: '1.0.0', author: '',
    mainCategory: 'traffic', category: 'vulnerability',
    default_severity: 'medium', description: '', tagsString: ''
  })

  // AI Generation State
  const aiPrompt = ref('')
  const aiPluginType = ref('vulnerability')
  const aiSeverity = ref('medium')
  const aiGenerating = ref(false)
  const aiGenerateError = ref('')

  // Test State
  const testing = ref(false)
  const testResult = ref<TestResult | null>(null)

  // Advanced Test State
  const advancedPlugin = ref<PluginRecord | null>(null)
  const advancedTesting = ref(false)
  const advancedError = ref('')
  const advancedResult = ref<AdvancedTestResult | null>(null)
  const advancedForm = ref<AdvancedForm>({
    url: 'https://example.com/test',
    method: 'GET',
    headersText: '{"User-Agent":"Sentinel-AdvTest/1.0"}',
    bodyText: '',
    agent_inputs_text: '{}',
    runs: 1,
    concurrency: 1,
  })

  let pluginChangedUnlisten: UnlistenFn | null = null

  // Categories Definition
  const categories = computed(() => [
    { value: 'all', label: t('plugins.categories.all', '全部'), icon: 'fas fa-th' },
    { value: 'traffic', label: t('plugins.categories.trafficAnalysis', '流量分析插件'), icon: 'fas fa-shield-alt' },
    { value: 'agents', label: t('plugins.categories.agents', 'Agent工具插件'), icon: 'fas fa-robot' },
  ])

  // Filtered Plugins
  const filteredPlugins = computed(() => {
    let filtered = plugins.value

    if (selectedCategory.value === 'all') {
      filtered = plugins.value
    } else if (selectedCategory.value === 'traffic') {
      filtered = plugins.value.filter(p => {
        if (p.metadata.main_category === 'traffic') return true
        if (trafficCategories.includes(p.metadata.category)) return true
        if (p.metadata.category === 'traffic') return true
        return false
      })
    } else if (selectedCategory.value === 'agents') {
      filtered = plugins.value.filter(p => {
        if (p.metadata.main_category === 'agent') return true
        if (agentsCategories.includes(p.metadata.category)) return true
        return false
      })
    } else {
      filtered = plugins.value.filter(p => p.metadata.category === selectedCategory.value)
    }

    // Favorite filter for traffic plugins
    if (selectedCategory.value === 'traffic' && pluginViewMode.value === 'favorited') {
      filtered = filtered.filter(p => isPluginFavorited(p))
    }

    // Search filter
    if (pluginSearchText.value.trim()) {
      const query = pluginSearchText.value.toLowerCase()
      filtered = filtered.filter(p =>
        p.metadata.name.toLowerCase().includes(query) ||
        p.metadata.id.toLowerCase().includes(query) ||
        p.metadata.description?.toLowerCase().includes(query) ||
        p.metadata.author?.toLowerCase().includes(query)
      )
    }

    // Subcategory filter
    if (selectedSubCategory.value) {
      filtered = filtered.filter(p => p.metadata.category === selectedSubCategory.value)
    }

    // Tag filter
    if (selectedTag.value) {
      filtered = filtered.filter(p => p.metadata.tags.includes(selectedTag.value))
    }

    return filtered
  })

  // Pagination
  const pluginTotalPages = computed(() => Math.max(1, Math.ceil(filteredPlugins.value.length / pluginPageSize.value)))
  
  const paginatedPlugins = computed(() => {
    const start = (pluginCurrentPage.value - 1) * pluginPageSize.value
    const end = start + pluginPageSize.value
    return filteredPlugins.value.slice(start, end)
  })

  const pluginPaginationInfo = computed(() => {
    const total = filteredPlugins.value.length
    const start = total > 0 ? (pluginCurrentPage.value - 1) * pluginPageSize.value + 1 : 0
    const end = Math.min(pluginCurrentPage.value * pluginPageSize.value, total)
    return { start, end, total }
  })

  // Review Stats
  const reviewStats = computed(() => reviewStatsData.value)
  const filteredReviewPlugins = computed(() => reviewPlugins.value)
  const paginatedReviewPlugins = computed(() => reviewPlugins.value)
  const reviewTotalPages = computed(() => reviewTotalPagesCount.value)

  const reviewPaginationInfo = computed(() => {
    const start = (reviewCurrentPage.value - 1) * reviewPageSize.value + 1
    const end = Math.min(reviewCurrentPage.value * reviewPageSize.value, reviewTotalCount.value)
    const total = reviewTotalCount.value
    return { start, end, total }
  })

  const isAllSelected = computed(() => {
    return paginatedReviewPlugins.value.length > 0 &&
      paginatedReviewPlugins.value.every(p => isPluginSelected(p))
  })

  const sortedRuns = computed(() => {
    if (!advancedResult.value) return []
    return [...advancedResult.value.runs].sort((a, b) => a.run_index - b.run_index)
  })

  const isAdvancedAgent = computed(() => {
    return advancedPlugin.value?.metadata?.main_category === 'agent'
  })

  // Helper Functions
  const isPluginFavorited = (plugin: PluginRecord): boolean => plugin.is_favorited || false

  const isTrafficPluginType = (plugin: PluginRecord): boolean => {
    if (plugin.metadata.main_category === 'traffic') return true
    if (trafficCategories.includes(plugin.metadata.category)) return true
    if (plugin.metadata.category === 'traffic') return true
    return false
  }

  const isAgentPluginType = (plugin: PluginRecord): boolean => {
    if (plugin.metadata.main_category === 'agent') return true
    if (agentsCategories.includes(plugin.metadata.category)) return true
    return false
  }

  const getStatusText = (status: string): string => {
    const statusMap: Record<string, string> = {
      'Enabled': t('plugins.enabled', '已启用'),
      'Disabled': t('plugins.disabled', '已禁用'),
      'Error': t('plugins.error', '错误')
    }
    return statusMap[status] || status
  }

  const getCategoryLabel = (category: string): string => {
    const cat = categories.value.find(c => c.value === category)
    return cat ? cat.label : category
  }

  const getCategoryIcon = (category: string): string => {
    const cat = categories.value.find(c => c.value === category)
    if (cat) return cat.icon
    
    const subCatIcons: Record<string, string> = {
      'scanner': 'fas fa-radar', 'analyzer': 'fas fa-microscope',
      'reporter': 'fas fa-file-alt', 'recon': 'fas fa-search',
      'exploit': 'fas fa-bomb', 'utility': 'fas fa-toolbox',
      'sqli': 'fas fa-database', 'command_injection': 'fas fa-terminal',
      'xss': 'fas fa-code', 'idor': 'fas fa-user-lock',
      'auth_bypass': 'fas fa-unlock', 'csrf': 'fas fa-shield-alt',
      'info_leak': 'fas fa-eye-slash', 'file_upload': 'fas fa-file-upload',
      'file_inclusion': 'fas fa-file-code', 'path_traversal': 'fas fa-folder-open',
      'xxe': 'fas fa-file-code', 'ssrf': 'fas fa-server'
    }
    return subCatIcons[category] || 'fas fa-wrench'
  }

  const getCategoryCount = (category: string): number => {
    if (category === 'all') return plugins.value.length
    if (category === 'traffic') {
      return plugins.value.filter(p => {
        if (p.metadata.main_category === 'traffic') return true
        if (trafficCategories.includes(p.metadata.category)) return true
        if (p.metadata.category === 'traffic') return true
        return false
      }).length
    }
    if (category === 'agents') {
      return plugins.value.filter(p => {
        if (p.metadata.main_category === 'agent') return true
        if (agentsCategories.includes(p.metadata.category)) return true
        return false
      }).length
    }
    return plugins.value.filter(p => p.metadata.category === category).length
  }

  // Toast
  const showToast = (message: string, type: 'success' | 'error' | 'info' | 'warning' = 'success') => {
    const toast = document.createElement('div')
    toast.className = 'toast toast-top toast-end z-50'
    toast.style.top = '5rem'
    const alertClass = { success: 'alert-success', error: 'alert-error', info: 'alert-info', warning: 'alert-warning' }[type]
    const icon = { success: 'fa-check-circle', error: 'fa-times-circle', info: 'fa-info-circle', warning: 'fa-exclamation-triangle' }[type]
    toast.innerHTML = `<div class="alert ${alertClass} shadow-lg"><i class="fas ${icon}"></i><span>${message}</span></div>`
    document.body.appendChild(toast)
    setTimeout(() => toast.remove(), 3000)
  }

  // Plugin CRUD Operations
  const refreshPlugins = async () => {
    try {
      const response = await invoke<CommandResponse<PluginRecord[]>>('list_plugins')
      if (response.success && response.data) {
        plugins.value = response.data
      } else {
        console.error('Failed to refresh plugins:', response.error)
      }
    } catch (error) {
      console.error('Error refreshing plugins:', error)
    }
  }

  const togglePlugin = async (plugin: PluginRecord) => {
    try {
      const command = plugin.status === 'Enabled' ? 'disable_plugin' : 'enable_plugin'
      const actionText = plugin.status === 'Enabled' ? t('plugins.disable', '禁用') : t('plugins.enable', '启用')
      const unknownErrorText = t('common.unknownError', '未知错误')
      const response = await invoke<CommandResponse<void>>(command, { pluginId: plugin.metadata.id })
      if (response.success) {
        await refreshPlugins()
        showToast(t('plugins.toggleSuccess', { action: actionText, name: plugin.metadata.name }), 'success')
      } else {
        showToast(t('plugins.toggleFailed', { action: actionText, error: response.error || unknownErrorText }), 'error')
      }
    } catch (error) {
      console.error('Error toggling plugin:', error)
      showToast(t('plugins.toggleError', '操作失败'), 'error')
    }
  }

  const togglePluginFavorite = async (plugin: PluginRecord) => {
    try {
      const response: any = await invoke('toggle_plugin_favorite', { pluginId: plugin.metadata.id, userId: null })
      if (response.success) {
        const isFavorited = response.data?.is_favorited || false
        showToast(isFavorited ? t('plugins.favoritedSuccess', '已收藏') : t('plugins.unfavoritedSuccess', '已取消收藏'), 'success')
        await refreshPlugins()
      } else {
        showToast(t('plugins.favoriteError', '操作失败'), 'error')
      }
    } catch (error) {
      console.error('Error toggling favorite:', error)
      showToast(t('plugins.favoriteError', '操作失败'), 'error')
    }
  }

  // Batch operations
  const getCurrentPluginIds = (): string[] => filteredPlugins.value.map(p => p.metadata.id)

  const batchEnableCurrent = async () => {
    const ids = getCurrentPluginIds()
    if (ids.length === 0) return
    batchToggling.value = true
    try {
      const resp = await invoke<CommandResponse<BatchToggleResult>>('batch_enable_plugins', { pluginIds: ids })
      if (resp.success) {
        await refreshPlugins()
        const data = resp.data!
        showToast(`已启用 ${data.enabled_count}/${ids.length}，失败 ${data.failed_ids.length}`, 'success')
      } else {
        showToast(resp.error || '批量开启失败', 'error')
      }
    } catch (e: any) {
      showToast(e?.message || '批量开启操作异常', 'error')
    } finally {
      batchToggling.value = false
    }
  }

  const batchDisableCurrent = async () => {
    const ids = getCurrentPluginIds()
    if (ids.length === 0) return
    batchToggling.value = true
    try {
      const resp = await invoke<CommandResponse<BatchToggleResult>>('batch_disable_plugins', { pluginIds: ids })
      if (resp.success) {
        await refreshPlugins()
        const data = resp.data!
        showToast(`已禁用 ${data.disabled_count}/${ids.length}，失败 ${data.failed_ids.length}`, 'success')
      } else {
        showToast(resp.error || '批量停止失败', 'error')
      }
    } catch (e: any) {
      showToast(e?.message || '批量停止操作异常', 'error')
    } finally {
      batchToggling.value = false
    }
  }

  // Pagination
  const goToPluginPage = (page: number) => {
    if (page >= 1 && page <= pluginTotalPages.value) {
      pluginCurrentPage.value = page
    }
  }

  const changePluginPageSize = (size: number) => {
    pluginPageSize.value = size
    pluginCurrentPage.value = 1
  }

  const clearFilters = () => {
    pluginSearchText.value = ''
    selectedSubCategory.value = ''
    selectedTag.value = ''
    pluginCurrentPage.value = 1
  }

  const getAvailableSubCategories = (): string[] => {
    if (selectedCategory.value === 'traffic') {
      const cats = new Set(filteredPlugins.value.filter(p => !selectedSubCategory.value || p.metadata.category === selectedSubCategory.value).map(p => p.metadata.category))
      return Array.from(cats).sort()
    } else if (selectedCategory.value === 'agents') {
      const cats = new Set(filteredPlugins.value.filter(p => !selectedSubCategory.value || p.metadata.category === selectedSubCategory.value).map(p => p.metadata.category))
      return Array.from(cats).sort()
    }
    return []
  }

  const getAvailableTags = (): string[] => {
    const tags = new Set<string>()
    filteredPlugins.value.forEach(p => p.metadata.tags.forEach(tag => tags.add(tag)))
    return Array.from(tags).sort()
  }

  // Review methods
  const goToReviewPage = (page: number) => {
    if (page >= 1 && page <= reviewTotalPages.value) {
      reviewCurrentPage.value = page
      refreshReviewPlugins()
    }
  }

  const changeReviewPageSize = (size: number) => {
    reviewPageSize.value = size
    reviewCurrentPage.value = 1
    refreshReviewPlugins()
  }

  const changeReviewStatusFilter = (status: string) => {
    reviewStatusFilter.value = status
    reviewCurrentPage.value = 1
    refreshReviewPlugins()
  }

  const refreshReviewStats = async () => {
    try {
      const response: any = await invoke('get_plugin_review_statistics')
      if (response.success && response.data) {
        reviewStatsData.value = {
          total: response.data.total || 0,
          pending: response.data.pending || 0,
          approved: response.data.approved || 0,
          rejected: response.data.rejected || 0,
          failed: response.data.failed || 0
        }
      }
    } catch (error) {
      console.error('Error loading review statistics:', error)
    }
  }

  const refreshReviewPlugins = async () => {
    try {
      const response: any = await invoke('get_plugins_paginated', {
        page: reviewCurrentPage.value,
        pageSize: reviewPageSize.value,
        statusFilter: reviewStatusFilter.value === 'all' ? null : reviewStatusFilter.value,
        searchText: reviewSearchText.value || null,
        userId: null
      })
      if (response.success && response.data) {
        reviewPlugins.value = Array.isArray(response.data.data) ? response.data.data : []
        reviewTotalCount.value = response.data.total || 0
        reviewTotalPagesCount.value = response.data.total_pages || 0
      } else {
        reviewPlugins.value = []
        reviewTotalCount.value = 0
        reviewTotalPagesCount.value = 0
      }
      await refreshReviewStats()
    } catch (error) {
      console.error('Error loading review plugins:', error)
      showToast(t('plugins.loadReviewError', '加载审核插件失败'), 'error')
      reviewPlugins.value = []
      reviewTotalCount.value = 0
      reviewTotalPagesCount.value = 0
    }
  }

  const toggleSelectAll = () => {
    if (isAllSelected.value) {
      selectedReviewPlugins.value = []
    } else {
      selectedReviewPlugins.value = [...filteredReviewPlugins.value]
    }
  }

  const togglePluginSelection = (plugin: ReviewPlugin) => {
    const index = selectedReviewPlugins.value.findIndex(p => p.plugin_id === plugin.plugin_id)
    if (index > -1) {
      selectedReviewPlugins.value.splice(index, 1)
    } else {
      selectedReviewPlugins.value.push(plugin)
    }
  }

  const isPluginSelected = (plugin: ReviewPlugin): boolean => {
    return selectedReviewPlugins.value.some(p => p.plugin_id === plugin.plugin_id)
  }

  const approvePlugin = async (plugin: ReviewPlugin) => {
    if (!plugin) return
    try {
      const response: any = await invoke('approve_plugin', { pluginId: plugin.plugin_id })
      if (response.success) {
        plugin.status = 'Approved'
        await refreshReviewPlugins()
        await refreshPlugins()
        showToast(t('plugins.approveSuccess', { name: plugin.plugin_name }), 'success')
      } else {
        showToast(t('plugins.approveFailed', { error: response.message || t('common.unknownError', '未知错误') }), 'error')
      }
    } catch (error) {
      console.error('Error approving plugin:', error)
      showToast(t('plugins.approveFailed', { error: t('common.unknownError', '未知错误') }), 'error')
    }
  }

  const rejectPlugin = async (plugin: ReviewPlugin) => {
    if (!plugin) return
    try {
      const response: any = await invoke('reject_plugin', { pluginId: plugin.plugin_id, reason: 'Manual rejection' })
      if (response.success) {
        plugin.status = 'Rejected'
        await refreshReviewPlugins()
        await refreshPlugins()
        showToast(t('plugins.rejectSuccess', { name: plugin.plugin_name }), 'success')
      } else {
        showToast(t('plugins.rejectFailed', { error: response.message || t('common.unknownError', '未知错误') }), 'error')
      }
    } catch (error) {
      console.error('Error rejecting plugin:', error)
      showToast(t('plugins.rejectFailed', { error: t('common.unknownError', '未知错误') }), 'error')
    }
  }

  const approveSelected = async () => {
    if (selectedReviewPlugins.value.length === 0) return
    try {
      const pluginIds = selectedReviewPlugins.value.map(p => p.plugin_id)
      const response: any = await invoke('batch_approve_plugins', { pluginIds })
      if (response.success) {
        await refreshReviewPlugins()
        await refreshPlugins()
        selectedReviewPlugins.value = []
        showToast(t('plugins.batchApproveSuccess', { count: pluginIds.length }), 'success')
      } else {
        showToast(t('plugins.batchApproveFailed', { error: response.message || t('common.unknownError', '未知错误') }), 'error')
      }
    } catch (error) {
      console.error('Error batch approving plugins:', error)
      showToast(t('plugins.batchApproveFailed', { error: t('common.unknownError', '未知错误') }), 'error')
    }
  }

  const rejectSelected = async () => {
    if (selectedReviewPlugins.value.length === 0) return
    try {
      const pluginIds = selectedReviewPlugins.value.map(p => p.plugin_id)
      const response: any = await invoke('batch_reject_plugins', { pluginIds, reason: 'Batch rejection' })
      if (response.success) {
        await refreshReviewPlugins()
        await refreshPlugins()
        selectedReviewPlugins.value = []
        showToast(t('plugins.batchRejectSuccess', { count: pluginIds.length }), 'success')
      } else {
        showToast(t('plugins.batchRejectFailed', { error: response.message || t('common.unknownError', '未知错误') }), 'error')
      }
    } catch (error) {
      console.error('Error batch rejecting plugins:', error)
      showToast(t('plugins.batchRejectFailed', { error: t('common.unknownError', '未知错误') }), 'error')
    }
  }

  const getReviewStatusText = (status: string): string => {
    const statusMap: Record<string, string> = {
      'PendingReview': t('plugins.pendingReview', '待审核'),
      'Approved': t('plugins.approved', '已批准'),
      'Rejected': t('plugins.rejected', '已拒绝'),
      'ValidationFailed': t('plugins.validationFailed', '验证失败')
    }
    return statusMap[status] || status
  }

  // Event Listeners
  const setupEventListeners = async () => {
    pluginChangedUnlisten = await listen('plugin:changed', () => {
      refreshPlugins()
    })
  }

  const cleanupEventListeners = () => {
    if (pluginChangedUnlisten) {
      pluginChangedUnlisten()
    }
  }

  // Watch category changes
  watch(selectedCategory, () => {
    pluginCurrentPage.value = 1
  })

  return {
    // State
    selectedCategory,
    plugins,
    uploadDialog,
    codeEditorDialog,
    deleteDialog,
    aiGenerateDialog,
    testResultDialog,
    advancedDialog,
    reviewDetailDialog,
    fileInput,
    codeEditorContainer,
    reviewCodeEditorContainer,
    fullscreenCodeEditorContainer,
    reviewPlugins,
    selectedReviewPlugins,
    selectedReviewPlugin,
    reviewSearchText,
    reviewEditMode,
    editedReviewCode,
    savingReview,
    reviewStatusFilter,
    reviewCurrentPage,
    reviewPageSize,
    reviewTotalCount,
    reviewTotalPagesCount,
    reviewStatsData,
    pluginViewMode,
    pluginCurrentPage,
    pluginPageSize,
    pluginSearchText,
    selectedSubCategory,
    selectedTag,
    batchToggling,
    selectedFile,
    uploading,
    uploadError,
    editingPlugin,
    pluginCode,
    originalCode,
    isEditing,
    saving,
    codeError,
    isFullscreenEditor,
    deletingPlugin,
    deleting,
    newPluginMetadata,
    aiPrompt,
    aiPluginType,
    aiSeverity,
    aiGenerating,
    aiGenerateError,
    testing,
    testResult,
    advancedPlugin,
    advancedTesting,
    advancedError,
    advancedResult,
    advancedForm,

    // Computed
    categories,
    filteredPlugins,
    pluginTotalPages,
    paginatedPlugins,
    pluginPaginationInfo,
    reviewStats,
    filteredReviewPlugins,
    paginatedReviewPlugins,
    reviewTotalPages,
    reviewPaginationInfo,
    isAllSelected,
    sortedRuns,
    isAdvancedAgent,

    // Methods
    isPluginFavorited,
    isTrafficPluginType,
    isAgentPluginType,
    getStatusText,
    getCategoryLabel,
    getCategoryIcon,
    getCategoryCount,
    showToast,
    refreshPlugins,
    togglePlugin,
    togglePluginFavorite,
    batchEnableCurrent,
    batchDisableCurrent,
    goToPluginPage,
    changePluginPageSize,
    clearFilters,
    getAvailableSubCategories,
    getAvailableTags,
    goToReviewPage,
    changeReviewPageSize,
    changeReviewStatusFilter,
    refreshReviewStats,
    refreshReviewPlugins,
    toggleSelectAll,
    togglePluginSelection,
    isPluginSelected,
    approvePlugin,
    rejectPlugin,
    approveSelected,
    rejectSelected,
    getReviewStatusText,
    setupEventListeners,
    cleanupEventListeners,

    // CodeMirror
    codeEditorReadOnly,
    reviewCodeEditorReadOnly,
  }
}
