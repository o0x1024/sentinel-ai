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
          <span
            v-if="conversationExecutionState"
            class="badge badge-sm"
            :class="conversationExecutionStateBadgeClass"
          >
            {{ conversationExecutionStateBadgeText }}
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
          <!-- Todos Button - always visible -->
          <button 
            @click="handleToggleTodos()"
            class="btn btn-sm gap-1"
            :class="activeRightPanel === 'todos' ? 'btn-primary' : 'btn-ghost text-primary'"
            :title="activeRightPanel === 'todos' ? t('agent.todosPanelOpen') : t('agent.viewTodos')"
          >
            <i class="fas fa-tasks"></i>
            <span>{{ t('agent.todos') }}</span>
            <span v-if="todoBadgeCount > 0" class="badge badge-xs badge-primary">{{ todoBadgeCount }}</span>
          </button>
          <!-- HTML Panel Button - shows when there is HTML content -->
          <button 
            v-if="hasHtmlPanelContent"
            @click="handleToggleHtmlPanel()"
            class="btn btn-sm gap-1"
            :class="activeRightPanel === 'html' ? 'btn-primary' : 'btn-ghost text-primary'"
            :title="activeRightPanel === 'html' ? t('agent.htmlPanelOpen') : t('agent.viewHtmlPanel')"
          >
            <i class="fas fa-code"></i>
            <span>{{ t('agent.htmlPanel') }}</span>
          </button>
          <!-- Terminal Button - always visible -->
          <button 
            @click="handleToggleTerminal()"
            class="btn btn-sm gap-1"
            :class="activeRightPanel === 'terminal' ? 'btn-primary' : 'btn-ghost text-primary'"
            :title="activeRightPanel === 'terminal' ? t('agent.terminalPanelOpen') : t('agent.viewTerminal')"
          >
            <i class="fas fa-terminal"></i>
            <span>{{ t('agent.terminal') }}</span>
          </button>
          <button
            v-if="teamModeEnabled"
            @click="handleToggleTeamWorkspace()"
            class="btn btn-sm gap-1"
            :class="activeRightPanel === 'team' ? 'btn-primary' : 'btn-ghost text-primary'"
            title="Team 工作台"
          >
            <i class="fas fa-users"></i>
            <span>Team</span>
            <span v-if="teamWorkspaceBadgeCount > 0" class="badge badge-xs badge-primary">{{ teamWorkspaceBadgeCount }}</span>
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
            :messages="visibleMessages"
            :is-executing="isExecuting"
            :is-streaming="isStreaming"
            :streaming-content="streamingContent"
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
            :web-search-enabled="webSearchEnabled"
            :team-enabled="teamModeEnabled"
            :pending-attachments="pendingAttachments"
            :pending-documents="pendingDocuments"
            :processed-documents="processedDocuments"
            :referenced-traffic="referencedTraffic"
            :referenced-assets="referencedAssets"
            :context-usage="contextUsage"
            :default-max-context-tokens="assistantDefaultMaxContextTokens"
            :available-models="assistantModelOptions"
            :selected-model="assistantSelectedModel"
            :model-loading="isLoadingAssistantModels"
            @send-message="handleSubmit"
            @stop-execution="handleStop"
            @toggle-rag="handleToggleRAG"
            @toggle-tools="handleToggleTools"
            @toggle-web-search="handleToggleWebSearch"
            @toggle-team="handleToggleTeamMode"
            @change-model="handleAssistantModelChange"
            @add-attachments="handleAddAttachments"
            @remove-attachment="handleRemoveAttachment"
            @add-documents="handleAddDocuments"
            @remove-document="handleRemoveDocument"
            @document-processed="handleDocumentProcessed"
            @remove-traffic="handleRemoveTraffic"
            @clear-traffic="handleClearTraffic"
            @remove-asset="handleRemoveAsset"
            @clear-assets="handleClearAssets"
            @create-new-conversation="handleCreateConversation"
            @clear-conversation="handleClearConversation"
            @open-tool-config="showToolConfig = true"
          />
        </div>
        
        <!-- Right: Side Panel (Todo, HTML, Terminal, or Team) -->
        <div 
          v-if="activeRightPanel"
          class="sidebar-container flex-shrink-0 border-l border-base-300 flex flex-col overflow-hidden bg-base-100 relative"
          :style="{ width: sidebarWidth + 'px' }"
        >
            <!-- Resize Handle -->
            <div 
              class="resize-handle absolute left-0 top-0 bottom-0 w-1 cursor-col-resize hover:bg-primary/50 transition-colors z-10"
              @mousedown="startResize"
            ></div>
            
            <TeamWorkspacePanel
              v-if="activeRightPanel === 'team'"
              v-model:tab="teamWorkspaceTab"
              :loading="teamWorkspaceLoading"
              :tasks="teamTasks"
              :selected-task-id="selectedTeamTaskId"
              :selected-task-title="selectedTeamTaskTitle"
              :session-messages="teamSessionMessages"
              :blackboard-entries="teamBlackboardEntries"
              :session-detail="teamSessionDetail"
              :resolve-agent-name="resolveAgentName"
              @clear-selected-task="clearSelectedTeamTask"
              @toggle-selected-task="toggleSelectedTeamTask"
            />

            <TodoPanel 
              v-else-if="activeRightPanel === 'todos'" 
              :todos="todos"
              :is-active="activeRightPanel === 'todos'"
              :source-options="todoSourceOptions"
              :selected-source-key="selectedTodoSourceKey"
              class="h-full p-4 overflow-y-auto border-0 bg-transparent"
              @close="handleCloseTodos"
              @source-change="handleTodoSourceChange"
            />
            <HtmlPanel
              v-else-if="activeRightPanel === 'html'"
              :html-content="htmlPanelContent"
              :is-active="activeRightPanel === 'html'"
              class="h-full p-4 overflow-y-auto border-0 bg-transparent"
              @close="handleCloseHtmlPanel"
            />
            <InteractiveTerminal
              v-else-if="activeRightPanel === 'terminal'"
              class="h-full border-0 rounded-none bg-transparent"
              @close="handleCloseTerminal"
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
import { ref, computed, onMounted, onUnmounted, onActivated, watch, nextTick, type Ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type { AgentMessage } from '@/types/agent'
import type { Todo } from '@/types/todo'
import type {
  AgentTeamMessage,
  AgentTeamSession,
  AgentTeamStateChangedEvent,
  AgentTeamMessageStreamStartEvent,
  AgentTeamMessageStreamDeltaEvent,
  AgentTeamMessageStreamDoneEvent,
  AgentTeamToolCallEvent,
  AgentTeamToolResultEvent,
  TeamBlackboardEntry,
  TeamTask,
} from '@/types/agentTeam'
import { agentTeamApi } from '@/api/agentTeam'
import { useAgentEvents } from '@/composables/useAgentEvents'
import { useTodos } from '@/composables/useTodos'
import { useTerminal } from '@/composables/useTerminal'
import { useAgentSessionManager } from '@/composables/useAgentSessionManager'
import MessageFlow from './MessageFlow.vue'
import TodoPanel from './TodoPanel.vue'
import HtmlPanel from './HtmlPanel.vue'
import SubagentPanel from './SubagentPanel.vue'
import SubagentDetailModal from './SubagentDetailModal.vue'
import InteractiveTerminal from '@/components/Tools/InteractiveTerminal.vue'
import InputAreaComponent from '@/components/InputAreaComponent.vue'
import ConversationList from './ConversationList.vue'
import ToolConfigPanel from './ToolConfigPanel.vue'
import TeamWorkspacePanel from './TeamWorkspacePanel.vue'
import {
  type AgentExecutionFinishedEvent,
  getExecutionStateBadgeClass,
  getExecutionStateLabelKey,
  type PersistedAgentExecutionState,
} from './executionState'
import {
  buildOrchestrationPresetPlan,
  defaultOrchestrationPlan,
  generateTeamStepId,
  getAllTeamAgentSteps,
  moveTeamStepByPath,
  nestTeamStep,
  normalizeTeamOrchestrationPlan,
  parseTeamOrchestrationPlanInput as parseTeamOrchestrationPlanInputSupport,
  promoteTeamStep,
  serializeTeamOrchestrationPlan,
  TEAM_ORCHESTRATION_PRESET_METAS,
  TEAM_RECOVERY_PRESETS,
  teamOrchestrationPlanToJson,
} from './teamOrchestrationSupport'
import { useAgentConversationFlow } from './useAgentConversationFlow'
import { useAgentModelAndToolConfig } from './useAgentModelAndToolConfig'
import { useAgentPanels } from './useAgentPanels'
import { useAgentTeamRuntime } from './useAgentTeamRuntime'
import { useAgentTeamViewState } from './useAgentTeamViewState'
import type { ReferencedAsset, ReferencedTraffic, TrafficSendType } from './agentDraftTypes'
import { useAgentDraftArtifacts } from './useAgentDraftArtifacts'
import type {
  TeamOrchestrationPlan,
  TeamOrchestrationPresetId,
  TeamOrchestrationPresetMeta,
  TeamOrchestrationStep,
  TeamRecoveryPreset,
  TeamRecoveryPresetId,
  TeamRuntimeFailureMode,
  TeamRuntimeStepStat,
  TeamStepMovePayload,
} from './teamOrchestrationTypes'
import {
  type UiToolConfigPayload,
} from './toolConfigRuntime'

interface AgentStartEvent {
  execution_id: string
  task: string
}

interface AgentAssistantMessageSavedEvent {
  execution_id: string
  message_id: string
  content: string
  reasoning_content?: string | null
  timestamp: number
}

interface TeamSplitMemberOption {
  key: string
  label: string
  memberId?: string
  memberName?: string
  status?: string
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
const submitInFlight = ref(false)
const conversationId = ref<string | null>(props.executionId ?? null)
const showConversations = ref(false) // Default hidden
const showToolConfig = ref(false)
const currentConversationTitle = ref(t('agent.newConversationTitle'))
const conversationExecutionState = ref<PersistedAgentExecutionState | null>(null)
const historyLoadToken = ref(0)
const autoTitleGeneratingConversationIds = new Set<string>()

const conversationExecutionStateBadgeText = computed(() => {
  const labelKey = getExecutionStateLabelKey(conversationExecutionState.value?.outcome)
  return labelKey ? t(labelKey) : ''
})

const conversationExecutionStateBadgeClass = computed(() => {
  return getExecutionStateBadgeClass(conversationExecutionState.value?.outcome)
})

// Feature toggles
const ragEnabled = ref(false)
const webSearchEnabled = ref(false)
const teamModeEnabled = ref(false)
const tenthManEnabled = ref(false)
const {
  addReferencedAssets,
  addReferencedTraffic,
  clearDraftArtifacts,
  handleAddAttachments,
  handleAddDocuments,
  handleClearAssets,
  handleClearTraffic,
  handleDocumentProcessed,
  handleRemoveAsset,
  handleRemoveAttachment,
  handleRemoveDocument,
  handleRemoveTraffic,
  pendingAttachments,
  pendingDocuments,
  processedDocuments,
  referencedAssets,
  referencedTraffic,
  restoreArtifactsFromMessage,
} = useAgentDraftArtifacts()
const activeTeamSessionId = ref<string | null>(null)
const teamSessionState = ref<string>('PENDING')
const isTeamWorkspaceActive = ref(false)
const teamWorkspaceTab = ref<'tasks' | 'inbox' | 'blackboard' | 'agents'>('tasks')
const teamWorkspaceLoading = ref(false)
const teamSessionMessages = ref<AgentTeamMessage[]>([])
const teamSessionDetail = ref<AgentTeamSession | null>(null)
const teamTasks = ref<TeamTask[]>([])
const selectedTeamTaskId = ref<string | null>(null)
const teamBlackboardEntries = ref<TeamBlackboardEntry[]>([])
const teamOrchestrationPlanText = ref('{\n  "version": 1,\n  "steps": []\n}')
const teamOrchestrationDraft = ref<TeamOrchestrationPlan>({ version: 1, steps: [] })
const teamPlanDirty = ref(false)
const teamPlanSaving = ref(false)
const teamPlanError = ref<string | null>(null)
const teamPlanSuccess = ref<string | null>(null)
const teamResumeStepId = ref('')
const teamSelectedOrchestrationPresetId = ref<TeamOrchestrationPresetId | null>(null)
const teamSelectedRecoveryPresetId = ref<TeamRecoveryPresetId>('balanced')
const teamRecoveryPresetApplying = ref(false)
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
let unlistenAiConfigUpdated: UnlistenFn | null = null
let unlistenTeamStateChanged: UnlistenFn | null = null
let unlistenTeamMessageStreamStart: UnlistenFn | null = null
let unlistenTeamMessageStreamDelta: UnlistenFn | null = null
let unlistenTeamMessageStreamDone: UnlistenFn | null = null
let unlistenTeamToolCall: UnlistenFn | null = null
let unlistenTeamToolResult: UnlistenFn | null = null
let unlistenAgentStart: UnlistenFn | null = null
let unlistenAgentComplete: UnlistenFn | null = null
let unlistenAgentAssistantSaved: UnlistenFn | null = null
const {
  assistantDefaultMaxContextTokens,
  assistantModelOptions,
  assistantSelectedModel,
  buildTeamToolPolicyFromUiConfig,
  flushPendingToolConfigSave,
  handleAssistantModelChange,
  handleToolConfigUpdate,
  isLoadingAssistantModels,
  loadAssistantModelOptions,
  loadToolConfig,
  toolConfig,
  toolsEnabled,
} = useAgentModelAndToolConfig({
  getFailedToSaveToolConfigLabel: () => t('agent.failedToSaveToolConfig'),
  localError,
})

// Agent events
const matchesCurrentTeamSubagentParent = (parentExecutionId: string) => {
  const sessionId = String(activeTeamSessionId.value || '').trim()
  const parentId = String(parentExecutionId || '').trim()
  if (!sessionId || !parentId) return false
  return (
    parentId.startsWith(`team-v3:${sessionId}:`) ||
    parentId.startsWith(`team-v3-planner:${sessionId}:`)
  )
}

const agentEvents = useAgentEvents(computed(() => conversationId.value || ''), {
  suppressUserMessages: computed(() => teamModeEnabled.value),
  defaultMaxContextTokens: assistantDefaultMaxContextTokens,
  subagentParentExecutionMatcher: matchesCurrentTeamSubagentParent,
})
const messages = computed(() => agentEvents.messages.value)
const isTeamScopedMainFlowMessage = (message: AgentMessage) => {
  const metadata = message.metadata || {}
  const kind = String(metadata.kind || '').toLowerCase()
  const hasTeamMemberMeta =
    String(metadata.team_member_id || '').trim().length > 0 ||
    String(metadata.team_member_name || '').trim().length > 0
  if (kind.startsWith('team_') || kind === 'team_bridge') return true
  if (kind === 'tool_call' || kind === 'tool_result') {
    return hasTeamMemberMeta || String(metadata.team_session_id || '').trim().length > 0
  }
  return (
    message.id.startsWith('team:') ||
    message.id.startsWith('team-stream:') ||
    message.id.startsWith('team-toolcall:')
  )
}
const normalizeOptionalText = (value: unknown): string | undefined => {
  if (typeof value !== 'string') return undefined
  const trimmed = value.trim()
  return trimmed.length > 0 ? trimmed : undefined
}
const {
  clearSelectedTeamTask,
  formatTimestamp,
  resolveAgentName,
  selectedTeamTask,
  selectedTeamTaskTitle,
  teamBlackboardEntryBadgeClass,
  teamCurrentHumanInterventionTimeoutSecs,
  teamCurrentMaxHumanInterventions,
  teamCurrentNoHumanInputPolicy,
  teamFlattenedStepOptions,
  teamLastRuntimeStepPath,
  teamMemberNameOptions,
  teamOrchestrationPresets,
  teamOrchestrationRuntime,
  teamRecoveryPresets,
  teamRuntimeBackendRecoverySuggestions,
  teamRuntimeFailureModes,
  teamRuntimeHotspots,
  teamRuntimeRecoverySuggestions,
  teamRuntimeStepStats,
  teamRuntimeSuggestedResumeStepId,
  teamRuntimeSummary,
  teamSelectedOrchestrationPresetDescription,
  teamSelectedRecoveryPresetDescription,
  teamSplitMembers,
  teamWorkspaceBadgeCount,
  toggleSelectedTeamTask,
  visibleMessages,
} = useAgentTeamViewState({
  activeTeamSessionId,
  agentIsExecuting: computed(() => agentEvents.isExecuting.value),
  isTeamScopedMainFlowMessage,
  messages,
  selectedTeamTaskId,
  teamModeEnabled,
  teamOrchestrationDraft,
  teamSelectedOrchestrationPresetId,
  teamSelectedRecoveryPresetId,
  teamSessionDetail,
  teamSessionState,
  teamTasks,
})
const isTeamRunActive = computed(() => {
  if (!teamModeEnabled.value || !activeTeamSessionId.value) return false
  const normalized = String(teamSessionState.value || '').trim().toUpperCase()
  return [
    'EXECUTING',
    'INITIALIZING',
    'PROPOSING',
    'CHALLENGING',
    'CONVERGENCE_CHECK',
    'REVISING',
    'DECIDING',
    'ARTIFACT_GENERATION',
  ].includes(normalized)
})
const isExecuting = computed(() => agentEvents.isExecuting.value || isTeamRunActive.value)
const isStreaming = computed(() => agentEvents.isExecuting.value && !!agentEvents.streamingContent.value)
const streamingContent = computed(() => agentEvents.streamingContent.value)
const contextUsage = computed(() => agentEvents.contextUsage.value)
const scrollMessageViewportToBottom = () => {
  messageFlowRef.value?.scrollToBottom()
}
const taskStatusBadgeClass = (status: string) => {
  const normalized = (status || '').toLowerCase()
  if (normalized === 'completed') return 'badge-success'
  if (normalized === 'running') return 'badge-info'
  if (normalized === 'failed') return 'badge-error'
  if (normalized === 'blocked') return 'badge-warning'
  return 'badge-ghost'
}

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

const loadSubagentRuns = async (parentExecutionId: string, loadToken?: number) => {
  try {
    const runs = await invoke<SubagentRunRecord[]>('get_subagent_runs', {
      parentExecutionId,
    })
    if (
      (typeof loadToken === 'number' && loadToken !== historyLoadToken.value) ||
      conversationId.value !== parentExecutionId
    ) {
      return
    }

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

const todosComposable = useTodos()
const parseTeamTodoExecutionId = (executionId: string) => {
  if (!executionId.startsWith('team-v3:')) return null
  const parts = executionId.split(':')
  if (parts.length < 4) return null
  const sessionId = parts[1]?.trim()
  const taskId = parts[2]?.trim()
  if (!sessionId || !taskId) return null
  const memberId = parts.length >= 5 ? parts[3]?.trim() : undefined
  return {
    sessionId,
    taskId,
    memberId: memberId || undefined,
  }
}
const terminalComposable = useTerminal()
const {
  activeRightPanel,
  activateRightPanel,
  clearError,
  clearTodosForCurrentContext,
  deactivateRightPanel,
  error,
  handleCloseHtmlPanel,
  handleCloseTerminal,
  handleCloseTodos,
  handleRenderHtml,
  handleTodoSourceChange,
  handleToggleHtmlPanel,
  handleToggleTerminal,
  handleToggleTodos,
  hasHtmlPanelContent,
  htmlPanelContent,
  loadSidebarWidth,
  selectedTodoSourceKey,
  selectedTaskTodoSourceKey,
  sidebarWidth,
  startResize,
  todoBadgeCount,
  todoSourceOptions,
  todos,
} = useAgentPanels({
  activeTeamSessionId,
  agentError: computed(() => agentEvents.error.value),
  clearTodosForExecution: (executionId) => {
    todosComposable.clearTodosForExecution(executionId)
  },
  conversationId,
  getTodosForExecution: (executionId) => todosComposable.getTodosForExecution(executionId),
  isTeamWorkspaceActive,
  isTodosPanelActive: computed(() => todosComposable.isTodosPanelActive.value),
  localError,
  parseTeamTodoExecutionId,
  propsShowTodos: props.showTodos,
  resetAgentError: () => {
    agentEvents.resetError()
  },
  resolveAgentName,
  selectedTeamTaskAssigneeId: computed(() => normalizeOptionalText(selectedTeamTask.value?.assignee_agent_id) || null),
  teamModeEnabled,
  terminalClose: () => {
    terminalComposable.closeTerminal()
  },
  terminalHasHistory: computed(() => terminalComposable.hasHistory.value),
  terminalIsActive: computed(() => terminalComposable.isTerminalActive.value),
  terminalOpen: () => {
    terminalComposable.openTerminal()
  },
  todosByExecutionId: computed(() => todosComposable.todosByExecutionId.value),
  todosClose: () => {
    todosComposable.close()
  },
  todosExecutionIds: computed(() => todosComposable.executionIds.value),
  todosOpen: () => {
    todosComposable.open()
  },
})

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

const handleToggleWebSearch = (enabled: boolean) => {
  webSearchEnabled.value = enabled
  console.log('[AgentView] Web search:', enabled ? 'enabled' : 'disabled')
}
const {
  appendTeamBridgeMessage,
  applyTeamState,
  ensureConversationForTeamSession,
  ensureTeamRunStatusPolling,
  handleTeamAssistantMessageSaved,
  handleTeamExecutionFinished,
  handleTeamMessageStreamDelta,
  handleTeamMessageStreamDone,
  handleTeamMessageStreamStart,
  handleTeamToolCall,
  handleTeamToolResult,
  handleToggleTeamMode,
  handleToggleTeamWorkspace,
  loadTeamWorkspaceData,
  routeTeamMessage,
  runTeamExecutionFromWorkspace,
  setMirroredConversationMessageIds,
  startTeamExecutionRun,
  stopTeamRunStatusPolling,
  syncActiveTeamSession,
  syncTeamMessagesToMainFlow,
} = useAgentTeamRuntime({
  activeTeamSessionId,
  activateRightPanel,
  activeRightPanel,
  agentMessages: agentEvents.messages,
  buildToolPolicyFromUiConfig: buildTeamToolPolicyFromUiConfig,
  clearLocalError: () => {
    localError.value = null
  },
  conversationId,
  currentConversationTitle,
  deactivateRightPanel,
  flushPendingToolConfigSave: async () => {
    await flushPendingToolConfigSave()
  },
  getDisplayConversationTitle: () => t('agent.newConversationTitle'),
  getNewConversationTitle: () => `${t('agent.newConversationTitle')} ${new Date().toLocaleString()}`,
  handleStopExecution: async () => {
    await handleStop()
  },
  isExecuting,
  isTeamScopedMainFlowMessage,
  isTeamWorkspaceActive,
  loadConversationList: () => {
    conversationListRef.value?.loadConversations()
  },
  markConversationExecutionPending: () => {
    conversationExecutionState.value = null
  },
  onTeamWorkspaceSnapshotLoaded: () => {
    syncTeamOrchestrationEditorFromSession()
  },
  ragEnabled,
  selectedTeamTaskId,
  setLocalError: (message) => {
    localError.value = message
  },
  teamBlackboardEntries,
  teamModeEnabled,
  teamSelectedOrchestrationPresetId,
  teamSelectedRecoveryPresetId,
  teamSessionDetail,
  teamSessionMessages,
  teamSessionState,
  teamTasks,
  teamWorkspaceLoading,
  teamWorkspaceTab,
  toolConfig: toolConfig as Ref<UiToolConfigPayload>,
  webSearchEnabled,
})

const updateTeamOrchestrationTextFromDraft = () => {
  teamOrchestrationPlanText.value = serializeTeamOrchestrationPlan(teamOrchestrationDraft.value)
}

const syncTeamOrchestrationEditorFromSession = (force = false) => {
  const plan = teamSessionDetail.value?.orchestration_plan ?? defaultOrchestrationPlan()
  if (teamPlanDirty.value && !force) return
  const normalized = normalizeTeamOrchestrationPlan(plan)
  teamOrchestrationDraft.value = normalized
  teamOrchestrationPlanText.value = serializeTeamOrchestrationPlan(normalized)
  teamPlanDirty.value = false
  teamPlanError.value = null
  if (force || !teamResumeStepId.value.trim()) {
    const lastStepId = teamSessionDetail.value?.state_machine?.orchestration_runtime?.last_step_id
    teamResumeStepId.value = typeof lastStepId === 'string' ? lastStepId : ''
  }
  teamSelectedOrchestrationPresetId.value = null
  teamSelectedRecoveryPresetId.value = teamCurrentNoHumanInputPolicy.value
}

const handleTeamOrchestrationInput = (event: Event) => {
  const target = event.target as HTMLTextAreaElement
  teamOrchestrationPlanText.value = target.value
  try {
    const parsed = JSON.parse(target.value)
    teamOrchestrationDraft.value = normalizeTeamOrchestrationPlan(parsed)
  } catch {
    // Keep text as source when json is temporarily invalid during editing.
  }
  teamPlanDirty.value = true
  teamPlanError.value = null
  teamPlanSuccess.value = null
}

const handleTeamReloadOrchestrationPlan = () => {
  syncTeamOrchestrationEditorFromSession(true)
  teamPlanSuccess.value = '已从会话重新载入编排计划。'
}

const markTeamVisualPlanDirty = () => {
  updateTeamOrchestrationTextFromDraft()
  teamPlanDirty.value = true
  teamPlanError.value = null
  teamPlanSuccess.value = null
}

const handleTeamVisualStepsUpdated = (steps: TeamOrchestrationStep[]) => {
  teamOrchestrationDraft.value.steps = steps
  markTeamVisualPlanDirty()
}

const handleTeamApplyOrchestrationPreset = (presetId: TeamOrchestrationPresetId) => {
  const presetPlan = buildOrchestrationPresetPlan({
    memberOptions: teamMemberNameOptions.value,
    presetId,
    version: Math.max(1, Number(teamOrchestrationDraft.value.version || 1)),
  })
  const normalized = normalizeTeamOrchestrationPlan(presetPlan)
  teamOrchestrationDraft.value = normalized
  updateTeamOrchestrationTextFromDraft()
  teamPlanDirty.value = true
  teamPlanError.value = null
  teamSelectedOrchestrationPresetId.value = presetId

  const missingMemberCount = getAllTeamAgentSteps(normalized.steps)
    .filter((step) => !step.member || !step.member.trim())
    .length
  if (missingMemberCount > 0) {
    teamPlanSuccess.value = `已应用预设（${missingMemberCount} 个节点未匹配 Agent，请手动选择后保存）。`
  } else {
    teamPlanSuccess.value = '已应用编排预设，请保存后运行。'
  }
}

const handleTeamApplyRecoveryPreset = async (presetId: TeamRecoveryPresetId) => {
  const preset = TEAM_RECOVERY_PRESETS.find((item) => item.id === presetId)
  if (!preset) return

  teamPlanError.value = null
  teamPlanSuccess.value = null
  teamSelectedRecoveryPresetId.value = presetId

  const agentSteps = getAllTeamAgentSteps(teamOrchestrationDraft.value.steps)
  for (const step of agentSteps) {
    step.retry = {
      max_attempts: preset.max_attempts,
      backoff_ms: preset.backoff_ms,
    }
  }
  if (agentSteps.length > 0) {
    markTeamVisualPlanDirty()
  }

  if (!activeTeamSessionId.value) {
    teamPlanSuccess.value = '已应用恢复策略 preset（会话未激活，仅更新本地编排草稿）。'
    return
  }

  const currentStateMachine = teamSessionDetail.value?.state_machine && typeof teamSessionDetail.value.state_machine === 'object'
    ? teamSessionDetail.value.state_machine
    : {}
  const currentIntervention = (currentStateMachine as any)?.human_intervention && typeof (currentStateMachine as any).human_intervention === 'object'
    ? (currentStateMachine as any).human_intervention
    : {}

  const nextStateMachine = {
    ...currentStateMachine,
    no_human_input_policy: preset.no_human_input_policy,
    human_intervention_timeout_secs: preset.human_intervention_timeout_secs,
    max_human_interventions: preset.max_human_interventions,
    human_intervention: {
      ...currentIntervention,
      policy: preset.no_human_input_policy,
      timeout_secs: preset.human_intervention_timeout_secs,
    },
  }

  teamRecoveryPresetApplying.value = true
  try {
    await agentTeamApi.updateSession(activeTeamSessionId.value, {
      state_machine: nextStateMachine,
    })
    if (teamSessionDetail.value) {
      teamSessionDetail.value = {
        ...teamSessionDetail.value,
        state_machine: nextStateMachine,
      }
    }
    teamPlanSuccess.value = '已应用恢复策略 preset，并同步会话恢复配置。'
  } catch (e: any) {
    teamPlanError.value = e?.message || String(e)
  } finally {
    teamRecoveryPresetApplying.value = false
  }
}

const handleTeamMoveStepByPath = (payload: TeamStepMovePayload) => {
  const changed = moveTeamStepByPath(teamOrchestrationDraft.value.steps, payload)
  if (!changed) {
    markTeamVisualPlanDirty()
    return
  }
  markTeamVisualPlanDirty()
}

const handleTeamPromoteStep = (path: number[]) => {
  if (!promoteTeamStep(teamOrchestrationDraft.value.steps, path)) return
  markTeamVisualPlanDirty()
}

const handleTeamNestStep = (path: number[]) => {
  if (!nestTeamStep(teamOrchestrationDraft.value.steps, path)) return
  markTeamVisualPlanDirty()
}

const parseTeamOrchestrationPlanInput = (): any => {
  const { jsonValue, normalized } = parseTeamOrchestrationPlanInputSupport(teamOrchestrationPlanText.value)
  teamOrchestrationDraft.value = normalized
  return jsonValue
}

const handleTeamSaveOrchestrationPlan = async () => {
  if (!activeTeamSessionId.value) return
  teamPlanSaving.value = true
  teamPlanError.value = null
  teamPlanSuccess.value = null
  try {
    parseTeamOrchestrationPlanInput()
    teamPlanError.value = '会话编排直改入口已下线。'
  } catch (e: any) {
    teamPlanError.value = e?.message || String(e)
  } finally {
    teamPlanSaving.value = false
  }
}

const handleTeamStartRunWithPlan = async () => {
  if (!activeTeamSessionId.value || isTeamRunActive.value) return
  teamPlanError.value = null
  teamPlanSuccess.value = null
  try {
    if (teamPlanDirty.value) {
      await handleTeamSaveOrchestrationPlan()
      if (teamPlanError.value) return
    }
    await runTeamExecutionFromWorkspace('[Team] 已按当前编排计划启动执行。')
    await loadTeamWorkspaceData()
  } catch (e: any) {
    teamPlanError.value = e?.message || String(e)
  }
}

const handleTeamRetryRun = async () => {
  if (!activeTeamSessionId.value || isTeamRunActive.value) return
  teamPlanError.value = null
  teamPlanSuccess.value = null
  try {
    if (teamPlanDirty.value) {
      await handleTeamSaveOrchestrationPlan()
      if (teamPlanError.value) return
    }
    await runTeamExecutionFromWorkspace('[Team] 已触发重试运行。')
    await loadTeamWorkspaceData()
  } catch (e: any) {
    teamPlanError.value = e?.message || String(e)
  }
}

const handleTeamResumeFromStep = async () => {
  if (!activeTeamSessionId.value || isTeamRunActive.value) return
  teamPlanError.value = null
  teamPlanSuccess.value = null
  try {
    const stepId = teamResumeStepId.value.trim()
    if (!stepId) {
      throw new Error('请先填写要恢复的 step_id。')
    }
    const currentStateMachine = teamSessionDetail.value?.state_machine && typeof teamSessionDetail.value.state_machine === 'object'
      ? teamSessionDetail.value.state_machine
      : {}
    const currentRuntime = currentStateMachine?.orchestration_runtime && typeof currentStateMachine.orchestration_runtime === 'object'
      ? currentStateMachine.orchestration_runtime
      : {}
    await agentTeamApi.updateSession(activeTeamSessionId.value, {
      state_machine: {
        ...currentStateMachine,
        orchestration_runtime: {
          ...currentRuntime,
          resume_from_step_id: stepId,
        },
      },
    })
    await runTeamExecutionFromWorkspace(`[Team] 已从 step '${stepId}' 发起恢复执行。`)
    await loadTeamWorkspaceData()
  } catch (e: any) {
    teamPlanError.value = e?.message || String(e)
  }
}

const handleTeamFillResumeStep = (stepId: string) => {
  const normalized = (stepId || '').trim()
  if (!normalized) return
  teamResumeStepId.value = normalized
  teamPlanError.value = null
  teamPlanSuccess.value = `已选择恢复节点：${normalized}`
}
const {
  handleClearConversation,
  handleConversationExecutionStateUpdate,
  handleCreateConversation: handleCreateConversationFlow,
  handleEditMessage,
  handleResendMessage,
  handleSelectConversation: handleSelectConversationFlow,
  handleStop,
  handleSubmit,
  loadConversationHistory,
  loadLatestConversation,
} = useAgentConversationFlow({
  activeTeamSessionId,
  agentMessages: agentEvents.messages,
  agentStreamingContent: agentEvents.streamingContent,
  agentSubagents: agentEvents.subagents,
  assistantSelectedModel,
  buildToolConfig: () => toolConfig.value as unknown as UiToolConfigPayload,
  clearAgentMessages: () => {
    agentEvents.clearMessages()
  },
  clearDraftArtifacts,
  clearTodosForCurrentContext,
  closeConversationDrawer: () => {
    showConversations.value = false
  },
  conversationExecutionState,
  conversationId,
  currentConversationTitle,
  emitComplete: (payload) => {
    emit('complete', payload)
  },
  emitError: (message) => {
    emit('error', message)
  },
  emitSubmit: (task) => {
    emit('submit', task)
  },
  ensureConversationForTeamSession,
  executionIdProp: props.executionId,
  forceTodos: props.showTodos,
  getFailedToClearConversationLabel: () => t('agent.failedToClearConversation'),
  getFailedToStopExecutionLabel: () => t('agent.failedToStopExecution'),
  getNewConversationTitle: () => `${t('agent.newConversationTitle')} ${new Date().toLocaleString()}`,
  getToolCallCompletedLabel: () => t('agent.toolCallCompleted'),
  getUnnamedConversationTitle: () => t('agent.newConversationTitle'),
  handleStopTeamState: applyTeamState,
  historyLoadToken,
  inputValue,
  isExecuting,
  isTeamModeEnabled: teamModeEnabled,
  isToolConfigEnabled: toolsEnabled,
  loadConversationList: () => {
    conversationListRef.value?.loadConversations()
  },
  loadSubagentRuns,
  localError,
  pendingAttachments,
  processedDocuments,
  ragEnabled,
  referencedAssets,
  referencedTraffic,
  resetTerminal: () => {
    terminalComposable.resetTerminal()
  },
  restoreArtifactsFromMessage,
  routeTeamMessage,
  scrollMessageViewportToBottom,
  setMirroredConversationMessageIds,
  setPendingDocumentAttachments: (documents) => {
    agentEvents.setPendingDocumentAttachments(documents)
  },
  startTeamExecutionRun,
  stopAgentExecutionState: () => {
    agentEvents.stopExecution()
  },
  submitInFlight,
  syncActiveTeamSession,
  syncTeamMessagesToMainFlow,
  teamModeEnabled,
  tenthManEnabled,
  webSearchEnabled,
})

// Handle view subagent details - open modal to show details
const handleViewSubagentDetails = (subagentId: string) => {
  console.log('[AgentView] View subagent details:', subagentId)
  const subagent = subagents.value.find(s => s.id === subagentId)
  if (subagent) {
    selectedSubagent.value = subagent
    showSubagentDetailModal.value = true
  }
}

const handleSelectConversation = async (convId: string) => {
  await handleSelectConversationFlow(convId)
}

const handleCreateConversation = async (newConvId?: string) => {
  await handleCreateConversationFlow(newConvId)
  nextTick(() => {
    inputAreaRef.value?.focusInput()
  })
}

// Initialize
onMounted(async () => {
  console.log('[AgentView] Mounted with executionId:', props.executionId)
  await loadAssistantModelOptions()
  unlistenAiConfigUpdated = await listen('ai_config_updated', async () => {
    await loadAssistantModelOptions()
  })
  unlistenTeamStateChanged = await listen<AgentTeamStateChangedEvent>('agent_team:state_changed', (event) => {
    if (!activeTeamSessionId.value || event.payload.session_id !== activeTeamSessionId.value) {
      return
    }
    applyTeamState(event.payload.state)
    void syncTeamMessagesToMainFlow(event.payload.session_id)
    if (isTeamWorkspaceActive.value) {
      void loadTeamWorkspaceData()
    }
  })
  unlistenTeamMessageStreamStart = await listen<AgentTeamMessageStreamStartEvent>('agent_team:message_stream_start', (event) => {
    handleTeamMessageStreamStart(event.payload)
  })
  unlistenTeamMessageStreamDelta = await listen<AgentTeamMessageStreamDeltaEvent>('agent_team:message_stream_delta', (event) => {
    handleTeamMessageStreamDelta(event.payload)
  })
  unlistenTeamMessageStreamDone = await listen<AgentTeamMessageStreamDoneEvent>('agent_team:message_stream_done', (event) => {
    handleTeamMessageStreamDone(event.payload)
  })
  unlistenTeamToolCall = await listen<AgentTeamToolCallEvent>('agent_team:tool_call', (event) => {
    handleTeamToolCall(event.payload)
  })
  unlistenTeamToolResult = await listen<AgentTeamToolResultEvent>('agent_team:tool_result', (event) => {
    handleTeamToolResult(event.payload)
  })
  unlistenAgentStart = await listen<AgentStartEvent>('agent:start', (event) => {
    if (event.payload.execution_id !== conversationId.value) return
    conversationExecutionState.value = null
  })
  unlistenAgentComplete = await listen<AgentExecutionFinishedEvent>('agent:execution_finished', (event) => {
    handleConversationExecutionStateUpdate(event.payload)
    void handleTeamExecutionFinished(event.payload)
  })
  unlistenAgentAssistantSaved = await listen<AgentAssistantMessageSavedEvent>('agent:assistant_message_saved', (event) => {
    void handleTeamAssistantMessageSaved(event.payload)
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
  if (unlistenTeamStateChanged) {
    unlistenTeamStateChanged()
    unlistenTeamStateChanged = null
  }
  if (unlistenTeamMessageStreamStart) {
    unlistenTeamMessageStreamStart()
    unlistenTeamMessageStreamStart = null
  }
  if (unlistenTeamMessageStreamDelta) {
    unlistenTeamMessageStreamDelta()
    unlistenTeamMessageStreamDelta = null
  }
  if (unlistenTeamMessageStreamDone) {
    unlistenTeamMessageStreamDone()
    unlistenTeamMessageStreamDone = null
  }
  if (unlistenTeamToolCall) {
    unlistenTeamToolCall()
    unlistenTeamToolCall = null
  }
  if (unlistenTeamToolResult) {
    unlistenTeamToolResult()
    unlistenTeamToolResult = null
  }
  if (unlistenAgentComplete) {
    unlistenAgentComplete()
    unlistenAgentComplete = null
  }
  if (unlistenAgentStart) {
    unlistenAgentStart()
    unlistenAgentStart = null
  }
  if (unlistenAgentAssistantSaved) {
    unlistenAgentAssistantSaved()
    unlistenAgentAssistantSaved = null
  }
})

// When component is activated (e.g., switching back from another page)
onActivated(() => {
  console.log('[AgentView] Activated, scrolling to bottom')
  // Scroll to bottom when returning to this page
  nextTick(() => {
    scrollMessageViewportToBottom()
  })
})

// Watch for conversation changes to update title
watch(conversationId, async (newId) => {
  setMirroredConversationMessageIds(new Set())
  if (!newId) {
    currentConversationTitle.value = t('agent.newConversationTitle')
    conversationExecutionState.value = null
  }
  await syncActiveTeamSession()
})

watch(teamTasks, (tasks) => {
  if (!selectedTeamTaskId.value) return
  if (tasks.some((task) => task.id === selectedTeamTaskId.value)) return
  selectedTeamTaskId.value = null
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
  scrollToBottom: () => scrollMessageViewportToBottom(),
  addReferencedTraffic,
  addReferencedAssets,
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
