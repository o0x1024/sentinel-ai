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
    </div>

    <!-- Plugin Manager Content -->
    <div class="card bg-base-100 shadow-xl">
      <div class="card-body">
        <!-- Plugin Review Section -->
        <PluginReviewSection
          v-if="selectedCategory === 'review'"
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
          :ispassive-plugin-type="ispassivePluginType"
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
    />

    <!-- Code Editor Dialog -->
    <PluginCodeEditorDialog
      ref="codeEditorDialogRef"
      :editing-plugin="editingPlugin"
      :new-plugin-metadata="newPluginMetadata"
      :is-editing="isEditing"
      :saving="saving"
      :code-error="codeError"
      :is-fullscreen-editor="isFullscreenEditor"
      :sub-categories="subCategories"
      :show-ai-panel="showAiPanel"
      :ai-messages="aiChatMessages"
      :ai-streaming="aiChatStreaming"
      :ai-streaming-content="aiChatStreamingContent"
      :selected-code-ref="selectedCodeRef"
      @update:new-plugin-metadata="newPluginMetadata = $event"
      @insert-template="insertTemplate"
      @format-code="formatCode"
      @copy-plugin="copyPlugin"
      @toggle-fullscreen="toggleFullscreenEditor"
      @enable-editing="enableEditing"
      @cancel-editing="cancelEditing"
      @save-plugin="savePlugin"
      @create-new-plugin="createNewPlugin"
      @close="closeCodeEditorDialog"
      @toggle-ai-panel="toggleAiPanel"
      @send-ai-message="sendAiChatMessage"
      @ai-quick-action="handleAiQuickAction"
      @apply-ai-code="applyAiCode"
      @preview-ai-code="previewAiCode"
      @add-selected-code="addSelectedCodeToContext"
      @add-full-code="addFullCodeToContext"
      @clear-code-ref="clearCodeRef"
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
import PluginDialogs from '@/components/PluginManagement/PluginDialogs.vue'
import PluginCodeEditorDialog from '@/components/PluginManagement/PluginCodeEditorDialog.vue'
import type {
  PluginRecord, ReviewPlugin, TestResult, AdvancedTestResult,
  CommandResponse, BatchToggleResult, NewPluginMetadata, AdvancedForm, SubCategory
} from '@/components/PluginManagement/types'
import { passiveCategories, agentsCategories, mainCategories } from '@/components/PluginManagement/types'

const { t } = useI18n()

// Component refs
const pluginDialogsRef = ref<InstanceType<typeof PluginDialogs>>()
const codeEditorDialogRef = ref<InstanceType<typeof PluginCodeEditorDialog>>()

// Component State
const selectedCategory = ref('all')
const plugins = ref<PluginRecord[]>([])

// CodeMirror
let codeEditorView: EditorView | null = null
let reviewCodeEditorView: EditorView | null = null
let fullscreenCodeEditorView: EditorView | null = null
const codeEditorReadOnly = new Compartment()
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

// Editor State
const editingPlugin = ref<PluginRecord | null>(null)
const pluginCode = ref('')
const originalCode = ref('')
const isEditing = ref(false)
const saving = ref(false)
const codeError = ref('')
const isFullscreenEditor = ref(false)

// Delete State
const deletingPlugin = ref<PluginRecord | null>(null)
const deleting = ref(false)

// New Plugin Metadata
const newPluginMetadata = ref<NewPluginMetadata>({
  id: '', name: '', version: '1.0.0', author: '',
  mainCategory: 'passive', category: 'vulnerability',
  default_severity: 'medium', description: '', tagsString: ''
})

// AI Generate State
const aiPrompt = ref('')
const aiPluginType = ref('passive')
const aiSeverity = ref('medium')
const aiGenerating = ref(false)
const aiGenerateError = ref('')

// Code reference type
interface CodeReference {
  code: string
  preview: string
  startLine: number
  endLine: number
  isFullCode: boolean
}

