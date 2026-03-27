<template>
  <div 
    class="input-area-container border-t border-base-300/50 bg-base-100 flex-shrink-0 relative z-0"
    @dragover.prevent="onDragOver"
    @dragleave.prevent="onDragLeave"
    @drop.prevent="onDrop"
    :class="{ 'drag-over': isDragOver }"
  >
    <!-- Drag overlay -->
    <div v-if="isDragOver" class="drag-overlay">
      <div class="drag-content">
        <i class="fas fa-file-upload text-4xl mb-2"></i>
        <span class="text-lg">{{ t('agent.document.dropDocuments') }}</span>
        <span class="text-sm opacity-70">{{ t('agent.document.supportedTypes') }}</span>
      </div>
    </div>

    <!-- Input area (refactored) -->
    <div class="px-4 pb-3 pt-2">
      <!-- 流量引用显示区 -->
      <div v-if="props.referencedTraffic && props.referencedTraffic.length > 0" class="mb-2">
        <div class="flex items-center justify-between mb-1">
          <span class="text-xs text-base-content/60 flex items-center gap-1">
            <i class="fas fa-network-wired text-accent"></i>
            引用的流量 ({{ props.referencedTraffic.length }})
          </span>
          <button 
            @click="emit('clear-traffic')"
            class="btn btn-xs btn-ghost text-base-content/60 hover:text-error"
            title="清除所有引用"
          >
            <i class="fas fa-times"></i>
            清除
          </button>
        </div>
        <div class="flex flex-wrap gap-2 max-h-32 overflow-y-auto">
          <div
            v-for="(traffic, idx) in props.referencedTraffic"
            :key="traffic.id"
            class="group relative flex items-center gap-2 px-2 py-1 bg-accent/10 border border-accent/30 rounded-lg text-xs"
          >
            <!-- 类型标签 -->
            <span :class="['badge badge-xs', getTypeBadgeClass(traffic.sendType)]">
              {{ getTypeLabel(traffic.sendType) }}
            </span>
            <span :class="['badge badge-xs', getMethodBadgeClass(traffic.method)]">
              {{ traffic.method }}
            </span>
            <span class="text-base-content/80 truncate max-w-40" :title="traffic.url">
              {{ traffic.host }}{{ getUrlPath(traffic.url) }}
            </span>
            <span v-if="traffic.sendType !== 'request'" :class="['badge badge-xs', getStatusBadgeClass(traffic.status_code)]">
              {{ traffic.status_code || 'N/A' }}
            </span>
            <button
              @click="emit('remove-traffic', idx)"
              class="w-4 h-4 rounded-full bg-error/80 text-error-content opacity-0 group-hover:opacity-100 transition-opacity flex items-center justify-center text-xs ml-1"
              title="移除"
            >
              <i class="fas fa-times text-[10px]"></i>
            </button>
          </div>
        </div>
      </div>

      <div v-if="props.referencedAssets && props.referencedAssets.length > 0" class="mb-2">
        <div class="flex items-center justify-between mb-1">
          <span class="text-xs text-base-content/60 flex items-center gap-1">
            <i class="fas fa-cubes text-primary"></i>
            引用的资产 ({{ props.referencedAssets.length }})
          </span>
          <button
            @click="emit('clear-assets')"
            class="btn btn-xs btn-ghost text-base-content/60 hover:text-error"
            title="清除所有资产引用"
          >
            <i class="fas fa-times"></i>
            清除
          </button>
        </div>
        <div class="flex flex-wrap gap-2 max-h-32 overflow-y-auto">
          <div
            v-for="(asset, idx) in props.referencedAssets"
            :key="asset.id"
            class="group relative flex items-center gap-2 px-2 py-1 bg-primary/10 border border-primary/25 rounded-lg text-xs"
          >
            <span class="badge badge-xs badge-outline">{{ asset.asset_type }}</span>
            <span :class="['badge badge-xs', getAssetRiskBadgeClass(asset.risk_level)]">
              {{ asset.risk_level || 'unknown' }}
            </span>
            <span class="font-medium text-base-content/80 truncate max-w-40" :title="asset.name">
              {{ asset.name }}
            </span>
            <span class="text-base-content/60 truncate max-w-44" :title="asset.value">
              {{ asset.value }}
            </span>
            <button
              @click="emit('remove-asset', idx)"
              class="w-4 h-4 rounded-full bg-error/80 text-error-content opacity-0 group-hover:opacity-100 transition-opacity flex items-center justify-center text-xs ml-1"
              title="移除"
            >
              <i class="fas fa-times text-[10px]"></i>
            </button>
          </div>
        </div>
      </div>

      <!-- 图片附件预览区 -->
      <div v-if="pendingAttachments && pendingAttachments.length > 0" class="mb-2 flex flex-wrap gap-2">
        <div
          v-for="(att, idx) in pendingAttachments"
          :key="idx"
          class="relative group"
        >
          <img
            :src="getAttachmentPreview(att)"
            class="h-16 w-16 object-cover rounded border border-base-300 bg-base-200"
            :alt="att.image?.filename || 'attachment'"
          />
          <button
            @click="removeAttachment(idx)"
            class="absolute -top-1 -right-1 w-5 h-5 rounded-full bg-error text-error-content opacity-0 group-hover:opacity-100 transition-opacity flex items-center justify-center text-xs"
            title="移除"
          >
            <i class="fas fa-times"></i>
          </button>
        </div>
      </div>

      <!-- 文档附件预览区 -->
      <div v-if="pendingDocuments && pendingDocuments.length > 0" class="mb-2 flex flex-wrap gap-2">
        <div
          v-for="(doc, idx) in pendingDocuments"
          :key="doc.id"
          class="inline-flex items-center gap-2 px-2 py-1 rounded-lg border border-base-300 bg-base-200 text-xs"
        >
          <i class="fas fa-file-lines text-primary"></i>
          <span class="font-medium truncate max-w-44" :title="doc.original_filename">{{ doc.original_filename }}</span>
          <span class="text-base-content/60">({{ formatFileSize(doc.file_size) }})</span>
          <span v-if="doc.status === 'processing'" class="badge badge-xs badge-info">
            {{ t('common.loading') }}
          </span>
          <span v-else-if="doc.status === 'failed'" class="badge badge-xs badge-error">
            failed
          </span>
          <button
            @click="removeDocument(idx)"
            class="w-4 h-4 rounded-full bg-error text-error-content flex items-center justify-center text-[10px]"
            title="移除"
          >
            <i class="fas fa-times"></i>
          </button>
          <button
            v-if="doc.status === 'ready'"
            @click="runSecurityAnalysis(doc)"
            class="btn btn-ghost btn-xs"
            title="安全分析"
          >
            <i class="fas fa-shield-halved text-warning"></i>
          </button>
          <button
            v-if="doc.status === 'failed'"
            @click="retryUploadDocument(doc)"
            class="btn btn-ghost btn-xs"
            title="重试"
          >
            <i class="fas fa-rotate-right"></i>
          </button>
        </div>
      </div>

      <div ref="containerRef" class="chat-input rounded-2xl bg-base-200/60 border border-base-300/60 backdrop-blur-sm flex flex-col gap-2 px-3 py-2 shadow-sm focus-within:border-primary transition-colors">
        <!-- Text input (auto-resize textarea) -->
        <div class="flex-1 min-w-0">
          <textarea
            ref="textareaRef"
            :value="inputMessage"
            @input="onInput"
            @keydown="onKeydown"
            @click="onCaretChanged"
            @keyup="onCaretChanged"
            @compositionstart="onCompositionStart"
            @compositionend="onCompositionEnd"
            :disabled="isLoading && !allowTakeover"
            :placeholder="placeholderText"
            class="w-full bg-transparent outline-none resize-none leading-relaxed text-sm placeholder:text-base-content/50 max-h-40"
            rows="1"
          />
        </div>
        <div v-if="slashOpen" class="slash-popover border border-base-300 bg-base-100 rounded-xl shadow-xl">
          <div class="px-3 py-2 border-b border-base-300/60 text-xs text-base-content/60">
            Slash Commands
          </div>
          <div v-if="filteredSlashCommands.length === 0" class="px-3 py-2 text-sm text-base-content/60">
            无匹配命令
          </div>
          <div v-else class="py-1 max-h-64 overflow-y-auto">
            <button
              v-for="(cmd, idx) in filteredSlashCommands"
              :key="cmd.id"
              class="w-full text-left px-3 py-2 transition-colors"
              :class="idx === slashActiveIndex ? 'bg-primary/15 text-primary' : 'hover:bg-base-200 text-base-content'"
              @mousedown.prevent="applySlashCommand(cmd)"
            >
              <div class="flex items-center gap-4">
                <span class="font-semibold text-base whitespace-nowrap min-w-28">/{{ cmd.name }}</span>
                <div class="text-sm opacity-75 truncate flex-1">
                  {{ cmd.description || (cmd.type === 'action' ? getActionLabel(cmd.action) : '自定义提示词命令') }}
                </div>
                <span class="text-xs opacity-70 whitespace-nowrap">{{ cmd.type === 'action' ? '功能' : '提示词' }}</span>
              </div>
            </button>
          </div>
        </div>

        <!-- Toolbar: left actions and right send/stop -->
        <div class="flex items-center justify-between gap-2">
          <InputToolbarActions
            :tools-enabled="localToolsEnabled"
            :team-enabled="localTeamEnabled"
            :rag-enabled="localRagEnabled"
            :web-search-enabled="localWebSearchEnabled"
            @trigger-file-select="triggerFileSelect"
            @toggle-tools="toggleTools"
            @open-tool-config="emit('open-tool-config')"
            @toggle-team="toggleTeam"
            @toggle-rag="toggleRAG"
            @toggle-web-search="toggleWebSearch"
            @open-slash-manager="openSlashManager"
            @clear-conversation="clearConversation"
          />

          <!-- Right side icons -->
          <div class="flex items-center gap-2 shrink-0">
            <!-- Context usage indicator -->
            <div 
              v-if="effectiveContextUsage" 
              class="context-usage-indicator flex items-center gap-1 px-2 py-1 rounded-md text-xs cursor-default"
              :class="contextUsageClass"
              :title="contextUsageTooltip"
            >
              <span class="font-medium">{{ contextUsagePercentage }}%</span>
              <span class="opacity-70">·</span>
              <span class="opacity-80">{{ formatTokenCount(effectiveContextUsage.usedTokens) }} / {{ formatTokenCount(effectiveContextUsage.maxTokens) }}</span>
              <span class="opacity-70 hidden sm:inline">{{ t('agent.contextUsed') }}</span>
            </div>
            <div class="assistant-model-switch">
              <SearchableSelect
                :model-value="localSelectedModel"
                :options="availableModelOptions"
                :placeholder="modelLoading ? '加载模型中...' : '选择模型'"
                search-placeholder="搜索模型..."
                no-results-text="无匹配模型"
                :disabled="modelLoading || availableModelOptions.length === 0"
                size="sm"
                direction="up"
                variant="toolbar"
                :auto-width="true"
                align="right"
                group-by="description"
                @update:model-value="localSelectedModel = $event"
                @change="onModelChanged"
              />
            </div>
            <button class="icon-btn" title="语言 / 翻译"><i class="fas fa-language"></i></button>
            <button
              v-if="!isLoading || allowTakeover"
              class="send-btn"
              :disabled="!canSend"
              :class="{ 'opacity-40 cursor-not-allowed': !canSend }"
              @click="emitSend"
              :title="isLoading ? '接管并发送 (Enter)' : '发送 (Enter)'"
            >
              <i class="fas fa-arrow-up"></i>
            </button>
            <button
              v-if="isLoading"
              class="send-btn bg-error text-error-content hover:bg-error/90"
              @click="handleStop"
              title="停止执行"
            >
              <i class="fas fa-stop"></i>
            </button>
          </div>
        </div>
      </div>
      <!-- Hidden file input for attachments -->
      <input
        ref="fileInputRef"
        type="file"
        class="hidden"
        multiple
        accept="*/*"
        @change="onFilesSelected"
      />

      <Teleport to="body">
        <dialog
          v-if="showSlashManager"
          class="modal modal-open slash-manager-modal"
        >
          <div class="modal-box max-w-3xl">
            <h3 class="font-bold text-lg">Slash Commands</h3>
            <p class="text-sm text-base-content/70 mt-1">输入框中键入 <code>/</code> 可调用命令</p>
            <div class="mt-3 flex flex-wrap items-center gap-2">
              <select v-model="slashManagerScope" class="select select-bordered select-sm w-40">
                <option value="global">全局命令</option>
                <option value="conversation" :disabled="!conversationScopeEnabled">会话命令</option>
              </select>
              <button type="button" class="btn btn-outline btn-sm" @click="exportSlashCommands">导出 JSON</button>
              <button type="button" class="btn btn-outline btn-sm" @click="triggerImportSlashCommands">导入 JSON</button>
              <span v-if="slashManagerScope === 'conversation' && conversationScopeEnabled" class="text-xs text-base-content/70">
                当前会话：{{ conversationScopeKey }}
              </span>
            </div>

            <div class="grid grid-cols-1 lg:grid-cols-2 gap-4 mt-4">
              <div class="border border-base-300 rounded-lg p-3 max-h-80 overflow-y-auto">
                <div class="text-xs uppercase tracking-wide text-base-content/60 mb-2">命令列表</div>
                <div v-if="managerBuiltinCommands.length > 0" class="space-y-2 mb-2">
                  <div
                    v-for="cmd in managerBuiltinCommands"
                    :key="cmd.id"
                    class="rounded-lg border border-base-300/70 px-3 py-2 bg-base-100"
                  >
                    <div class="flex items-center justify-between gap-2">
                      <div>
                        <div class="font-medium">/{{ cmd.name }} <span class="text-xs opacity-60">(内置)</span></div>
                        <div class="text-xs opacity-70 truncate">{{ cmd.description || '-' }}</div>
                      </div>
                      <span class="badge badge-ghost badge-sm">只读</span>
                    </div>
                  </div>
                </div>

                <draggable
                  v-model="managerCustomCommands"
                  item-key="id"
                  handle=".drag-handle"
                  :animation="150"
                  class="space-y-2"
                  @end="onDragSortEnd"
                >
                  <template #item="{ element: cmd }">
                    <div class="rounded-lg border border-base-300/70 px-3 py-2 bg-base-100">
                      <div class="flex items-center justify-between gap-2">
                        <div class="flex items-center gap-2 min-w-0">
                          <button type="button" class="btn btn-ghost btn-xs drag-handle cursor-grab" title="拖拽排序">
                            <i class="fas fa-grip-vertical"></i>
                          </button>
                          <div class="min-w-0">
                            <div class="font-medium">/{{ cmd.name }} <span v-if="cmd.scope" class="text-xs opacity-60">({{ getScopeLabel(cmd.scope) }})</span></div>
                            <div class="text-xs opacity-70 truncate">{{ cmd.description || '-' }}</div>
                          </div>
                        </div>
                        <div class="flex items-center gap-1">
                          <button type="button" class="btn btn-ghost btn-xs" @click="startEditCommand(cmd)" title="编辑">编辑</button>
                          <button type="button" class="btn btn-ghost btn-xs text-error" @click="deleteSlashCommand(cmd.id)">删除</button>
                        </div>
                      </div>
                      <label class="label py-1">
                        <span class="label-text text-xs">启用</span>
                        <input
                          type="checkbox"
                          class="toggle toggle-xs"
                          :checked="cmd.enabled"
                          @change="onCommandEnabledChange(cmd, $event)"
                        />
                      </label>
                    </div>
                  </template>
                </draggable>
                <div v-if="managerCustomCommands.length === 0" class="text-xs text-base-content/60 px-1 py-2">
                  当前作用域暂无自定义命令
                </div>
              </div>

              <div class="border border-base-300 rounded-lg p-3">
                <div class="text-xs uppercase tracking-wide text-base-content/60 mb-2">
                  {{ editingSlashId ? '编辑命令' : '新增自定义命令' }}
                </div>
                <div class="space-y-3">
                  <label class="form-control">
                    <span class="label-text text-xs">命令名</span>
                    <input v-model.trim="newSlashCommand.name" class="input input-bordered input-sm" placeholder="review" />
                  </label>
                  <label class="form-control">
                    <span class="label-text text-xs">描述</span>
                    <input v-model.trim="newSlashCommand.description" class="input input-bordered input-sm" placeholder="审查当前改动" />
                  </label>
                  <label class="form-control">
                    <span class="label-text text-xs">类型</span>
                    <select v-model="newSlashCommand.type" class="select select-bordered select-sm">
                      <option value="prompt">提示词</option>
                      <option value="action">功能</option>
                    </select>
                  </label>
                  <label class="form-control">
                    <span class="label-text text-xs">作用域</span>
                    <select v-model="newSlashCommand.scope" class="select select-bordered select-sm">
                      <option value="global">全局</option>
                      <option value="conversation" :disabled="!conversationScopeEnabled">会话</option>
                    </select>
                  </label>
                  <label v-if="newSlashCommand.type === 'prompt'" class="form-control">
                    <span class="label-text text-xs">提示词模板</span>
                    <textarea v-model="newSlashCommand.template" class="textarea textarea-bordered textarea-sm h-24" placeholder="请审查当前改动：{{input}}"></textarea>
                  </label>
                  <label v-else class="form-control">
                    <span class="label-text text-xs">功能</span>
                    <select v-model="newSlashCommand.action" class="select select-bordered select-sm">
                      <option value="new_conversation">新建会话</option>
                      <option value="clear_conversation">清空会话</option>
                      <option value="toggle_rag">切换 RAG</option>
                      <option value="toggle_tools">切换 Tools</option>
                      <option value="open_tool_config">打开工具配置</option>
                    </select>
                  </label>
                  <label v-if="newSlashCommand.type === 'prompt'" class="label cursor-pointer justify-start gap-2">
                    <input type="checkbox" class="checkbox checkbox-sm" v-model="newSlashCommand.auto_send" />
                    <span class="label-text text-xs">执行后立即发送</span>
                  </label>
                  <div v-if="slashFormError" class="text-xs text-error bg-error/10 border border-error/20 rounded px-2 py-1">
                    {{ slashFormError }}
                  </div>
                  <button type="button" class="btn btn-primary btn-sm w-full" @click="editingSlashId ? saveEditedSlashCommand() : addCustomSlashCommand()">
                    {{ editingSlashId ? '保存修改' : '添加命令' }}
                  </button>
                  <button v-if="editingSlashId" type="button" class="btn btn-ghost btn-sm w-full" @click="cancelEditCommand">取消编辑</button>
                </div>
              </div>
            </div>

            <div class="modal-action">
              <button type="button" class="btn" @click="closeSlashManager">关闭</button>
            </div>
          </div>
          <form method="dialog" class="modal-backdrop">
            <button @click.prevent="closeSlashManager">close</button>
          </form>
        </dialog>
      </Teleport>
      <input
        ref="importSlashInputRef"
        type="file"
        class="hidden"
        accept="application/json,.json"
        @change="onImportSlashFileChange"
      />

    </div>
  </div>
