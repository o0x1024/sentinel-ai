<template>
  <div class="container mx-auto p-6">
    <div class="mb-6">
      <h1 class="text-3xl font-bold">{{ $t('plugins.title', '插件管理') }}</h1>
      <p class="text-base-content/70 mt-2">{{ $t('plugins.description', '管理和配置安全测试插件') }}</p>
    </div>

    <!-- Operation Bar -->
    <div class="flex gap-2 mb-6 flex-wrap">
      <button class="btn btn-primary" @click="openCreateDialog">
        <i class="fas fa-plus mr-2"></i>
        {{ $t('plugins.newPlugin', '新增插件') }}
      </button>
      <button class="btn btn-secondary" @click="openUploadDialog">
        <i class="fas fa-upload mr-2"></i>
        {{ $t('plugins.uploadPlugin', '上传插件') }}
      </button>
      <button class="btn btn-accent" @click="openAIGenerateDialog">
        <i class="fas fa-magic mr-2"></i>
        {{ $t('plugins.aiGenerate', 'AI生成插件') }}
      </button>
      <button class="btn btn-info" @click="refreshPlugins">
        <i class="fas fa-sync-alt mr-2"></i>
        {{ $t('common.refresh', '刷新列表') }}
      </button>
    </div>

    <!-- Category Filter -->
    <div class="tabs tabs-boxed mb-6 flex-wrap gap-2">
      <button v-for="cat in categories" :key="cat.value" class="tab"
        :class="{ 'tab-active': selectedCategory === cat.value }" @click="selectedCategory = cat.value">
        <i :class="cat.icon" class="mr-2"></i>
        {{ cat.label }}
        <span v-if="getCategoryCount(cat.value) > 0" class="ml-2 badge badge-sm">
          {{ getCategoryCount(cat.value) }}
        </span>
      </button>

      <!-- Plugin Review Tab -->
      <button class="tab" :class="{ 'tab-active': selectedCategory === 'review' }" @click="selectedCategory = 'review'">
        <i class="fas fa-check-double mr-2"></i>
        {{ $t('plugins.pluginReview', '插件审核') }}
        <span v-if="reviewStats.pending > 0" class="ml-2 badge badge-sm badge-warning">
          {{ reviewStats.pending }}
        </span>
      </button>

      <!-- Plugin Store Tab -->
      <button class="tab" :class="{ 'tab-active': selectedCategory === 'store' }" @click="selectedCategory = 'store'">
        <i class="fas fa-store mr-2"></i>
        {{ $t('plugins.store.title', '插件商店') }}
      </button>
    </div>

    <!-- Plugin Manager Content -->
    <div class="card bg-base-100 shadow-xl">
      <div class="card-body">
        <!-- Plugin Store Section -->
        <PluginStoreSection
          v-if="selectedCategory === 'store'"
          ref="pluginStoreSectionRef"
          :installed-plugin-ids="installedPluginIds"
          @plugin-installed="onPluginInstalled"
        />

        <!-- Plugin Review Section -->
        <PluginReviewSection
          v-else-if="selectedCategory === 'review'"
          :review-stats="reviewStats"
          :status-filter="reviewStatusFilter"
          :search-text="reviewSearchText"
          :paginated-plugins="paginatedReviewPlugins"
          :pagination-info="reviewPaginationInfo"
          :current-page="reviewCurrentPage"
          :page-size="reviewPageSize"
          :total-pages="reviewTotalPages"
          :selected-plugins="selectedReviewPlugins"
          :is-all-selected="isAllSelected"
          :is-plugin-selected="isPluginSelected"
          :get-review-status-text="getReviewStatusText"
          @change-status-filter="changeReviewStatusFilter"
          @update:search-text="reviewSearchText = $event; refreshReviewPlugins()"
          @approve-selected="approveSelected"
          @reject-selected="rejectSelected"
          @toggle-select-all="toggleSelectAll"
          @toggle-selection="togglePluginSelection"
          @view-detail="viewReviewPluginDetail"
          @approve-plugin="approvePlugin"
          @reject-plugin="rejectPlugin"
          @change-page-size="changeReviewPageSize"
          @go-to-page="goToReviewPage"
        />

        <!-- Regular Plugin List -->
        <PluginListSection
          v-else
          :selected-category="selectedCategory"
          :plugin-view-mode="pluginViewMode"
          :filtered-plugins="filteredPlugins"
          :paginated-plugins="paginatedPlugins"
          :pagination-info="pluginPaginationInfo"
          :current-page="pluginCurrentPage"
          :page-size="pluginPageSize"
          :total-pages="pluginTotalPages"
          :plugin-search-text="pluginSearchText"
          :selected-sub-category="selectedSubCategory"
          :selected-tag="selectedTag"
          :available-sub-categories="getAvailableSubCategories()"
          :available-tags="getAvailableTags()"
          :batch-toggling="batchToggling"
          :get-status-text="getStatusText"
          :get-category-label="getCategoryLabel"
          :get-category-icon="getCategoryIcon"
          :is-plugin-favorited="isPluginFavorited"
          :is-traffic-plugin-type="isTrafficPluginType"
          :is-agent-plugin-type="isAgentPluginType"
          @update:plugin-view-mode="pluginViewMode = $event"
          @update:search-text="pluginSearchText = $event"
          @update:sub-category="selectedSubCategory = $event"
          @update:tag="selectedTag = $event"
          @clear-filters="clearFilters"
          @batch-enable="batchEnableCurrent"
          @batch-disable="batchDisableCurrent"
          @change-page-size="changePluginPageSize"
          @go-to-page="goToPluginPage"
          @toggle-favorite="togglePluginFavorite"
          @test-plugin="testPlugin"
          @advanced-test="openAdvancedDialog"
          @toggle-plugin="togglePlugin"
          @view-code="viewPluginCode"
          @delete-plugin="confirmDeletePlugin"
        />
      </div>
    </div>

    <!-- Dialogs -->
    <PluginDialogs
      ref="pluginDialogsRef"
      :selected-review-plugin="selectedReviewPlugin"
      :review-edit-mode="reviewEditMode"
      :saving-review="savingReview"
      :selected-file="selectedFile"
      :uploading="uploading"
      :upload-error="uploadError"
      :deleting-plugin="deletingPlugin"
      :deleting="deleting"
      :ai-prompt="aiPrompt"
      :ai-plugin-type="aiPluginType"
      :ai-severity="aiSeverity"
      :ai-generating="aiGenerating"
      :ai-generate-error="aiGenerateError"
      :testing="testing"
      :test-result="testResult"
      :is-fullscreen-editor-mode="false"
      :advanced-plugin="advancedPlugin"
      :advanced-testing="advancedTesting"
      :advanced-error="advancedError"
      :advanced-result="advancedResult"
      :advanced-form="advancedForm"
      :is-advanced-agent="isAdvancedAgent"
      :sorted-runs="sortedRuns"
      @copy-review-code="copyReviewCode"
      @toggle-review-edit-mode="reviewEditMode = !reviewEditMode"
      @save-review-edit="saveReviewEdit"
      @approve-review-plugin="approvePlugin(selectedReviewPlugin!)"
      @reject-review-plugin="rejectPlugin(selectedReviewPlugin!)"
      @handle-file-select="handleFileSelect"
      @upload-plugin="uploadPlugin"
      @delete-plugin="deletePlugin"
      @update:ai-prompt="aiPrompt = $event"
      @update:ai-plugin-type="aiPluginType = $event"
      @update:ai-severity="aiSeverity = $event"
      @generate-plugin-with-ai="generatePluginWithAI"
      @run-advanced-test="runAdvancedTest"
      @update:advanced-form="advancedForm = $event"
      @close-review-detail-dialog="closeReviewDetailDialog"
      @close-upload-dialog="closeUploadDialog"
      @close-delete-dialog="closeDeleteDialog"
      @close-ai-generate-dialog="closeAIGenerateDialog"
      @close-test-result-dialog="closeTestResultDialog"
      @close-advanced-dialog="closeAdvancedDialog"
      @refer-test-result-to-ai="referTestResultToAi"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch, nextTick } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { useI18n } from 'vue-i18n'