// AI Chat State (for code editor)
interface AiChatMessage {
  role: 'user' | 'assistant'
  content: string
  codeBlock?: string
  codeRef?: CodeReference
}
const showAiPanel = ref(false)
const aiChatMessages = ref<AiChatMessage[]>([])
const aiChatStreaming = ref(false)
const aiChatStreamingContent = ref('')
const selectedCodeRef = ref<CodeReference | null>(null)

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
  { value: 'passive', label: t('plugins.categories.passive', '被动扫描插件'), icon: 'fas fa-shield-alt' },
  { value: 'agents', label: t('plugins.categories.agents', 'Agent工具插件'), icon: 'fas fa-robot' },
])

const subCategories = computed<SubCategory[]>(() => {
  if (newPluginMetadata.value.mainCategory === 'passive') {
    return [
      { value: 'sqli', label: 'SQL注入', icon: 'fas fa-database' },
      { value: 'command_injection', label: '命令注入', icon: 'fas fa-terminal' },
      { value: 'xss', label: '跨站脚本', icon: 'fas fa-code' },
      { value: 'idor', label: '越权访问', icon: 'fas fa-user-lock' },
      { value: 'auth_bypass', label: '认证绕过', icon: 'fas fa-unlock' },
      { value: 'csrf', label: 'CSRF', icon: 'fas fa-shield-alt' },
      { value: 'info_leak', label: '信息泄露', icon: 'fas fa-eye-slash' },
      { value: 'file_upload', label: '文件上传', icon: 'fas fa-file-upload' },
      { value: 'file_inclusion', label: '文件包含', icon: 'fas fa-file-code' },
      { value: 'path_traversal', label: '目录穿越', icon: 'fas fa-folder-open' },
      { value: 'xxe', label: 'XXE', icon: 'fas fa-file-code' },
      { value: 'ssrf', label: 'SSRF', icon: 'fas fa-server' },
      { value: 'custom', label: '自定义', icon: 'fas fa-wrench' }
    ]
  } else if (newPluginMetadata.value.mainCategory === 'agent') {
    return [
      { value: 'scanner', label: '扫描工具', icon: 'fas fa-radar' },
      { value: 'analyzer', label: '分析工具', icon: 'fas fa-microscope' },
      { value: 'reporter', label: '报告工具', icon: 'fas fa-file-alt' },
      { value: 'recon', label: '信息收集', icon: 'fas fa-search' },
      { value: 'exploit', label: '漏洞利用', icon: 'fas fa-bomb' },
      { value: 'utility', label: '实用工具', icon: 'fas fa-toolbox' },
      { value: 'custom', label: '自定义', icon: 'fas fa-wrench' }
    ]
  }
  return []
})