</template>

<script setup lang="ts">
import { onMounted, onUnmounted, ref, computed, nextTick, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import draggable from 'vuedraggable'
import SearchableSelect from '@/components/SearchableSelect.vue'
import InputToolbarActions from '@/components/InputArea/InputToolbarActions.vue'
import { useInputAttachments } from '@/components/InputArea/useInputAttachments'
import { useInputSlashCommands } from '@/components/InputArea/useInputSlashCommands'
import type { PendingDocumentAttachment, ProcessedDocumentResult } from '@/types/agent'

const { t } = useI18n()

// 流量引用类型
type TrafficSendType = 'request' | 'response' | 'both'
interface ReferencedTraffic {
  id: number
  url: string
  method: string
  host: string
  status_code: number
  request_headers?: string
  request_body?: string
  response_headers?: string
  response_body?: string
  sendType?: TrafficSendType
}

interface ReferencedAsset {
  id: string
  name: string
  value: string
  asset_type: string
  risk_level?: string
  status?: string
  description?: string
  tags?: string[]
  metadata?: Record<string, any>
}

// Context usage info type
interface ContextUsageInfo {
  usedTokens: number
  maxTokens: number
  usagePercentage: number
  systemPromptTokens: number
  historyTokens: number
  historyCount: number
  summaryTokens: number
  summaryGlobalTokens: number
  summarySegmentTokens: number
  summarySegmentCount: number
}

interface ModelOption {
  value: string
  label: string
  description?: string
}

const props = defineProps<{
  inputMessage: string
  conversationId?: string | null
  isLoading: boolean
  showDebugInfo: boolean
  allowTakeover?: boolean
  ragEnabled?: boolean
  toolsEnabled?: boolean
  webSearchEnabled?: boolean
  teamEnabled?: boolean
  pendingAttachments?: any[]
  pendingDocuments?: PendingDocumentAttachment[]
  processedDocuments?: ProcessedDocumentResult[]
  referencedTraffic?: ReferencedTraffic[]
  referencedAssets?: ReferencedAsset[]
  contextUsage?: ContextUsageInfo | null
  availableModels?: ModelOption[]
  selectedModel?: string
  modelLoading?: boolean
  defaultMaxContextTokens?: number
}>()

const emit = defineEmits<{
  (e: 'update:input-message', value: string): void
  (e: 'send-message'): void
  (e: 'stop-execution'): void
  (e: 'toggle-debug', value: boolean): void
  (e: 'create-new-conversation'): void
  (e: 'clear-conversation'): void
  (e: 'toggle-rag', enabled: boolean): void
  (e: 'toggle-tools', enabled: boolean): void
  (e: 'toggle-web-search', enabled: boolean): void
  (e: 'toggle-team', enabled: boolean): void
  (e: 'open-tool-config'): void
  (e: 'add-attachments', files: string[]): void
  (e: 'remove-attachment', index: number): void
  (e: 'add-documents', files: PendingDocumentAttachment[]): void
  (e: 'remove-document', index: number): void
  (e: 'document-processed', result: ProcessedDocumentResult): void
  (e: 'remove-traffic', index: number): void
  (e: 'clear-traffic'): void
  (e: 'remove-asset', index: number): void
  (e: 'clear-assets'): void
  (e: 'change-model', value: string): void
}>()

// removed architecture utilities

const allowTakeover = computed(() => props.allowTakeover === true)

const getAssetRiskBadgeClass = (level?: string) => {
  switch (String(level || 'unknown').toLowerCase()) {
    case 'critical':
      return 'badge-error'
    case 'high':
      return 'badge-warning'
    case 'medium':
      return 'badge-info'
    case 'low':
      return 'badge-success'
    default:
      return 'badge-ghost'
  }
}

// --- New input logic ---
const textareaRef = ref<HTMLTextAreaElement | null>(null)
const containerRef = ref<HTMLDivElement | null>(null)

// --- Persistence helpers ---
const STORAGE_KEYS = {
  rag: 'sentinel:input:ragEnabled',
  tools: 'sentinel:input:toolsEnabled',
  webSearch: 'sentinel:input:webSearchEnabled',
  team: 'sentinel:input:teamEnabled',
} as const

const getBool = (key: string, fallback = false) => {
  try {
    const v = localStorage.getItem(key)
    if (v === null) return fallback
    return v === '1' || v === 'true'
  } catch {
    return fallback
  }
}

const setBool = (key: string, value: boolean) => {
  try {
    localStorage.setItem(key, value ? '1' : '0')
  } catch {
    // ignore
  }
}

// Feature states (controlled by parent via props, with persistence)
const localRagEnabled = ref<boolean>(!!props.ragEnabled)
const localToolsEnabled = ref<boolean>(!!props.toolsEnabled)
const localWebSearchEnabled = ref<boolean>(!!props.webSearchEnabled)
const localTeamEnabled = ref<boolean>(!!props.teamEnabled)
const localSelectedModel = ref(props.selectedModel || '')
const availableModelOptions = computed(() => props.availableModels || [])

// init guard
const initialized = ref(false)

const placeholderText = computed(() => '在这里输入消息，按 Enter 发送')

const autoResize = () => {
  const el = textareaRef.value
  if (!el) return
  el.style.height = 'auto'
  el.style.height = Math.min(el.scrollHeight, 320) + 'px'
}

const onInput = (e: Event) => {
  const target = e.target as HTMLTextAreaElement
  emit('update:input-message', target.value)
  updateSlashState(target.value, target.selectionStart || 0)
  autoResize()
}

const onCaretChanged = (e: Event) => {
  const target = e.target as HTMLTextAreaElement
  updateSlashState(target.value, target.selectionStart || 0)
}
const {
  fileInputRef,
  formatFileSize,
  getAttachmentPreview,
  isDragOver,
  onDragLeave,
  onDragOver,
  onDrop,
  onFilesSelected,
  removeAttachment,
  removeDocument,
  retryUploadDocument,
  runSecurityAnalysis,
  setupNativeDragDrop,
  teardownNativeDragDrop,
  triggerFileSelect,
} = useInputAttachments({
  conversationId: () => props.conversationId ?? null,
  emitAddAttachments: (files) => {
    emit('add-attachments', files)
  },
  emitAddDocuments: (files) => {
    emit('add-documents', files)
  },
  emitDocumentProcessed: (result) => {
    emit('document-processed', result)
  },
  emitRemoveAttachment: (index) => {
    emit('remove-attachment', index)
  },
  emitRemoveDocument: (index) => {
    emit('remove-document', index)
  },
})

const {
  addCustomSlashCommand,
  applySlashCommand,
  cancelEditCommand,
  closeSlashManager,
  closeSlashPopover,
  conversationScopeEnabled,
  conversationScopeKey,
  deleteSlashCommand,
  editingSlashId,
  filteredSlashCommands,
  getActionLabel,
  getScopeLabel,
  importSlashInputRef,
  loadSlashCommands,
  managerBuiltinCommands,
  managerCustomCommands,
  newSlashCommand,
  onCommandEnabledChange,
  onDragSortEnd,
  onImportSlashFileChange,
  openSlashManager,
  exportSlashCommands,
  saveEditedSlashCommand,
  showSlashManager,
  slashActiveIndex,
  slashFormError,
  slashManagerScope,
  slashOpen,
  startEditCommand,
  triggerImportSlashCommands,
  updateSlashState,
} = useInputSlashCommands({
  conversationId: computed(() => props.conversationId ?? null),
  createNewConversation: () => {
    createNewConversation()
  },
  clearConversation: () => {
    clearConversation()
  },
  emitOpenToolConfig: () => {
    emit('open-tool-config')
  },
  emitSend: () => {
    emitSend()
  },
  getInputMessage: () => props.inputMessage || '',
  onInputValueChange: (value) => {
    emit('update:input-message', value)
    nextTick(() => {
      const el = textareaRef.value
      if (!el) return
      el.focus()
      el.setSelectionRange(value.length, value.length)
      updateSlashState(value, value.length)
      autoResize()
    })
  },
  toggleRAG: () => {
    toggleRAG()
  },
  toggleTools: () => {
    toggleTools()
  },
})

// Context usage computed properties
const estimateTokens = (text: string): number => {
  if (!text) return 0
  // Heuristic: CJK chars are closer to 1 token each, others ~4 chars/token
  const cjkCount = (text.match(/[\u4e00-\u9fff]/g) || []).length
  const nonCjk = Math.max(0, text.length - cjkCount)
  const cjkTokens = cjkCount
  const nonCjkTokens = Math.ceil(nonCjk / 4)
  return cjkTokens + nonCjkTokens
}

const inputTokenEstimate = computed(() => estimateTokens(props.inputMessage || ''))
const resolvedDefaultMaxContextTokens = computed(() => {
  const parsed = Number(props.defaultMaxContextTokens)
  if (Number.isFinite(parsed) && parsed > 0) {
    return Math.floor(parsed)
  }
  return 128000
})

const effectiveContextUsage = computed(() => {
  const base = props.contextUsage
  const inputTokens = inputTokenEstimate.value
  if (!base) {
    if (inputTokens === 0) return null
    const maxTokens = resolvedDefaultMaxContextTokens.value
    const usedTokens = inputTokens
    const usagePercentage = maxTokens > 0
      ? Math.min(100, (usedTokens / maxTokens) * 100)
      : 0
    return {
      usedTokens,
      maxTokens,
      usagePercentage,
      systemPromptTokens: 0,
      historyTokens: inputTokens,
      historyCount: 0,
      summaryTokens: 0,
      summaryGlobalTokens: 0,
      summarySegmentTokens: 0,
      summarySegmentCount: 0,
    }
  }
  if (inputTokens === 0) return base
  const usedTokens = base.usedTokens + inputTokens
  const historyTokens = base.historyTokens + inputTokens
  const usagePercentage = base.maxTokens > 0
    ? Math.min(100, (usedTokens / base.maxTokens) * 100)
    : 0
  return {
    ...base,
    usedTokens,
    historyTokens,
    usagePercentage,
  }
})

const contextUsagePercentage = computed(() => {
  if (!effectiveContextUsage.value) return 0
  return Math.round(effectiveContextUsage.value.usagePercentage * 10) / 10
})

const contextUsageClass = computed(() => {
  const percentage = contextUsagePercentage.value
  if (percentage >= 90) return 'bg-error/20 text-error border border-error/30'
  if (percentage >= 70) return 'bg-warning/20 text-warning border border-warning/30'
  if (percentage >= 50) return 'bg-info/20 text-info border border-info/30'
  return 'bg-base-300/50 text-base-content/70 border border-base-300'
})

const contextUsageTooltip = computed(() => {
  if (!effectiveContextUsage.value) return ''
  const { 
    usedTokens, 
    maxTokens, 
    systemPromptTokens, 
    historyTokens, 
    historyCount, 
    summaryTokens, 
    summaryGlobalTokens, 
    summarySegmentTokens, 
    summarySegmentCount 
  } = effectiveContextUsage.value
  const inputHint = inputTokenEstimate.value > 0 ? `\n${t('agent.inputTokens')}: ~${formatTokenCount(inputTokenEstimate.value)}` : ''
  return `${t('agent.contextUsageDetails')}
${t('agent.systemPromptTokens')}: ${formatTokenCount(systemPromptTokens)}
${t('agent.summaryTokens')}: ${formatTokenCount(summaryTokens)}
${t('agent.summaryGlobalTokens')}: ${formatTokenCount(summaryGlobalTokens)}
${t('agent.summarySegmentTokens')}: ${formatTokenCount(summarySegmentTokens)} (${t('agent.summarySegments')}: ${summarySegmentCount})
${t('agent.historyTokens')}: ${formatTokenCount(historyTokens)}
${t('agent.historyMessages')}: ${historyCount}
${t('agent.totalUsed')}: ${formatTokenCount(usedTokens)} / ${formatTokenCount(maxTokens)}${inputHint}`
})

const formatTokenCount = (count: number): string => {
  if (count >= 1000000) {
    return (count / 1000000).toFixed(1) + 'M'
  }
  if (count >= 1000) {
    return (count / 1000).toFixed(1) + 'K'
  }
  return count.toString()
}

// 检查是否可以发送
const canSend = computed(() => {
  if (!props.inputMessage.trim()) return false
  const hasProcessingUploads = (props.pendingDocuments || []).some((d) => d.status === 'processing')
  if (hasProcessingUploads) return false
  return true
})

const SEND_EVENT_DEDUP_WINDOW_MS = 300
let lastSendEmitAt = 0
const emitSend = () => {
  const now = Date.now()
  if (now - lastSendEmitAt < SEND_EVENT_DEDUP_WINDOW_MS) {
    return
  }
  if (!canSend.value) return
  if (props.isLoading && !allowTakeover.value) return
  lastSendEmitAt = now
  emit('send-message')
  // 发送后恢复高度
  requestAnimationFrame(() => autoResize())
}

const handleStop = () => {
  console.log('InputAreaComponent: 停止按钮被点击')
  emit('stop-execution')
}

// Track IME composition state
const isComposing = ref(false)

const onCompositionStart = () => {
  isComposing.value = true
}

const onCompositionEnd = () => {
  isComposing.value = false
}

const onKeydown = (e: KeyboardEvent) => {
  // Ignore Enter key during IME composition
  if (isComposing.value && e.key === 'Enter') {
    return
  }

  if (slashOpen.value) {
    if (e.key === 'ArrowDown') {
      e.preventDefault()
      if (filteredSlashCommands.value.length > 0) {
        slashActiveIndex.value = (slashActiveIndex.value + 1) % filteredSlashCommands.value.length
      }
      return
    }
    if (e.key === 'ArrowUp') {
      e.preventDefault()
      if (filteredSlashCommands.value.length > 0) {
        slashActiveIndex.value =
          (slashActiveIndex.value - 1 + filteredSlashCommands.value.length) % filteredSlashCommands.value.length
      }
      return
    }
    if ((e.key === 'Enter' || e.key === 'Tab') && filteredSlashCommands.value.length > 0) {
      e.preventDefault()
      const cmd = filteredSlashCommands.value[Math.max(0, slashActiveIndex.value)]
      if (cmd) applySlashCommand(cmd)
      return
    }
    if (e.key === 'Escape') {
      e.preventDefault()
      closeSlashPopover()
      return
    }
  }

  if (e.key === 'Enter' && !e.shiftKey) {
    e.preventDefault()
    emitSend()
  } else if (e.key === 'Enter' && e.shiftKey) {
    // allow newline
    return
  } else if (e.key === 'Escape') {
    // blur
    (e.target as HTMLTextAreaElement).blur()
  }
}

const clearCurrent = () => {
  emit('update:input-message', '')
  requestAnimationFrame(() => autoResize())
}

const createNewConversation = () => {
  emit('create-new-conversation')
  // 清空输入框
  emit('update:input-message', '')
  requestAnimationFrame(() => autoResize())
}

const clearConversation = () => {
  emit('clear-conversation')
  // 清空输入框
  emit('update:input-message', '')
  requestAnimationFrame(() => autoResize())
}

const toggleRAG = () => {
  localRagEnabled.value = !localRagEnabled.value
  setBool(STORAGE_KEYS.rag, localRagEnabled.value)
  // 通知父组件RAG状态变化
  emit('toggle-rag', localRagEnabled.value)
}

const toggleTools = () => {
  localToolsEnabled.value = !localToolsEnabled.value
  setBool(STORAGE_KEYS.tools, localToolsEnabled.value)
  // 通知父组件Tools状态变化
  emit('toggle-tools', localToolsEnabled.value)
}

const toggleWebSearch = () => {
  localWebSearchEnabled.value = !localWebSearchEnabled.value
  setBool(STORAGE_KEYS.webSearch, localWebSearchEnabled.value)
  emit('toggle-web-search', localWebSearchEnabled.value)
}

const toggleTeam = () => {
  localTeamEnabled.value = !localTeamEnabled.value
  setBool(STORAGE_KEYS.team, localTeamEnabled.value)
  emit('toggle-team', localTeamEnabled.value)
}

const onModelChanged = (value: string) => {
  if (!value) return
  localSelectedModel.value = value
  emit('change-model', value)
}

// 点击外部区域关闭弹层
const handleClickOutside = (e: MouseEvent) => {
  const target = e.target as Node | null
  const container = containerRef.value
  if (!target || !container) return
  if (!container.contains(target)) {
    closeSlashPopover()
  }
}

// 流量显示辅助函数
const getMethodBadgeClass = (method: string): string => {
  switch (method?.toUpperCase()) {
    case 'GET': return 'badge-info'
    case 'POST': return 'badge-success'
    case 'PUT': return 'badge-warning'
    case 'DELETE': return 'badge-error'
    case 'PATCH': return 'badge-accent'
    default: return 'badge-ghost'
  }
}

const getStatusBadgeClass = (status: number): string => {
  if (!status || status === 0) return 'badge-ghost'
  if (status >= 200 && status < 300) return 'badge-success'
  if (status >= 300 && status < 400) return 'badge-info'
  if (status >= 400 && status < 500) return 'badge-warning'
  if (status >= 500) return 'badge-error'
  return 'badge-ghost'
}

const getUrlPath = (url: string): string => {
  try {
    const urlObj = new URL(url)
    const path = urlObj.pathname + urlObj.search
    return path.length > 30 ? path.substring(0, 30) + '...' : path
  } catch {
    return url.length > 30 ? url.substring(0, 30) + '...' : url
  }
}

const getTypeBadgeClass = (type?: TrafficSendType): string => {
  switch (type) {
    case 'request': return 'badge-primary'
    case 'response': return 'badge-secondary'
    default: return 'badge-accent'
  }
}

const getTypeLabel = (type?: TrafficSendType): string => {
  switch (type) {
    case 'request': return 'REQ'
    case 'response': return 'RES'
    default: return 'ALL'
  }
}

// 聚焦输入框
const focusInput = () => {
  nextTick(() => {
    textareaRef.value?.focus()
  })
}


onMounted(async () => {
  autoResize()
  await loadSlashCommands()
  // 同步父组件传入的初始值
  // Initialize persistent states (persisted values take precedence)
  try {
    // RAG: prefer persisted value if exists, otherwise use prop
    const hasPersistedRag = localStorage.getItem(STORAGE_KEYS.rag) !== null
    const savedRag = hasPersistedRag ? getBool(STORAGE_KEYS.rag) : !!props.ragEnabled
    localRagEnabled.value = savedRag
    setBool(STORAGE_KEYS.rag, savedRag)
    emit('toggle-rag', savedRag)
    
    // Tools: prefer persisted value if exists, otherwise use prop
    const hasPersistedTools = localStorage.getItem(STORAGE_KEYS.tools) !== null
    const savedTools = hasPersistedTools ? getBool(STORAGE_KEYS.tools) : !!props.toolsEnabled
    localToolsEnabled.value = savedTools
    setBool(STORAGE_KEYS.tools, savedTools)
    emit('toggle-tools', savedTools)

    const hasPersistedWebSearch = localStorage.getItem(STORAGE_KEYS.webSearch) !== null
    const savedWebSearch = hasPersistedWebSearch ? getBool(STORAGE_KEYS.webSearch) : !!props.webSearchEnabled
    localWebSearchEnabled.value = savedWebSearch
    setBool(STORAGE_KEYS.webSearch, savedWebSearch)
    emit('toggle-web-search', savedWebSearch)

    // Team: prefer persisted value if exists, otherwise use prop
    const hasPersistedTeam = localStorage.getItem(STORAGE_KEYS.team) !== null
    const savedTeam = hasPersistedTeam ? getBool(STORAGE_KEYS.team) : !!props.teamEnabled
    localTeamEnabled.value = savedTeam
    setBool(STORAGE_KEYS.team, savedTeam)
    emit('toggle-team', savedTeam)
  } catch {
    // fallback to props on any error
    localRagEnabled.value = !!props.ragEnabled
    localToolsEnabled.value = !!props.toolsEnabled
    localWebSearchEnabled.value = !!props.webSearchEnabled
    localTeamEnabled.value = !!props.teamEnabled
  }
  initialized.value = true
  window.addEventListener('click', handleClickOutside, true)
  
  // 设置 Tauri 拖放监听
  await setupNativeDragDrop()
  
  // 自动聚焦输入框
  focusInput()
})

onUnmounted(() => {
  window.removeEventListener('click', handleClickOutside, true)
  teardownNativeDragDrop()
})

// 监听父组件状态变化，保持本地按钮状态一致（并持久化）
watch(
  () => props.ragEnabled,
  (val) => {
    if (typeof val === 'boolean') {
      localRagEnabled.value = val
      setBool(STORAGE_KEYS.rag, val)
    }
  }
)

// 监听工具状态变化（父组件从数据库加载后会更新）
watch(
  () => props.toolsEnabled,
  (val) => {
    if (typeof val === 'boolean') {
      localToolsEnabled.value = val
      setBool(STORAGE_KEYS.tools, val)
    }
  }
)

watch(
  () => props.webSearchEnabled,
  (val) => {
    if (typeof val === 'boolean') {
      localWebSearchEnabled.value = val
      setBool(STORAGE_KEYS.webSearch, val)
    }
  }
)

watch(
  () => props.teamEnabled,
  (val) => {
    if (typeof val === 'boolean') {
      localTeamEnabled.value = val
      setBool(STORAGE_KEYS.team, val)
    }
  }
)

watch(
  () => props.selectedModel,
  (val) => {
    if (typeof val === 'string') {
      localSelectedModel.value = val
    }
  },
  { immediate: true },
)

watch(
  () => props.inputMessage,
  (val) => {
    if (!val || !val.includes('/')) {
      closeSlashPopover()
    }
  }
)

// 暴露方法供父组件调用
defineExpose({
  focusInput,
})

// End script
</script>

<style scoped>
.input-area-container {
  /* Ensure input area doesn't overlap sidebar */
  width: 100%;
  max-width: 100%;
}

.chat-input { 
  position: relative; 
}

.slash-popover {
  position: absolute;
  left: 0.5rem;
  right: 0.5rem;
  bottom: calc(100% + 6px);
  z-index: 120;
  backdrop-filter: blur(8px);
}

.slash-manager-modal {
  z-index: 1300;
}

.slash-manager-modal .modal-box {
  max-height: calc(100vh - 3rem);
}

.icon-btn { 
  width:1.75rem; 
  height:1.75rem; 
  display:flex; 
  align-items:center; 
  justify-content:center; 
  border-radius:0.375rem; 
  font-size:calc(var(--font-size-base, 14px) * 0.75); 
  transition:background-color .15s,color .15s; 
}

.icon-btn:hover { 
  background-color: hsl(var(--b3)/0.7); 
}

.icon-btn.active { 
  background: hsl(var(--p)); 
  color: hsl(var(--pc)); 
  box-shadow:0 2px 4px rgba(0,0,0,.15); 
}

.assistant-model-switch {
  min-width: 6rem;
  max-width: 14rem;
  flex-shrink: 1;
}

.assistant-model-switch :deep(.input) {
  border-radius: 0.5rem;
  font-size: 0.75rem;
}

@media (max-width: 1024px) {
  .assistant-model-switch {
    max-width: 10rem;
  }
}

.send-btn { 
  width:2rem; 
  height:2rem; 
  border-radius:9999px; 
  background: hsl(var(--b3)); 
  color: hsl(var(--bc)); 
  display:flex; 
  align-items:center; 
  justify-content:center; 
  transition: background-color .15s,color .15s; 
}

.send-btn:hover { 
  background: hsl(var(--p)); 
  color: hsl(var(--pc)); 
}

.send-btn:disabled { 
  opacity:.4; 
  cursor:not-allowed; 
}

/* Search popover positioned above the toolbar */
.search-popover {
  position: absolute;
  bottom: calc(100% + 8px);
  left: 0;
  width: 24rem;
  max-width: 90vw;
  z-index: 50;
}

@media (max-width: 640px) {
  .search-popover { 
    width: 18rem; 
  }
}

/* Drag and drop styles */
.input-area-container.drag-over {
  position: relative;
}

.drag-overlay {
  position: absolute;
  inset: 0;
  background: hsl(var(--p) / 0.1);
  border: 2px dashed hsl(var(--p));
  border-radius: 0.5rem;
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 100;
  backdrop-filter: blur(4px);
}

.drag-content {
  display: flex;
  flex-direction: column;
  align-items: center;
  color: hsl(var(--p));
  text-align: center;
  padding: 1rem;
}

/* Context usage indicator */
.context-usage-indicator {
  font-variant-numeric: tabular-nums;
  white-space: nowrap;
  transition: all 0.2s ease;
}

.context-usage-indicator:hover {
  opacity: 0.9;
}
</style>