import { EditorView, type ViewUpdate } from '@codemirror/view'
import { EditorState, Compartment } from '@codemirror/state'
import { basicSetup } from 'codemirror'
import { javascript } from '@codemirror/lang-javascript'
import { oneDark } from '@codemirror/theme-one-dark'

import PluginListSection from '@/components/PluginManagement/PluginListSection.vue'
import PluginReviewSection from '@/components/PluginManagement/PluginReviewSection.vue'
import PluginStoreSection from '@/components/PluginManagement/PluginStoreSection.vue'
import PluginDialogs from '@/components/PluginManagement/PluginDialogs.vue'
import { usePluginEditorStore } from '@/stores/pluginEditor'
import type {
  PluginRecord, ReviewPlugin, TestResult, AdvancedTestResult,
  CommandResponse, BatchToggleResult, NewPluginMetadata, AdvancedForm
} from '@/components/PluginManagement/types'
import { trafficCategories, agentsCategories } from '@/components/PluginManagement/types'

const { t } = useI18n()
const pluginEditorStore = usePluginEditorStore()

defineOptions({
  name: 'Plugin'
});

// Component refs
const pluginDialogsRef = ref<InstanceType<typeof PluginDialogs>>()
const pluginStoreSectionRef = ref<InstanceType<typeof PluginStoreSection>>()

