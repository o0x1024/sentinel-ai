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
            @click="retryUploadDocument(doc, idx)"
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
          <!-- Leading action icons -->
          <div class="flex items-center gap-2 text-base-content/60 shrink-0">
            <button class="icon-btn" title="附件" @click="triggerFileSelect"><i class="fas fa-paperclip"></i></button>
            <button class="icon-btn" :class="{ active: localToolsEnabled }" title="工具调用" @click="toggleTools"><i class="fas fa-tools"></i></button>
            <button v-if="localToolsEnabled" class="icon-btn" title="工具配置" @click="emit('open-tool-config')"><i class="fas fa-cog"></i></button>
            <button
              class="icon-btn"
              :class="{ active: localTeamEnabled }"
              title="Team 模式"
              @click="toggleTeam"
            >
              <i class="fas fa-users"></i>
            </button>
            <button 
              class="icon-btn" 
              :class="{ active: localRagEnabled }" 
              title="知识检索增强 - AI将使用 [SOURCE n] 格式引用知识库内容" 
              @click="toggleRAG"
            >
              <i class="fas fa-brain"></i>
            </button>
            <button class="icon-btn " title="@ 引用"><i class="fas fa-at"></i></button>
            <button class="icon-btn" title="快速指令" @click="openSlashManager"><i class="fas fa-bolt"></i></button>
            <button class="icon-btn" title="选择"><i class="fas fa-border-all"></i></button>
            <button class="icon-btn" title="清空会话" @click="clearConversation"><i class="fas fa-eraser"></i></button>
          </div>

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
import { invoke } from '@tauri-apps/api/core'
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
import type { UnlistenFn } from '@tauri-apps/api/event'
import draggable from 'vuedraggable'
import SearchableSelect from '@/components/SearchableSelect.vue'
import type { PendingDocumentAttachment, ProcessedDocumentResult } from '@/types/agent'
import { dialog } from '@/composables/useDialog'

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

type SlashCommandType = 'prompt' | 'action'
type SlashCommandScope = 'global' | 'conversation'
type SlashActionType =
  | 'new_conversation'
  | 'clear_conversation'
  | 'toggle_rag'
  | 'toggle_tools'
  | 'open_tool_config'

interface SlashCommandItem {
  id: string
  name: string
  description?: string
  type: SlashCommandType
  template?: string
  action?: SlashActionType
  auto_send?: boolean
  enabled: boolean
  scope?: SlashCommandScope
  sort?: number
  is_builtin?: boolean
}

const props = defineProps<{
  inputMessage: string
  conversationId?: string | null
  isLoading: boolean
  showDebugInfo: boolean
  allowTakeover?: boolean
  ragEnabled?: boolean
  toolsEnabled?: boolean
  teamEnabled?: boolean
  pendingAttachments?: any[]
  pendingDocuments?: PendingDocumentAttachment[]
  processedDocuments?: ProcessedDocumentResult[]
  referencedTraffic?: ReferencedTraffic[]
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
  (e: 'toggle-team', enabled: boolean): void
  (e: 'open-tool-config'): void
  (e: 'add-attachments', files: string[]): void
  (e: 'remove-attachment', index: number): void
  (e: 'add-documents', files: PendingDocumentAttachment[]): void
  (e: 'remove-document', index: number): void
  (e: 'document-processed', result: ProcessedDocumentResult): void
  (e: 'remove-traffic', index: number): void
  (e: 'clear-traffic'): void
  (e: 'change-model', value: string): void
}>()

// removed architecture utilities

const allowTakeover = computed(() => props.allowTakeover === true)

// --- New input logic ---
const textareaRef = ref<HTMLTextAreaElement | null>(null)
const containerRef = ref<HTMLDivElement | null>(null)
const fileInputRef = ref<HTMLInputElement | null>(null)

// --- Persistence helpers ---
const STORAGE_KEYS = {
  rag: 'sentinel:input:ragEnabled',
  tools: 'sentinel:input:toolsEnabled',
  team: 'sentinel:input:teamEnabled',
} as const

const SLASH_COMMANDS_CATEGORY = 'agent'
const SLASH_COMMANDS_KEY = 'slash_commands'

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
const localTeamEnabled = ref<boolean>(!!props.teamEnabled)
const localSelectedModel = ref(props.selectedModel || '')
const availableModelOptions = computed(() => props.availableModels || [])

// init guard
const initialized = ref(false)

