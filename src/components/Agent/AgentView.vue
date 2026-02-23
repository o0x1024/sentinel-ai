<template>
  <div class="agent-view h-full flex bg-gradient-to-br from-base-100 to-base-200 overflow-hidden relative">
    <!-- Backdrop -->
    <div 
      v-if="showConversations || showToolConfig"
      class="conversation-backdrop absolute inset-0 bg-black/20 z-40 transition-opacity"
      @click="showConversations = false; showToolConfig = false"
    ></div>

    <!-- Conversation List Drawer -->
    <Transition name="slide-drawer">
      <div 
        v-if="showConversations"
        class="conversation-drawer absolute left-0 top-0 bottom-0 w-80 bg-base-100 shadow-2xl z-50 overflow-hidden"
      >
        <ConversationList 
          ref="conversationListRef"
          :current-conversation-id="conversationId"
          @select="handleSelectConversation"
          @create="handleCreateConversation"
          @close="showConversations = false"
        />
      </div>
    </Transition>

    <!-- Tool Config Panel -->
    <Transition name="slide-drawer-right">
      <div 
        v-if="showToolConfig"
        class="tool-config-drawer absolute right-0 top-0 bottom-0 w-[420px] bg-base-100 shadow-2xl z-50 overflow-hidden"
      >
        <ToolConfigPanel 
          :config="toolConfig"
          @update:config="handleToolConfigUpdate"
          @close="showToolConfig = false"
        />
      </div>
    </Transition>

    <!-- Main content area -->
    <div class="flex-1 flex flex-col overflow-hidden min-h-0">
      <!-- {{ t('agent.conversationHeader') }} -->
      <div class="conversation-header px-4 py-2 border-b border-base-300 flex items-center justify-between bg-base-100/50">
        <div class="flex items-center gap-2">
          <button 
            @click="showConversations = !showConversations"
            class="btn btn-sm btn-ghost"
            :title="t('agent.switchConversationList')"
          >
            <i class="fas fa-bars"></i>
          </button>
          <span class="text-sm font-medium text-base-content/70">
            {{ currentConversationTitle }}
          </span>
          
          <!-- RAG {{ t('agent.statusIndicator') }} -->
          <div v-if="ragEnabled" class="flex items-center gap-1 px-2 py-1 bg-info/10 rounded-md border border-info/30">
            <i class="fas fa-book text-info text-xs"></i>
            <span class="text-xs text-info font-medium">{{ t('agent.knowledgeBase') }}</span>
          </div>

          <!-- Tenth Man Toggle -->
          <button 
            @click="tenthManEnabled = !tenthManEnabled"
            class="btn btn-xs gap-1 transition-colors"
            :class="tenthManEnabled ? 'btn-error text-white' : 'btn-ghost text-base-content/50'"
            :title="tenthManEnabled ? 'Disable Tenth Man Rule (Strict Review)' : 'Enable Tenth Man Rule (Strict Review)'"
          >
            <i class="fas fa-user-secret"></i>
            <span class="text-xs font-medium hidden sm:inline">10th Man</span>
          </button>
        </div>
        <div class="flex items-center gap-2">
          <!-- Web Explorer History Button - shows when there's exploration history -->
          <button 
            v-if="webExplorerEvents.hasHistory.value"
            @click="handleToggleWebExplorer()"
            class="btn btn-sm gap-1"
            :class="isWebExplorerActive ? 'btn-primary' : 'btn-ghost text-primary'"
            :title="isWebExplorerActive ? t('agent.webExplorerPanelOpen') : t('agent.viewWebExplorerHistory')"
          >
            <i class="fas fa-globe"></i>
            <span>{{ t('agent.explore') }}</span>
            <span class="badge badge-xs badge-primary">{{ webExplorerEvents.steps.value.length }}</span>
          </button>
          <!-- Todos Button - always visible -->
          <button 
            @click="handleToggleTodos()"
            class="btn btn-sm gap-1"
            :class="isTodosPanelActive ? 'btn-primary' : 'btn-ghost text-primary'"
            :title="isTodosPanelActive ? t('agent.todosPanelOpen') : t('agent.viewTodos')"
          >
            <i class="fas fa-tasks"></i>
            <span>{{ t('agent.todos') }}</span>
            <span v-if="todosComposable.rootTodos.value.length > 0" class="badge badge-xs badge-primary">{{ todosComposable.rootTodos.value.length }}</span>
          </button>
          <!-- HTML Panel Button - shows when there is HTML content -->
          <button 
            v-if="hasHtmlPanelContent"
            @click="handleToggleHtmlPanel()"
            class="btn btn-sm gap-1"
            :class="isHtmlPanelActive ? 'btn-primary' : 'btn-ghost text-primary'"
            :title="isHtmlPanelActive ? t('agent.htmlPanelOpen') : t('agent.viewHtmlPanel')"
          >
            <i class="fas fa-code"></i>
            <span>{{ t('agent.htmlPanel') }}</span>
          </button>
          <!-- Terminal Button - always visible -->
          <button 
            @click="handleToggleTerminal()"
            class="btn btn-sm gap-1"
            :class="isTerminalActive ? 'btn-primary' : 'btn-ghost text-primary'"
            :title="isTerminalActive ? t('agent.terminalPanelOpen') : t('agent.viewTerminal')"
          >
            <i class="fas fa-terminal"></i>
            <span>{{ t('agent.terminal') }}</span>
          </button>
          <button
            v-if="toolConfig.audit_mode"
            @click="handleToggleAuditFindings()"
            class="btn btn-sm gap-1"
            :class="isAuditFindingsPanelActive ? 'btn-primary' : 'btn-ghost text-primary'"
            :title="isAuditFindingsPanelActive ? t('agent.auditFindingsPanelOpen') : t('agent.viewAuditFindings')"
          >
            <i class="fas fa-shield-halved"></i>
            <span>{{ t('agent.auditFindings') }}</span>
            <span v-if="auditFindings.length > 0" class="badge badge-xs badge-primary">{{ auditFindings.length }}</span>
          </button>
          <button 
            @click="handleCreateConversation()"
            class="btn btn-sm btn-ghost gap-1"
            :title="t('agent.newConversation')"
          >
            <i class="fas fa-plus"></i>
            <span>{{ t('agent.newConversation') }}</span>
          </button>
        </div>
      </div>

      <!-- {{ t('agent.messagesAndTodos') }} -->
      <div class="flex flex-1 overflow-hidden min-h-0">
        <!-- Left: Message flow + Input Area -->
        <div class="message-area flex-1 flex flex-col overflow-hidden min-h-0">
          <SubagentPanel
            :subagents="subagents"
            :is-open="isSubagentPanelOpen"
            @toggle="isSubagentPanelOpen = !isSubagentPanelOpen"
            @view-details="handleViewSubagentDetails"
          />
          <!-- Message flow -->
          <MessageFlow 
            ref="messageFlowRef"
            :messages="messages"
            :is-executing="isExecuting"
            :is-streaming="isStreaming"
            :streaming-content="streamingContent"
            :is-web-explorer-active="isWebExplorerActive"
            class="flex-1"
            @resend="handleResendMessage"
            @edit="handleEditMessage"
            @render-html="handleRenderHtml"
          />
          
          <!-- {{ t('agent.inputArea') }} -->
          <InputAreaComponent
            ref="inputAreaRef"
            v-model:input-message="inputValue"
            :conversation-id="conversationId"
            :is-loading="isExecuting"
            :allow-takeover="true"
            :show-debug-info="false"
            :rag-enabled="ragEnabled"
            :tools-enabled="toolsEnabled"
            :pending-attachments="pendingAttachments"
            :pending-documents="pendingDocuments"
            :processed-documents="processedDocuments"
            :referenced-traffic="referencedTraffic"
            :context-usage="contextUsage"
            :available-models="assistantModelOptions"
            :selected-model="assistantSelectedModel"
            :model-loading="isLoadingAssistantModels"
            @send-message="handleSubmit"
            @stop-execution="handleStop"
            @toggle-rag="handleToggleRAG"
            @toggle-tools="handleToggleTools"
            @change-model="handleAssistantModelChange"
            @add-attachments="handleAddAttachments"
            @remove-attachment="handleRemoveAttachment"
            @add-documents="handleAddDocuments"
            @remove-document="handleRemoveDocument"
            @document-processed="handleDocumentProcessed"
            @remove-traffic="handleRemoveTraffic"
            @clear-traffic="handleClearTraffic"
            @create-new-conversation="handleCreateConversation"
            @clear-conversation="handleClearConversation"
            @open-tool-config="showToolConfig = true"
          />
        </div>
        
        <!-- Right: Side Panel (WebExplorer, Todo, HTML, or Terminal) -->
        <div 
          v-if="isWebExplorerActive || isTodosPanelActive || isHtmlPanelActive || isTerminalActive || isAuditFindingsPanelActive"
          class="sidebar-container flex-shrink-0 border-l border-base-300 flex flex-col overflow-hidden bg-base-100 relative"
          :style="{ width: sidebarWidth + 'px' }"
        >
            <!-- Resize Handle -->
            <div 
              class="resize-handle absolute left-0 top-0 bottom-0 w-1 cursor-col-resize hover:bg-primary/50 transition-colors z-10"
              @mousedown="startResize"
            ></div>
            
            <WebExplorerPanel 
               v-if="isWebExplorerActive"
               :steps="webExplorerEvents.steps.value" 
               :coverage="webExplorerEvents.coverage.value"
               :discovered-apis="webExplorerEvents.discoveredApis.value"
               :is-active="isWebExplorerActive"
               :current-url="webExplorerEvents.currentUrl.value"
               :current-plan="webExplorerEvents.currentPlan.value"
               :current-progress="webExplorerEvents.currentProgress.value"
               :multi-agent="webExplorerEvents.multiAgent.value"
               :is-multi-agent-mode="webExplorerEvents.isMultiAgentMode.value"
               :activity="webExplorerEvents.activity.value"
               :show-takeover-form="webExplorerEvents.showTakeoverForm.value"
               :takeover-message="webExplorerEvents.takeoverMessage.value"
               :takeover-fields="webExplorerEvents.takeoverFields.value"
               :login-timeout-seconds="webExplorerEvents.loginTimeoutSeconds.value"
               :execution-id="webExplorerEvents.currentExecutionId.value"
               class="h-full border-0 rounded-none bg-transparent"
               @close="webExplorerEvents.close()"
            />
            <TodoPanel 
              v-else-if="isTodosPanelActive" 
              :todos="todos"
              :is-active="isTodosPanelActive"
              class="h-full p-4 overflow-y-auto border-0 bg-transparent"
              @close="handleCloseTodos"
            />
            <HtmlPanel
              v-else-if="isHtmlPanelActive"
              :html-content="htmlPanelContent"
              :is-active="isHtmlPanelActive"
              class="h-full p-4 overflow-y-auto border-0 bg-transparent"
              @close="handleCloseHtmlPanel"
            />
            <InteractiveTerminal
              v-else-if="isTerminalActive"
              class="h-full border-0 rounded-none bg-transparent"
              @close="handleCloseTerminal"
            />
            <AuditFindingsPanel
              v-else-if="isAuditFindingsPanelActive"
              :findings="auditFindings"
              :policy-gate="auditPolicyGate"
              class="h-full border-0 rounded-none bg-transparent"
              @close="handleCloseAuditFindings"
            />
        </div>
      </div>

      <!-- {{ t('agent.errorDisplay') }} -->
      <div v-if="error" class="error-banner flex items-center gap-2 px-4 py-3 bg-error/10 border-t border-error text-error text-sm">
        <span class="error-icon flex-shrink-0">⚠️</span>
        <span class="error-message flex-1 overflow-hidden text-ellipsis whitespace-nowrap">{{ error }}</span>
        <button @click="clearError" class="error-close bg-transparent border-none text-error cursor-pointer text-xl leading-none px-1 hover:text-base-content">×</button>
      </div>
    </div>

    <!-- Subagent Detail Modal -->
    <SubagentDetailModal
      :visible="showSubagentDetailModal"
      :subagent="selectedSubagent"
      @close="showSubagentDetailModal = false"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, onActivated, watch, nextTick } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type { AgentMessage, PendingDocumentAttachment, ProcessedDocumentResult } from '@/types/agent'