// Component State
const selectedCategory = ref('all')
const plugins = ref<PluginRecord[]>([])

// Review Editor logic
let reviewCodeEditorView: EditorView | null = null
const reviewCodeEditorReadOnly = new Compartment()

// Review State
const reviewPlugins = ref<ReviewPlugin[]>([])
const selectedReviewPlugins = ref<ReviewPlugin[]>([])
const selectedReviewPlugin = ref<ReviewPlugin | null>(null)
const reviewSearchText = ref('')
const reviewEditMode = ref(false)
const editedReviewCode = ref('')
const savingReview = ref(false)
const reviewStatusFilter = ref<string>('all')
const reviewCurrentPage = ref(1)
const reviewPageSize = ref(10)
const reviewTotalCount = ref(0)
const reviewTotalPagesCount = ref(0)
const reviewStatsData = ref({ total: 0, pending: 0, approved: 0, rejected: 0, failed: 0 })

// Plugin List State
const pluginViewMode = ref<'favorited' | 'all'>('all')
const pluginCurrentPage = ref(1)
const pluginPageSize = ref(10)
const pluginSearchText = ref('')
const selectedSubCategory = ref('')
const selectedTag = ref('')
const batchToggling = ref(false)

// Upload State
const selectedFile = ref<File | null>(null)
const uploading = ref(false)
const uploadError = ref('')

// Delete State
const deletingPlugin = ref<PluginRecord | null>(null)
const deleting = ref(false)

// AI Generate State
const aiPrompt = ref('')
const aiPluginType = ref('traffic')
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
  url: 'https://example.com/test', method: 'GET',
  headersText: '{"User-Agent":"Sentinel-AdvTest/1.0"}', bodyText: '',
  agent_inputs_text: '{}', runs: 1, concurrency: 1
})

let pluginChangedUnlisten: UnlistenFn | null = null

// Computed Properties
const categories = computed(() => [
  { value: 'all', label: t('plugins.categories.all', '全部'), icon: 'fas fa-th' },
  { value: 'traffic', label: t('plugins.categories.trafficAnalysis', '流量分析插件'), icon: 'fas fa-shield-alt' },
  { value: 'agents', label: t('plugins.categories.agents', 'Agent工具插件'), icon: 'fas fa-robot' },
])

const filteredPlugins = computed(() => {
  let filtered = plugins.value

  if (selectedCategory.value === 'traffic') {
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
  } else if (selectedCategory.value !== 'all') {
    filtered = plugins.value.filter(p => p.metadata.category === selectedCategory.value)
  }

  if (['all', 'traffic', 'agents'].includes(selectedCategory.value) && pluginViewMode.value === 'favorited') {
    filtered = filtered.filter(p => isPluginFavorited(p))
  }

  if (pluginSearchText.value.trim()) {
    const query = pluginSearchText.value.toLowerCase()
    filtered = filtered.filter(p =>
      p.metadata.name.toLowerCase().includes(query) ||
      p.metadata.id.toLowerCase().includes(query) ||
      p.metadata.description?.toLowerCase().includes(query)
    )
  }

  if (selectedSubCategory.value) {
    filtered = filtered.filter(p => p.metadata.category === selectedSubCategory.value)
  }

  if (selectedTag.value) {
    filtered = filtered.filter(p => p.metadata.tags.includes(selectedTag.value))
  }

  return filtered
})

const pluginTotalPages = computed(() => Math.max(1, Math.ceil(filteredPlugins.value.length / pluginPageSize.value)))

const paginatedPlugins = computed(() => {
  const start = (pluginCurrentPage.value - 1) * pluginPageSize.value
  return filteredPlugins.value.slice(start, start + pluginPageSize.value)
})

const pluginPaginationInfo = computed(() => {
  const total = filteredPlugins.value.length
  const start = total > 0 ? (pluginCurrentPage.value - 1) * pluginPageSize.value + 1 : 0
  const end = Math.min(pluginCurrentPage.value * pluginPageSize.value, total)
  return { start, end, total }
})

const reviewStats = computed(() => reviewStatsData.value)
const paginatedReviewPlugins = computed(() => reviewPlugins.value)
const reviewTotalPages = computed(() => reviewTotalPagesCount.value)

const reviewPaginationInfo = computed(() => {
  const start = (reviewCurrentPage.value - 1) * reviewPageSize.value + 1
  const end = Math.min(reviewCurrentPage.value * reviewPageSize.value, reviewTotalCount.value)
  return { start, end, total: reviewTotalCount.value }
})

const isAllSelected = computed(() => {
  return paginatedReviewPlugins.value.length > 0 &&
    paginatedReviewPlugins.value.every(p => isPluginSelected(p))
})