// Slash commands
const slashOpen = ref(false)
const slashActiveIndex = ref(0)
const slashQuery = ref('')
const slashRange = ref<{ start: number; end: number } | null>(null)
const showSlashManager = ref(false)
const editingSlashId = ref<string | null>(null)
const slashFormError = ref('')
const slashManagerScope = ref<SlashCommandScope>('global')
const globalSlashCommands = ref<SlashCommandItem[]>([])
const conversationSlashCommands = ref<Record<string, SlashCommandItem[]>>({})
const importSlashInputRef = ref<HTMLInputElement | null>(null)
const conversationScopeEnabled = computed(() => !!props.conversationId)
const conversationScopeKey = computed(() => props.conversationId || '__no_conversation__')

const builtinSlashCommands = computed<SlashCommandItem[]>(() => [
  {
    id: 'builtin-new',
    name: 'new',
    description: '新建会话',
    type: 'action',
    action: 'new_conversation',
    enabled: true,
    is_builtin: true,
  },
  {
    id: 'builtin-clear',
    name: 'clear',
    description: '清空当前会话',
    type: 'action',
    action: 'clear_conversation',
    enabled: true,
    is_builtin: true,
  },
  {
    id: 'builtin-rag',
    name: 'rag',
    description: '切换 RAG',
    type: 'action',
    action: 'toggle_rag',
    enabled: true,
    is_builtin: true,
  },
  {
    id: 'builtin-tools',
    name: 'tools',
    description: '切换 Tools',
    type: 'action',
    action: 'toggle_tools',
    enabled: true,
    is_builtin: true,
  },
  {
    id: 'builtin-toolcfg',
    name: 'toolcfg',
    description: '打开工具配置',
    type: 'action',
    action: 'open_tool_config',
    enabled: true,
    is_builtin: true,
  },
])

const sortByOrder = (a: SlashCommandItem, b: SlashCommandItem) => (a.sort ?? 0) - (b.sort ?? 0)

const currentConversationCommands = computed(() => {
  const key = conversationScopeKey.value
  return (conversationSlashCommands.value[key] || []).slice().sort(sortByOrder)
})

const runtimeCustomCommands = computed(() => {
  const map = new Map<string, SlashCommandItem>()
  globalSlashCommands.value
    .filter((c) => c.enabled !== false)
    .slice()
    .sort(sortByOrder)
    .forEach((cmd) => map.set(cmd.name.toLowerCase(), cmd))
  currentConversationCommands.value
    .filter((c) => c.enabled !== false)
    .forEach((cmd) => map.set(cmd.name.toLowerCase(), cmd))
  return [...map.values()]
})

const slashCommands = computed(() => {
  return [...builtinSlashCommands.value, ...runtimeCustomCommands.value]
})

const filteredSlashCommands = computed(() => {
  const q = slashQuery.value.trim().toLowerCase()
  const list = slashCommands.value.filter((cmd) => {
    if (!q) return true
    const text = `${cmd.name} ${cmd.description || ''}`.toLowerCase()
    return text.includes(q)
  })
  return list.slice(0, 20)
})

const scopeCommandsForManager = computed(() => {
  if (slashManagerScope.value === 'conversation') {
    return currentConversationCommands.value
  }
  return globalSlashCommands.value.slice().sort(sortByOrder)
})

const managerBuiltinCommands = computed(() => {
  return slashManagerScope.value === 'global' ? builtinSlashCommands.value : []
})

const managerCustomCommands = computed<SlashCommandItem[]>({
  get: () => scopeCommandsForManager.value,
  set: (value) => {
    if (slashManagerScope.value === 'conversation') {
      const key = conversationScopeKey.value
      conversationSlashCommands.value = {
        ...conversationSlashCommands.value,
        [key]: value,
      }
    } else {
      globalSlashCommands.value = value
    }
  },
})

const newSlashCommand = ref<SlashCommandItem>({
  id: '',
  name: '',
  description: '',
  type: 'prompt',
  template: '{{input}}',
  action: 'new_conversation',
  auto_send: false,
  enabled: true,
  scope: 'global',
  sort: 0,
})

const getActionLabel = (action?: SlashActionType) => {
  switch (action) {
    case 'new_conversation':
      return '新建会话'
    case 'clear_conversation':
      return '清空会话'
    case 'toggle_rag':
      return '切换 RAG'
    case 'toggle_tools':
      return '切换 Tools'
    case 'open_tool_config':
      return '打开工具配置'
    default:
      return ''
  }
}

const getScopeCommandsMutable = (scope: SlashCommandScope): SlashCommandItem[] => {
  if (scope === 'conversation') {
    const key = conversationScopeKey.value
    if (!conversationSlashCommands.value[key]) {
      conversationSlashCommands.value[key] = []
    }
    return conversationSlashCommands.value[key]
  }
  return globalSlashCommands.value
}

const getScopeLabel = (scope?: SlashCommandScope) => (scope === 'conversation' ? '会话' : '全局')