const filteredPlugins = computed(() => {
  let filtered = plugins.value

  if (selectedCategory.value === 'passive') {
    filtered = plugins.value.filter(p => {
      if (p.metadata.main_category === 'passive') return true
      if (passiveCategories.includes(p.metadata.category)) return true
      if (p.metadata.category === 'passive') return true
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

  if (['all', 'passive', 'agents'].includes(selectedCategory.value) && pluginViewMode.value === 'favorited') {
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

// Helper Functions
const isPluginFavorited = (plugin: PluginRecord): boolean => plugin.is_favorited || false

const ispassivePluginType = (plugin: PluginRecord): boolean => {
  if (plugin.metadata.main_category === 'passive') return true
  if (passiveCategories.includes(plugin.metadata.category)) return true
  return plugin.metadata.category === 'passive'
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
  if (category === 'passive') {
    return plugins.value.filter(p => p.metadata.main_category === 'passive' || passiveCategories.includes(p.metadata.category)).length
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
  if (selectedCategory.value === 'passive' || selectedCategory.value === 'agents') {
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
      pluginType: isAgentPlugin ? 'agent' : 'passive',
      vulnType: 'custom',
      severity: aiSeverity.value
    })
    
    const userPrompt = `请根据以下需求生成${isAgentPlugin ? 'Agent工具' : '被动扫描'}插件代码：\n\n${aiPrompt.value}`

    let generatedCode = ''
    let streamCompleted = false
    let streamError = ''

    // Listen to lightweight plugin generation events
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
        streamError = event.payload.error || 'AI生成失败'
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
      if (!generatedCode.trim()) throw new Error('AI未返回任何代码')

      generatedCode = generatedCode.trim()
        .replace(/```typescript\n?/g, '').replace(/```ts\n?/g, '')
        .replace(/```javascript\n?/g, '').replace(/```js\n?/g, '')
        .replace(/```\n?/g, '').trim()

      const pluginId = aiPrompt.value.toLowerCase().replace(/[^a-z0-9]+/g, '_').substring(0, 50) || 'ai_generated_plugin'

      newPluginMetadata.value = {
        id: pluginId,
        name: aiPrompt.value.substring(0, 50),
        version: '1.0.0',
        author: 'AI Generated',
        mainCategory: isAgentPlugin ? 'agent' : 'passive',
        category: '',
        default_severity: aiSeverity.value,
        description: aiPrompt.value,
        tagsString: `ai-generated, ${aiPluginType.value}`
      }

      pluginCode.value = generatedCode
      editingPlugin.value = null

      pluginDialogsRef.value?.closeAIGenerateDialog()
      codeEditorDialogRef.value?.showDialog()
      await nextTick()
      initCodeEditor()
    } finally {
      unlistenDelta()
      unlistenComplete()
      unlistenError()
    }
  } catch (error) {
    aiGenerateError.value = error instanceof Error ? error.message : 'AI生成失败'
  } finally {
    aiGenerating.value = false
  }
}

// Code Editor methods
const openCreateDialog = async () => {
  newPluginMetadata.value = {
    id: '', name: '', version: '1.0.0', author: '',
    mainCategory: 'passive', category: 'vulnerability',
    default_severity: 'medium', description: '', tagsString: ''
  }
  pluginCode.value = ''
  editingPlugin.value = null
  isEditing.value = false
  codeError.value = ''
  codeEditorDialogRef.value?.showDialog()
  await nextTick()
  initCodeEditor()
}

const viewPluginCode = async (plugin: PluginRecord) => {
  try {
    const response = await invoke<CommandResponse<string>>('get_plugin_code', { pluginId: plugin.metadata.id })
    if (response.success) {
      pluginCode.value = response.data || ''
      originalCode.value = response.data || ''
      editingPlugin.value = plugin
      newPluginMetadata.value = {
        id: plugin.metadata.id,
        name: plugin.metadata.name,
        version: plugin.metadata.version,
        author: plugin.metadata.author || '',
        mainCategory: plugin.metadata.main_category,
        category: plugin.metadata.category,
        default_severity: plugin.metadata.default_severity,
        description: plugin.metadata.description || '',
        tagsString: plugin.metadata.tags.join(', ')
      }
      isEditing.value = false
      codeError.value = ''
      codeEditorDialogRef.value?.showDialog()
      await nextTick()
      initCodeEditor()
    } else {
      showToast(response.error || '读取代码失败', 'error')
    }
  } catch (error) {
    showToast('读取代码失败', 'error')
  }
}

const closeCodeEditorDialog = () => {
  if (fullscreenCodeEditorView) {
    fullscreenCodeEditorView.destroy()
    fullscreenCodeEditorView = null
  }
  isFullscreenEditor.value = false
  if (codeEditorView) {
    codeEditorView.destroy()
    codeEditorView = null
  }
  editingPlugin.value = null
  pluginCode.value = ''
  originalCode.value = ''
  isEditing.value = false
  codeError.value = ''
  // Reset AI chat state
  showAiPanel.value = false
  aiChatMessages.value = []
  aiChatStreaming.value = false
  aiChatStreamingContent.value = ''
  selectedCodeRef.value = null
}

// AI Chat methods
const toggleAiPanel = () => {
  showAiPanel.value = !showAiPanel.value
}

// Get selected code from editor
const getSelectedCode = (): { code: string; from: number; to: number } | null => {
  const editorView = isFullscreenEditor.value ? fullscreenCodeEditorView : codeEditorView
  if (!editorView) return null
  
  const selection = editorView.state.selection.main
  if (selection.empty) return null
  
  const code = editorView.state.sliceDoc(selection.from, selection.to)
  return { code, from: selection.from, to: selection.to }
}

// Get line numbers from position
const getLineNumbers = (from: number, to: number): { startLine: number; endLine: number } => {
  const editorView = isFullscreenEditor.value ? fullscreenCodeEditorView : codeEditorView
  if (!editorView) return { startLine: 1, endLine: 1 }
  
  const doc = editorView.state.doc
  const startLine = doc.lineAt(from).number
  const endLine = doc.lineAt(to).number
  return { startLine, endLine }
}

// Create preview with ellipsis for long code
const createCodePreview = (code: string, maxLines: number = 5): string => {
  const lines = code.split('\n')
  if (lines.length <= maxLines) return code
  return lines.slice(0, maxLines).join('\n') + '\n... (' + (lines.length - maxLines) + ' more lines)'
}

// Add selected code to context
const addSelectedCodeToContext = () => {
  const selected = getSelectedCode()
  if (!selected || !selected.code.trim()) {
    showToast(t('plugins.noCodeSelected', '请先选择代码'), 'warning')
    return
  }
  
  const { startLine, endLine } = getLineNumbers(selected.from, selected.to)
  selectedCodeRef.value = {
    code: selected.code,
    preview: createCodePreview(selected.code),
    startLine,
    endLine,
    isFullCode: false
  }
}

// Add full code to context
const addFullCodeToContext = () => {
  if (!pluginCode.value.trim()) {
    showToast(t('plugins.noCode', '没有代码'), 'warning')
    return
  }
  
  const lines = pluginCode.value.split('\n')
  selectedCodeRef.value = {
    code: pluginCode.value,
    preview: createCodePreview(pluginCode.value),
    startLine: 1,
    endLine: lines.length,
    isFullCode: true
  }
}

// Clear code reference
const clearCodeRef = () => {
  selectedCodeRef.value = null
}

const sendAiChatMessage = async (message: string) => {
  if (!message.trim() || aiChatStreaming.value) return
  
  // Get current code reference
  const codeRef = selectedCodeRef.value
  
  // Add user message with code reference
  aiChatMessages.value.push({ 
    role: 'user', 
    content: message,
    codeRef: codeRef || undefined
  })
  aiChatStreaming.value = true
  aiChatStreamingContent.value = ''
  
  const streamId = `plugin_edit_${Date.now()}`
  
  try {
    // Build system prompt for code editing
    const isAgentPlugin = newPluginMetadata.value.mainCategory === 'agent'
    const systemPrompt = await invoke<string>('get_combined_plugin_prompt_api', {
      pluginType: isAgentPlugin ? 'agent' : 'passive',
      vulnType: 'custom',
      severity: newPluginMetadata.value.default_severity
    })
    
    // Build user prompt with context
    let userPrompt = message
    if (codeRef) {
      // Use specific code reference
      if (codeRef.isFullCode) {
        userPrompt = `完整插件代码：\n\`\`\`typescript\n${codeRef.code}\n\`\`\`\n\n用户需求：${message}\n\n请根据需求修改代码，直接返回完整的修改后代码。`
      } else {
        userPrompt = `选中的代码片段 (第${codeRef.startLine}-${codeRef.endLine}行)：\n\`\`\`typescript\n${codeRef.code}\n\`\`\`\n\n完整代码上下文：\n\`\`\`typescript\n${pluginCode.value}\n\`\`\`\n\n用户需求：${message}\n\n请根据需求修改选中的代码片段，返回修改后的完整代码。`
      }
    } else if (pluginCode.value) {
      // Fallback to full code if no reference
      userPrompt = `当前插件代码：\n\`\`\`typescript\n${pluginCode.value}\n\`\`\`\n\n用户需求：${message}\n\n请根据需求修改代码，直接返回完整的修改后代码。`
    }
    
    // Clear code reference after sending
    selectedCodeRef.value = null
    
    let generatedContent = ''
    
    const unlistenDelta = await listen('plugin_gen_delta', (event: any) => {
      if (event.payload.stream_id === streamId) {
        const delta = event.payload.delta || ''
        generatedContent += delta
        aiChatStreamingContent.value = generatedContent
      }
    })
    
    const unlistenComplete = await listen('plugin_gen_complete', (event: any) => {
      if (event.payload.stream_id === streamId) {
        generatedContent = event.payload.content || generatedContent
        finishAiChat(generatedContent)
      }
    })
    
    const unlistenError = await listen('plugin_gen_error', (event: any) => {
      if (event.payload.stream_id === streamId) {
        const errorMsg = event.payload.error || 'AI 处理失败'
        aiChatMessages.value.push({ role: 'assistant', content: `❌ ${errorMsg}` })
        aiChatStreaming.value = false
        aiChatStreamingContent.value = ''
      }
    })
    
    await invoke('generate_plugin_stream', {
      request: {
        stream_id: streamId,
        message: userPrompt,
        system_prompt: systemPrompt,
        service_name: 'default',
      }
    })
    
    // Cleanup listeners after timeout
    setTimeout(() => {
      unlistenDelta()
      unlistenComplete()
      unlistenError()
    }, 180000)
    
  } catch (error) {
    aiChatMessages.value.push({ 
      role: 'assistant', 
      content: `❌ ${error instanceof Error ? error.message : 'AI 处理失败'}` 
    })
    aiChatStreaming.value = false
    aiChatStreamingContent.value = ''
  }
}

const finishAiChat = (content: string) => {
  aiChatStreaming.value = false
  aiChatStreamingContent.value = ''
  
  // Extract code block if present
  const codeMatch = content.match(/```(?:typescript|ts|javascript|js)?\n?([\s\S]*?)```/)
  const codeBlock = codeMatch ? codeMatch[1].trim() : undefined
  
  // Format content for display
  let displayContent = content
  if (codeBlock) {
    displayContent = content.replace(/```(?:typescript|ts|javascript|js)?\n?([\s\S]*?)```/, 
      '<pre><code>$1</code></pre>')
  }
  
  aiChatMessages.value.push({ 
    role: 'assistant', 
    content: displayContent,
    codeBlock 
  })
}

const handleAiQuickAction = async (action: string) => {
  const actions: Record<string, string> = {
    'explain': '请解释这段插件代码的功能和工作原理',
    'optimize': '请优化这段代码，提高性能和可读性',
    'fix': '请检查并修复这段代码中可能存在的问题'
  }
  const message = actions[action] || action
  await sendAiChatMessage(message)
}

const applyAiCode = (code: string) => {
  if (!code) return
  pluginCode.value = code
  updateCodeEditorContent(code)
  updateFullscreenCodeEditorContent(code)
  if (!isEditing.value && editingPlugin.value) {
    enableEditing()
  }
  showToast('代码已应用', 'success')
}

const previewAiCode = (code: string) => {
  // For now, just show in a simple way - could be enhanced with diff view
  console.log('Preview code:', code)
  showToast('预览功能开发中', 'info')
}

const enableEditing = () => {
  isEditing.value = true
  updateCodeEditorReadonly(false)
  updateFullscreenCodeEditorReadonly(false)
}

const cancelEditing = () => {
  pluginCode.value = originalCode.value
  updateCodeEditorContent(originalCode.value)
  updateFullscreenCodeEditorContent(originalCode.value)
  isEditing.value = false
  codeError.value = ''
  updateCodeEditorReadonly(true)
  updateFullscreenCodeEditorReadonly(true)

  if (editingPlugin.value) {
    newPluginMetadata.value = {
      id: editingPlugin.value.metadata.id,
      name: editingPlugin.value.metadata.name,
      version: editingPlugin.value.metadata.version,
      author: editingPlugin.value.metadata.author || '',
      mainCategory: editingPlugin.value.metadata.main_category,
      category: editingPlugin.value.metadata.category,
      default_severity: editingPlugin.value.metadata.default_severity,
      description: editingPlugin.value.metadata.description || '',
      tagsString: editingPlugin.value.metadata.tags.join(', ')
    }
  }
}

const toggleFullscreenEditor = () => {
  if (!isFullscreenEditor.value) {
    // 进入全屏模式时，临时关闭 dialog 的 modal 状态，让其离开 top layer
    // 这样全屏编辑器覆盖层才能正确接收事件
    codeEditorDialogRef.value?.hideModalTemporary()
    isFullscreenEditor.value = true
    nextTick(() => initFullscreenCodeEditor())
  } else {
    exitFullscreenEditor()
  }
}

const savePlugin = async () => {
  if (!editingPlugin.value) return
  saving.value = true
  codeError.value = ''

  try {
    const tags = newPluginMetadata.value.tagsString.split(',').map(t => t.trim()).filter(t => t.length > 0)
    const backendCategory = newPluginMetadata.value.category

    const metadataComment = `/**
 * @plugin ${newPluginMetadata.value.id}
 * @name ${newPluginMetadata.value.name}
 * @version ${newPluginMetadata.value.version}
 * @author ${newPluginMetadata.value.author || 'Unknown'}
 * @category ${backendCategory}
 * @default_severity ${newPluginMetadata.value.default_severity}
 * @tags ${tags.join(', ')}
 * @description ${newPluginMetadata.value.description || ''}
 */
`

    const codeWithoutMetadata = pluginCode.value.replace(/\/\*\*\s*[\s\S]*?\*\/\s*/, '')
    const fullCode = metadataComment + '\n' + codeWithoutMetadata

    // 构建完整元数据用于全量更新
    const metadata = {
      id: newPluginMetadata.value.id,
      name: newPluginMetadata.value.name,
      version: newPluginMetadata.value.version,
      author: newPluginMetadata.value.author || 'Unknown',
      main_category: newPluginMetadata.value.mainCategory,
      category: backendCategory,
      description: newPluginMetadata.value.description || '',
      default_severity: newPluginMetadata.value.default_severity,
      tags: tags
    }

    const response = await invoke<CommandResponse<void>>('update_plugin', {
      metadata,
      pluginCode: fullCode
    })

    if (response.success) {
      originalCode.value = pluginCode.value
      isEditing.value = false
      updateCodeEditorReadonly(true)
      updateFullscreenCodeEditorReadonly(true)
      await refreshPlugins()
      showToast('插件已保存', 'success')
    } else {
      codeError.value = response.error || '保存失败'
    }
  } catch (error) {
    codeError.value = error instanceof Error ? error.message : '保存失败'
  } finally {
    saving.value = false
  }
}

const createNewPlugin = async () => {
  saving.value = true
  codeError.value = ''

  try {
    const tags = newPluginMetadata.value.tagsString.split(',').map(t => t.trim()).filter(t => t.length > 0)
    const backendCategory = newPluginMetadata.value.category

    const metadataComment = `/**
 * @plugin ${newPluginMetadata.value.id}
 * @name ${newPluginMetadata.value.name}
 * @version ${newPluginMetadata.value.version}
 * @author ${newPluginMetadata.value.author || 'Unknown'}
 * @category ${backendCategory}
 * @default_severity ${newPluginMetadata.value.default_severity}
 * @tags ${tags.join(', ')}
 * @description ${newPluginMetadata.value.description || ''}
 */
`

    const fullCode = metadataComment + '\n' + pluginCode.value

    const metadata = {
      id: newPluginMetadata.value.id,
      name: newPluginMetadata.value.name,
      version: newPluginMetadata.value.version,
      author: newPluginMetadata.value.author || 'Unknown',
      main_category: newPluginMetadata.value.mainCategory,
      category: backendCategory,
      description: newPluginMetadata.value.description || '',
      default_severity: newPluginMetadata.value.default_severity,
      tags: tags
    }

    const response = await invoke<CommandResponse<string>>('create_plugin_in_db', {
      metadata,
      pluginCode: fullCode
    })

    if (response.success) {
      codeEditorDialogRef.value?.closeDialog()
      await refreshPlugins()
      showToast('插件创建成功', 'success')
    } else {
      codeError.value = response.error || '创建失败'
    }
  } catch (error) {
    codeError.value = error instanceof Error ? error.message : '创建失败'
  } finally {
    saving.value = false
  }
}

const insertTemplate = async () => {
  const isAgentPlugin = newPluginMetadata.value.mainCategory === 'agent'
  try {
    const templateType = isAgentPlugin ? 'agent' : 'passive'
    const combinedTemplate = await invoke<string>('get_combined_plugin_prompt_api', {
      pluginType: templateType,
      vulnType: newPluginMetadata.value.category || 'custom',
      severity: newPluginMetadata.value.default_severity || 'medium'
    })

    let codeTemplate = ''
    const patterns = [/```typescript\n([\s\S]*?)\n```/, /```ts\n([\s\S]*?)\n```/, /```javascript\n([\s\S]*?)\n```/]
    for (const pattern of patterns) {
      const match = combinedTemplate.match(pattern)
      if (match) {
        codeTemplate = match[1].trim()
        break
      }
    }

    if (!codeTemplate) {
      codeTemplate = isAgentPlugin ? getAgentFallbackTemplate() : getPassiveFallbackTemplate()
    }

    pluginCode.value = codeTemplate
    updateCodeEditorContent(codeTemplate)
    updateFullscreenCodeEditorContent(codeTemplate)
    showToast('已插入模板代码', 'success')
  } catch (error) {
    const fallback = isAgentPlugin ? getAgentFallbackTemplate() : getPassiveFallbackTemplate()
    pluginCode.value = fallback
    updateCodeEditorContent(fallback)
    updateFullscreenCodeEditorContent(fallback)
    showToast('使用内置模板', 'info')
  }
}

const getAgentFallbackTemplate = () => `export interface ToolInput { [key: string]: any; }
export interface ToolOutput { success: boolean; data?: any; error?: string; }

export async function analyze(input: ToolInput): Promise<ToolOutput> {
  try {
    // TODO: Implement your Agent tool logic
    return { success: true, data: {} };
  } catch (error) {
    return { success: false, error: error instanceof Error ? error.message : 'Unknown error' };
  }
}

globalThis.analyze = analyze;`

const getPassiveFallbackTemplate = () => `export interface HttpRequest { method: string; url: string; headers: Record<string, string>; body?: string; }
export interface HttpResponse { status: number; headers: Record<string, string>; body?: string; }
export interface PluginContext { request: HttpRequest; response: HttpResponse; }
export interface Finding { title: string; description: string; severity: 'info' | 'low' | 'medium' | 'high' | 'critical'; }

export async function analyze(context: PluginContext): Promise<Finding[]> {
  const findings: Finding[] = [];
  // TODO: Implement your passive scan logic
  return findings;
}

globalThis.analyze = analyze;`

const formatCode = () => {
  try {
    const lines = pluginCode.value.split('\n')
    const formatted = lines.map(line => line.trimEnd()).join('\n').replace(/\n{3,}/g, '\n\n')
    pluginCode.value = formatted
    updateCodeEditorContent(formatted)
    updateFullscreenCodeEditorContent(formatted)
    showToast('代码已格式化', 'success')
  } catch (error) {
    showToast('格式化失败', 'error')
  }
}

const copyPlugin = async () => {
  try {
    const tags = newPluginMetadata.value.tagsString.split(',').map(t => t.trim()).filter(t => t.length > 0)
    const backendCategory = newPluginMetadata.value.category
    const metadataComment = `/**
 * @plugin ${newPluginMetadata.value.id}
 * @name ${newPluginMetadata.value.name}
 * @version ${newPluginMetadata.value.version}
 * @author ${newPluginMetadata.value.author || 'Unknown'}
 * @category ${backendCategory}
 * @default_severity ${newPluginMetadata.value.default_severity}
 * @tags ${tags.join(', ')}
 * @description ${newPluginMetadata.value.description || ''}
 */
`
    const codeWithoutMetadata = pluginCode.value.replace(/\/\*\*\s*[\s\S]*?\*\/\s*/, '')
    const fullCode = metadataComment + '\n' + codeWithoutMetadata
    await navigator.clipboard.writeText(fullCode)
    showToast(t('plugins.copySuccess', '已复制'), 'success')
  } catch (error) {
    console.error('Failed to copy plugin:', error)
    showToast(t('plugins.copyFailed', '复制失败'), 'error')
  }
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
      await testpassivePlugin(plugin)
    }
  } catch (e) {
    const msg = e instanceof Error ? e.message : '测试失败'
    testResult.value = { success: false, message: msg, error: msg }
    pluginDialogsRef.value?.showTestResultDialog()
  } finally {
    testing.value = false
  }
}

const testpassivePlugin = async (plugin: PluginRecord) => {
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
      // 从后端获取插件的输入参数 schema
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
        return
      }

      const startTime = Date.now()
      const runs = []
      let totalFindings = 0

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
            error: result?.error || null
          })
          totalFindings += result?.output?.findings?.length || 0
        } catch (e: any) {
          runs.push({ run_index: i + 1, duration_ms: Date.now() - runStart, findings: 0, error: e?.message || 'Error' })
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
        runs
      }
    } else {
      let headers: Record<string, string> = {}
      try {
        headers = JSON.parse(advancedForm.value.headersText || '{}')
      } catch (e) {
        advancedError.value = '请求头JSON格式错误'
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
    }
  } catch (error) {
    advancedError.value = error instanceof Error ? error.message : '高级测试失败'
  } finally {
    advancedTesting.value = false
  }
}