const sortedRuns = computed(() => {
  if (!advancedResult.value) return []
  return [...advancedResult.value.runs].sort((a, b) => a.run_index - b.run_index)
})

const isAdvancedAgent = computed(() => advancedPlugin.value?.metadata?.main_category === 'agent')

// Installed plugin IDs for store section
const installedPluginIds = computed(() => plugins.value.map(p => p.metadata.id))

// Helper Functions
const isPluginFavorited = (plugin: PluginRecord): boolean => plugin.is_favorited || false

const isTrafficPluginType = (plugin: PluginRecord): boolean => {
  if (plugin.metadata.main_category === 'traffic') return true
  if (trafficCategories.includes(plugin.metadata.category)) return true
  return plugin.metadata.category === 'traffic'
}

const isAgentPluginType = (plugin: PluginRecord): boolean => {
  if (plugin.metadata.main_category === 'agent') return true
  return agentsCategories.includes(plugin.metadata.category)
}

const getStatusText = (status: string): string => {
  const map: Record<string, string> = { 'Enabled': t('plugins.enabled', '已启用'), 'Disabled': t('plugins.disabled', '已禁用'), 'Error': t('plugins.error', '错误') }
  return map[status] || status
}

const getCategoryLabel = (category: string): string => {
  const cat = categories.value.find(c => c.value === category)
  return cat ? cat.label : category
}

const getCategoryIcon = (category: string): string => {
  const cat = categories.value.find(c => c.value === category)
  if (cat) return cat.icon
  const icons: Record<string, string> = {
    'scanner': 'fas fa-radar', 'analyzer': 'fas fa-microscope', 'reporter': 'fas fa-file-alt',
    'sqli': 'fas fa-database', 'xss': 'fas fa-code', 'csrf': 'fas fa-shield-alt'
  }
  return icons[category] || 'fas fa-wrench'
}

const getCategoryCount = (category: string): number => {
  if (category === 'all') return plugins.value.length
  if (category === 'traffic') {
    return plugins.value.filter(p => p.metadata.main_category === 'traffic' || trafficCategories.includes(p.metadata.category)).length
  }
  if (category === 'agents') {
    return plugins.value.filter(p => p.metadata.main_category === 'agent' || agentsCategories.includes(p.metadata.category)).length
  }
  return plugins.value.filter(p => p.metadata.category === category).length
}

const getReviewStatusText = (status: string): string => {
  const map: Record<string, string> = {
    'PendingReview': t('plugins.pendingReview', '待审核'), 'Approved': t('plugins.approved', '已批准'),
    'Rejected': t('plugins.rejected', '已拒绝'), 'ValidationFailed': t('plugins.validationFailed', '验证失败')
  }
  return map[status] || status
}

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

// Plugin CRUD
const refreshPlugins = async () => {
  try {
    const response = await invoke<CommandResponse<PluginRecord[]>>('list_plugins')
    if (response.success && response.data) plugins.value = response.data
  } catch (error) {
    console.error('Error refreshing plugins:', error)
  }
}

const togglePlugin = async (plugin: PluginRecord) => {
  try {
    const command = plugin.status === 'Enabled' ? 'disable_plugin' : 'enable_plugin'
    const response = await invoke<CommandResponse<void>>(command, { pluginId: plugin.metadata.id })
    if (response.success) {
      await refreshPlugins()
      showToast(t('plugins.toggleSuccess', '操作成功'), 'success')
    } else {
      showToast(response.error || '操作失败', 'error')
    }
  } catch (error) {
    showToast('操作失败', 'error')
  }
}

const togglePluginFavorite = async (plugin: PluginRecord) => {
  try {
    const response: any = await invoke('toggle_plugin_favorite', { pluginId: plugin.metadata.id, userId: null })
    if (response.success) {
      showToast(response.data?.is_favorited ? '已收藏' : '已取消收藏', 'success')
      await refreshPlugins()
    }
  } catch (error) {
    showToast('操作失败', 'error')
  }
}

// Plugin store callback
const onPluginInstalled = async () => {
  await refreshPlugins()
}

// Batch operations
const batchEnableCurrent = async () => {
  const ids = filteredPlugins.value.map(p => p.metadata.id)
  if (ids.length === 0) return
  batchToggling.value = true
  try {
    const resp = await invoke<CommandResponse<BatchToggleResult>>('batch_enable_plugins', { pluginIds: ids })
    if (resp.success) {
      await refreshPlugins()
      showToast(`已启用 ${resp.data?.enabled_count}/${ids.length}`, 'success')
    }
  } catch (e: any) {
    showToast(e?.message || '操作失败', 'error')
  } finally {
    batchToggling.value = false
  }
}

