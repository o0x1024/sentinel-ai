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
      :is-fullscreen-editor-mode="isFullscreenEditor"
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
      :selected-test-result-ref="selectedTestResultRef"
      :plugin-testing="pluginTesting"
      :is-preview-mode="isPreviewMode"
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
      @exit-preview-mode="exitPreviewMode"
      @add-selected-code="addSelectedCodeToContext"
      @add-full-code="addFullCodeToContext"
      @clear-code-ref="clearCodeRef"
      @clear-test-result-ref="clearTestResultRef"
      @add-test-result-to-context="addTestResultToContext"
      @test-current-plugin="testCurrentPlugin"
    />

    <!-- Editor Context Menu -->
    <Teleport to="body">
      <div 
        v-if="showContextMenu" 
        class="editor-context-menu"
        :style="{ left: contextMenuPosition.x + 'px', top: contextMenuPosition.y + 'px' }"
      >
        <div class="context-menu-header">
          <i class="fas fa-robot text-primary text-xs mr-1"></i>
          <span class="text-xs font-semibold">{{ $t('plugins.aiAssistant', 'AI 助手') }}</span>
        </div>
        <button 
          v-if="contextMenuHasSelection"
          class="context-menu-item" 
          @click="handleContextMenuAddSelection"
        >
          <i class="fas fa-highlighter text-warning"></i>
          <span>{{ $t('plugins.addSelection', '添加选中代码') }}</span>
          <kbd class="kbd kbd-xs ml-auto">Ctrl+Shift+A</kbd>
        </button>
        <button class="context-menu-item" @click="handleContextMenuAddAll">
          <i class="fas fa-file-code text-info"></i>
          <span>{{ $t('plugins.addAll', '添加完整代码') }}</span>
          <kbd class="kbd kbd-xs ml-auto">Ctrl+Shift+F</kbd>
        </button>
      </div>
    </Teleport>
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
import { marked } from 'marked'
import DOMPurify from 'dompurify'

import PluginListSection from '@/components/PluginManagement/PluginListSection.vue'
import PluginReviewSection from '@/components/PluginManagement/PluginReviewSection.vue'
import PluginStoreSection from '@/components/PluginManagement/PluginStoreSection.vue'
import PluginDialogs from '@/components/PluginManagement/PluginDialogs.vue'
import PluginCodeEditorDialog from '@/components/PluginManagement/PluginCodeEditorDialog.vue'
import type {
  PluginRecord, ReviewPlugin, TestResult, AdvancedTestResult,
  CommandResponse, BatchToggleResult, NewPluginMetadata, AdvancedForm, SubCategory
} from '@/components/PluginManagement/types'
import { trafficCategories, agentsCategories, mainCategories } from '@/components/PluginManagement/types'

const { t } = useI18n()

defineOptions({
  name: 'Plugin'
});

// Component refs
const pluginDialogsRef = ref<InstanceType<typeof PluginDialogs>>()
const codeEditorDialogRef = ref<InstanceType<typeof PluginCodeEditorDialog>>()
const pluginStoreSectionRef = ref<InstanceType<typeof PluginStoreSection>>()

// Component State
const selectedCategory = ref('all')
const plugins = ref<PluginRecord[]>([])

// CodeMirror
let codeEditorView: EditorView | null = null
let reviewCodeEditorView: EditorView | null = null
let fullscreenCodeEditorView: EditorView | null = null
let diffEditorViewA: EditorView | null = null  // Original code in diff view
let diffEditorViewB: EditorView | null = null  // Modified code in diff view
const codeEditorReadOnly = new Compartment()
const reviewCodeEditorReadOnly = new Compartment()

// Diff preview state
const isPreviewMode = ref(false)
const previewCode = ref('')

// Context menu state
const showContextMenu = ref(false)
const contextMenuPosition = ref({ x: 0, y: 0 })
const contextMenuHasSelection = ref(false)

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

// Test result reference type
interface TestResultReference {
  success: boolean
  message: string
  preview: string
  findings?: Array<{ title: string; description: string; severity: string }>
  error?: string
  executionTime?: number
  timestamp: number
}