const normalizeSlashCommand = (
  item: Partial<SlashCommandItem>,
  fallbackScope: SlashCommandScope,
  fallbackSort: number,
): SlashCommandItem => {
  const scope: SlashCommandScope = item.scope === 'conversation' ? 'conversation' : fallbackScope
  return {
    id: item.id || generateCommandId(),
    name: String(item.name || '').replace(/^\//, ''),
    description: typeof item.description === 'string' ? item.description : '',
    type: item.type === 'action' ? 'action' : 'prompt',
    template: typeof item.template === 'string' ? item.template : '',
    action: item.action,
    auto_send: !!item.auto_send,
    enabled: item.enabled !== false,
    scope,
    sort: typeof item.sort === 'number' ? item.sort : fallbackSort,
  }
}

const generateCommandId = (): string => {
  try {
    if (typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function') {
      return crypto.randomUUID()
    }
  } catch {
    // ignore and fallback
  }
  return `slash_${Date.now()}_${Math.random().toString(36).slice(2, 10)}`
}

// popover positioning (fixed)
const popoverStyle = ref<Record<string, string>>({})
const updatePopoverPosition = () => {
  const el = containerRef.value
  if (!el) return
  const rect = el.getBoundingClientRect()
  const desiredWidth = Math.min(rect.width, 384) // 24rem max
  popoverStyle.value = {
    top: `${rect.top}px`,
    left: `${rect.left}px`,
    width: `${desiredWidth}px`,
    transform: 'translateY(calc(-100% - 8px))',
  }
}

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

const closeSlashPopover = () => {
  slashOpen.value = false
  slashActiveIndex.value = 0
  slashQuery.value = ''
  slashRange.value = null
}

const findSlashRange = (text: string, caret: number): { start: number; end: number; query: string } | null => {
  if (caret < 0 || caret > text.length) return null
  let idx = caret - 1
  while (idx >= 0 && text[idx] !== '\n') {
    if (text[idx] === '/') {
      const prev = idx === 0 ? ' ' : text[idx - 1]
      if (idx === 0 || /\s/.test(prev)) {
        const segment = text.slice(idx + 1, caret)
        if (!/\s/.test(segment)) {
          return { start: idx, end: caret, query: segment }
        }
      }
      return null
    }
    if (/\s/.test(text[idx])) {
      break
    }
    idx -= 1
  }
  return null
}

const updateSlashState = (text: string, caret: number) => {
  const range = findSlashRange(text, caret)
  if (!range) {
    closeSlashPopover()
    return
  }
  const previousQuery = slashQuery.value
  const wasOpen = slashOpen.value
  slashRange.value = { start: range.start, end: range.end }
  slashQuery.value = range.query
  slashOpen.value = true
  if (!wasOpen || range.query !== previousQuery) {
    slashActiveIndex.value = 0
  } else {
    const maxIndex = Math.max(0, filteredSlashCommands.value.length - 1)
    slashActiveIndex.value = Math.min(slashActiveIndex.value, maxIndex)
  }
}

const setInputValueAtCaret = (newValue: string, caret: number) => {
  emit('update:input-message', newValue)
  nextTick(() => {
    const el = textareaRef.value
    if (!el) return
    el.focus()
    el.setSelectionRange(caret, caret)
    updateSlashState(newValue, caret)
    autoResize()
  })
}

const executeSlashAction = (action?: SlashActionType) => {
  if (!action) return
  switch (action) {
    case 'new_conversation':
      createNewConversation()
      break
    case 'clear_conversation':
      clearConversation()
      break
    case 'toggle_rag':
      toggleRAG()
      break
    case 'toggle_tools':
      toggleTools()
      break
    case 'open_tool_config':
      emit('open-tool-config')
      break
  }
}

const applySlashCommand = (cmd: SlashCommandItem) => {
  const range = slashRange.value
  const original = props.inputMessage || ''
  if (!range) return

  if (cmd.type === 'action') {
    const before = original.slice(0, range.start)
    const after = original.slice(range.end)
    const newValue = `${before}${after}`.replace(/^\s+/, '')
    setInputValueAtCaret(newValue, Math.max(0, range.start))
    executeSlashAction(cmd.action)
    closeSlashPopover()
    return
  }

  const afterSlash = original.slice(range.end).trimStart()
  const template = cmd.template || ''
  const rendered = template.includes('{{input}}')
    ? template.split('{{input}}').join(afterSlash)
    : [template, afterSlash].filter(Boolean).join(' ')
  const before = original.slice(0, range.start)
  const newValue = `${before}${rendered}`
  setInputValueAtCaret(newValue, newValue.length)
  closeSlashPopover()
  if (cmd.auto_send) {
    nextTick(() => emitSend())
  }
}

const loadSlashCommands = async () => {
  try {
    const items = await invoke<Array<{ key: string; value: string }>>('get_config', {
      request: { category: SLASH_COMMANDS_CATEGORY, key: SLASH_COMMANDS_KEY },
    })
    if (!items || items.length === 0 || !items[0]?.value) {
      globalSlashCommands.value = []
      conversationSlashCommands.value = {}
      return
    }
    const parsed = JSON.parse(items[0].value)
    if (Array.isArray(parsed)) {
      globalSlashCommands.value = parsed
        .filter((item) => item && typeof item.name === 'string')
        .map((item, idx) => normalizeSlashCommand(item, 'global', idx))
      conversationSlashCommands.value = {}
      return
    }
    const rawGlobal = Array.isArray(parsed?.global) ? parsed.global : []
    const rawConversations = parsed?.conversations && typeof parsed.conversations === 'object'
      ? parsed.conversations
      : {}
    globalSlashCommands.value = rawGlobal
      .filter((item: any) => item && typeof item.name === 'string')
      .map((item: any, idx: number) => normalizeSlashCommand(item, 'global', idx))
    const convStore: Record<string, SlashCommandItem[]> = {}
    Object.entries(rawConversations).forEach(([key, list]) => {
      if (!Array.isArray(list)) return
      convStore[key] = list
        .filter((item: any) => item && typeof item.name === 'string')
        .map((item: any, idx: number) => normalizeSlashCommand(item, 'conversation', idx))
    })
    conversationSlashCommands.value = convStore
  } catch (e) {
    console.warn('[InputArea] Failed to load slash commands:', e)
    globalSlashCommands.value = []
    conversationSlashCommands.value = {}
  }
}

const saveSlashCommands = async () => {
  try {
    const value = JSON.stringify({
      version: 2,
      global: globalSlashCommands.value,
      conversations: conversationSlashCommands.value,
    })
    await invoke('set_config', {
      category: SLASH_COMMANDS_CATEGORY,
      key: SLASH_COMMANDS_KEY,
      value,
    })
  } catch (e) {
    console.error('[InputArea] Failed to save slash commands:', e)
    dialog.toast.error('保存 Slash 命令失败')
  }
}

const addCustomSlashCommand = async () => {
  try {
    slashFormError.value = ''
    const name = newSlashCommand.value.name.trim().replace(/^\//, '')
    if (!name) {
      slashFormError.value = '命令名不能为空'
      dialog.toast.error('命令名不能为空')
      return
    }
    const scope = newSlashCommand.value.scope === 'conversation' ? 'conversation' : 'global'
    if (scope === 'conversation' && !conversationScopeEnabled.value) {
      slashFormError.value = '当前没有会话，无法创建会话级命令'
      dialog.toast.error('当前没有会话，无法创建会话级命令')
      return
    }

    const targetList = getScopeCommandsMutable(scope)
    const exists = [
      ...builtinSlashCommands.value,
      ...targetList,
    ].some((cmd) => cmd.name.toLowerCase() === name.toLowerCase())
    if (exists) {
      slashFormError.value = `命令 /${name} 已存在`
      dialog.toast.error(`命令 /${name} 已存在`)
      return
    }
    if (newSlashCommand.value.type === 'prompt' && !newSlashCommand.value.template?.trim()) {
      slashFormError.value = '提示词模板不能为空'
      dialog.toast.error('提示词模板不能为空')
      return
    }
    if (newSlashCommand.value.type === 'action' && !newSlashCommand.value.action) {
      slashFormError.value = '请选择功能动作'
      dialog.toast.error('请选择功能动作')
      return
    }

    targetList.push({
      id: generateCommandId(),
      name,
      description: newSlashCommand.value.description?.trim() || '',
      type: newSlashCommand.value.type,
      template: newSlashCommand.value.type === 'prompt' ? newSlashCommand.value.template || '' : '',
      action: newSlashCommand.value.type === 'action' ? newSlashCommand.value.action : undefined,
      auto_send: !!newSlashCommand.value.auto_send,
      enabled: true,
      scope,
      sort: targetList.length,
    })
    await saveSlashCommands()
    resetSlashForm()
    dialog.toast.success('Slash 命令已添加')
  } catch (err: any) {
    const message = `添加命令失败: ${String(err)}`
    slashFormError.value = message
    dialog.toast.error(message)
    console.error('[InputArea] addCustomSlashCommand failed:', err)
  }
}

const startEditCommand = (cmd: SlashCommandItem) => {
  if (cmd.is_builtin) return
  editingSlashId.value = cmd.id
  newSlashCommand.value = {
    id: cmd.id,
    name: cmd.name,
    description: cmd.description || '',
    type: cmd.type,
    template: cmd.template || '',
    action: cmd.action || 'new_conversation',
    auto_send: !!cmd.auto_send,
    enabled: cmd.enabled !== false,
    scope: cmd.scope || slashManagerScope.value,
    sort: cmd.sort ?? 0,
  }
}

const saveEditedSlashCommand = async () => {
  try {
    slashFormError.value = ''
    const editId = editingSlashId.value
    if (!editId) return
    const name = newSlashCommand.value.name.trim().replace(/^\//, '')
    if (!name) {
      slashFormError.value = '命令名不能为空'
      dialog.toast.error('命令名不能为空')
      return
    }
    const targetScope = newSlashCommand.value.scope === 'conversation' ? 'conversation' : 'global'
    if (targetScope === 'conversation' && !conversationScopeEnabled.value) {
      slashFormError.value = '当前没有会话，无法保存会话级命令'
      dialog.toast.error('当前没有会话，无法保存会话级命令')
      return
    }
    const targetListForConflict = getScopeCommandsMutable(targetScope)
    const existingConflict = [...builtinSlashCommands.value, ...targetListForConflict].some((cmd) => {
      if (cmd.id === editId) return false
      return cmd.name.toLowerCase() === name.toLowerCase()
    })
    if (existingConflict) {
      slashFormError.value = `命令 /${name} 已存在`
      dialog.toast.error(`命令 /${name} 已存在`)
      return
    }
    const removeFromAllScopes = () => {
      globalSlashCommands.value = globalSlashCommands.value.filter((cmd) => cmd.id !== editId)
      Object.keys(conversationSlashCommands.value).forEach((key) => {
        conversationSlashCommands.value[key] = (conversationSlashCommands.value[key] || []).filter((cmd) => cmd.id !== editId)
      })
    }
    removeFromAllScopes()
    const targetList = getScopeCommandsMutable(targetScope)
    targetList.push({
      id: editId,
      name,
      description: newSlashCommand.value.description?.trim() || '',
      type: newSlashCommand.value.type,
      template: newSlashCommand.value.type === 'prompt' ? newSlashCommand.value.template || '' : '',
      action: newSlashCommand.value.type === 'action' ? newSlashCommand.value.action : undefined,
      auto_send: !!newSlashCommand.value.auto_send,
      enabled: newSlashCommand.value.enabled !== false,
      scope: targetScope,
      sort: targetList.length,
    })
    normalizeSortForScope(targetScope)
    await saveSlashCommands()
    resetSlashForm()
    dialog.toast.success('Slash 命令已更新')
  } catch (err: any) {
    const message = `保存命令失败: ${String(err)}`
    slashFormError.value = message
    dialog.toast.error(message)
    console.error('[InputArea] saveEditedSlashCommand failed:', err)
  }
}

const cancelEditCommand = () => {
  resetSlashForm()
}

const resetSlashForm = () => {
  editingSlashId.value = null
  slashFormError.value = ''
  newSlashCommand.value = {
    id: '',
    name: '',
    description: '',
    type: 'prompt',
    template: '{{input}}',
    action: 'new_conversation',
    auto_send: false,
    enabled: true,
    scope: slashManagerScope.value,
    sort: 0,
  }
}

const deleteSlashCommand = async (id: string) => {
  if (editingSlashId.value === id) {
    resetSlashForm()
  }
  if (slashManagerScope.value === 'conversation') {
    const key = conversationScopeKey.value
    conversationSlashCommands.value[key] = (conversationSlashCommands.value[key] || []).filter((cmd) => cmd.id !== id)
  } else {
    globalSlashCommands.value = globalSlashCommands.value.filter((cmd) => cmd.id !== id)
  }
  normalizeSortForScope(slashManagerScope.value)
  await saveSlashCommands()
}

const toggleCommandEnabled = async (cmd: SlashCommandItem, enabled: boolean) => {
  if (cmd.is_builtin) return
  const target = getScopeCommandsMutable(slashManagerScope.value).find((item) => item.id === cmd.id)
  if (!target) return
  target.enabled = enabled
  await saveSlashCommands()
}

const onCommandEnabledChange = async (cmd: SlashCommandItem, event: Event) => {
  const target = event.target as HTMLInputElement | null
  await toggleCommandEnabled(cmd, !!target?.checked)
}

const normalizeSortForScope = (scope: SlashCommandScope) => {
  const list = getScopeCommandsMutable(scope)
  list.sort(sortByOrder)
  list.forEach((item, idx) => {
    item.sort = idx
  })
}

const onDragSortEnd = async () => {
  normalizeSortForScope(slashManagerScope.value)
  await saveSlashCommands()
}

const exportSlashCommands = () => {
  const scope = slashManagerScope.value
  const payload = {
    version: 1,
    scope,
    commands: scopeCommandsForManager.value.filter((c) => !c.is_builtin),
  }
  const blob = new Blob([JSON.stringify(payload, null, 2)], { type: 'application/json' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = `slash-commands-${scope}-${new Date().toISOString().slice(0, 10)}.json`
  a.click()
  URL.revokeObjectURL(url)
}

const triggerImportSlashCommands = () => {
  importSlashInputRef.value?.click()
}

const onImportSlashFileChange = async (e: Event) => {
  const input = e.target as HTMLInputElement
  const file = input.files?.[0]
  if (!file) return
  try {
    const text = await file.text()
    const parsed = JSON.parse(text)
    const importedRaw = Array.isArray(parsed) ? parsed : parsed?.commands
    if (!Array.isArray(importedRaw)) {
      dialog.toast.error('导入文件格式无效')
      return
    }
    const scope = slashManagerScope.value
    const list = getScopeCommandsMutable(scope)
    const byName = new Map<string, SlashCommandItem>()
    list.forEach((item) => byName.set(item.name.toLowerCase(), item))
    const plan = importedRaw.map((item: any, idx: number) => {
      if (!item || typeof item.name !== 'string') return
      const normalized = normalizeSlashCommand(item, scope, list.length + idx)
      normalized.scope = scope
      normalized.is_builtin = false
      return normalized
    }).filter(Boolean) as SlashCommandItem[]

    const toAdd: string[] = []
    const toUpdate: string[] = []
    plan.forEach((cmd) => {
      const existing = byName.get(cmd.name.toLowerCase())
      if (existing) toUpdate.push(`/${cmd.name}`)
      else toAdd.push(`/${cmd.name}`)
    })

    const previewParts: string[] = [
      `作用域: ${scope === 'conversation' ? '会话' : '全局'}`,
      `新增: ${toAdd.length}`,
      `覆盖: ${toUpdate.length}`,
    ]
    if (toAdd.length > 0) {
      previewParts.push(`新增命令: ${toAdd.slice(0, 8).join(', ')}${toAdd.length > 8 ? ' ...' : ''}`)
    }
    if (toUpdate.length > 0) {
      previewParts.push(`覆盖命令: ${toUpdate.slice(0, 8).join(', ')}${toUpdate.length > 8 ? ' ...' : ''}`)
    }

    const confirmed = await dialog.confirm({
      title: '导入 Slash 命令',
      message: `${previewParts.join('\n')}\n\n确认执行导入？`,
      variant: 'warning',
    })
    if (!confirmed) return

    plan.forEach((normalized) => {
      const existing = byName.get(normalized.name.toLowerCase())
      if (existing) {
        existing.description = normalized.description
        existing.type = normalized.type
        existing.template = normalized.template
        existing.action = normalized.action
        existing.auto_send = normalized.auto_send
        existing.enabled = normalized.enabled
      } else {
        list.push(normalized)
        byName.set(normalized.name.toLowerCase(), normalized)
      }
    })
    normalizeSortForScope(scope)
    await saveSlashCommands()
    dialog.toast.success(`Slash 命令导入完成（新增 ${toAdd.length}，覆盖 ${toUpdate.length}）`)
  } catch (err) {
    console.error('[InputArea] Failed to import slash commands:', err)
    dialog.toast.error('导入 Slash 命令失败')
  } finally {
    input.value = ''
  }
}

const openSlashManager = async () => {
  await loadSlashCommands()
  if (slashManagerScope.value === 'conversation' && !conversationScopeEnabled.value) {
    slashManagerScope.value = 'global'
  }
  resetSlashForm()
  showSlashManager.value = true
}

const closeSlashManager = () => {
  showSlashManager.value = false
}

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

const triggerFileSelect = async () => {
  try {
    const { open } = await import('@tauri-apps/plugin-dialog')
    const selected = await open({
      multiple: true,
    })

    if (selected) {
      const filePaths = Array.isArray(selected) ? selected : [selected]
      await processDroppedFiles(filePaths)
    }
  } catch (error) {
    console.error('[InputArea] Tauri 文件选择失败:', error)
  }
}

// 保留 onFilesSelected 仅作为兜底（理论上不会触发）
const onFilesSelected = (e: Event) => {
  const input = e.target as HTMLInputElement
  if (!input.files || input.files.length === 0) return
  const files = Array.from(input.files)
  console.warn('[InputArea] 收到 File 对象，当前默认按 Tauri 模式运行，建议通过对话框选择文件')
  // 重置 input
  input.value = ''
}

// 获取附件预览图
const getAttachmentPreview = (att: any): string => {
  try {
    // 兼容两种结构：MessageAttachment::Image{ image } 或直接 { data, media_type, filename }
    const img = att.image ?? att
    const mediaTypeRaw: string | undefined = img?.media_type
    const mime = toMimeType(mediaTypeRaw)
    const dataField = img?.data
    const base64 = typeof dataField === 'string' ? dataField : dataField?.data
    if (!base64) return ''
    return `data:${mime};base64,${base64}`
  } catch (e) {
    console.error('[InputArea] 构造图片预览失败:', e, att)
    return ''
  }
}

// 将枚举/简写媒体类型转换为标准MIME
const toMimeType = (mediaType?: string): string => {
  if (!mediaType) return 'image/jpeg'
  const t = mediaType.toLowerCase()
  if (t === 'jpeg' || t === 'jpg') return 'image/jpeg'
  if (t === 'png') return 'image/png'
  if (t === 'gif') return 'image/gif'
  if (t === 'webp') return 'image/webp'
  return t.startsWith('image/') ? t : `image/${t}`
}

const formatFileSize = (bytes: number): string => {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
}

// 移除附件
const removeAttachment = (index: number) => {
  emit('remove-attachment', index)
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

// ====== 文档拖放功能 ======
const isDragOver = ref(false)
let unlistenDragDrop: UnlistenFn | null = null

// 设置 Tauri 原生拖放监听
const setupTauriDragDrop = async () => {
  try {
    const webview = getCurrentWebviewWindow()
    
    // Listen for drag-drop events
    unlistenDragDrop = await webview.onDragDropEvent(async (event) => {
      console.log('[InputArea] Drag drop event:', event.payload.type)
      const payload = event.payload as any
      const paths = Array.isArray(payload?.paths) ? payload.paths : []
      const hasFilePaths = paths.length > 0
      
      if (event.payload.type === 'over' || event.payload.type === 'enter') {
        // 仅对文件拖拽显示上传态，避免干扰页面内普通节点拖拽
        if (hasFilePaths) {
          isDragOver.value = true
        }
      } else if (event.payload.type === 'leave') {
        isDragOver.value = false
      } else if (event.payload.type === 'drop') {
        isDragOver.value = false
        console.log('[InputArea] Dropped files:', paths)
        
        if (!paths || paths.length === 0) return
        
        await processDroppedFiles(paths)
      }
    })
    
    console.log('[InputArea] Tauri drag-drop listener registered')
  } catch (error) {
    console.error('[InputArea] Failed to setup Tauri drag-drop:', error)
  }
}

// 处理拖放的文件
const processDroppedFiles = async (paths: string[]) => {
  const imageFiles: string[] = []

  for (const filePath of paths) {
    const fileName = filePath.split('/').pop() || filePath.split('\\').pop() || 'unknown'
    const ext = fileName.split('.').pop()?.toLowerCase() || ''
    
    console.log('[InputArea] Processing file:', fileName, 'ext:', ext)
    
    // 判断是图片还是其他文件（其他文件统一作为文档处理）
    if (['jpg', 'jpeg', 'png', 'gif', 'webp'].includes(ext)) {
      imageFiles.push(filePath)
    } else {
      let fileSize = 0
      try {
        const stat = await invoke<{ size: number, is_file: boolean }>('get_file_stat', { path: filePath })
        if (stat.is_file === false) {
          continue
        }
        fileSize = stat.size
      } catch {
        console.warn('[InputArea] Could not get file size for:', filePath)
      }

      const queuedId = crypto.randomUUID()
      const queuedDoc: PendingDocumentAttachment = {
        id: queuedId,
        original_path: filePath,
        original_filename: fileName,
        file_size: fileSize,
        mime_type: getMimeTypeFromExt(ext),
        status: 'processing',
      }
      emit('add-documents', [queuedDoc])

      try {
        const uploaded = await invoke<ProcessedDocumentResult>('upload_document_attachment', {
          filePath,
          clientId: queuedId,
          conversationId: props.conversationId ?? null
        })
        emit('document-processed', uploaded)
      } catch (error) {
        console.error('[InputArea] Failed to upload document:', error)
        emit('document-processed', {
          id: queuedId,
          file_id: queuedId,
          original_filename: fileName,
          file_size: fileSize,
          mime_type: getMimeTypeFromExt(ext),
          status: 'failed',
          error_message: String(error),
        } as ProcessedDocumentResult)
      }
    }
  }
  
  if (imageFiles.length > 0) {
    console.log('[InputArea] Adding', imageFiles.length, 'image(s)')
    emit('add-attachments', imageFiles)
  }
}

// 从扩展名获取 MIME 类型
const getMimeTypeFromExt = (ext: string): string => {
  const mimeMap: Record<string, string> = {
    // Office 文档
    docx: 'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
    doc: 'application/msword',
    xlsx: 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet',
    xls: 'application/vnd.ms-excel',
    pptx: 'application/vnd.openxmlformats-officedocument.presentationml.presentation',
    ppt: 'application/vnd.ms-powerpoint',
    pdf: 'application/pdf',
    rtf: 'application/rtf',
    // 邮件
    eml: 'message/rfc822',
    msg: 'application/vnd.ms-outlook',
    // 文本类
    txt: 'text/plain',
    md: 'text/markdown',
    json: 'application/json',
    xml: 'application/xml',
    csv: 'text/csv',
    html: 'text/html',
    htm: 'text/html',
    css: 'text/css',
    // 代码文件（统一作为文本处理）
    js: 'text/javascript',
    ts: 'text/typescript',
    jsx: 'text/javascript',
    tsx: 'text/typescript',
    py: 'text/x-python',
    java: 'text/x-java',
    c: 'text/x-c',
    cpp: 'text/x-c++',
    h: 'text/x-c',
    hpp: 'text/x-c++',
    rs: 'text/x-rust',
    go: 'text/x-go',
    rb: 'text/x-ruby',
    php: 'text/x-php',
    sh: 'text/x-shellscript',
    bash: 'text/x-shellscript',
    zsh: 'text/x-shellscript',
    sql: 'text/x-sql',
    yaml: 'text/yaml',
    yml: 'text/yaml',
    toml: 'text/x-toml',
    ini: 'text/x-ini',
    conf: 'text/plain',
    cfg: 'text/plain',
    log: 'text/plain',
    // 压缩文件
    zip: 'application/zip',
    tar: 'application/x-tar',
    gz: 'application/gzip',
    rar: 'application/vnd.rar',
    '7z': 'application/x-7z-compressed',
  }
  // 未知类型默认作为文本处理
  return mimeMap[ext] || 'text/plain'
}

// 保留 HTML5 拖放作为备用（用于视觉反馈）
const onDragOver = (e: DragEvent) => {
  // Tauri 会处理实际的拖放，这里只用于视觉反馈
  if (e.dataTransfer?.types.includes('Files')) {
    isDragOver.value = true
  }
}

const onDragLeave = () => {
  isDragOver.value = false
}

const onDrop = async (_e: DragEvent) => {
  // Tauri 的 onDragDropEvent 会处理实际的文件
  // 这里只重置状态
  isDragOver.value = false
}

// 移除文档
const removeDocument = (index: number) => {
  emit('remove-document', index)
}

const retryUploadDocument = async (doc: PendingDocumentAttachment, _index: number) => {
  try {
    const uploaded = await invoke<ProcessedDocumentResult>('upload_document_attachment', {
      filePath: doc.original_path,
      clientId: doc.id,
      conversationId: props.conversationId ?? null
    })
    emit('document-processed', uploaded)
  } catch (error) {
    console.error('[InputArea] Retry upload document failed:', error)
  }
}

const runSecurityAnalysis = async (doc: PendingDocumentAttachment) => {
  const fileId = doc.file_id || doc.id
  if (!fileId) return
  try {
    const message = await invoke<string>('run_file_security_analysis', { fileId })
    dialog.toast.success(message)
  } catch (error) {
    dialog.toast.error(String(error))
  }
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
    localTeamEnabled.value = !!props.teamEnabled
  }
  initialized.value = true
  window.addEventListener('resize', updatePopoverPosition)
  window.addEventListener('scroll', updatePopoverPosition, true)
  window.addEventListener('click', handleClickOutside, true)
  
  // 设置 Tauri 拖放监听
  await setupTauriDragDrop()
  
  // 自动聚焦输入框
  focusInput()
})

onUnmounted(() => {
  window.removeEventListener('resize', updatePopoverPosition)
  window.removeEventListener('scroll', updatePopoverPosition, true)
  window.removeEventListener('click', handleClickOutside, true)
  
  // 清理 Tauri 拖放监听
  if (unlistenDragDrop) {
    unlistenDragDrop()
    unlistenDragDrop = null
  }
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

watch(
  () => filteredSlashCommands.value.length,
  (len) => {
    if (len <= 0) {
      slashActiveIndex.value = 0
      return
    }
    if (slashActiveIndex.value >= len) {
      slashActiveIndex.value = len - 1
    }
  },
)

watch(slashManagerScope, (scope) => {
  if (scope === 'conversation' && !conversationScopeEnabled.value) {
    slashManagerScope.value = 'global'
    return
  }
  slashFormError.value = ''
  newSlashCommand.value.scope = scope
})

watch(
  () => [
    newSlashCommand.value.name,
    newSlashCommand.value.description,
    newSlashCommand.value.type,
    newSlashCommand.value.template,
    newSlashCommand.value.action,
    newSlashCommand.value.scope,
  ],
  () => {
    if (slashFormError.value) {
      slashFormError.value = ''
    }
  },
)

watch(
  () => props.conversationId,
  () => {
    if (slashManagerScope.value === 'conversation' && !conversationScopeEnabled.value) {
      slashManagerScope.value = 'global'
    }
  },
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