import { useAgentEvents } from '@/composables/useAgentEvents'
import { useWebExplorerEvents } from '@/composables/useWebExplorerEvents'
import { useTodos } from '@/composables/useTodos'
import { useTerminal } from '@/composables/useTerminal'
import { useAgentSessionManager } from '@/composables/useAgentSessionManager'
import MessageFlow from './MessageFlow.vue'
import TodoPanel from './TodoPanel.vue'
import HtmlPanel from './HtmlPanel.vue'
import WebExplorerPanel from './WebExplorerPanel.vue'
import SubagentPanel from './SubagentPanel.vue'
import SubagentDetailModal from './SubagentDetailModal.vue'
import AuditFindingsPanel from './AuditFindingsPanel.vue'
import InteractiveTerminal from '@/components/Tools/InteractiveTerminal.vue'
import InputAreaComponent from '@/components/InputAreaComponent.vue'
import ConversationList from './ConversationList.vue'
import ToolConfigPanel from './ToolConfigPanel.vue'

// Traffic reference type
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

type AuditScope = 'repo' | 'git_diff' | 'paths'
type VerificationLevel = 'low' | 'medium' | 'high'
type PolicyProfile = 'balanced' | 'prod_strict'

interface AuditConfig {
  enabled: boolean
  scope: AuditScope
  verification_level: VerificationLevel
  policy_profile: PolicyProfile
  required_tools: string[]
}

interface UiToolConfigPayload {
  enabled: boolean
  selection_strategy: any
  max_tools: number
  fixed_tools: string[]
  disabled_tools: string[]
  audit_mode?: boolean
  audit_config?: Partial<AuditConfig>
}

interface AuditFinding {
  id: string
  title?: string
  severity?: string
  severity_raw?: string
  confidence?: number
  files?: string[]
  fix?: string
  status?: string
  cwe?: string
  description?: string
  source?: Record<string, any>
  sink?: Record<string, any>
  hits?: Array<Record<string, any>>
  sources?: Array<Record<string, any>>
  sinks?: Array<Record<string, any>>
  source_sinks?: Array<Record<string, any>>
  trace_path?: Array<Record<string, any>>
  evidence?: string[]
}

interface PolicyGateResult {
  passed: boolean
  reason?: string
}

interface ParsedAuditPayload {
  findings: AuditFinding[]
  policyGate?: PolicyGateResult
}

interface AssistantModelOption {
  value: string
  label: string
  description?: string
}


const props = withDefaults(defineProps<{
  executionId?: string
  showTodos?: boolean
  selectedRole?: any
}>(), {
  showTodos: true,
})

const emit = defineEmits<{
  (e: 'submit', task: string): void
  (e: 'complete', result: any): void
  (e: 'error', error: string): void
}>()

// i18n & router
const { t } = useI18n()
const router = useRouter()

// Refs
const messageFlowRef = ref<InstanceType<typeof MessageFlow> | null>(null)
const conversationListRef = ref<InstanceType<typeof ConversationList> | null>(null)
const inputAreaRef = ref<InstanceType<typeof InputAreaComponent> | null>(null)
const inputValue = ref('')
const localError = ref<string | null>(null)
const conversationId = ref<string | null>(props.executionId ?? null)
const showConversations = ref(false) // Default hidden
const showToolConfig = ref(false)
const currentConversationTitle = ref(t('agent.newConversationTitle'))

// Feature toggles
const ragEnabled = ref(false)
const toolsEnabled = ref(false)
const tenthManEnabled = ref(false)
const pendingAttachments = ref<any[]>([])
const pendingDocuments = ref<PendingDocumentAttachment[]>([])
const processedDocuments = ref<ProcessedDocumentResult[]>([])
const referencedTraffic = ref<ReferencedTraffic[]>([])
const assistantModelOptions = ref<AssistantModelOption[]>([])
const assistantSelectedModel = ref('')
const isLoadingAssistantModels = ref(false)
const isSubagentPanelOpen = ref(false)
const subagents = computed(() => agentEvents.subagents.value)

// Subagent detail modal
const showSubagentDetailModal = ref(false)
const selectedSubagent = ref<{
  id: string
  role?: string
  status: 'running' | 'queued' | 'completed' | 'failed'
  progress?: number
  tools?: string[]
  parentId: string
  summary?: string
  task?: string
  error?: string
  startedAt?: number
  duration?: number
} | null>(null)

// Tool configuration
const toolConfig = ref({
  enabled: false,
  selection_strategy: 'Keyword',
  max_tools: 5,
  fixed_tools: ['interactive_shell'],
  disabled_tools: [],
  audit_mode: false,
  audit_config: {
    enabled: false,
    scope: 'git_diff' as AuditScope,
    verification_level: 'high' as VerificationLevel,
    policy_profile: 'balanced' as PolicyProfile,
    required_tools: [],
  },
})

const AUDIT_CONFIG_STORAGE_KEY = 'sentinel:agent:audit-config'

const defaultAuditConfig = (): AuditConfig => ({
  enabled: false,
  scope: 'git_diff',
  verification_level: 'high',
  policy_profile: 'balanced',
  required_tools: [],
})

const normalizeAuditConfig = (raw?: Partial<AuditConfig> | null): AuditConfig => {
  const base = defaultAuditConfig()
  const scope = raw?.scope === 'repo' || raw?.scope === 'paths' || raw?.scope === 'git_diff'
    ? raw.scope
    : base.scope
  const verificationLevel = raw?.verification_level === 'low' || raw?.verification_level === 'medium' || raw?.verification_level === 'high'
    ? raw.verification_level
    : base.verification_level
  const policyProfile = raw?.policy_profile === 'balanced' || raw?.policy_profile === 'prod_strict'
    ? raw.policy_profile
    : base.policy_profile
  const requiredTools = Array.isArray(raw?.required_tools)
    ? raw.required_tools.filter((item) => typeof item === 'string' && item.trim().length > 0)
    : base.required_tools
  return {
    enabled: raw?.enabled ?? base.enabled,
    scope,
    verification_level: verificationLevel,
    policy_profile: policyProfile,
    required_tools: requiredTools,
  }
}

const saveAuditConfigToLocal = (config: AuditConfig) => {
  try {
    localStorage.setItem(AUDIT_CONFIG_STORAGE_KEY, JSON.stringify(config))
  } catch (e) {
    console.warn('[AgentView] Failed to persist audit config:', e)
  }
}

const loadAuditConfigFromLocal = (): AuditConfig => {
  try {
    const raw = localStorage.getItem(AUDIT_CONFIG_STORAGE_KEY)
    if (!raw) return defaultAuditConfig()
    const parsed = JSON.parse(raw)
    return normalizeAuditConfig(parsed)
  } catch (e) {
    console.warn('[AgentView] Failed to load audit config, using defaults:', e)
    return defaultAuditConfig()
  }
}

