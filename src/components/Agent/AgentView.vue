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
        class="tool-config-drawer absolute right-0 top-0 bottom-0 bg-base-100 shadow-2xl z-50 overflow-hidden"
        :style="{ width: toolConfigDrawerWidth + 'px' }"
      >
        <div
          class="resize-handle drawer-resize-handle absolute left-0 top-0 bottom-0 z-10 w-1 cursor-col-resize"
          @mousedown="startToolConfigDrawerResize"
        ></div>
        <AssistantWorkConfigPanel
          :available-models="assistantModelOptions"
          :context-mode="assistantSessionSettings.contextMode"
          :model-loading="isLoadingAssistantModels"
          :profile-id="assistantSessionSettings.profileId"
          :profile-loading="isLoadingAssistantProfiles"
          :profile-options="assistantProfileOptions"
          :run-mode="assistantSessionSettings.runMode"
          :selected-model="assistantSelectedModel"
          :tool-config="toolConfig"
          @update:context-mode="handleAssistantContextModeChange"
          @update:model="handleAssistantModelSelection"
          @update:profile-id="handleAssistantProfileChange"
          @update:run-mode="handleAssistantRunModeChange"
          @update:tool-config="handleToolConfigUpdate"
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
        </div>
        <div class="flex items-center gap-2">
          <!-- Tasks Button - always visible -->
          <button 
            @click="handleToggleTasks()"
            class="btn btn-sm gap-1"
            :class="activeRightPanel === 'tasks' ? 'btn-primary' : 'btn-ghost text-primary'"
            :title="activeRightPanel === 'tasks' ? t('agent.tasksPanelOpen') : t('agent.viewTasks')"
          >
            <i class="fas fa-tasks"></i>
            <span>{{ t('agent.tasks') }}</span>
            <span v-if="taskBadgeCount > 0" class="badge badge-xs badge-primary">{{ taskBadgeCount }}</span>
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
            v-if="teamWorkspaceAvailable"
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

      <!-- {{ t('agent.messagesAndTasks') }} -->
      <div class="flex flex-1 overflow-hidden min-h-0">
        <!-- Left: Message flow + Input Area -->
        <div class="message-area flex-1 flex flex-col overflow-hidden min-h-0">
          <SubagentPanel
            :subagents="subagents"
            :is-open="isSubagentPanelOpen"
            @toggle="isSubagentPanelOpen = !isSubagentPanelOpen"
            @view-details="handleViewSubagentDetails"
          />
          <div
            v-if="isFocusBannerVisible"
            class="mx-4 mt-2 rounded-lg border border-info/25 bg-info/10 px-3 py-2"
          >
            <div class="flex flex-col gap-2 lg:flex-row lg:items-center lg:justify-between">
              <div class="min-w-0">
                <div class="text-sm font-medium text-info">已定位到 memory 关联消息</div>
                <div class="text-xs text-base-content/70 break-all">
                  <span v-if="focusBannerMemoryId">memory: {{ focusBannerMemoryId }}</span>
                  <span v-if="focusedMemoryMessageId" class="ml-2">message: {{ focusedMemoryMessageId }}</span>
                </div>
              </div>
              <div class="flex flex-wrap gap-2">
                <button
                  v-if="focusBannerMemoryId"
                  class="btn btn-xs btn-outline btn-info"
                  @click="openFocusedMemoryInTools"
                >
                  <i class="fas fa-external-link-alt mr-1"></i>
                  返回 Tools
                </button>
                <button
                  class="btn btn-xs btn-outline"
                  @click="clearFocusedLocation"
                >
                  <i class="fas fa-times mr-1"></i>
                  清除定位
                </button>
              </div>
            </div>
          </div>
          <!-- Message flow -->
          <div class="relative flex-1 min-h-0">
            <MessageFlow
              ref="messageFlowRef"
              :messages="visibleMessages"
              :is-executing="isExecuting"
              :is-streaming="isStreaming"
              :streaming-content="streamingContent"
              :focused-message-id="focusedMemoryMessageId"
              class="h-full"
              @resend="handleResendMessage"
              @edit="handleEditMessage"
              @focus-team-task="handleFocusTeamTask"
              @message-focused="handleFocusedMessage"
              @render-html="handleRenderHtml"
            />
            <div
              v-if="isHistoryLoading"
              class="absolute inset-0 flex flex-col items-center justify-center gap-3 bg-base-100/85 backdrop-blur-sm text-base-content/70"
            >
              <span class="loading loading-spinner loading-lg text-primary"></span>
              <p class="text-sm">{{ t('agent.loadingConversation', '正在加载历史对话...') }}</p>
            </div>
          </div>
          
          <!-- {{ t('agent.inputArea') }} -->
          <InputAreaComponent
            ref="inputAreaRef"
            v-model:input-message="inputValue"
            :conversation-id="conversationId"
            :is-loading="isExecuting"
            :allow-takeover="true"
            :show-debug-info="false"
            :rag-enabled="ragEnabled"
            :web-search-enabled="webSearchEnabled"
            :team-enabled="teamModeEnabled"
            :pending-attachments="pendingAttachments"
            :pending-documents="pendingDocuments"
            :processed-documents="processedDocuments"
            :referenced-files="referencedFiles"
            :referenced-messages="referencedMessages"
            :referenced-traffic="referencedTraffic"
            :referenced-assets="referencedAssets"
            :available-conversation-messages="visibleMessages"
            :context-usage="contextUsage"
            :default-max-context-tokens="assistantDefaultMaxContextTokens"
            :available-agents="assistantAgentOptions"
            :selected-agent="assistantSessionSettings.profileId"
            :agent-loading="isLoadingAssistantProfiles"
            @send-message="handleSubmit"
            @stop-execution="handleStop"
            @toggle-rag="handleToggleRAG"
            @toggle-web-search="handleToggleWebSearch"
            @toggle-team="handleToggleTeamMode"
            @change-agent="handleAssistantProfileChange"
            @add-attachments="handleAddAttachments"
            @remove-attachment="handleRemoveAttachment"
            @add-documents="handleAddDocuments"
            @remove-document="handleRemoveDocument"
            @document-processed="handleDocumentProcessed"
            @remove-file="handleRemoveFile"
            @clear-files="handleClearFiles"
            @add-file-reference="addReferencedFiles"
            @sync-file-references="syncReferencedFiles"
            @remove-message="handleRemoveMessage"
            @clear-messages="handleClearMessages"
            @add-message-reference="addReferencedMessages"
            @sync-message-references="syncReferencedMessages"
            @add-traffic-reference="addReferencedTraffic"
            @sync-traffic-references="syncReferencedTraffic"
            @remove-traffic="handleRemoveTraffic"
            @clear-traffic="handleClearTraffic"
            @add-asset-reference="addReferencedAssets"
            @sync-asset-references="syncReferencedAssets"
            @remove-asset="handleRemoveAsset"
            @clear-assets="handleClearAssets"
            @create-new-conversation="handleCreateConversation"
            @clear-conversation="handleClearConversation"
            @open-tool-config="openToolConfigDrawer"
          />
        </div>
        
        <!-- Right: Side Panel (Task, HTML, Terminal, or Team) -->
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
              :pending-create-task="pendingTeamCreateTask"
              :tasks="teamTasks"
              :selected-task-id="selectedTeamTaskId"
              :selected-task-title="selectedTeamTaskTitle"
              :pending-task-action-kind="pendingTeamTaskActionKind"
              :pending-task-action-task-id="pendingTeamTaskActionTaskId"
              :session-messages="teamSessionMessages"
              :blackboard-entries="teamBlackboardEntries"
              :session-detail="teamSessionDetail"
              :resolve-agent-name="resolveAgentName"
              @block-task="handleBlockTeamTask"
              @clear-selected-task="clearSelectedTeamTask"
              @claim-task="handleClaimTeamTask"
              @complete-task="handleCompleteTeamTask"
              @create-task="handleCreateTeamTask"
              @fail-task="handleFailTeamTask"
              @release-task="handleReleaseTeamTask"
              @toggle-selected-task="toggleSelectedTeamTask"
            />

            <TaskPanel 
              v-else-if="activeRightPanel === 'tasks'" 
              :tasks="tasks"
              :is-active="activeRightPanel === 'tasks'"
              :source-options="taskSourceOptions"
              :selected-source-key="selectedTaskSourceKey"
              class="h-full p-4 overflow-y-auto border-0 bg-transparent"
              @close="handleCloseTasks"
              @source-change="handleTaskSourceChange"
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
      <div
        v-if="error"
        class="error-banner flex items-start gap-3 px-4 py-3 bg-error/10 border-t border-error text-error text-sm"
      >
        <span class="error-icon flex-shrink-0 pt-0.5">⚠️</span>
        <div class="min-w-0 flex-1">
          <div class="error-message break-words">{{ error }}</div>
          <button
            v-if="isVisionModelUnsupportedFailure"
            class="btn btn-xs btn-outline btn-error mt-2"
            @click="openToolConfigDrawer"
          >
            {{ t('agent.openWorkConfig') }}
          </button>
        </div>
        <button @click="clearError" class="error-close bg-transparent border-none text-error cursor-pointer text-xl leading-none px-1 hover:text-base-content">×</button>
      </div>
    </div>

    <!-- Subagent Detail Modal -->
    <SubagentDetailModal
      :visible="showSubagentDetailModal"
      :subagent="selectedSubagent"
      @close="showSubagentDetailModal = false"
    />
    <AskUserQuestionModal :execution-id="conversationId" />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, nextTick, type Ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'