// AI Chat State (for code editor)
interface AiChatMessage {
  role: 'user' | 'assistant'
  content: string
  codeBlock?: string
  codeBlocks?: string[] // Multiple code blocks from AI
  codeRef?: CodeReference
  testResultRef?: TestResultReference
}
const showAiPanel = ref(false)
const aiChatMessages = ref<AiChatMessage[]>([])
const aiChatStreaming = ref(false)
const aiChatStreamingContent = ref('')
const selectedCodeRef = ref<CodeReference | null>(null)
const selectedTestResultRef = ref<TestResultReference | null>(null)
const lastTestResult = ref<TestResultReference | null>(null)  // Store last test result for reference
const pluginTesting = ref(false)  // Testing state for current editing plugin

// Editor state preservation
interface EditorViewState {
  cursorPos: number
  scrollTop: number
  scrollLeft: number
  selectionRanges: Array<{ from: number; to: number }>
}
let savedEditorState: EditorViewState | null = null

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

  if (selectedCategory.value === 'traffic') {
    filtered = plugins.value.filter(p => {
      if (p.metadata.main_category === 'passive') return true
      if (trafficCategories.includes(p.metadata.category)) return true
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
  if (plugin.metadata.main_category === 'passive') return true
  if (trafficCategories.includes(plugin.metadata.category)) return true
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
  if (category === 'traffic') {
    return plugins.value.filter(p => p.metadata.main_category === 'passive' || trafficCategories.includes(p.metadata.category)).length
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
  if (diffEditorViewA) {
    diffEditorViewA.destroy()
    diffEditorViewA = null
  }
  if (diffEditorViewB) {
    diffEditorViewB.destroy()
    diffEditorViewB = null
  }
  isFullscreenEditor.value = false
  isPreviewMode.value = false
  previewCode.value = ''
  if (codeEditorView) {
    codeEditorView.destroy()
    codeEditorView = null
  }
  editingPlugin.value = null
  pluginCode.value = ''
  originalCode.value = ''
  isEditing.value = false
  codeError.value = ''
  savedEditorState = null
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

// Get current code from active editor (to ensure real-time accuracy)
const getCurrentEditorCode = (): string => {
  const editorView = isFullscreenEditor.value ? fullscreenCodeEditorView : codeEditorView
  if (!editorView) return pluginCode.value
  return editorView.state.doc.toString()
}

// Context menu handlers
const handleContextMenuAddSelection = () => {
  showContextMenu.value = false
  addSelectedCodeToContext()
  // Auto open AI panel if not open
  if (!showAiPanel.value) {
    showAiPanel.value = true
  }
}

const handleContextMenuAddAll = () => {
  showContextMenu.value = false
  addFullCodeToContext()
  // Auto open AI panel if not open
  if (!showAiPanel.value) {
    showAiPanel.value = true
  }
}

// Add selected code to context (with real-time update)
const addSelectedCodeToContext = () => {
  // Get fresh code from editor
  const currentCode = getCurrentEditorCode()
  
  const selected = getSelectedCode()
  if (!selected || !selected.code.trim()) {
    showToast(t('plugins.noCodeSelected', '请先选择代码'), 'warning')
    return
  }
  
  // Re-verify selection in current code (in case code changed)
  const editorView = isFullscreenEditor.value ? fullscreenCodeEditorView : codeEditorView
  if (editorView) {
    const freshSelection = editorView.state.sliceDoc(selected.from, selected.to)
    if (freshSelection) {
      selected.code = freshSelection
    }
  }
  
  const { startLine, endLine } = getLineNumbers(selected.from, selected.to)
  selectedCodeRef.value = {
    code: selected.code,
    preview: createCodePreview(selected.code),
    startLine,
    endLine,
    isFullCode: false
  }
  
  showToast(t('plugins.codeRefAdded', '已添加选中代码到上下文'), 'success')
}

// Add full code to context (with real-time update)
const addFullCodeToContext = () => {
  // Always get fresh code from editor
  const currentCode = getCurrentEditorCode()
  
  if (!currentCode.trim()) {
    showToast(t('plugins.noCode', '没有代码'), 'warning')
    return
  }
  
  // Update pluginCode ref to match editor state
  pluginCode.value = currentCode
  
  const lines = currentCode.split('\n')
  selectedCodeRef.value = {
    code: currentCode,
    preview: createCodePreview(currentCode),
    startLine: 1,
    endLine: lines.length,
    isFullCode: true
  }
  
  showToast(t('plugins.fullCodeRefAdded', '已添加完整代码到上下文'), 'success')
}

// Clear code reference
const clearCodeRef = () => {
  selectedCodeRef.value = null
}

// Clear test result reference
const clearTestResultRef = () => {
  selectedTestResultRef.value = null
}

// Format test result for preview
const formatTestResultPreview = (result: TestResult): string => {
  const lines: string[] = []
  lines.push(`Status: ${result.success ? 'SUCCESS' : 'FAILED'}`)
  if (result.message) lines.push(`Message: ${result.message}`)
  if (result.error) lines.push(`Error: ${result.error}`)
  if (result.findings && result.findings.length > 0) {
    lines.push(`Findings (${result.findings.length}):`)
    result.findings.slice(0, 3).forEach((f, i) => {
      lines.push(`  ${i + 1}. [${f.severity}] ${f.title}`)
    })
    if (result.findings.length > 3) {
      lines.push(`  ... and ${result.findings.length - 3} more`)
    }
  }
  return lines.join('\n')
}

// Add test result to AI context
const addTestResultToContext = () => {
  if (!lastTestResult.value) {
    showToast(t('plugins.noTestResult', '请先运行插件测试'), 'warning')
    return
  }
  selectedTestResultRef.value = lastTestResult.value
}

// Test current editing plugin
const testCurrentPlugin = async () => {
  if (!editingPlugin.value) return
  
  pluginTesting.value = true
  
  try {
    const isAgentPlugin = editingPlugin.value.metadata.main_category === 'agent'
    let result: TestResult
    
    if (isAgentPlugin) {
      const resp = await invoke<CommandResponse<any>>('test_agent_plugin', { 
        pluginId: editingPlugin.value.metadata.id,
        inputs: {}
      })
      
      if (resp.success && resp.data) {
        const data = resp.data
        result = {
          success: data.success,
          message: data.message || (data.success ? `Plugin executed (${data.execution_time_ms}ms)` : 'Test failed'),
          findings: [{ 
            title: 'Agent Tool Result', 
            description: JSON.stringify(data.output ?? { error: data.error }, null, 2), 
            severity: data.success ? 'info' : 'error' 
          }],
          error: data.error
        }
      } else {
        result = {
          success: false,
          message: resp.error || 'Test failed',
          error: resp.error
        }
      }
    } else {
      const resp = await invoke<CommandResponse<TestResult>>('test_plugin', { pluginId: editingPlugin.value.metadata.id })
      if (resp.success && resp.data) {
        result = resp.data
      } else {
        result = { success: false, message: resp.error || 'Test failed', error: resp.error }
      }
    }
    
    // Store the test result for display and potential AI reference
    testResult.value = result
    lastTestResult.value = {
      success: result.success,
      message: result.message || '',
      preview: formatTestResultPreview(result),
      findings: result.findings,
      error: result.error,
      timestamp: Date.now()
    }
    
    // Show the test result dialog (same as normal test)
    pluginDialogsRef.value?.showTestResultDialog()
  } catch (e) {
    const errorMsg = e instanceof Error ? e.message : 'Test failed'
    testResult.value = { success: false, message: errorMsg, error: errorMsg }
    lastTestResult.value = {
      success: false,
      message: errorMsg,
      preview: `Status: FAILED\nError: ${errorMsg}`,
      error: errorMsg,
      timestamp: Date.now()
    }
    pluginDialogsRef.value?.showTestResultDialog()
  } finally {
    pluginTesting.value = false
  }
}

// Reference test result to AI assistant
const referTestResultToAi = () => {
  if (!lastTestResult.value) {
    showToast(t('plugins.noTestResult', '请先运行插件测试'), 'warning')
    return
  }
  
  // Add test result to AI context
  selectedTestResultRef.value = lastTestResult.value
  
  // Ensure AI panel is open
  if (!showAiPanel.value) {
    showAiPanel.value = true
  }
  
  showToast(t('plugins.testResultAdded', '已添加测试结果到AI助手'), 'success')
}

const sendAiChatMessage = async (message: string) => {
  if (!message.trim() || aiChatStreaming.value) return
  
  // Get current references
  const codeRef = selectedCodeRef.value
  const testResultRef = selectedTestResultRef.value
  
  // Refresh code reference if it exists (ensure real-time accuracy)
  if (codeRef) {
    const currentCode = getCurrentEditorCode()
    if (codeRef.isFullCode) {
      // Update full code reference
      codeRef.code = currentCode
      codeRef.preview = createCodePreview(currentCode)
      codeRef.endLine = currentCode.split('\n').length
    } else {
      // For partial selection, try to maintain line range
      const lines = currentCode.split('\n')
      const startIdx = Math.max(0, codeRef.startLine - 1)
      const endIdx = Math.min(lines.length, codeRef.endLine)
      const refreshedCode = lines.slice(startIdx, endIdx).join('\n')
      codeRef.code = refreshedCode
      codeRef.preview = createCodePreview(refreshedCode)
    }
  }
  
  // Add user message with references
  aiChatMessages.value.push({ 
    role: 'user', 
    content: message,
    codeRef: codeRef || undefined,
    testResultRef: testResultRef || undefined
  })
  aiChatStreaming.value = true
  aiChatStreamingContent.value = ''
  
  const streamId = `plugin_edit_${Date.now()}`
  
  try {
    // Build system prompt for code editing (Vibe Coding Agent Mode)
    const isAgentPlugin = newPluginMetadata.value.mainCategory === 'agent'
    const baseSystemPrompt = await invoke<string>('get_combined_plugin_prompt_api', {
      pluginType: isAgentPlugin ? 'agent' : 'passive',
      vulnType: 'custom',
      severity: newPluginMetadata.value.default_severity
    })
    
    const agentInstructions = `
you are a senior code editor Agent, writing a plugin for the "Sentinel AI" security testing platform.
you are goal is to modify the TypeScript code directly and efficiently according to the user's needs.

[Behavior Guidelines]:
1. **Direct Modification**: If the user requires modifying the code, please provide the modified code block directly.
2. **Partial vs Full**:
   - If the user only wants to modify a specific function or add a small logic, you can only return the relevant code block (wrapped in \`\`\`typescript).
   - If the user requires global structure adjustments or explicitly requests, please return the complete code.
3. **Keep the Context**: When modifying, please refer to the user's [full code context] to ensure that the new code is compatible with the existing logic and type definitions.
4. **Security First**: As a security plugin, the code must be robust and avoid injection risks and performance bottlenecks.
5. **Simple Communication**: No need to add too many开场白, directly state what you have modified and then provide the code.

[Special Instructions]:
- User can send specific code snippets to you through the "right-click menu", please handle this part of content first.
- If the user does not provide any code context, please answer as a general programming assistant.
`
    const systemPrompt = `${baseSystemPrompt}\n\n${agentInstructions}`
    
    // Get latest code from editor
    const latestCode = getCurrentEditorCode()
    
    // Build user prompt with context - ONLY if user explicitly added code reference
    let userPrompt = message
    const contextParts: string[] = []
    
    // Check if user has explicitly provided context
    const hasExplicitCodeRef = codeRef !== null
    
    if (hasExplicitCodeRef && codeRef) {
      if (codeRef.isFullCode) {
        contextParts.push(`[Current Full Plugin Code]:\n\`\`\`typescript\n${codeRef.code}\n\`\`\``)
      } else {
        contextParts.push(`[Current Focused Code Block] (Lines ${codeRef.startLine}-${codeRef.endLine}):\n\`\`\`typescript\n${codeRef.code}\n\`\`\``)
        contextParts.push(`[Full Code Context] (仅供参考，请重点修改关注的片段):\n\`\`\`typescript\n${latestCode}\n\`\`\``)
      }
    }
    
    // Add test result context
    if (testResultRef) {
      const testInfo = [`[Latest Plugin Test Result]:`, `- Status: ${testResultRef.success ? 'Success' : 'Failed'}`]
      if (testResultRef.message) testInfo.push(`- Message: ${testResultRef.message}`)
      if (testResultRef.error) testInfo.push(`- Error: ${testResultRef.error}`)
      if (testResultRef.findings && testResultRef.findings.length > 0) {
        testInfo.push(`- Findings (${testResultRef.findings.length}):`)
        testResultRef.findings.forEach((f, i) => {
          testInfo.push(`  ${i + 1}. [${f.severity}] ${f.title}: ${f.description}`)
        })
      }
      contextParts.push(testInfo.join('\n'))
    }
    
    // Determine instruction based on context
    let instruction = ""
    if (hasExplicitCodeRef) {
      instruction = "\n\nPlease modify the code according to the above code context and my needs. If the modification is small, you can only return the relevant code block; if the modification is large or involves structural adjustments, please return the complete code. Please return the code directly, without any unnecessary words."
    } else {
      instruction = "\n\nNote: I currently do not provide you with specific code context, please directly answer my questions or provide generic coding suggestions."
    }
    
    // Combine context with user message
    if (contextParts.length > 0) {
      userPrompt = `${contextParts.join('\n\n')}\n\n[User Requirement]: ${message}${instruction}`
    } else {
      userPrompt = `${message}${instruction}`
    }
    
    // Clear references after sending
    selectedCodeRef.value = null
    selectedTestResultRef.value = null
    
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
        const errorMsg = event.payload.error || 'AI processing failed'
        aiChatMessages.value.push({ role: 'assistant', content: `❌ ${errorMsg}` })
        aiChatStreaming.value = false
        aiChatStreamingContent.value = ''
      }
    })
    
    // Log request for debugging
    console.log('[AI Chat] Sending message:', {
      streamId,
      messageLength: userPrompt.length,
      hasCodeContext: !!codeRef,
      hasTestContext: !!testResultRef,
      serviceName: 'default'
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

// Enhanced markdown rendering with marked library
const renderMarkdown = (content: string): { html: string; codeBlocks: string[] } => {
  const codeBlocks: string[] = []
  
  // Extract all code blocks before rendering (for Apply functionality)
  const codeBlockRegex = /```(?:typescript|ts|javascript|js)?\n?([\s\S]*?)```/g
  let match
  while ((match = codeBlockRegex.exec(content)) !== null) {
    codeBlocks.push(match[1].trim())
  }
  
  // Configure marked for better rendering
  marked.setOptions({
    breaks: true,
    gfm: true,
  })
  
  // Use marked to render markdown
  const rawHtml = marked.parse(content) as string
  
  // Sanitize HTML to prevent XSS
  const cleanHtml = DOMPurify.sanitize(rawHtml, {
    ALLOWED_TAGS: ['p', 'br', 'strong', 'em', 'code', 'pre', 'ul', 'ol', 'li', 'h1', 'h2', 'h3', 'h4', 'h5', 'h6', 'blockquote', 'a'],
    ALLOWED_ATTR: ['href', 'class']
  })
  
  return { html: cleanHtml, codeBlocks }
}

// Escape HTML for safe rendering
const escapeHtml = (text: string): string => {
  const map: Record<string, string> = {
    '&': '&amp;',
    '<': '&lt;',
    '>': '&gt;',
    '"': '&quot;',
    "'": '&#039;'
  }
  return text.replace(/[&<>"']/g, m => map[m])
}

const finishAiChat = (content: string) => {
  aiChatStreaming.value = false
  aiChatStreamingContent.value = ''
  
  // Use enhanced markdown rendering
  const { html, codeBlocks } = renderMarkdown(content)
  
  aiChatMessages.value.push({ 
    role: 'assistant', 
    content: html,
    codeBlock: codeBlocks[0],
    codeBlocks: codeBlocks
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

// Smart code application: detect if it's a partial or full replacement
const detectCodeApplicationMode = (aiCode: string, currentCode: string): 'full' | 'partial' | 'append' => {
  // If AI code contains typical plugin structure markers AND is long, treat as full replacement
  const fullReplacementMarkers = [
    'export interface ToolInput',
    'export async function analyze',
    'globalThis.analyze',
    '@plugin'
  ]
  
  const hasMarkers = fullReplacementMarkers.every(marker => aiCode.includes(marker))
  const aiLines = aiCode.split('\n').length
  const currentLines = currentCode.split('\n').length
  
  if (hasMarkers && aiLines > currentLines * 0.7) {
    return 'full'
  }
  
  // If it's a single function or small block
  if (aiCode.includes('function ') || aiCode.includes('const ') || aiCode.includes('interface ')) {
    return 'partial'
  }
  
  return 'append'
}

// Attempt to smart merge partial code into current document
const smartMergeCode = (snippet: string, currentCode: string): string => {
  // 1. Try to find if the snippet starts with a function/const that already exists
  const funcMatch = snippet.match(/(?:export\s+)?(?:async\s+)?function\s+([a-zA-Z0-9_]+)/)
  if (funcMatch) {
    const funcName = funcMatch[1]
    const escapedName = funcName.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
    // Regex to match the existing function body (simple version)
    const existingFuncRegex = new RegExp(`(?:export\\s+)?(?:async\\s+)?function\\s+${escapedName}\\s*\\([\\s\\S]*?\\)\\s*\\{[\\s\\S]*?\\}`, 'g')
    
    if (currentCode.match(existingFuncRegex)) {
      console.log(`[Smart Merge] Found existing function: ${funcName}, replacing it.`)
      return currentCode.replace(existingFuncRegex, snippet)
    }
  }
  
  // 2. Try to find if it's a ToolInput/ToolOutput interface
  const interfaceMatch = snippet.match(/interface\s+(ToolInput|ToolOutput|HttpRequest|HttpResponse|PluginContext)/)
  if (interfaceMatch) {
    const interfaceName = interfaceMatch[1]
    const existingInterfaceRegex = new RegExp(`interface\\s+${interfaceName}\\s*\\{[\\s\\S]*?\\}`, 'g')
    if (currentCode.match(existingInterfaceRegex)) {
      console.log(`[Smart Merge] Found existing interface: ${interfaceName}, replacing it.`)
      return currentCode.replace(existingInterfaceRegex, snippet)
    }
  }
  
  // 3. Fallback: Append or Full Replace based on size
  if (snippet.length > currentCode.length * 0.8) {
    return snippet
  }
  
  return currentCode + '\n\n' + snippet
}

const applyAiCode = (code: string, context?: CodeReference | null) => {
  if (!code) return
  
  const currentCode = getCurrentEditorCode()
  const codeToApply = isPreviewMode.value ? previewCode.value : code
  
  // 1. Detect application mode
  const mode = detectCodeApplicationMode(codeToApply, currentCode)
  
  let finalCode = currentCode
  let message = ''
  
  if (mode === 'full') {
    finalCode = codeToApply
    message = '代码已全量替换'
  } else {
    // Partial or Append mode
    let replaced = false
    
    // 2. Try context-based replacement if available
    // Only if context code is not the whole file
    if (context && context.code && !context.isFullCode) {
      if (currentCode.includes(context.code)) {
        finalCode = currentCode.replace(context.code, codeToApply)
        replaced = true
        message = '选中代码已替换'
      }
    }
    
    // 3. Fallback to smart merge if context replacement failed or no context
    if (!replaced) {
      if (mode === 'partial') {
        finalCode = smartMergeCode(codeToApply, currentCode)
        message = '代码已智能合并'
      } else {
        finalCode = currentCode + '\n\n' + codeToApply
        message = '代码已追加到末尾'
      }
    }
  }
  
  pluginCode.value = finalCode
  updateCodeEditorContent(finalCode)
  updateFullscreenCodeEditorContent(finalCode)
  
  if (isPreviewMode.value) {
    exitPreviewMode()
  }
  
  if (!isEditing.value && editingPlugin.value) {
    enableEditing()
  }
  showToast(message, 'success')
}

const previewAiCode = (code: string) => {
  if (!code || !isFullscreenEditor.value) {
    showToast('请在全屏模式下使用预览功能', 'warning')
    return
  }
  
  previewCode.value = code
  isPreviewMode.value = true
  
  nextTick(() => {
    initDiffEditor()
  })
}

const exitPreviewMode = () => {
  // Cleanup diff editors
  if (diffEditorViewA) {
    diffEditorViewA.destroy()
    diffEditorViewA = null
  }
  if (diffEditorViewB) {
    diffEditorViewB.destroy()
    diffEditorViewB = null
  }
  
  isPreviewMode.value = false
  previewCode.value = ''
  
  // Re-init fullscreen editor
  nextTick(() => {
    if (!fullscreenCodeEditorView && isFullscreenEditor.value) {
      initFullscreenCodeEditor()
    }
  })
}

const initDiffEditor = () => {
  const container = codeEditorDialogRef.value?.fullscreenDiffEditorContainerRef
  if (!container) return
  
  // Clear container first to avoid multiple wrappers
  container.innerHTML = ''
  
  // Clear existing diff editors
  if (diffEditorViewA) {
    diffEditorViewA.destroy()
    diffEditorViewA = null
  }
  if (diffEditorViewB) {
    diffEditorViewB.destroy()
    diffEditorViewB = null
  }
  
  // Create a wrapper for side-by-side layout
  const wrapper = document.createElement('div')
  wrapper.className = 'diff-editor-wrapper'
  wrapper.style.cssText = 'display: flex; height: calc(100% - 3rem); width: 100%;'
  
  const leftContainer = document.createElement('div')
  leftContainer.className = 'diff-left'
  leftContainer.style.cssText = 'flex: 1; border-right: 2px solid oklch(var(--bc) / 0.2);'
  
  const rightContainer = document.createElement('div')
  rightContainer.className = 'diff-right'
  rightContainer.style.cssText = 'flex: 1;'
  
  wrapper.appendChild(leftContainer)
  wrapper.appendChild(rightContainer)
  container.appendChild(wrapper)
  
  // Create original code editor (read-only)
  diffEditorViewA = new EditorView({
    doc: pluginCode.value,
    extensions: [
      basicSetup,
      javascript(),
      oneDark,
      EditorView.lineWrapping,
      EditorView.editable.of(false),
    ],
    parent: leftContainer
  })
  
  // Create modified code editor (editable)
  diffEditorViewB = new EditorView({
    doc: previewCode.value,
    extensions: [
      basicSetup,
      javascript(),
      oneDark,
      EditorView.lineWrapping,
      EditorView.updateListener.of((update: ViewUpdate) => {
        if (update.docChanged) {
          previewCode.value = update.state.doc.toString()
        }
      })
    ],
    parent: rightContainer
  })
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

// Save editor state (cursor, scroll, selection)
const saveEditorState = (editorView: EditorView | null): EditorViewState | null => {
  if (!editorView) return null
  
  const selection = editorView.state.selection.main
  const allSelections = editorView.state.selection.ranges.map(r => ({ from: r.from, to: r.to }))
  
  return {
    cursorPos: selection.head,
    scrollTop: editorView.scrollDOM.scrollTop,
    scrollLeft: editorView.scrollDOM.scrollLeft,
    selectionRanges: allSelections
  }
}

// Restore editor state
const restoreEditorState = (editorView: EditorView | null, state: EditorViewState | null) => {
  if (!editorView || !state) return
  
  nextTick(() => {
    // Restore selection and cursor
    editorView.dispatch({
      selection: {
        anchor: state.selectionRanges[0]?.from || state.cursorPos,
        head: state.selectionRanges[0]?.to || state.cursorPos
      },
      scrollIntoView: true
    })
    
    // Restore scroll position
    setTimeout(() => {
      editorView.scrollDOM.scrollTop = state.scrollTop
      editorView.scrollDOM.scrollLeft = state.scrollLeft
    }, 10)
  })
}

const toggleFullscreenEditor = () => {
  if (!isFullscreenEditor.value) {
    // Save current editor state before switching
    savedEditorState = saveEditorState(codeEditorView)
    
    // 进入全屏模式时，临时关闭 dialog 的 modal 状态，让其离开 top layer
    // 这样全屏编辑器覆盖层才能正确接收事件
    codeEditorDialogRef.value?.hideModalTemporary()
    isFullscreenEditor.value = true
    nextTick(() => {
      initFullscreenCodeEditor()
      // Restore state to fullscreen editor
      restoreEditorState(fullscreenCodeEditorView, savedEditorState)
    })
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
  // TODO: Implement your traffic analysis logic
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

// Setup context menu for editor
const setupEditorContextMenu = (editorView: EditorView) => {
  const editorDom = editorView.dom
  
  const handleContextMenu = (e: MouseEvent) => {
    // Only show context menu if AI panel is open
    if (!showAiPanel.value) return
    
    e.preventDefault()
    e.stopPropagation()
    
    // Check if there's a selection
    const selection = editorView.state.selection.main
    contextMenuHasSelection.value = !selection.empty
    
    // Position context menu
    contextMenuPosition.value = { x: e.clientX, y: e.clientY }
    showContextMenu.value = true
    
    // Close menu on any click outside
    const closeMenu = () => {
      showContextMenu.value = false
      document.removeEventListener('click', closeMenu)
      document.removeEventListener('contextmenu', closeMenu)
    }
    
    setTimeout(() => {
      document.addEventListener('click', closeMenu)
      document.addEventListener('contextmenu', closeMenu)
    }, 0)
  }
  
  // Keyboard shortcuts
  const handleKeydown = (e: KeyboardEvent) => {
    // Ctrl+Shift+A: Add selection to AI context
    if (e.ctrlKey && e.shiftKey && e.key === 'A') {
      e.preventDefault()
      const selection = editorView.state.selection.main
      if (!selection.empty) {
        handleContextMenuAddSelection()
      }
    }
    
    // Ctrl+Shift+F: Add full code to AI context
    if (e.ctrlKey && e.shiftKey && e.key === 'F') {
      e.preventDefault()
      handleContextMenuAddAll()
    }
  }
  
  editorDom.addEventListener('contextmenu', handleContextMenu)
  editorDom.addEventListener('keydown', handleKeydown)
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
  
  // Setup context menu
  setupEditorContextMenu(codeEditorView)
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
  
  // Setup context menu
  setupEditorContextMenu(fullscreenCodeEditorView)
}

const exitFullscreenEditor = () => {
  if (fullscreenCodeEditorView) {
    // Save state before exiting fullscreen
    savedEditorState = saveEditorState(fullscreenCodeEditorView)
    
    const content = fullscreenCodeEditorView.state.doc.toString()
    pluginCode.value = content
    
    fullscreenCodeEditorView.destroy()
    fullscreenCodeEditorView = null
  }
  
  isFullscreenEditor.value = false
  // 恢复 dialog 的 modal 状态
  codeEditorDialogRef.value?.restoreModal()
  
  // Restore state to normal editor
  nextTick(() => {
    if (codeEditorView && savedEditorState) {
      // Update content first
      codeEditorView.dispatch({ 
        changes: { from: 0, to: codeEditorView.state.doc.length, insert: pluginCode.value } 
      })
      // Then restore state
      restoreEditorState(codeEditorView, savedEditorState)
    }
  })
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
  if (diffEditorViewA) { diffEditorViewA.destroy(); diffEditorViewA = null }
  if (diffEditorViewB) { diffEditorViewB.destroy(); diffEditorViewB = null }
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

/* Editor Context Menu */
.editor-context-menu {
  position: fixed;
  min-width: 240px;
  background: oklch(var(--b1));
  border: 1px solid oklch(var(--bc) / 0.2);
  border-radius: 0.5rem;
  box-shadow: 0 10px 25px -5px rgb(0 0 0 / 0.3), 0 8px 10px -6px rgb(0 0 0 / 0.2);
  z-index: 999999;
  padding: 0.25rem;
  animation: contextMenuFadeIn 0.15s ease-out;
}

@keyframes contextMenuFadeIn {
  from {
    opacity: 0;
    transform: scale(0.95);
  }
  to {
    opacity: 1;
    transform: scale(1);
  }
}

.context-menu-header {
  display: flex;
  align-items: center;
  padding: 0.5rem 0.75rem;
  border-bottom: 1px solid oklch(var(--bc) / 0.1);
  margin-bottom: 0.25rem;
  color: oklch(var(--bc) / 0.7);
}

.context-menu-item {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  width: 100%;
  padding: 0.5rem 0.75rem;
  text-align: left;
  background: transparent;
  border: none;
  border-radius: 0.375rem;
  cursor: pointer;
  color: oklch(var(--bc));
  font-size: 0.875rem;
  transition: all 0.15s;
}

.context-menu-item:hover {
  background: oklch(var(--b3));
}

.context-menu-item i {
  width: 1rem;
  font-size: 0.875rem;
}

.context-menu-item .kbd {
  background: oklch(var(--b3));
  border-color: oklch(var(--bc) / 0.2);
  font-size: 0.7rem;
  padding: 0.125rem 0.375rem;
}
</style>