const ASSISTANT_MODEL_STORAGE_KEY = 'sentinel:agent:assistant-model'
let unlistenAiConfigUpdated: UnlistenFn | null = null

const normalizeProviderName = (provider: string) => {
  const lower = provider.toLowerCase()
  const names: Record<string, string> = {
    openai: 'OpenAI',
    anthropic: 'Anthropic',
    gemini: 'Gemini',
    deepseek: 'DeepSeek',
    moonshot: 'Moonshot',
    ollama: 'Ollama',
    openrouter: 'OpenRouter',
    modelscope: 'ModelScope',
    groq: 'Groq',
    perplexity: 'Perplexity',
    togetherai: 'TogetherAI',
    xai: 'xAI',
    cohere: 'Cohere',
    'lm studio': 'LM Studio',
    lmstudio: 'LM Studio',
    lm_studio: 'LM Studio',
  }
  return names[lower] || provider
}

const extractModelId = (item: any): string => {
  if (!item) return ''
  if (typeof item === 'string') return item
  if (typeof item.id === 'string') return item.id
  if (typeof item.name === 'string') return item.name
  return ''
}

const loadAssistantModelOptions = async () => {
  isLoadingAssistantModels.value = true
  try {
    const aiConfig = await invoke<any>('get_ai_config')
    const providers = aiConfig?.providers && typeof aiConfig.providers === 'object'
      ? aiConfig.providers
      : {}
    const defaultModel = typeof aiConfig?.default_llm_model === 'string' ? aiConfig.default_llm_model : ''
    const options: AssistantModelOption[] = []

    Object.entries(providers).forEach(([providerKey, providerValue]) => {
      const cfg = providerValue as any
      if (cfg?.enabled === false) return

      const providerRaw = String(cfg?.provider || providerKey).trim()
      const provider = providerRaw.toLowerCase()
      if (!provider) return

      const modelsRaw = Array.isArray(cfg?.models) ? cfg.models : []
      const modelIds = modelsRaw.map(extractModelId).filter((v: string) => !!v)
      if (typeof cfg?.default_model === 'string' && cfg.default_model.trim()) {
        const providerDefaultModel = cfg.default_model.trim()
        if (!modelIds.some((id) => id === providerDefaultModel)) {
          modelIds.push(providerDefaultModel)
        }
      }

      Array.from(new Set<string>(modelIds)).forEach((modelId: string) => {
        options.push({
          value: `${provider}/${modelId}`,
          label: modelId,
          description: normalizeProviderName(providerRaw),
        })
      })
    })

    options.sort((a, b) => a.label.localeCompare(b.label))

    if (defaultModel && defaultModel.includes('/')) {
      const [defaultProvider, ...defaultModelParts] = defaultModel.split('/')
      const defaultModelName = defaultModelParts.join('/')
      const providerLower = defaultProvider.toLowerCase()
      const key = `${providerLower}/${defaultModelName}`
      if (
        !options.some((item) => item.value.toLowerCase() === key.toLowerCase()) &&
        providerLower &&
        defaultModelName
      ) {
        options.unshift({
          value: key,
          label: defaultModelName,
          description: normalizeProviderName(providerLower),
        })
      }
    }

    assistantModelOptions.value = options

    let stored = ''
    try {
      stored = localStorage.getItem(ASSISTANT_MODEL_STORAGE_KEY) || ''
    } catch {
      stored = ''
    }

    const preferred = stored || assistantSelectedModel.value || defaultModel
    if (preferred && options.some((item) => item.value === preferred)) {
      assistantSelectedModel.value = preferred
    } else if (defaultModel && options.some((item) => item.value === defaultModel)) {
      assistantSelectedModel.value = defaultModel
    } else if (options.length > 0) {
      assistantSelectedModel.value = options[0].value
    } else {
      assistantSelectedModel.value = ''
    }
  } catch (e) {
    console.warn('[AgentView] Failed to load assistant model options:', e)
    assistantModelOptions.value = []
  } finally {
    isLoadingAssistantModels.value = false
  }
}

const handleAssistantModelChange = (value: string) => {
  assistantSelectedModel.value = value
  try {
    if (value) {
      localStorage.setItem(ASSISTANT_MODEL_STORAGE_KEY, value)
    } else {
      localStorage.removeItem(ASSISTANT_MODEL_STORAGE_KEY)
    }
  } catch {
    // ignore storage errors
  }
}

const buildEffectiveToolConfigForExecution = () => {
  const baseConfig = {
    enabled: toolConfig.value.enabled,
    selection_strategy: toolConfig.value.selection_strategy,
    max_tools: toolConfig.value.max_tools,
    fixed_tools: [...toolConfig.value.fixed_tools],
    disabled_tools: [...toolConfig.value.disabled_tools],
  }

  if (!toolConfig.value.audit_mode) {
    return baseConfig
  }

  const requiredTools = (toolConfig.value.audit_config?.required_tools || [])
    .filter((item) => typeof item === 'string' && item.trim().length > 0)

  const fixedSet = new Set([...baseConfig.fixed_tools, ...requiredTools])
  const disabledSet = new Set(baseConfig.disabled_tools.filter((item) => !fixedSet.has(item)))

  return {
    ...baseConfig,
    fixed_tools: [...fixedSet],
    disabled_tools: [...disabledSet],
  }
}

// Agent events
const agentEvents = useAgentEvents(computed(() => conversationId.value || ''))
const messages = computed(() => agentEvents.messages.value)
const isExecuting = computed(() => agentEvents.isExecuting.value)
const isStreaming = computed(() => agentEvents.isExecuting.value && !!agentEvents.streamingContent.value)
const streamingContent = computed(() => agentEvents.streamingContent.value)
const contextUsage = computed(() => agentEvents.contextUsage.value)

type SubagentRunRecord = {
  id: string
  parent_execution_id: string
  role?: string | null
  task: string
  status: 'running' | 'queued' | 'completed' | 'failed'
  output?: string | null
  error?: string | null
  started_at: string
  completed_at?: string | null
  created_at: string
  updated_at: string
}

const loadSubagentRuns = async (parentExecutionId: string) => {
  try {
    const runs = await invoke<SubagentRunRecord[]>('get_subagent_runs', {
      parentExecutionId,
    })

    const toMillis = (v: any) => {
      const ms = new Date(v).getTime()
      return Number.isFinite(ms) ? ms : undefined
    }

    const mapped = (runs || []).map(r => {
      const startedAt = toMillis(r.started_at)
      const completedAt = toMillis(r.completed_at)
      const duration = startedAt !== undefined && completedAt !== undefined
        ? Math.max(0, completedAt - startedAt)
        : undefined

      const summary = (r.output || '').trim()
      return {
        id: r.id,
        parentId: r.parent_execution_id,
        role: r.role || undefined,
        status: r.status,
        progress: r.status === 'running' || r.status === 'queued' ? 0 : 100,
        task: r.task,
        summary: summary.length > 0 ? summary.slice(0, 200) : undefined,
        error: r.error || undefined,
        startedAt,
        duration,
      }
    })

    // Merge by id (do not drop live in-memory updates)
    const existing = agentEvents.subagents.value
    const byId = new Map<string, any>()
    existing.forEach(s => byId.set(s.id, s))
    mapped.forEach(s => {
      const prev = byId.get(s.id)
      byId.set(s.id, prev ? { ...s, ...prev } : s)
    })

    // Prefer newest first (startedAt desc), fallback by id
    agentEvents.subagents.value = [...byId.values()].sort((a: any, b: any) => {
      const at = a.startedAt ?? 0
      const bt = b.startedAt ?? 0
      if (bt !== at) return bt - at
      return String(b.id).localeCompare(String(a.id))
    })
  } catch (e) {
    console.error('[AgentView] Failed to load subagent runs:', e)
    // Keep existing in-memory list if any
  }
}


// Web Explorer Events
// Important: pass through the nullable execution id ref so Web Explorer can
// receive early events (start/plan/progress) and then bind itself to the session.
const webExplorerEvents = useWebExplorerEvents(agentEvents.currentExecutionId)
const isWebExplorerActive = computed(() => webExplorerEvents.isVisionActive.value)

// Todos management
const todosComposable = useTodos(computed(() => conversationId.value || ''))
const todos = computed(() => todosComposable.todos.value)
const hasTodos = computed(() => props.showTodos && todosComposable.hasTodos.value)
const isTodosPanelActive = computed(() => todosComposable.isTodosPanelActive.value)

// HTML panel - user manually triggers rendering
const isHtmlPanelActive = ref(false)
const htmlPanelContent = ref('')
const isAuditFindingsPanelActive = ref(false)
const lastPersistedPolicyGateSignature = ref('')

// Handle render HTML from code block
const handleRenderHtml = (htmlContent: string) => {
  htmlPanelContent.value = htmlContent
  // Close other panels and open HTML panel
  webExplorerEvents.close()
  terminalComposable.closeTerminal()
  todosComposable.close()
  isAuditFindingsPanelActive.value = false
  isHtmlPanelActive.value = true
}

const hasHtmlPanelContent = computed(() => !!htmlPanelContent.value)

// Handle close todos panel
const handleCloseTodos = () => {
  todosComposable.close()
}

const handleCloseHtmlPanel = () => {
  isHtmlPanelActive.value = false
}

const handleCloseAuditFindings = () => {
  isAuditFindingsPanelActive.value = false
}