const batchDisableCurrent = async () => {
  const ids = filteredPlugins.value.map(p => p.metadata.id)
  if (ids.length === 0) return
  batchToggling.value = true
  try {
    const resp = await invoke<CommandResponse<BatchToggleResult>>('batch_disable_plugins', { pluginIds: ids })
    if (resp.success) {
      await refreshPlugins()
      showToast(`已禁用 ${resp.data?.disabled_count}/${ids.length}`, 'success')
    }
  } catch (e: any) {
    showToast(e?.message || '操作失败', 'error')
  } finally {
    batchToggling.value = false
  }
}

// Pagination
const goToPluginPage = (page: number) => {
  if (page >= 1 && page <= pluginTotalPages.value) pluginCurrentPage.value = page
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
  if (selectedCategory.value === 'traffic' || selectedCategory.value === 'agents') {
    const cats = new Set(filteredPlugins.value.map(p => p.metadata.category))
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
    reviewPlugins.value = []
  }
}

const toggleSelectAll = () => {
  if (isAllSelected.value) {
    selectedReviewPlugins.value = []
  } else {
    selectedReviewPlugins.value = [...paginatedReviewPlugins.value]
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

const viewReviewPluginDetail = async (plugin: ReviewPlugin) => {
  selectedReviewPlugin.value = plugin
  editedReviewCode.value = plugin.code
  reviewEditMode.value = false
  pluginDialogsRef.value?.showReviewDetailDialog()
  await nextTick()
  initReviewCodeEditor()
}

const initReviewCodeEditor = () => {
  const container = pluginDialogsRef.value?.reviewCodeEditorContainerRef
  if (!container) return

  if (reviewCodeEditorView) {
    reviewCodeEditorView.destroy()
    reviewCodeEditorView = null
  }

  reviewCodeEditorView = new EditorView({
    doc: selectedReviewPlugin.value?.code || '',
    extensions: [
      basicSetup,
      javascript(),
      oneDark,
      EditorView.lineWrapping,
      reviewCodeEditorReadOnly.of(EditorView.editable.of(reviewEditMode.value)),
      EditorView.updateListener.of((update: ViewUpdate) => {
        if (update.docChanged) editedReviewCode.value = update.state.doc.toString()
      })
    ],
    parent: container
  })
}

const closeReviewDetailDialog = () => {
  if (reviewCodeEditorView) {
    reviewCodeEditorView.destroy()
    reviewCodeEditorView = null
  }
  selectedReviewPlugin.value = null
  reviewEditMode.value = false
}

const approvePlugin = async (plugin: ReviewPlugin) => {
  if (!plugin) return
  try {
    const response: any = await invoke('approve_plugin', { pluginId: plugin.plugin_id })
    if (response.success) {
      plugin.status = 'Approved'
      await refreshReviewPlugins()
      await refreshPlugins()
      showToast('插件已批准', 'success')
      pluginDialogsRef.value?.closeReviewDetailDialog()
    } else {
      showToast(response.message || '批准失败', 'error')
    }
  } catch (error) {
    showToast('批准失败', 'error')
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
      showToast('插件已拒绝', 'success')
      pluginDialogsRef.value?.closeReviewDetailDialog()
    } else {
      showToast(response.message || '拒绝失败', 'error')
    }
  } catch (error) {
    showToast('拒绝失败', 'error')
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
      showToast(`已批准 ${pluginIds.length} 个插件`, 'success')
    }
  } catch (error) {
    showToast('批量批准失败', 'error')
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
      showToast(`已拒绝 ${pluginIds.length} 个插件`, 'success')
    }
  } catch (error) {
    showToast('批量拒绝失败', 'error')
  }
}

const copyReviewCode = () => {
  if (selectedReviewPlugin.value) {
    navigator.clipboard.writeText(selectedReviewPlugin.value.code)
    showToast('代码已复制', 'success')
  }
}

const saveReviewEdit = async () => {
  if (!selectedReviewPlugin.value) return
  savingReview.value = true
  try {
    const response: any = await invoke('review_update_plugin_code', {
      pluginId: selectedReviewPlugin.value.plugin_id,
      pluginCode: editedReviewCode.value
    })
    if (response.success) {
      selectedReviewPlugin.value.code = editedReviewCode.value
      reviewEditMode.value = false
      await refreshReviewPlugins()
      showToast('代码已保存', 'success')
    }
  } catch (error) {
    showToast('保存失败', 'error')
  } finally {
    savingReview.value = false
  }
}

// Upload methods
const openUploadDialog = () => {
  uploadError.value = ''
  selectedFile.value = null
  pluginDialogsRef.value?.showUploadDialog()
}

const closeUploadDialog = () => {
  selectedFile.value = null
  uploadError.value = ''
}