// CodeMirror initialization
const initCodeEditor = () => {
  const container = codeEditorDialogRef.value?.codeEditorContainerRef
  if (!container) return

  if (codeEditorView) {
    codeEditorView.destroy()
    codeEditorView = null
  }

  const isReadOnly = editingPlugin.value ? !isEditing.value : false

  codeEditorView = new EditorView({
    doc: pluginCode.value,
    extensions: [
      basicSetup,
      javascript(),
      oneDark,
      EditorView.lineWrapping,
      codeEditorReadOnly.of(EditorView.editable.of(!isReadOnly)),
      EditorView.updateListener.of((update: ViewUpdate) => {
        if (update.docChanged) pluginCode.value = update.state.doc.toString()
      })
    ],
    parent: container
  })
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

const initFullscreenCodeEditor = () => {
  const container = codeEditorDialogRef.value?.fullscreenCodeEditorContainerRef
  if (!container) return

  if (fullscreenCodeEditorView) {
    fullscreenCodeEditorView.destroy()
    fullscreenCodeEditorView = null
  }

  const isReadOnly = editingPlugin.value ? !isEditing.value : false

  fullscreenCodeEditorView = new EditorView({
    doc: pluginCode.value,
    extensions: [
      basicSetup,
      javascript(),
      oneDark,
      EditorView.lineWrapping,
      codeEditorReadOnly.of(EditorView.editable.of(!isReadOnly)),
      EditorView.updateListener.of((update: ViewUpdate) => {
        if (update.docChanged) pluginCode.value = update.state.doc.toString()
      })
    ],
    parent: container
  })
}

const exitFullscreenEditor = () => {
  if (fullscreenCodeEditorView) {
    const content = fullscreenCodeEditorView.state.doc.toString()
    pluginCode.value = content
    if (codeEditorView) {
      codeEditorView.dispatch({ changes: { from: 0, to: codeEditorView.state.doc.length, insert: content } })
    }
    fullscreenCodeEditorView.destroy()
    fullscreenCodeEditorView = null
  }
  isFullscreenEditor.value = false
  // 恢复 dialog 的 modal 状态
  codeEditorDialogRef.value?.restoreModal()
}

const updateCodeEditorContent = (content: string) => {
  if (codeEditorView) {
    codeEditorView.dispatch({ changes: { from: 0, to: codeEditorView.state.doc.length, insert: content } })
  }
}

const updateFullscreenCodeEditorContent = (content: string) => {
  if (fullscreenCodeEditorView) {
    fullscreenCodeEditorView.dispatch({ changes: { from: 0, to: fullscreenCodeEditorView.state.doc.length, insert: content } })
  }
}

const updateCodeEditorReadonly = (readonly: boolean) => {
  if (codeEditorView) {
    codeEditorView.dispatch({ effects: codeEditorReadOnly.reconfigure(EditorView.editable.of(!readonly)) })
  }
}

const updateFullscreenCodeEditorReadonly = (readonly: boolean) => {
  if (fullscreenCodeEditorView) {
    fullscreenCodeEditorView.dispatch({ effects: codeEditorReadOnly.reconfigure(EditorView.editable.of(!readonly)) })
  }
}

const onFullscreenKeydown = (e: KeyboardEvent) => {
  if (e.key === 'Escape' && isFullscreenEditor.value) {
    e.preventDefault()
    e.stopPropagation()
    exitFullscreenEditor()
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

watch(isFullscreenEditor, (enabled) => {
  if (enabled) {
    document.body.style.overflow = 'hidden'
    window.addEventListener('keydown', onFullscreenKeydown, true)
  } else {
    window.removeEventListener('keydown', onFullscreenKeydown, true)
    document.body.style.overflow = ''
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
  if (codeEditorView) { codeEditorView.destroy(); codeEditorView = null }
  if (reviewCodeEditorView) { reviewCodeEditorView.destroy(); reviewCodeEditorView = null }
  if (fullscreenCodeEditorView) { fullscreenCodeEditorView.destroy(); fullscreenCodeEditorView = null }
  window.removeEventListener('keydown', onFullscreenKeydown, true)
  document.body.style.overflow = ''
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