// Terminal management
const terminalComposable = useTerminal()
const isTerminalActive = computed(() => terminalComposable.isTerminalActive.value)
const hasTerminalHistory = computed(() => terminalComposable.hasHistory.value)

// Handle close terminal panel
const handleCloseTerminal = () => {
  terminalComposable.closeTerminal()
}

// Handle toggle panel functions - ensure only one panel is active at a time
const handleToggleWebExplorer = () => {
  if (isWebExplorerActive.value) {
    webExplorerEvents.close()
  } else {
    // Close other panels
    todosComposable.close()
    terminalComposable.closeTerminal()
    isHtmlPanelActive.value = false
    isAuditFindingsPanelActive.value = false
    webExplorerEvents.open()
  }
}

const handleToggleTodos = () => {
  if (isTodosPanelActive.value) {
    todosComposable.close()
  } else {
    // Close other panels
    webExplorerEvents.close()
    terminalComposable.closeTerminal()
    isHtmlPanelActive.value = false
    isAuditFindingsPanelActive.value = false
    todosComposable.open()
  }
}

const handleToggleHtmlPanel = () => {
  if (isHtmlPanelActive.value) {
    isHtmlPanelActive.value = false
  } else {
    webExplorerEvents.close()
    terminalComposable.closeTerminal()
    todosComposable.close()
    isAuditFindingsPanelActive.value = false
    isHtmlPanelActive.value = true
  }
}

const handleToggleAuditFindings = () => {
  if (isAuditFindingsPanelActive.value) {
    isAuditFindingsPanelActive.value = false
  } else {
    webExplorerEvents.close()
    terminalComposable.closeTerminal()
    todosComposable.close()
    isHtmlPanelActive.value = false
    isAuditFindingsPanelActive.value = true
  }
}

const handleToggleTerminal = () => {
  if (isTerminalActive.value) {
    terminalComposable.closeTerminal()
  } else {
    // Close other panels
    webExplorerEvents.close()
    todosComposable.close()
    isHtmlPanelActive.value = false
    isAuditFindingsPanelActive.value = false
    terminalComposable.openTerminal()
  }
}

const parseAuditFindingsFromText = (content: string): AuditFinding[] => {
  return parseAuditPayloadFromText(content).findings
}