const handleFileSelect = (event: Event) => {
  const target = event.target as HTMLInputElement
  if (target.files && target.files.length > 0) {
    selectedFile.value = target.files[0]
  }
}

const uploadPlugin = async () => {
  if (!selectedFile.value) return
  uploading.value = true
  uploadError.value = ''
  try {
    const content = await selectedFile.value.text()
    const response = await invoke<CommandResponse<string>>('upload_plugin', {
      filename: selectedFile.value.name,
      content
    })
    if (response.success) {
      pluginDialogsRef.value?.closeUploadDialog()
      await refreshPlugins()
      showToast('插件上传成功', 'success')
    } else {
      uploadError.value = response.error || '上传失败'
    }
  } catch (error) {
    uploadError.value = error instanceof Error ? error.message : '上传失败'
  } finally {
    uploading.value = false
  }
}

// Delete methods
const confirmDeletePlugin = (plugin: PluginRecord) => {
  deletingPlugin.value = plugin
  pluginDialogsRef.value?.showDeleteDialog()
}

const closeDeleteDialog = () => {
  deletingPlugin.value = null
}

const deletePlugin = async () => {
  if (!deletingPlugin.value) return
  deleting.value = true
  try {
    const response = await invoke<CommandResponse<void>>('delete_plugin', { pluginId: deletingPlugin.value.metadata.id })
    if (response.success) {
      pluginDialogsRef.value?.closeDeleteDialog()
      await refreshPlugins()
      showToast('插件已删除', 'success')
    } else {
      showToast(response.error || '删除失败', 'error')
    }
  } catch (error) {
    showToast('删除失败', 'error')
  } finally {
    deleting.value = false
  }
}

// AI Generate methods
const openAIGenerateDialog = () => {
  aiPrompt.value = ''
  aiGenerateError.value = ''
  pluginDialogsRef.value?.showAIGenerateDialog()
}

const closeAIGenerateDialog = () => {
  aiPrompt.value = ''
  aiGenerateError.value = ''
}