import type { AgentMessage } from '@/types/agent'
import type {
  AgentTeamMessage,
  AgentTeamSession,
  TeamBlackboardEntry,
  TeamTask,
} from '@/types/agentTeam'
import { agentTeamApi } from '@/api/agentTeam'
import { useAgentEvents } from '@/composables/useAgentEvents'
import { useAgentTasks } from '@/composables/useAgentTasks'
import { useTerminal } from '@/composables/useTerminal'
import { useAgentSessionManager } from '@/composables/useAgentSessionManager'
import AskUserQuestionModal from './AskUserQuestionModal.vue'
import MessageFlow from './MessageFlow.vue'
import TaskPanel from './TaskPanel.vue'
import HtmlPanel from './HtmlPanel.vue'
import SubagentPanel from './SubagentPanel.vue'
import SubagentDetailModal from './SubagentDetailModal.vue'
import InteractiveTerminal from '@/components/Tools/InteractiveTerminal.vue'
import InputAreaComponent from '@/components/InputAreaComponent.vue'
import ConversationList from './ConversationList.vue'
import AssistantWorkConfigPanel from './AssistantWorkConfigPanel.vue'
import TeamWorkspacePanel from './TeamWorkspacePanel.vue'
import {
  type AgentExecutionFinishedEvent,
  getExecutionStateBadgeClass,
  getExecutionStateLabelKey,
  type PersistedAgentExecutionState,
} from './executionState'
import {
} from './teamOrchestrationSupport'
import { useAgentConversationFlow } from './useAgentConversationFlow'
import { useAgentModelAndToolConfig } from './useAgentModelAndToolConfig'
import { useAgentPanels } from './useAgentPanels'
import { useAssistantProfiles } from './assistantProfiles'
import { useAssistantSessionSettings } from './useAssistantSessionSettings'
import { useAgentTeamRuntime } from './useAgentTeamRuntime'
import { useAgentTeamViewState } from './useAgentTeamViewState'
import type {
  ReferencedAsset,
  ReferencedTraffic,
  TrafficSendType,
} from './agentDraftTypes'
import { useAgentDraftArtifacts } from './useAgentDraftArtifacts'
import type {
  TeamOrchestrationPresetId,
  TeamOrchestrationPlan,
  TeamOrchestrationPresetMeta,
  TeamRecoveryPreset,
  TeamRecoveryPresetId,
  TeamRuntimeFailureMode,
  TeamRuntimeStepStat,
} from './teamOrchestrationTypes'
import {
  type UiToolConfigPayload,
} from './toolConfigRuntime'
import { mapPersistedAgentTasks } from './agentTaskHistorySupport'
import { useAgentMessageFocus } from './useAgentMessageFocus'
import { useAgentTeamOrchestration } from './useAgentTeamOrchestration'
import { useAgentTeamTaskActions } from './useAgentTeamTaskActions'
import { useAgentConversationBinding } from './useAgentConversationBinding'
import { useAgentViewLifecycle } from './useAgentViewLifecycle'
import { useAgentSubagents } from './useAgentSubagents'
import { useAgentViewEffects } from './useAgentViewEffects'
import { isVisionModelUnsupportedError } from './agentVisionErrorSupport'