const extractFilePaths = (text: string): string[] => {
  if (!text) return []
  const pathRegex = /(?:^|[\s"'`(])((?:[\w.-]+\/)+[\w.-]+\.[a-zA-Z0-9]+)(?=$|[\s"'`):,])/g
  const found: string[] = []
  let match: RegExpExecArray | null
  while ((match = pathRegex.exec(text)) !== null) {
    if (match[1]) found.push(match[1])
  }
  return Array.from(new Set(found))
}

const parseAuditPayloadFromText = (content: string): ParsedAuditPayload => {
  if (!content) return { findings: [] }
  const candidates: string[] = []
  const direct = content.trim()
  if (direct.startsWith('{') || direct.startsWith('[')) {
    candidates.push(direct)
  }
  const blockRegex = /```json\s*([\s\S]*?)\s*```/gi
  let match: RegExpExecArray | null
  while ((match = blockRegex.exec(content)) !== null) {
    if (match[1]) candidates.push(match[1].trim())
  }

  for (const raw of candidates) {
    try {
      const parsed = JSON.parse(raw)
      const findings = Array.isArray(parsed?.findings) ? parsed.findings : Array.isArray(parsed) ? parsed : null
      if (!findings) continue
      const normalizedFindings = findings
        .filter((item: any) => item && typeof item === 'object')
        .map((item: any, index: number) => ({
          id: String(item.id || `F-${index + 1}`),
          title: item.title ? String(item.title) : undefined,
          severity: item.severity ? String(item.severity) : undefined,
          severity_raw: item.severity_raw ? String(item.severity_raw) : (item.severity ? String(item.severity) : undefined),
          confidence: typeof item.confidence === 'number' ? item.confidence : undefined,
          files: Array.isArray(item.files) ? item.files.map((v: any) => String(v)) : undefined,
          fix: item.fix ? String(item.fix) : undefined,
          status: item.status ? String(item.status) : undefined,
          cwe: item.cwe ? String(item.cwe) : undefined,
          description: item.description ? String(item.description) : undefined,
          source: item.source && typeof item.source === 'object' ? item.source : undefined,
          sink: item.sink && typeof item.sink === 'object' ? item.sink : undefined,
          hits: Array.isArray(item.hits) ? item.hits.filter((v: any) => v && typeof v === 'object') : undefined,
          sources: Array.isArray(item.sources) ? item.sources.filter((v: any) => v && typeof v === 'object') : undefined,
          sinks: Array.isArray(item.sinks) ? item.sinks.filter((v: any) => v && typeof v === 'object') : undefined,
          source_sinks: Array.isArray(item.source_sinks) ? item.source_sinks.filter((v: any) => v && typeof v === 'object') : undefined,
          trace_path: Array.isArray(item.trace_path) ? item.trace_path.filter((v: any) => v && typeof v === 'object') : undefined,
          evidence: Array.isArray(item.evidence) ? item.evidence.map((v: any) => String(v)) : undefined,
        }))
      const rawGate = parsed?.policy_gate
      const policyGate = rawGate && typeof rawGate === 'object'
        ? {
            passed: !!rawGate.passed,
            reason: rawGate.reason ? String(rawGate.reason) : undefined,
          }
        : undefined
      return {
        findings: normalizedFindings,
        policyGate,
      }
    } catch {
      // Continue trying next candidate payload.
    }
  }

  // Fallback: parse markdown-style audit report sections (e.g. "1. SQL注入漏洞")
  const fallbackFindings: AuditFinding[] = []
  const sectionRegex = /^\s*(\d+)\.\s+([^\n]+)\n([\s\S]*?)(?=^\s*\d+\.\s+|\s*$)/gm
  let sectionMatch: RegExpExecArray | null
  while ((sectionMatch = sectionRegex.exec(content)) !== null) {
    const index = sectionMatch[1]
    const rawTitle = (sectionMatch[2] || '').trim()
    const body = sectionMatch[3] || ''
    const lowered = `${rawTitle}\n${body}`.toLowerCase()

    let severity: string | undefined
    if (/(critical|严重)/.test(lowered)) severity = 'critical'
    else if (/(high|高危)/.test(lowered)) severity = 'high'
    else if (/(medium|中危)/.test(lowered)) severity = 'medium'
    else if (/(low|低危)/.test(lowered)) severity = 'low'

    const rawLocations = Array.from(body.matchAll(/位置[:：]\s*([^\n]+)/g))
      .map((m) => (m[1] || '').trim())
      .filter((v) => !!v)
    const files = Array.from(
      new Set(
        rawLocations.flatMap((item) => {
          const extracted = extractFilePaths(item)
          return extracted.length > 0 ? extracted : [item]
        }),
      ),
    )

    const description =
      body.match(/风险[:：]\s*([^\n]+)/)?.[1]?.trim() ||
      body.match(/详情[:：]\s*([^\n]+)/)?.[1]?.trim() ||
      undefined
    const fix =
      body.match(/修复建议[:：]\s*([^\n]+)/)?.[1]?.trim() ||
      body.match(/建议修复[:：]\s*([^\n]+)/)?.[1]?.trim() ||
      undefined

    fallbackFindings.push({
      id: `F-${index}`,
      title: rawTitle || `Finding ${index}`,
      severity,
      severity_raw: severity,
      files: files.length > 0 ? files : undefined,
      description,
      fix,
      evidence: description ? [description] : undefined,
    })
  }

  if (fallbackFindings.length > 0) {
    return { findings: fallbackFindings }
  }

  return { findings: [] }
}

const auditFindings = computed<AuditFinding[]>(() => {
  if (!toolConfig.value.audit_mode) return []
  const findings: AuditFinding[] = []
  for (const message of messages.value) {
    if (message.type !== 'final') continue
    const parsed = parseAuditFindingsFromText(message.content || '')
    if (parsed.length > 0) {
      findings.splice(0, findings.length, ...parsed)
    }
  }
  return findings
})

const evaluatePolicyGate = (
  findings: AuditFinding[],
  profile: PolicyProfile,
): PolicyGateResult => {
  const active = findings.filter((item) => {
    const status = (item.status || '').toLowerCase()
    return !['rejected', 'false_positive', 'fixed'].includes(status)
  })
  const critical = active.filter((item) => (item.severity || '').toLowerCase() === 'critical').length
  const high = active.filter((item) => (item.severity || '').toLowerCase() === 'high').length

  if (profile === 'prod_strict') {
    const blocked = critical + high > 0
    return {
      passed: !blocked,
      reason: blocked
        ? `Blocked by prod_strict policy: critical=${critical}, high=${high}`
        : `Passed prod_strict policy: no active high/critical findings`,
    }
  }

  const blocked = critical > 0
  return {
    passed: !blocked,
    reason: blocked
      ? `Blocked by balanced policy: critical=${critical}`
      : `Passed balanced policy: no active critical findings`,
  }
}

const auditPolicyGate = computed<PolicyGateResult | null>(() => {
  if (!toolConfig.value.audit_mode) return null
  let parsedPolicyGate: PolicyGateResult | undefined
  for (const message of messages.value) {
    if (message.type !== 'final') continue
    const payload = parseAuditPayloadFromText(message.content || '')
    if (payload.findings.length > 0 && payload.policyGate) {
      parsedPolicyGate = payload.policyGate
    }
  }
  if (parsedPolicyGate) return parsedPolicyGate
  const profile = toolConfig.value.audit_config?.policy_profile || 'balanced'
  return evaluatePolicyGate(auditFindings.value, profile)
})

const persistAuditPolicyGate = async (gate: PolicyGateResult | null) => {
  if (!conversationId.value || !gate || !toolConfig.value.audit_mode) return
  const active = auditFindings.value.filter((item) => {
    const status = (item.status || '').toLowerCase()
    return !['rejected', 'false_positive', 'fixed'].includes(status)
  })
  const summary = {
    total: auditFindings.value.length,
    active: active.length,
    critical: active.filter((item) => (item.severity || '').toLowerCase() === 'critical').length,
    high: active.filter((item) => (item.severity || '').toLowerCase() === 'high').length,
    medium: active.filter((item) => (item.severity || '').toLowerCase() === 'medium').length,
    low: active.filter((item) => (item.severity || '').toLowerCase() === 'low').length,
  }
  const profile = toolConfig.value.audit_config?.policy_profile || 'balanced'
  const signature = JSON.stringify({
    conversationId: conversationId.value,
    passed: gate.passed,
    reason: gate.reason,
    profile,
    summary,
  })
  if (signature === lastPersistedPolicyGateSignature.value) {
    return
  }

  try {
    await invoke('save_audit_policy_gate', {
      conversationId: conversationId.value,
      gate: {
        passed: gate.passed,
        reason: gate.reason,
        profile,
        summary,
      },
    })
    lastPersistedPolicyGateSignature.value = signature
  } catch (e) {
    console.warn('[AgentView] Persist policy gate failed:', e)
  }
}

// Sidebar resize
const SIDEBAR_MIN_WIDTH = 300
const SIDEBAR_MAX_WIDTH = 800
const SIDEBAR_DEFAULT_WIDTH = 350
const sidebarWidth = ref(SIDEBAR_DEFAULT_WIDTH)
const isResizing = ref(false)

// Load saved sidebar width from localStorage
const loadSidebarWidth = () => {
  try {
    const saved = localStorage.getItem('sentinel:sidebar:width')
    if (saved) {
      const width = parseInt(saved, 10)
      if (width >= SIDEBAR_MIN_WIDTH && width <= SIDEBAR_MAX_WIDTH) {
        sidebarWidth.value = width
      }
    }
  } catch (e) {
    console.warn('[AgentView] Failed to load sidebar width:', e)
  }
}

// Save sidebar width to localStorage
const saveSidebarWidth = (width: number) => {
  try {
    localStorage.setItem('sentinel:sidebar:width', width.toString())
  } catch (e) {
    console.warn('[AgentView] Failed to save sidebar width:', e)
  }
}

const startResize = (e: MouseEvent) => {
  e.preventDefault()
  isResizing.value = true
  const startX = e.clientX
  const startWidth = sidebarWidth.value

  // Add resizing class to body to prevent text selection
  document.body.classList.add('resizing')
  document.body.style.cursor = 'col-resize'

  const onMouseMove = (moveEvent: MouseEvent) => {
    if (!isResizing.value) return
    
    // Calculate new width (dragging left decreases width, right increases)
    const delta = startX - moveEvent.clientX
    const newWidth = Math.max(SIDEBAR_MIN_WIDTH, Math.min(SIDEBAR_MAX_WIDTH, startWidth + delta))
    sidebarWidth.value = newWidth
  }

  const onMouseUp = () => {
    if (isResizing.value) {
      isResizing.value = false
      saveSidebarWidth(sidebarWidth.value)
    }
    
    // Remove resizing class from body
    document.body.classList.remove('resizing')
    document.body.style.cursor = ''
    
    document.removeEventListener('mousemove', onMouseMove)
    document.removeEventListener('mouseup', onMouseUp)
  }

  document.addEventListener('mousemove', onMouseMove)
  document.addEventListener('mouseup', onMouseUp)
}

// Combined error
const error = computed(() => localError.value || agentEvents.error.value)

// Clear error
const clearError = () => {
  localError.value = null
}

// Handle RAG toggle
const handleToggleRAG = (enabled: boolean) => {
  ragEnabled.value = enabled
  console.log('[AgentView] RAG:', enabled ? 'enabled' : 'disabled')
}

// Handle Tools toggle
const handleToggleTools = (enabled: boolean) => {
  toolsEnabled.value = enabled
  toolConfig.value.enabled = enabled
  console.log('[AgentView] Tools:', enabled ? 'enabled' : 'disabled')
}

const buildPersistableToolConfig = (config: UiToolConfigPayload) => ({
  enabled: config.enabled,
  selection_strategy: config.selection_strategy,
  max_tools: config.max_tools,
  fixed_tools: config.fixed_tools,
  disabled_tools: config.disabled_tools,
})

const TOOL_CONFIG_SAVE_DEBOUNCE_MS = 300
const lastPersistedToolConfigSignature = ref('')
let pendingToolConfigSignature = ''
let toolConfigSaveTimer: ReturnType<typeof setTimeout> | null = null

const schedulePersistToolConfig = (config: UiToolConfigPayload) => {
  const persistableToolConfig = buildPersistableToolConfig(config)
  const signature = JSON.stringify(persistableToolConfig)
  if (signature === lastPersistedToolConfigSignature.value || signature === pendingToolConfigSignature) {
    return
  }

  pendingToolConfigSignature = signature
  if (toolConfigSaveTimer) {
    clearTimeout(toolConfigSaveTimer)
  }

  toolConfigSaveTimer = setTimeout(async () => {
    try {
      await invoke('save_tool_config', {
        toolConfig: persistableToolConfig,
      })
      lastPersistedToolConfigSignature.value = signature
      console.log('[AgentView] Tool config saved globally')
    } catch (e) {
      console.error('[AgentView] Failed to save tool config:', e)
      localError.value = t('agent.failedToSaveToolConfig') + ': ' + e
    } finally {
      if (pendingToolConfigSignature === signature) {
        pendingToolConfigSignature = ''
      }
      toolConfigSaveTimer = null
    }
  }, TOOL_CONFIG_SAVE_DEBOUNCE_MS)
}

// Handle Tool Config update
const handleToolConfigUpdate = (config: UiToolConfigPayload) => {
  const normalizedAuditConfig = normalizeAuditConfig(config.audit_config)
  const auditModeEnabled = !!config.audit_mode
  const runtimeConfig = {
    ...config,
    audit_mode: auditModeEnabled,
    audit_config: normalizedAuditConfig,
  }
  toolConfig.value = runtimeConfig as any
  toolsEnabled.value = config.enabled
  console.log('[AgentView] Tool config updated:', config)

  saveAuditConfigToLocal({
    ...normalizedAuditConfig,
    enabled: auditModeEnabled,
  })

  // Save tool config to database (global config, not bound to conversation)
  // Audit fields are frontend-owned for now and are stored in localStorage.
  schedulePersistToolConfig(config)
}

// Handle attachments
const handleAddAttachments = async (filePaths: string[]) => {
  if (!filePaths || filePaths.length === 0) return
  
  try {
    const attachments = await invoke<any[]>('upload_multiple_images', { filePaths: filePaths })
    if (attachments && attachments.length > 0) {
      pendingAttachments.value.push(...attachments)
      console.log('[AgentView] Uploaded', attachments.length, 'attachments')
    }
  } catch (error) {
    console.error('[AgentView] Upload failed:', error)
  }
}

const handleRemoveAttachment = (index: number) => {
  if (index >= 0 && index < pendingAttachments.value.length) {
    pendingAttachments.value.splice(index, 1)
  }
}

// Handle document attachments
const handleAddDocuments = (docs: import('@/types/agent').PendingDocumentAttachment[]) => {
  pendingDocuments.value.push(...docs)
  console.log('[AgentView] Added', docs.length, 'document(s) for processing')
}

const handleRemoveDocument = (index: number) => {
  if (index >= 0 && index < pendingDocuments.value.length) {
    const removed = pendingDocuments.value.splice(index, 1)
    // Also remove from processed list
    if (removed[0]) {
      const idx = processedDocuments.value.findIndex(d => d.id === removed[0].id)
      if (idx >= 0) {
        processedDocuments.value.splice(idx, 1)
      }
    }
  }
}

const handleDocumentProcessed = (result: import('@/types/agent').ProcessedDocumentResult) => {
  // Update processed documents list
  const existingIdx = processedDocuments.value.findIndex(d => d.id === result.id)
  if (existingIdx >= 0) {
    processedDocuments.value[existingIdx] = result
  } else {
    processedDocuments.value.push(result)
  }
  
  // Update pending document's mode
  const pendingIdx = pendingDocuments.value.findIndex(d => d.id === result.id)
  if (pendingIdx >= 0) {
    pendingDocuments.value[pendingIdx].status = result.status
    pendingDocuments.value[pendingIdx].file_id = result.file_id
    pendingDocuments.value[pendingIdx].file_path = result.file_path
    pendingDocuments.value[pendingIdx].error_message = result.error_message
  }

  console.log('[AgentView] Document processed:', result.original_filename, result.file_id)
}

const parseMetadataArray = (value: unknown): any[] => {
  if (Array.isArray(value)) return value
  if (typeof value === 'string') {
    try {
      const parsed = JSON.parse(value)
      return Array.isArray(parsed) ? parsed : []
    } catch {
      return []
    }
  }
  return []
}

const restoreAttachmentsFromMessage = (message: AgentMessage) => {
  // Always reset first so edited/resend message uses only source message attachments.
  pendingAttachments.value = []
  pendingDocuments.value = []
  processedDocuments.value = []

  const metadata = (message.metadata as any) || {}

  // Restore image attachments for resend/edit send path.
  const imageAttachments = parseMetadataArray(metadata.image_attachments)
  if (imageAttachments.length > 0) {
    pendingAttachments.value = [...imageAttachments]
  }

  // Restore ready document attachments for resend/edit send path.
  const documentAttachments = parseMetadataArray(metadata.document_attachments)
  if (documentAttachments.length > 0) {
    const normalized = documentAttachments.map((doc: any) => {
      const status = doc?.status === 'failed' || doc?.status === 'processing' || doc?.status === 'pending'
        ? doc.status
        : 'ready'
      return {
        ...doc,
        status,
      } as ProcessedDocumentResult
    })

    processedDocuments.value = normalized
    pendingDocuments.value = normalized.map((doc) => ({
      id: doc.id,
      file_id: doc.file_id,
      original_path: doc.file_path || doc.original_filename || '',
      original_filename: doc.original_filename,
      file_size: doc.file_size,
      mime_type: doc.mime_type,
      status: doc.status,
      file_path: doc.file_path,
      error_message: doc.error_message,
    }))
  }
}

// Handle traffic references
const handleRemoveTraffic = (index: number) => {
  if (index >= 0 && index < referencedTraffic.value.length) {
    referencedTraffic.value.splice(index, 1)
  }
}

const handleClearTraffic = () => {
  referencedTraffic.value = []
}

// Add traffic references (for external use)
const addReferencedTraffic = (traffic: ReferencedTraffic[], type: TrafficSendType = 'both') => {
  const existingIds = new Set(referencedTraffic.value.map(t => t.id))
  const newTraffic = traffic
    .filter(t => !existingIds.has(t.id))
    .map(t => ({ ...t, sendType: type }))
  referencedTraffic.value.push(...newTraffic)
}

// Build traffic context for prompt
const buildTrafficContext = (traffic: ReferencedTraffic[]): string => {
  const parts: string[] = ['Referenced HTTP traffic:\n']
  
  traffic.forEach((t, index) => {
    const sendType = t.sendType || 'both'
    const typeLabel = sendType === 'request' ? 'Request' : sendType === 'response' ? 'Response' : 'Traffic'
    parts.push(`\n--- ${typeLabel} #${index + 1} ---`)
    parts.push(`URL: ${t.url}`)
    parts.push(`Method: ${t.method}`)
    parts.push(`Host: ${t.host}`)
    
    const showRequest = sendType === 'request' || sendType === 'both'
    const showResponse = sendType === 'response' || sendType === 'both'
    
    if (showResponse) {
      parts.push(`Status: ${t.status_code || 'N/A'}`)
    }
    
    if (showRequest && t.request_headers) {
      try {
        const headers = JSON.parse(t.request_headers)
        const headerStr = Object.entries(headers).map(([k, v]) => `  ${k}: ${v}`).join('\n')
        parts.push(`\nRequest Headers:\n${headerStr}`)
      } catch {
        parts.push(`\nRequest Headers: ${t.request_headers}`)
      }
    }
    
    if (showRequest && t.request_body) {
      const body = t.request_body.length > 2000 
        ? t.request_body.substring(0, 2000) + '... [truncated]'
        : t.request_body
      parts.push(`\nRequest Body:\n${body}`)
    }
    
    if (showResponse && t.response_headers) {
      try {
        const headers = JSON.parse(t.response_headers)
        const headerStr = Object.entries(headers).map(([k, v]) => `  ${k}: ${v}`).join('\n')
        parts.push(`\nResponse Headers:\n${headerStr}`)
      } catch {
        parts.push(`\nResponse Headers: ${t.response_headers}`)
      }
    }
    
    if (showResponse && t.response_body) {
      const body = t.response_body.length > 3000 
        ? t.response_body.substring(0, 3000) + '... [truncated]'
        : t.response_body
      parts.push(`\nResponse Body:\n${body}`)
    }
  })
  
  return parts.join('\n')
}

// Handle clear conversation
const handleClearConversation = async () => {
  if (!conversationId.value) {
    console.log('[AgentView] No conversation to clear')
    return
  }

  try {
    console.log('[AgentView] Clearing conversation:', conversationId.value)
    
    // Call backend to clear conversation messages
    await invoke('clear_conversation_messages', {
      conversationId: conversationId.value
    })
    
    // Clear frontend messages
    agentEvents.clearMessages()
    
    // Clear attachments and references
    pendingAttachments.value = []
    referencedTraffic.value = []
    inputValue.value = ''
    
    // Refresh conversation list (update message count)
    conversationListRef.value?.loadConversations()
    
    console.log('[AgentView] Conversation cleared successfully')
  } catch (e) {
    console.error('[AgentView] Failed to clear conversation:', e)
    localError.value = t('agent.failedToClearConversation') + ': ' + e
  }
}

// Handle view subagent details - open modal to show details
const handleViewSubagentDetails = (subagentId: string) => {
  console.log('[AgentView] View subagent details:', subagentId)
  const subagent = subagents.value.find(s => s.id === subagentId)
  if (subagent) {
    selectedSubagent.value = subagent
    showSubagentDetailModal.value = true
  }
}

// Handle stop
const handleStop = async () => {
  console.log('[AgentView] Stop requested for conversation:', conversationId.value)
  
  if (!conversationId.value) {
    console.warn('[AgentView] No conversation ID to stop')
    return
  }
  
  try {
    // Call backend to cancel command
    await invoke('cancel_ai_stream', {
      conversationId: conversationId.value
    })
    
    console.log('[AgentView] Stop command sent successfully')
    
    // Notify useAgentEvents to stop execution status
    agentEvents.stopExecution()
    
    // Also stop Web Explorer if it's running
    if (webExplorerEvents.isVisionActive.value) {
      console.log('[AgentView] Stopping Web Explorer')
      webExplorerEvents.stop()
    }
    
  } catch (e) {
    console.error('[AgentView] Failed to stop execution:', e)
    localError.value = t('agent.failedToStopExecution') + ': ' + e
  }
}

// Handle resend message - resend user message, delete all messages after it
const handleResendMessage = async (message: AgentMessage) => {
  if (isExecuting.value) {
    console.log('[AgentView] Cannot resend while executing')
    return
  }

  console.log('[AgentView] Resending message:', message.id, message.content)
  
  // Find the position of the message in the list
  const messageIndex = messages.value.findIndex(m => m.id === message.id)
  if (messageIndex === -1) {
    console.error('[AgentView] Message not found')
    return
  }

  // Get the timestamp of the message for filtering subagents
  const messageTimestamp = message.timestamp || Date.now()

  // Delete this message and all messages after it from database
  if (conversationId.value) {
    try {
      // First delete all messages after this message
      const deletedCount = await invoke<number>('delete_ai_messages_after', {
        conversationId: conversationId.value,
        messageId: message.id
      })
      console.log(`[AgentView] Deleted ${deletedCount} messages after the original message from database`)
      
      // Then delete the message itself from database
      await invoke('delete_ai_message', {
        messageId: message.id
      })
      console.log(`[AgentView] Deleted the original message from database`)

      // Delete subagent runs that started after this message
      const deletedSubagents = await invoke<number>('delete_subagent_runs_after', {
        parentExecutionId: conversationId.value,
        afterTimestampMs: messageTimestamp
      })
      console.log(`[AgentView] Deleted ${deletedSubagents} subagent runs from database`)
    } catch (e) {
      console.error('[AgentView] Failed to delete messages from database:', e)
      // Continue anyway to update frontend
    }
  }

  // Delete this message and all messages after it from frontend
  const messagesToKeep = messages.value.slice(0, messageIndex)
  agentEvents.messages.value = messagesToKeep

  // Remove subagents that started after this message from frontend
  agentEvents.subagents.value = agentEvents.subagents.value.filter(s => {
    return !s.startedAt || s.startedAt <= messageTimestamp
  })

  // Restore attachments from original message so resend keeps images/documents.
  restoreAttachmentsFromMessage(message)

  // Clear todos before resending
  todosComposable.clearTodos()

  // Set user message content to input box
  inputValue.value = message.content

  // Auto trigger send
  await handleSubmit()
}

// Handle edit message - edit user message and resend
const handleEditMessage = async (message: AgentMessage, newContent: string) => {
  if (isExecuting.value) {
    console.log('[AgentView] Cannot edit while executing')
    return
  }

  console.log('[AgentView] Editing message:', message.id, 'new content:', newContent)
  
  // Find the position of the message in the list
  const messageIndex = messages.value.findIndex(m => m.id === message.id)
  if (messageIndex === -1) {
    console.error('[AgentView] Message not found')
    return
  }

  // Get the timestamp of the message for filtering subagents
  const messageTimestamp = message.timestamp || Date.now()

  // Delete this message and all messages after it from database
  if (conversationId.value) {
    try {
      // First delete all messages after this message
      const deletedCount = await invoke<number>('delete_ai_messages_after', {
        conversationId: conversationId.value,
        messageId: message.id
      })
      console.log(`[AgentView] Deleted ${deletedCount} messages after the edited message from database`)
      
      // Then delete the message itself from database
      await invoke('delete_ai_message', {
        messageId: message.id
      })
      console.log(`[AgentView] Deleted the edited message from database`)

      // Delete subagent runs that started after this message
      const deletedSubagents = await invoke<number>('delete_subagent_runs_after', {
        parentExecutionId: conversationId.value,
        afterTimestampMs: messageTimestamp
      })
      console.log(`[AgentView] Deleted ${deletedSubagents} subagent runs from database`)
    } catch (e) {
      console.error('[AgentView] Failed to delete messages from database:', e)
      // Continue anyway to update frontend
    }
  }

  // Delete this message and all messages after it from frontend
  const messagesToKeep = messages.value.slice(0, messageIndex)
  agentEvents.messages.value = messagesToKeep

  // Remove subagents that started after this message from frontend
  agentEvents.subagents.value = agentEvents.subagents.value.filter(s => {
    return !s.startedAt || s.startedAt <= messageTimestamp
  })

  // Restore attachments from original message so edit+resend keeps images/documents.
  restoreAttachmentsFromMessage(message)

  // Clear todos before resending edited message
  todosComposable.clearTodos()

  // Set edited content to input box
  inputValue.value = newContent

  // Auto trigger send with edited content
  await handleSubmit()
}

// Load conversation history
const loadConversationHistory = async (convId: string) => {
  console.log('[AgentView] Loading conversation history for:', convId)
  try {
    const messages = await invoke<any[]>('get_ai_messages_by_conversation', {
      conversationId: convId
    })
    
    console.log('[AgentView] Received messages:', messages)
    
    // Clear current messages
    agentEvents.clearMessages()
    // Restore subagent list from persistent storage
    await loadSubagentRuns(convId)

    if (messages && messages.length > 0) {
      // DB already returns messages ordered by timestamp ASC.
      // Keep DB order directly to avoid frontend re-sorting instability
      // when multiple rows share the same millisecond timestamp.
      const timeline: AgentMessage[] = []

      const toMillis = (v: any) => {
        const ms = new Date(v).getTime()
        return Number.isFinite(ms) ? ms : Date.now()
      }

      // Collect tool_call_ids from role=tool messages to avoid duplicates
      const existingToolCallIds = new Set<string>()
      messages.forEach((row: any) => {
        if (row.role === 'tool') {
          // The row.id is the tool_call_id for tool messages
          existingToolCallIds.add(row.id)
          // Also check metadata for tool_call_id
          try {
            const meta = row.metadata && typeof row.metadata === 'string' ? JSON.parse(row.metadata) : row.metadata
            if (meta?.tool_call_id) {
              existingToolCallIds.add(meta.tool_call_id)
            }
          } catch { /* ignore */ }
        }
      })

      messages.forEach((row: any) => {
        const parsedMetadata =
          row.metadata && typeof row.metadata === 'string' ? JSON.parse(row.metadata) : row.metadata
        const parsedStructured =
          row.structured_data && typeof row.structured_data === 'string'
            ? JSON.parse(row.structured_data)
            : row.structured_data
        const ts = toMillis(row.timestamp)

        if (row.role === 'tool') {
          // Persisted tool event message
          const kind = parsedMetadata?.kind
          const type = kind === 'tool_result' ? 'tool_result' : 'tool_call'
          timeline.push({
            id: row.id,
            type: type as any,
            content: row.content || '',
            timestamp: ts,
            metadata: parsedMetadata,
          })
          return
        }

        if (row.role === 'system') {
          if (parsedMetadata?.kind === 'skill_loaded') {
            const tools = Array.isArray(parsedMetadata?.tools) ? parsedMetadata.tools : []
            const toolsPreview = parsedMetadata?.tools_preview || (() => {
              const preview = tools.slice(0, 6).join(', ')
              const suffix = tools.length > 6 ? ` +${tools.length - 6}` : ''
              return `${preview}${suffix}`.trim()
            })()
            const content = row.content || `Skill loaded: ${parsedMetadata?.skill_name || 'unknown'} (${parsedMetadata?.skill_id || 'unknown'})`
            timeline.push({
              id: row.id,
              type: 'system' as any,
              content,
              timestamp: ts,
              metadata: {
                ...parsedMetadata,
                tools,
                tools_preview: toolsPreview,
              },
            })
            return
          }
          // System message (e.g., history summarized)
          timeline.push({
            id: row.id,
            type: 'system' as any,
            content: row.content || '',
            timestamp: ts,
            metadata: parsedMetadata,
          })
          return
        }

        // Legacy fallback: assistant rows may contain tool_calls (older data).
        // Skip if the tool_call already exists as a standalone role=tool message.
        if (row.role === 'assistant' && row.tool_calls) {
          try {
            const toolCalls = typeof row.tool_calls === 'string' ? JSON.parse(row.tool_calls) : row.tool_calls
            if (Array.isArray(toolCalls) && toolCalls.length > 0) {
              toolCalls.forEach((tc: any, i: number) => {
                // Skip if this tool_call already exists as a standalone message
                if (tc.id && existingToolCallIds.has(tc.id)) {
                  return
                }
                let parsedArgs: any = {}
                try {
                  parsedArgs = typeof tc.arguments === 'string' ? JSON.parse(tc.arguments) : (tc.arguments ?? {})
                } catch {
                  parsedArgs = { raw: tc.arguments }
                }
                timeline.push({
                  id: `toolcall:${row.id}:${tc.id || i}`,
                  type: 'tool_call' as any,
                  content: `${t('agent.toolCallCompleted')}: ${tc.name || 'unknown'}`,
                  timestamp: ts,
                  metadata: {
                    tool_name: tc.name,
                    tool_args: parsedArgs,
                    tool_result: tc.result,
                    tool_call_id: tc.id,
                    status: 'completed',
                    success: tc.success !== false,
                  },
                })
              })
            }
          } catch (e) {
            console.warn('[AgentView] Failed to parse legacy tool_calls:', e)
          }
        }

        const messageType = row.role === 'user' ? 'user' : 'final'
        const displayContent =
          row.role === 'user' && parsedStructured?.display_content
            ? parsedStructured.display_content
            : row.content

        // Extract document_attachments from metadata or structured_data
        let finalMetadata = parsedMetadata || {}
        if (row.role === 'user') {
          // Try to get document_attachments from metadata first, then structured_data
          const docAttachments = parsedMetadata?.document_attachments || parsedStructured?.document_attachments
          if (docAttachments && Array.isArray(docAttachments) && docAttachments.length > 0) {
            finalMetadata = { ...finalMetadata, document_attachments: docAttachments }
          }
        }

        timeline.push({
          id: row.id,
          type: messageType as any,
          content: displayContent,
          timestamp: ts,
          metadata: finalMetadata,
        })
      })

      agentEvents.messages.value = timeline

      console.log('[AgentView] Loaded', messages.length, 'messages from conversation:', convId)
      
      // Scroll to bottom after loading messages
      nextTick(() => {
        messageFlowRef.value?.scrollToBottom()
      })
    } else {
      console.log('[AgentView] No messages found for conversation:', convId)
    }
  } catch (e) {
    console.error('[AgentView] Failed to load conversation history:', e)
  }
}

// Handle conversation selection
const handleSelectConversation = async (convId: string) => {
  conversationId.value = convId
  await loadConversationHistory(convId)
  
  // Reset terminal session when switching conversations
  terminalComposable.resetTerminal()
  
  // Update conversation title
  try {
    const conversations = await invoke<any[]>('get_ai_conversations')
    const conv = conversations.find(c => c.id === convId)
    if (conv) {
      currentConversationTitle.value = conv.title || t('agent.unnamedConversation')
    }
  } catch (e) {
    console.error('[AgentView] Failed to get conversation title:', e)
  }
  
  // Auto close drawer after selecting conversation
  showConversations.value = false
}

// Handle conversation creation
const handleCreateConversation = async (newConvId?: string) => {
  if (newConvId) {
    conversationId.value = newConvId
    currentConversationTitle.value = t('agent.newConversationTitle')
    agentEvents.clearMessages()
    
    // Reset terminal session for new conversation
    terminalComposable.resetTerminal()
    
    // Focus input after creating conversation
    nextTick(() => {
      inputAreaRef.value?.focusInput()
    })
  } else {
    // Create new conversation
    try {
      const convId = await invoke<string>('create_ai_conversation', {
        request: {
          title: `${t('agent.newConversationTitle')} ${new Date().toLocaleString()}`,
          service_name: 'default'
        }
      })
      conversationId.value = convId
      currentConversationTitle.value = t('agent.newConversationTitle')
      agentEvents.clearMessages()
      
      // Reset terminal session for new conversation
      terminalComposable.resetTerminal()
      
      // Refresh conversation list
      conversationListRef.value?.loadConversations()
      
      // Focus input after creating conversation
      nextTick(() => {
        inputAreaRef.value?.focusInput()
      })
    } catch (e) {
      console.error('[AgentView] Failed to create conversation:', e)
    }
  }
}

// Handle submit
const handleSubmit = async () => {
  const task = inputValue.value.trim()
  if (!task) return
  
  localError.value = null
  
  // Takeover: if currently executing, cancel previous stream first
  if (isExecuting.value && conversationId.value) {
    console.log('[AgentView] Takeover: stopping current execution to handle new message')
    try {
      const partial = agentEvents.streamingContent.value?.trim()
      if (partial) {
        const partialMsgId = crypto.randomUUID()
        console.log('[AgentView] Takeover: saving partial response:', partial.substring(0, 100))
        // First solidify the partial output of the old stream locally as the final assistant message
        agentEvents.messages.value.push({
          id: partialMsgId,
          type: 'final' as any,
          content: partial,
          timestamp: Date.now(),
        })
        // Then write to database to ensure conversation history consistency
        await invoke('save_ai_message', {
          request: {
            id: partialMsgId,
            conversation_id: conversationId.value,
            role: 'assistant',
            content: partial,
          },
        })
      }
      
      // Stop the current execution
      await handleStop()
      
      // Wait a bit for the backend to fully stop
      await new Promise(resolve => setTimeout(resolve, 500))
      
      console.log('[AgentView] Takeover: previous execution stopped, proceeding with new message')
    } catch (e) {
      console.warn('[AgentView] Takeover stop failed, continuing:', e)
    }
  }

  // Build full task with traffic context
  let fullTask = task
  let displayContent: string | undefined = undefined
  if (referencedTraffic.value.length > 0) {
    const trafficContext = buildTrafficContext(referencedTraffic.value)
    fullTask = `${trafficContext}\n\nUser task: ${task}`
    // Display content is just the user's original input
    displayContent = task
  }
  
  // Clear input and references
  inputValue.value = ''
  const usedAttachments = [...pendingAttachments.value]
  const usedDocuments = processedDocuments.value.filter(d => d.status === 'ready')
  const usedTraffic = [...referencedTraffic.value]
  pendingAttachments.value = []
  pendingDocuments.value = []
  processedDocuments.value = []
  referencedTraffic.value = []
  
  // Store document attachments for later injection into user message
  // We'll inject them when the user_message event arrives from backend
  if (usedDocuments.length > 0) {
    agentEvents.setPendingDocumentAttachments(usedDocuments)
  }
  
  // Force scroll to bottom when user sends a message
  nextTick(() => {
    messageFlowRef.value?.scrollToBottom()
  })
  
  // Emit submit event
  emit('submit', fullTask)
  
  try {
    // Ensure conversation exists or create new one
    if (!conversationId.value) {
      console.log('[AgentView] No conversation ID, creating new conversation')
      const convId = await invoke<string>('create_ai_conversation', {
        request: {
          title: `${t('agent.newConversationTitle')} ${new Date().toLocaleString()}`,
          service_name: 'default'
        }
      })
      conversationId.value = convId
      currentConversationTitle.value = t('agent.newConversationTitle')
      console.log('[AgentView] Created new conversation:', convId)
      
      // Refresh conversation list
      conversationListRef.value?.loadConversations()
    }
    
    const modelOverrideValue = assistantSelectedModel.value && assistantSelectedModel.value.includes('/')
      ? assistantSelectedModel.value
      : undefined

    // Call agent_execute command (tool config passed directly from frontend to ensure latest config takes effect immediately)
    const result = await invoke('agent_execute', {
      task: fullTask,
      config: {
        max_iterations: 10,
        timeout_secs: 300,
        force_todos: props.showTodos,
        enable_rag: ragEnabled.value,
        enable_tenth_man_rule: tenthManEnabled.value,
        conversation_id: conversationId.value,
        message_id: null,
        attachments: usedAttachments.length > 0 ? usedAttachments : undefined,
        document_attachments: usedDocuments.length > 0 ? usedDocuments : undefined,
        tool_config: buildEffectiveToolConfigForExecution(),
        audit_config: toolConfig.value.audit_mode
          ? {
              ...(toolConfig.value.audit_config || defaultAuditConfig()),
              enabled: true,
            }
          : undefined,
        display_content: displayContent,
        model_override: modelOverrideValue,
      }
    })
    
    emit('complete', result)
  } catch (e: any) {
    const errorMsg = e.toString()
    localError.value = errorMsg
    emit('error', errorMsg)
  }
  // isExecuting and isStreaming are automatically managed by useAgentEvents, no manual setting needed
}

// Load tool config from database
const loadToolConfig = async () => {
  try {
    const savedConfig = await invoke<any>('get_tool_config')
    const localAuditConfig = loadAuditConfigFromLocal()
    if (savedConfig) {
      toolConfig.value = {
        ...savedConfig,
        audit_mode: localAuditConfig.enabled,
        audit_config: localAuditConfig,
      }
      toolsEnabled.value = savedConfig.enabled
      lastPersistedToolConfigSignature.value = JSON.stringify(
        buildPersistableToolConfig({
          ...(savedConfig as any),
          audit_mode: localAuditConfig.enabled,
          audit_config: localAuditConfig,
        }),
      )
      console.log('[AgentView] Loaded tool config from database:', savedConfig)
    } else {
      toolConfig.value.audit_mode = localAuditConfig.enabled
      toolConfig.value.audit_config = localAuditConfig
      console.log('[AgentView] No saved tool config found, using defaults')
    }
  } catch (e) {
    console.error('[AgentView] Failed to load tool config:', e)
  }
}

// Load latest conversation on startup
const loadLatestConversation = async () => {
  try {
    const conversations = await invoke<any[]>('get_ai_conversations')
    if (conversations && conversations.length > 0) {
      // Sort by update time in reverse order, take the latest conversation
      const sorted = conversations.sort((a, b) => 
        new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime()
      )
      const latest = sorted[0]
      conversationId.value = latest.id
      currentConversationTitle.value = latest.title || t('agent.unnamedConversation')
      await loadConversationHistory(latest.id)
      console.log('[AgentView] Loaded latest conversation:', latest.id)
    }
  } catch (e) {
    console.error('[AgentView] Failed to load latest conversation:', e)
  }
}

// Initialize
onMounted(async () => {
  console.log('[AgentView] Mounted with executionId:', props.executionId)
  await loadAssistantModelOptions()
  unlistenAiConfigUpdated = await listen('ai_config_updated', async () => {
    await loadAssistantModelOptions()
  })
  
  // Load saved tool configuration from database
  await loadToolConfig()
  
  // Load saved sidebar width
  loadSidebarWidth()
  
  // Load conversation history if executionId is provided
  if (props.executionId) {
    conversationId.value = props.executionId
    await loadConversationHistory(props.executionId)
  } else {
    // Default load the last conversation
    await loadLatestConversation()
  }
  
  // Preconnect terminal server in background (non-blocking)
  terminalComposable.preconnect()
  
  // 自动聚焦输入框
  nextTick(() => {
    inputAreaRef.value?.focusInput()
  })
})

onUnmounted(() => {
  if (unlistenAiConfigUpdated) {
    unlistenAiConfigUpdated()
    unlistenAiConfigUpdated = null
  }
})

// When component is activated (e.g., switching back from another page)
onActivated(() => {
  console.log('[AgentView] Activated, scrolling to bottom')
  // Scroll to bottom when returning to this page
  nextTick(() => {
    messageFlowRef.value?.scrollToBottom()
  })
})

// Watch for conversation changes to update title
watch(conversationId, async (newId) => {
  lastPersistedPolicyGateSignature.value = ''
  if (newId) {
    try {
      const conversations = await invoke<any[]>('get_ai_conversations')
      const conv = conversations.find(c => c.id === newId)
      if (conv) {
        currentConversationTitle.value = conv.title || t('agent.unnamedConversation')
      }
    } catch (e) {
      console.error('[AgentView] Failed to update conversation title:', e)
    }
  }
})

watch(auditPolicyGate, async (newGate) => {
  if (!toolConfig.value.audit_mode) return
  await persistAuditPolicyGate(newGate)
}, { deep: true })

// Update session title in manager
const { updateSessionTitle } = useAgentSessionManager()
watch(currentConversationTitle, (newTitle) => {
  if (conversationId.value && newTitle) {
    updateSessionTitle(conversationId.value, newTitle)
  }
})

// Expose methods
defineExpose({
  clearMessages: agentEvents.clearMessages,
  scrollToBottom: () => messageFlowRef.value?.scrollToBottom(),
  addReferencedTraffic,
  loadConversationHistory,
  conversationId,
  focusInput: () => inputAreaRef.value?.focusInput(),
})
</script>

<style scoped>
.agent-view {
  font-family: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
}

/* Conversation Drawer Styles */
.conversation-backdrop {
  animation: fadeIn 0.2s ease-out;
}

@keyframes fadeIn {
  from {
    opacity: 0;
  }
  to {
    opacity: 1;
  }
}

/* Drawer Slide Animation */
.slide-drawer-enter-active,
.slide-drawer-leave-active {
  transition: transform 0.3s cubic-bezier(0.4, 0, 0.2, 1);
}

.slide-drawer-enter-from {
  transform: translateX(-100%);
}

.slide-drawer-leave-to {
  transform: translateX(-100%);
}

/* Drawer Slide Animation Right */
.slide-drawer-right-enter-active,
.slide-drawer-right-leave-active {
  transition: transform 0.3s cubic-bezier(0.4, 0, 0.2, 1);
}

.slide-drawer-right-enter-from,
.slide-drawer-right-leave-to {
  transform: translateX(100%);
}

/* Resize handle */
.resize-handle {
  transition: background-color 0.2s;
}

.resize-handle:hover {
  width: 4px;
}

/* Prevent text selection during resize */
body.resizing {
  user-select: none;
  cursor: col-resize !important;
}

/* Message area container */
.message-area {
  display: flex;
  flex-direction: column;
}

/* Responsive */
@media (max-width: 768px) {
  .agent-main {
    flex-direction: column;
  }
  
  .todo-sidebar {
    width: 100%;
    border-left: none;
    border-top: 1px solid hsl(var(--b3));
    max-height: 200px;
  }

  .conversation-drawer {
    width: 85vw !important;
    max-width: 320px;
  }
  
  .sidebar-container {
    width: 100% !important;
    border-left: none;
    border-top: 1px solid hsl(var(--b3));
  }
  
  .resize-handle {
    display: none;
  }
}
</style>