const generatePluginWithAI = async () => {
  if (!aiPrompt.value.trim()) return
  aiGenerating.value = true
  aiGenerateError.value = ''
  
  const isAgentPlugin = aiPluginType.value === 'agent'
  const streamId = `plugin_gen_${Date.now()}`
  
  try {
    const systemPrompt = await invoke<string>('get_combined_plugin_prompt_api', {
      pluginType: isAgentPlugin ? 'agent' : 'traffic',
      vulnType: 'custom',
      severity: aiSeverity.value
    })
    
    const userPrompt = `please generate ${isAgentPlugin ? 'Agent tool' : 'traffic analysis'} plugin code based on the following requirements:\n\n${aiPrompt.value}`

    let generatedCode = ''
    let streamCompleted = false
    let streamError = ''

    const unlistenDelta = await listen('plugin_gen_delta', (event: any) => {
      if (event.payload.stream_id === streamId) {
        generatedCode += event.payload.delta || ''
      }
    })

    const unlistenComplete = await listen('plugin_gen_complete', (event: any) => {
      if (event.payload.stream_id === streamId) {
        generatedCode = event.payload.content || generatedCode
        streamCompleted = true
      }
    })

    const unlistenError = await listen('plugin_gen_error', (event: any) => {
      if (event.payload.stream_id === streamId) {
        streamError = event.payload.error || 'AI generation failed'
        streamCompleted = true
      }
    })

    try {
      await invoke('generate_plugin_stream', {
        request: {
          stream_id: streamId,
          message: userPrompt,
          system_prompt: systemPrompt,
          service_name: 'default_llm_provider',
        }
      })

      const maxWaitTime = 120000
      const startTime = Date.now()
      while (!streamCompleted && (Date.now() - startTime < maxWaitTime)) {
        await new Promise(resolve => setTimeout(resolve, 100))
      }

      if (streamError) throw new Error(streamError)
      if (!generatedCode.trim()) throw new Error('AI did not return any code')

      generatedCode = generatedCode.trim()
        .replace(/```typescript\n?/g, '').replace(/```ts\n?/g, '')
        .replace(/```javascript\n?/g, '').replace(/```js\n?/g, '')
        .replace(/```\n?/g, '').trim()

      const pluginId = aiPrompt.value.toLowerCase().replace(/[^a-z0-9]+/g, '_').substring(0, 50) || 'ai_generated_plugin'

      const metadata: NewPluginMetadata = {
        id: pluginId,
        name: aiPrompt.value.substring(0, 50),
        version: '1.0.0',
        author: 'AI Generated',
        mainCategory: isAgentPlugin ? 'agent' : 'traffic',
        category: '',
        default_severity: aiSeverity.value,
        description: aiPrompt.value,
        tagsString: `ai-generated, ${aiPluginType.value}`
      }

      pluginDialogsRef.value?.closeAIGenerateDialog()
      pluginEditorStore.openEditor(null, generatedCode, metadata)
    } finally {
      unlistenDelta()
      unlistenComplete()
      unlistenError()
    }
  } catch (error) {
    aiGenerateError.value = error instanceof Error ? error.message : 'AI generation failed'
  } finally {
    aiGenerating.value = false
  }
}

// Code Editor methods
const openCreateDialog = async () => {
  pluginEditorStore.openEditor()
}

const viewPluginCode = async (plugin: PluginRecord) => {
  try {
    const response = await invoke<CommandResponse<string>>('get_plugin_code', { pluginId: plugin.metadata.id })
    if (response.success) {
      pluginEditorStore.openEditor(plugin, response.data || '')
    } else {
      showToast(response.error || 'Failed to read code', 'error')
    }
  } catch (error) {
    showToast('Failed to read code', 'error')
  }
}

const referTestResultToAi = () => {
  // Not implemented here anymore, handled by store/GlobalPluginEditor
}

// Test methods
const testPlugin = async (plugin: PluginRecord) => {
  if (!plugin?.metadata?.id) return
  const isAgentPlugin = plugin.metadata.main_category === 'agent'
  testing.value = true
  testResult.value = null

  try {
    if (isAgentPlugin) {
      await testAgentPlugin(plugin)
    } else {
      await testTrafficPlugin(plugin)
    }
  } catch (e) {
    const msg = e instanceof Error ? e.message : '测试失败'
    testResult.value = { success: false, message: msg, error: msg }
    pluginDialogsRef.value?.showTestResultDialog()
  } finally {
    testing.value = false
  }
}

const testTrafficPlugin = async (plugin: PluginRecord) => {
  const resp = await invoke<CommandResponse<TestResult>>('test_plugin', { pluginId: plugin.metadata.id })
  if (resp.success && resp.data) {
    testResult.value = resp.data
    pluginDialogsRef.value?.showTestResultDialog()
    showToast(resp.data.success ? '测试完成' : '测试失败', resp.data.success ? 'success' : 'error')
  } else {
    testResult.value = { success: false, message: resp.error || '测试失败', error: resp.error }
    pluginDialogsRef.value?.showTestResultDialog()
  }
}

const testAgentPlugin = async (plugin: PluginRecord) => {
  const result = await invoke<CommandResponse<any>>('test_agent_plugin', { 
    pluginId: plugin.metadata.id,
    inputs: {}
  })
  
  if (result.success && result.data) {
    const data = result.data
    testResult.value = {
      success: data.success,
      message: data.message || (data.success ? `插件执行完成 (${data.execution_time_ms}ms)` : '测试失败'),
      findings: [{ 
        title: 'Agent工具执行结果', 
        description: JSON.stringify(data.output ?? { error: data.error }, null, 2), 
        severity: data.success ? 'info' : 'error' 
      }],
      error: data.error
    }
  } else {
    testResult.value = {
      success: false,
      message: result.error || '测试失败',
      error: result.error
    }
  }
  pluginDialogsRef.value?.showTestResultDialog()
}

const closeTestResultDialog = () => {
  testResult.value = null
}

// Advanced Test methods
const openAdvancedDialog = async (plugin: PluginRecord) => {
  advancedPlugin.value = plugin
  advancedError.value = ''
  advancedResult.value = null
  
  const isAgent = plugin.metadata.main_category === 'agent'
  if (isAgent) {
    try {
      const schemaResp = await invoke<CommandResponse<any>>('get_plugin_input_schema', {
        pluginId: plugin.metadata.id
      })
      if (schemaResp.success && schemaResp.data) {
        const skeleton = buildSkeletonFromSchema(schemaResp.data)
        advancedForm.value.agent_inputs_text = JSON.stringify(skeleton, null, 2)
      } else {
        advancedForm.value.agent_inputs_text = '{}'
      }
    } catch (e) {
      console.error('Failed to get plugin input schema:', e)
      advancedForm.value.agent_inputs_text = '{}'
    }
  }
  
  pluginDialogsRef.value?.showAdvancedDialog()
}

const closeAdvancedDialog = () => {
  advancedPlugin.value = null
  advancedError.value = ''
  advancedResult.value = null
}

const buildSkeletonFromSchema = (schema: any): any => {
  const t = String(schema?.type || '').toLowerCase()
  if (t === 'object' || (!t && schema?.properties)) {
    const props = schema?.properties || {}
    const result: Record<string, any> = {}
    for (const key of Object.keys(props)) {
      const propSchema = props[key]
      if (propSchema?.default !== undefined) {
        result[key] = propSchema.default
      } else {
        result[key] = buildSkeletonFromSchema(propSchema)
      }
    }
    return result
  }
  if (t === 'array') return schema?.items ? [buildSkeletonFromSchema(schema.items)] : []
  if (t === 'string') return ''
  if (t === 'number' || t === 'integer') return 0
  if (t === 'boolean') return false
  return {}
}

const runAdvancedTest = async () => {
  if (!advancedPlugin.value) return
  advancedTesting.value = true
  advancedError.value = ''
  advancedResult.value = null

  try {
    const isAgent = advancedPlugin.value.metadata.main_category === 'agent'
    
    if (isAgent) {
      let inputs: Record<string, any> = {}
      try {
        inputs = JSON.parse(advancedForm.value.agent_inputs_text || '{}')
      } catch (e) {
        advancedError.value = 'JSON格式错误'
        advancedTesting.value = false
        return
      }

      const startTime = Date.now()
      const runs = []
      let totalFindings = 0
      const allOutputs = []

      for (let i = 0; i < advancedForm.value.runs; i++) {
        const runStart = Date.now()
        try {
          const resp = await invoke<CommandResponse<any>>('test_agent_plugin', {
            pluginId: advancedPlugin.value.metadata.id,
            inputs
          })
          const result = resp.data
          runs.push({
            run_index: i + 1,
            duration_ms: Date.now() - runStart,
            findings: result?.output?.findings?.length || 0,
            error: result?.error || null,
            output: result
          })
          allOutputs.push(result)
          totalFindings += result?.output?.findings?.length || 0
        } catch (e: any) {
          const errorOutput = { error: e?.message || 'Error', success: false }
          runs.push({ 
            run_index: i + 1, 
            duration_ms: Date.now() - runStart, 
            findings: 0, 
            error: e?.message || 'Error',
            output: errorOutput
          })
          allOutputs.push(errorOutput)
        }
      }

      advancedResult.value = {
        plugin_id: advancedPlugin.value.metadata.id,
        success: true,
        total_runs: advancedForm.value.runs,
        concurrency: advancedForm.value.concurrency,
        total_duration_ms: Date.now() - startTime,
        avg_duration_ms: (Date.now() - startTime) / advancedForm.value.runs,
        total_findings: totalFindings,
        unique_findings: totalFindings,
        findings: [],
        runs,
        outputs: allOutputs
      }
      advancedTesting.value = false
    } else {
      let headers: Record<string, string> = {}
      try {
        headers = JSON.parse(advancedForm.value.headersText || '{}')
      } catch (e) {
        advancedError.value = '请求头JSON格式错误'
        advancedTesting.value = false
        return
      }

      const response = await invoke<CommandResponse<AdvancedTestResult>>('test_plugin_advanced', {
        pluginId: advancedPlugin.value.metadata.id,
        runs: advancedForm.value.runs,
        concurrency: advancedForm.value.concurrency,
        url: advancedForm.value.url,
        method: advancedForm.value.method,
        headers,
        body: advancedForm.value.bodyText || null
      })

      if (response.success && response.data) {
        advancedResult.value = response.data
      } else {
        advancedError.value = response.error || '高级测试失败'
      }
      advancedTesting.value = false
    }
  } catch (error) {
    advancedError.value = error instanceof Error ? error.message : '高级测试失败'
    advancedTesting.value = false
  }
} 

// Event listeners
const setupEventListeners = async () => {
  pluginChangedUnlisten = await listen('plugin:changed', () => refreshPlugins())
}

// Watchers
watch(reviewEditMode, (newValue) => {
  if (reviewCodeEditorView) {
    reviewCodeEditorView.dispatch({ effects: reviewCodeEditorReadOnly.reconfigure(EditorView.editable.of(newValue)) })
  }
})

watch(selectedCategory, () => {
  pluginCurrentPage.value = 1
})

// Lifecycle
onMounted(async () => {
  await refreshPlugins()
  await refreshReviewPlugins()
  await setupEventListeners()
})

onUnmounted(() => {
  if (pluginChangedUnlisten) pluginChangedUnlisten()
  if (reviewCodeEditorView) { reviewCodeEditorView.destroy(); reviewCodeEditorView = null }
})
</script>

<style scoped>
.tabs {
  max-width: 100%;
}

.tab {
  flex-shrink: 0;
}

.table th {
  background-color: hsl(var(--b2));
}

:deep(.cm-editor) {
  height: 600px;
  font-size: calc(var(--font-size-base, 14px) * 0.857);
  font-family: 'Consolas', 'Monaco', 'Courier New', monospace;
}

:deep(.cm-scroller) {
  overflow: auto;
}

:deep(.cm-gutters) {
  background-color: #282c34;
  color: #5c6370;
  border: none;
}
</style>