interface TeamSplitMemberOption {
  key: string
  label: string
  memberId?: string
  memberName?: string
  status?: string
}

const props = withDefaults(defineProps<{
  executionId?: string
  showTasks?: boolean
  selectedRole?: any
  focusedMemoryId?: string | null
  focusedMessageId?: string | null
}>(), {
  showTasks: true,
  focusedMemoryId: null,
  focusedMessageId: null,
})

const emit = defineEmits<{
  (e: 'submit', task: string): void
  (e: 'complete', result: any): void
  (e: 'error', error: string): void
  (e: 'conversation-changed', payload: {
    previousConversationId: string | null
    conversationId: string
    title: string | null
  }): void
  (e: 'memory-message-focused', payload: { memoryId: string; messageId: string }): void
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
const isHistoryLoading = ref(false)
const autoTitleGeneratingConversationIds = new Set<string>()
const lastAutoOpenedVisionError = ref<string | null>(null)

const conversationExecutionStateBadgeText = computed(() => {
  const labelKey = getExecutionStateLabelKey(conversationExecutionState.value?.outcome)
  return labelKey ? t(labelKey) : ''
})

const conversationExecutionStateBadgeClass = computed(() => {
  return getExecutionStateBadgeClass(conversationExecutionState.value?.outcome)
})

const {
  defaultAssistantProfileId,
  getAssistantProfileOption,
  loadDefaultAssistantProfile,
  isLoadingAssistantProfiles,
  loadAssistantProfiles,
  profileOptions: assistantProfileOptions,
} = useAssistantProfiles()
const assistantAgentOptions = computed(() =>
  assistantProfileOptions.value.map((profile) => ({
    value: profile.id,
    label: profile.label,
    description: profile.runMode === 'team' ? 'Team' : 'Assistant',
  })),
)

// Feature toggles
const {
  applyConversationBinding,
  applyProfilePreset,
  ragEnabled,
  resetSessionSettings,
  sessionSettings: assistantSessionSettings,
  setContextMode,
  setProfileId,
  setRunMode,
  teamModeEnabled,
  tenthManEnabled,
  toConversationBinding,
  webSearchEnabled,
} = useAssistantSessionSettings()
const {
  addReferencedAssets,
  addReferencedFiles,
  addReferencedMessages,
  addReferencedTraffic,
  clearDraftArtifacts,
  handleAddAttachments,
  handleAddDocuments,
  handleClearAssets,
  handleClearFiles,
  handleClearMessages,
  handleClearTraffic,
  handleDocumentProcessed,
  handleRemoveAsset,
  handleRemoveAttachment,
  handleRemoveDocument,
  handleRemoveFile,
  handleRemoveMessage,
  handleRemoveTraffic,
  pendingAttachments,
  pendingDocuments,
  processedDocuments,
  referencedAssets,
  referencedFiles,
  referencedMessages,
  referencedTraffic,
  restoreArtifactsFromMessage,
  syncReferencedAssets,
  syncReferencedFiles,
  syncReferencedMessages,
  syncReferencedTraffic,
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
const teamOrchestrationDraft = ref<TeamOrchestrationPlan>({ version: 1, steps: [] })
const teamSelectedOrchestrationPresetId = ref<TeamOrchestrationPresetId | null>(null)
const teamSelectedRecoveryPresetId = ref<TeamRecoveryPresetId>('balanced')
const isSubagentPanelOpen = ref(false)
const subagents = computed(() => agentEvents.subagents.value)
const {
  assistantDefaultMaxContextTokens,
  assistantModelOptions,
  assistantSelectedModel,
  buildTeamToolPolicyFromUiConfig,
  defaultToolConfig,
  flushPendingToolConfigSave,
  handleAssistantModelChange,
  handleToolConfigUpdate,
  isLoadingAssistantModels,
  loadAssistantModelOptions,
  loadToolConfig,
  setAssistantSelectedModel,
  toolConfig,
  toolsEnabled,
} = useAgentModelAndToolConfig({
  getFailedToSaveToolConfigLabel: () => t('agent.failedToSaveToolConfig'),
  localError,
})

const assistantContextMode = computed(() => assistantSessionSettings.value.contextMode)

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

const teamWorkspaceAvailable = computed(() =>
  Boolean(
    teamModeEnabled.value ||
    activeTeamSessionId.value ||
    teamSessionDetail.value ||
    teamSessionMessages.value.length ||
    teamTasks.value.length ||
    teamBlackboardEntries.value.length,
  ),
)
const isExecuting = computed(() => agentEvents.isExecuting.value || isTeamRunActive.value)
const isStreaming = computed(() => agentEvents.isExecuting.value && !!agentEvents.streamingContent.value)
const streamingContent = computed(() => agentEvents.streamingContent.value)
const contextUsage = computed(() => agentEvents.contextUsage.value)
const focusTeamTaskInWorkspace = (taskId: string) => {
  const normalizedTaskId = String(taskId || '').trim()
  if (!normalizedTaskId) return
  activateRightPanel('team')
  isTeamWorkspaceActive.value = true
  teamWorkspaceTab.value = 'tasks'
  selectedTeamTaskId.value = normalizedTaskId
}
const {
  clearFocusedLocation,
  focusBannerMemoryId,
  focusedMemoryMessageId,
  handleFocusedMessage,
  handleFocusTeamTask,
  isFocusBannerVisible,
  openFocusedMemoryInTools,
} = useAgentMessageFocus({
  focusedMemoryId: computed(() => props.focusedMemoryId),
  focusedMessageId: computed(() => props.focusedMessageId),
  visibleMessages,
  emitMemoryMessageFocused: (payload) => emit('memory-message-focused', payload),
  focusTeamTaskInWorkspace,
})

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
const {
  handleViewSubagentDetails,
  loadSubagentRuns,
  selectedSubagent,
  showSubagentDetailModal,
} = useAgentSubagents({
  conversationId,
  historyLoadToken,
  subagents: agentEvents.subagents,
})

const taskComposable = useAgentTasks()
const parseTeamTaskExecutionId = (executionId: string) => {
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
  clearTasksForCurrentContext,
  deactivateRightPanel,
  error,
  handleCloseHtmlPanel,
  handleCloseTasks,
  handleCloseTerminal,
  handleRenderHtml,
  handleTaskSourceChange,
  handleToggleHtmlPanel,
  handleToggleTasks,
  handleToggleTerminal,
  hasHtmlPanelContent,
  htmlPanelContent,
  loadSidebarWidth,
  loadToolConfigDrawerWidth,
  selectedTaskSourceKey,
  sidebarWidth,
  startToolConfigDrawerResize,
  startResize,
  taskBadgeCount,
  taskSourceOptions,
  tasks,
  toolConfigDrawerWidth,
} = useAgentPanels({
  activeTeamSessionId,
  agentError: computed(() => agentEvents.error.value),
  clearTasksForExecution: taskComposable.clearTasksForExecution,
  conversationId,
  getTasksForExecution: taskComposable.getTasksForExecution,
  isTeamWorkspaceActive,
  isTaskPanelActive: computed(() => taskComposable.isTaskPanelActive.value),
  localError,
  parseTeamTaskExecutionId,
  propsShowTasks: props.showTasks,
  resetAgentError: () => {
    agentEvents.resetError()
  },
  resolveAgentName,
  selectedTeamTaskAssigneeId: computed(() => normalizeOptionalText(selectedTeamTask.value?.assignee_agent_id) || null),
  teamWorkspaceAvailable,
  terminalClose: () => {
    terminalComposable.closeTerminal()
  },
  terminalHasHistory: computed(() => terminalComposable.hasHistory.value),
  terminalIsActive: computed(() => terminalComposable.isTerminalActive.value),
  terminalOpen: () => {
    terminalComposable.openTerminal()
  },
  taskExecutionIds: computed(() => taskComposable.executionIds.value),
  tasksByExecutionId: computed(() => taskComposable.tasksByExecutionId.value),
  tasksClose: () => {
    taskComposable.close()
  },
  tasksOpen: () => {
    taskComposable.open()
  },
})

const isVisionModelUnsupportedFailure = computed(() => isVisionModelUnsupportedError(error.value))

const openToolConfigDrawer = () => {
  showConversations.value = false
  showToolConfig.value = true
}

// Handle retrieval toggle
const handleToggleRAG = (enabled: boolean) => {
  ragEnabled.value = enabled
  console.log('[AgentView] retrieval:', enabled ? 'enabled' : 'disabled')
}

const handleToggleWebSearch = (enabled: boolean) => {
  webSearchEnabled.value = enabled
  console.log('[AgentView] Web search:', enabled ? 'enabled' : 'disabled')
}

const hydrateTaskHistory = async (targetConversationId: string) => {
  const persistedTasks = await invoke<any[]>('get_agent_tasks', { executionId: targetConversationId })
  taskComposable.setTasksForExecution(targetConversationId, mapPersistedAgentTasks(persistedTasks))
}

const handleAssistantModelSelection = (value: string | null) => {
  handleAssistantModelChange(value || '')
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
  teamWorkspaceAvailable,
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
const {
  handleBlockTeamTask,
  handleClaimTeamTask,
  handleCompleteTeamTask,
  handleCreateTeamTask,
  handleFailTeamTask,
  handleReleaseTeamTask,
  pendingTeamCreateTask,
  pendingTeamTaskActionKind,
  pendingTeamTaskActionTaskId,
} = useAgentTeamTaskActions({
  activeTeamSessionId,
  loadTeamWorkspaceData,
})
const {
  handleTeamApplyOrchestrationPreset,
  handleTeamApplyRecoveryPreset,
  handleTeamFillResumeStep,
  handleTeamMoveStepByPath,
  handleTeamNestStep,
  handleTeamOrchestrationInput,
  handleTeamPromoteStep,
  handleTeamReloadOrchestrationPlan,
  handleTeamResumeFromStep,
  handleTeamRetryRun,
  handleTeamSaveOrchestrationPlan,
  handleTeamStartRunWithPlan,
  handleTeamVisualStepsUpdated,
  parseTeamOrchestrationPlanInput,
  syncTeamOrchestrationEditorFromSession,
  teamOrchestrationPlanText,
  teamPlanDirty,
  teamPlanError,
  teamPlanSaving,
  teamPlanSuccess,
  teamRecoveryPresetApplying,
  teamResumeStepId,
} = useAgentTeamOrchestration({
  activeTeamSessionId,
  isTeamRunActive,
  loadTeamWorkspaceData,
  runTeamExecutionFromWorkspace,
  teamOrchestrationDraft,
  teamCurrentNoHumanInputPolicy,
  teamMemberNameOptions,
  teamSelectedOrchestrationPresetId,
  teamSelectedRecoveryPresetId,
  teamSessionDetail,
})
const {
  assistantProfileRegistryReady,
  handleAssistantContextModeChange,
  handleAssistantProfileChange,
  handleAssistantRunModeChange,
  loadConversationBinding,
  schedulePersistConversationBinding,
} = useAgentConversationBinding({
  conversationId,
  activeTeamSessionId,
  assistantSelectedModel,
  defaultAssistantProfileId,
  defaultToolConfig,
  toolConfig,
  toolsEnabled,
  teamSelectedOrchestrationPresetId,
  teamSelectedRecoveryPresetId,
  applyConversationBinding,
  applyProfilePreset,
  getAssistantProfileOption,
  handleToggleTeamMode,
  resetSessionSettings,
  setAssistantSelectedModel,
  setContextMode,
  setProfileId,
  setRunMode,
  toConversationBinding,
})
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
  assistantModelOptions,
  assistantContextMode,
  assistantSelectedModel,
  buildToolConfig: () => toolConfig.value as unknown as UiToolConfigPayload,
  clearAgentMessages: () => {
    agentEvents.clearMessages()
  },
  clearDraftArtifacts,
  clearTasksForCurrentContext,
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
  forceTasks: props.showTasks,
  getFailedToClearConversationLabel: () => t('agent.failedToClearConversation'),
  getFailedToStopExecutionLabel: () => t('agent.failedToStopExecution'),
  getNewConversationTitle: () => `${t('agent.newConversationTitle')} ${new Date().toLocaleString()}`,
  getToolCallCompletedLabel: () => t('agent.toolCallCompleted'),
  getUnnamedConversationTitle: () => t('agent.newConversationTitle'),
  handleStopTeamState: applyTeamState,
  hydrateTaskHistory,
  historyLoadToken,
  inputValue,
  isHistoryLoading,
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
  referencedFiles,
  referencedMessages,
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

const handleSelectConversation = async (convId: string) => {
  const previousConversationId = conversationId.value
  await handleSelectConversationFlow(convId)
  const nextConversationId = String(conversationId.value || '').trim()
  if (nextConversationId && nextConversationId !== String(previousConversationId || '').trim()) {
    emit('conversation-changed', {
      previousConversationId: previousConversationId?.trim() || null,
      conversationId: nextConversationId,
      title: currentConversationTitle.value.trim() || null,
    })
  }
}

const handleCreateConversation = async (newConvId?: string) => {
  const previousConversationId = conversationId.value
  await handleCreateConversationFlow(newConvId)
  const nextConversationId = String(conversationId.value || '').trim()
  if (nextConversationId && nextConversationId !== String(previousConversationId || '').trim()) {
    emit('conversation-changed', {
      previousConversationId: previousConversationId?.trim() || null,
      conversationId: nextConversationId,
      title: currentConversationTitle.value.trim() || null,
    })
  }
  nextTick(() => {
    inputAreaRef.value?.focusInput()
  })
}
useAgentViewLifecycle({
  activeTeamSessionId,
  assistantProfileRegistryReady,
  conversationExecutionState,
  conversationId,
  executionId: props.executionId,
  isTeamWorkspaceActive,
  loadAssistantModelOptions,
  loadAssistantProfiles,
  loadDefaultAssistantProfile,
  loadConversationHistory,
  loadLatestConversation,
  loadSidebarWidth,
  loadToolConfigDrawerWidth,
  loadTeamWorkspaceData,
  loadToolConfig,
  focusInput: () => inputAreaRef.value?.focusInput(),
  handleConversationExecutionStateUpdate,
  handleTeamAssistantMessageSaved,
  handleTeamExecutionFinished,
  handleTeamMessageStreamDelta,
  handleTeamMessageStreamDone,
  handleTeamMessageStreamStart,
  handleTeamToolCall,
  handleTeamToolResult,
  preconnectTerminal: () => terminalComposable.preconnect(),
  scrollMessageViewportToBottom,
  syncTeamMessagesToMainFlow,
  applyTeamState,
})

watch(
  [assistantProfileRegistryReady, conversationId],
  ([ready, value]) => {
    if (!ready) return
    void loadConversationBinding(value)
  },
  { immediate: true },
)

watch(
  [
    conversationId,
    ragEnabled,
    webSearchEnabled,
    tenthManEnabled,
    teamModeEnabled,
    assistantSelectedModel,
    toolsEnabled,
    toolConfig,
  ],
  () => {
    schedulePersistConversationBinding()
  },
)

watch(error, (value) => {
  const normalized = typeof value === 'string' ? value.trim() : ''
  if (!normalized) {
    lastAutoOpenedVisionError.value = null
    return
  }
  if (!isVisionModelUnsupportedError(normalized)) return
  if (lastAutoOpenedVisionError.value === normalized) return
  lastAutoOpenedVisionError.value = normalized
  openToolConfigDrawer()
})

const { updateSessionTitle } = useAgentSessionManager()
useAgentViewEffects({
  conversationId,
  conversationExecutionState,
  currentConversationTitle,
  getNewConversationTitle: () => t('agent.newConversationTitle'),
  selectedTeamTaskId,
  setMirroredConversationMessageIds,
  syncActiveTeamSession,
  teamTasks,
  updateSessionTitle,
})

// Expose methods
defineExpose({
  clearMessages: agentEvents.clearMessages,
  scrollToBottom: () => scrollMessageViewportToBottom(),
  addReferencedTraffic,
  addReferencedAssets,
  addReferencedFiles,
  loadConversationHistory,
  conversationId,
  focusInput: () => inputAreaRef.value?.focusInput(),
})
</script>

<style scoped>
.agent-view {
  font-family: var(--app-font-sans);
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

.tool-config-drawer {
  max-width: calc(100vw - 1rem);
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
  
  .task-sidebar {
    width: 100%;
    border-left: none;
    border-top: 1px solid hsl(var(--b3));
    max-height: 200px;
  }

  .conversation-drawer {
    width: 85vw !important;
    max-width: 320px;
  }

  .tool-config-drawer {
    width: 100% !important;
    max-width: none;
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
