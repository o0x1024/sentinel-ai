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
            v-if="teamModeEnabled"
            @click="handleToggleTeamWorkspace()"
            class="btn btn-sm gap-1"
            :class="isTeamWorkspaceActive ? 'btn-primary' : 'btn-ghost text-primary'"
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
            :team-enabled="teamModeEnabled"
            :pending-attachments="pendingAttachments"
            :pending-documents="pendingDocuments"
            :processed-documents="processedDocuments"
            :referenced-traffic="referencedTraffic"
            :context-usage="contextUsage"
            :available-models="assistantModelOptions"
            :selected-model="assistantSelectedModel"
            :model-loading="isLoadingAssistantModels"
            :team-template-options="teamTemplateOptions"
            :selected-team-template="selectedTeamTemplateId"
            :team-template-loading="isLoadingTeamTemplates"
            @send-message="handleSubmit"
            @stop-execution="handleStop"
            @toggle-rag="handleToggleRAG"
            @toggle-tools="handleToggleTools"
            @toggle-team="handleToggleTeamMode"
            @change-model="handleAssistantModelChange"
            @change-team-template="handleTeamTemplateChange"
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
          v-if="isWebExplorerActive || isTodosPanelActive || isHtmlPanelActive || isTerminalActive || isAuditFindingsPanelActive || isTeamWorkspaceActive"
          class="sidebar-container flex-shrink-0 border-l border-base-300 flex flex-col overflow-hidden bg-base-100 relative"
          :style="{ width: sidebarWidth + 'px' }"
        >
            <!-- Resize Handle -->
            <div 
              class="resize-handle absolute left-0 top-0 bottom-0 w-1 cursor-col-resize hover:bg-primary/50 transition-colors z-10"
              @mousedown="startResize"
            ></div>
            
            <div v-if="isTeamWorkspaceActive" class="h-full flex flex-col overflow-hidden">
              <div class="flex border-b border-base-300 overflow-x-auto">
                <button
                  class="flex-1 py-2 text-xs font-medium transition-colors whitespace-nowrap px-2"
                  :class="teamWorkspaceTab === 'templates' ? 'text-primary border-b-2 border-primary bg-primary/5' : 'text-base-content/50 hover:text-base-content'"
                  @click="teamWorkspaceTab = 'templates'"
                >
                  <i class="fas fa-layer-group mr-1"></i> 模板库
                </button>
                <button
                  class="flex-1 py-2 text-xs font-medium transition-colors whitespace-nowrap px-2"
                  :class="teamWorkspaceTab === 'blackboard' ? 'text-secondary border-b-2 border-secondary bg-secondary/5' : 'text-base-content/50 hover:text-base-content'"
                  @click="teamWorkspaceTab = 'blackboard'"
                >
                  <i class="fas fa-chalkboard mr-1"></i> 白板
                </button>
                <button
                  class="flex-1 py-2 text-xs font-medium transition-colors whitespace-nowrap px-2"
                  :class="teamWorkspaceTab === 'artifacts' ? 'text-accent border-b-2 border-accent bg-accent/5' : 'text-base-content/50 hover:text-base-content'"
                  @click="teamWorkspaceTab = 'artifacts'"
                >
                  <i class="fas fa-file-alt mr-1"></i> 产物
                </button>
                <button
                  class="flex-1 py-2 text-xs font-medium transition-colors whitespace-nowrap px-2"
                  :class="teamWorkspaceTab === 'timeline' ? 'text-info border-b-2 border-info bg-info/5' : 'text-base-content/50 hover:text-base-content'"
                  @click="teamWorkspaceTab = 'timeline'"
                >
                  <i class="fas fa-stream mr-1"></i> 时间线
                </button>
                <button
                  class="flex-1 py-2 text-xs font-medium transition-colors whitespace-nowrap px-2"
                  :class="teamWorkspaceTab === 'challenge' ? 'text-warning border-b-2 border-warning bg-warning/5' : 'text-base-content/50 hover:text-base-content'"
                  @click="teamWorkspaceTab = 'challenge'"
                >
                  <i class="fas fa-code-compare mr-1"></i> 对比
                </button>
              </div>

              <div v-if="teamWorkspaceLoading" class="flex-1 flex items-center justify-center text-sm text-base-content/50">
                <i class="fas fa-spinner fa-spin mr-2"></i> Team 工作台加载中...
              </div>

              <AgentTeamTemplateLibrary
                v-else-if="teamWorkspaceTab === 'templates'"
                :conversation-id="conversationId || undefined"
                class="flex-1 overflow-hidden"
                @close="isTeamWorkspaceActive = false"
                @templates-updated="handleTeamTemplatesUpdated"
                @session-created="handleTeamSessionCreated"
              />
              <AgentTeamBlackboardPanel
                v-else-if="teamWorkspaceTab === 'blackboard'"
                class="flex-1 overflow-hidden"
                :entries="teamBlackboardEntries"
                :archive-entry-id="teamBlackboardArchiveEntryId"
                :archive-messages="teamBlackboardArchiveMessages"
                :archive-scope="teamBlackboardArchiveScope"
                :archive-loading="teamBlackboardArchiveLoading"
                :archive-error="teamBlackboardArchiveError"
                :can-annotate="true"
                @resolve="handleTeamResolveBlackboardEntry"
                @view-archive="handleTeamViewBlackboardArchive"
                @add-entry="handleTeamAddBlackboardEntry"
                @annotate="handleTeamAnnotateBlackboardEntry"
              />
              <AgentTeamArtifactPanel
                v-else-if="teamWorkspaceTab === 'artifacts'"
                class="flex-1 overflow-hidden"
                :artifacts="teamArtifacts"
              />
              <AgentTeamTimeline
                v-else-if="teamWorkspaceTab === 'timeline'"
                class="flex-1 overflow-hidden"
                :rounds="teamRounds"
                :messages="teamSessionMessages"
                :is-running="isTeamRunActive"
              />
              <AgentTeamChallengeSplitView
                v-else-if="teamWorkspaceTab === 'challenge'"
                class="flex-1 overflow-hidden"
                :messages="teamSessionMessages"
                :round-id="latestTeamRoundId"
                :divergence-score="latestTeamDivergence"
                :threshold="0.6"
              />
              <div v-else class="flex-1 overflow-auto p-4">
                <div class="rounded-xl border border-base-300 bg-base-100 p-4 text-sm text-base-content/70">
                  编排入口已迁移到模板库。
                  请在「模板库 → 编辑模板」中配置 `orchestration_plan`。
                </div>
              </div>
            </div>

            <WebExplorerPanel 
               v-else-if="isWebExplorerActive"
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
import type {
  AgentTeamTemplate,
  AgentTeamRound,
  AgentTeamMessage,
  AgentTeamArtifact,
  AgentTeamBlackboardEntry,
  AgentTeamBlackboardArchive,
  AgentTeamSession,
  AgentTeamRoundEvent,
  AgentTeamArtifactEvent,
  AgentTeamStateChangedEvent,
} from '@/types/agentTeam'
import { agentTeamApi } from '@/api/agentTeam'
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
import AgentTeamTemplateLibrary from './AgentTeamTemplateLibrary.vue'
import AgentTeamBlackboardPanel from './AgentTeamBlackboardPanel.vue'
import AgentTeamArtifactPanel from './AgentTeamArtifactPanel.vue'
import AgentTeamTimeline from './AgentTeamTimeline.vue'
import AgentTeamChallengeSplitView from './AgentTeamChallengeSplitView.vue'
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

interface TeamTemplateOption {
  value: string
  label: string
  description?: string
}

type TeamOrchestrationStepType = 'agent' | 'serial' | 'parallel'

interface TeamOrchestrationRetry {
  max_attempts?: number
  backoff_ms?: number
}

interface TeamOrchestrationStep {
  id: string
  type: TeamOrchestrationStepType
  name?: string
  member?: string
  phase?: string
  instruction?: string
  retry?: TeamOrchestrationRetry
  children?: TeamOrchestrationStep[]
}

interface TeamOrchestrationPlan {
  version: number
  steps: TeamOrchestrationStep[]
}

interface TeamStepMovePayload {
  sourcePath: number[]
  targetPath: number[]
  mode: 'before' | 'inside'
}

interface TeamRuntimeStepStat {
  step_id: string
  total_attempts: number
  success_count: number
  failure_count: number
  avg_duration_ms: number
  last_duration_ms?: number
  last_status?: string
  last_error?: string
}

interface TeamRuntimeFailureMode {
  mode: string
  count: number
  latest_step_id?: string
  latest_error?: string
  hint?: string
}

type TeamOrchestrationPresetId =
  | 'product_delivery_chain'
  | 'security_audit_matrix'
  | 'incident_response_flow'

type TeamRecoveryPresetId = 'conservative' | 'balanced' | 'aggressive'

interface TeamOrchestrationPresetMeta {
  id: TeamOrchestrationPresetId
  label: string
  description: string
}

interface TeamRecoveryPreset {
  id: TeamRecoveryPresetId
  label: string
  description: string
  max_attempts: number
  backoff_ms: number
  human_intervention_timeout_secs: number
  max_human_interventions: number
  no_human_input_policy: TeamRecoveryPresetId
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
const historyLoadToken = ref(0)

// Feature toggles
const ragEnabled = ref(false)
const toolsEnabled = ref(false)
const teamModeEnabled = ref(false)
const tenthManEnabled = ref(false)
const pendingAttachments = ref<any[]>([])
const pendingDocuments = ref<PendingDocumentAttachment[]>([])
const processedDocuments = ref<ProcessedDocumentResult[]>([])
const referencedTraffic = ref<ReferencedTraffic[]>([])
const activeTeamSessionId = ref<string | null>(null)
const teamSessionState = ref<string>('PENDING')
const isTeamWorkspaceActive = ref(false)
const teamWorkspaceTab = ref<'templates' | 'blackboard' | 'artifacts' | 'timeline' | 'challenge'>('templates')
const teamWorkspaceLoading = ref(false)
const teamSessionMessages = ref<AgentTeamMessage[]>([])
const teamRounds = ref<AgentTeamRound[]>([])
const teamBlackboardEntries = ref<AgentTeamBlackboardEntry[]>([])
const teamBlackboardArchiveEntryId = ref<string | null>(null)
const teamBlackboardArchiveMessages = ref<AgentTeamMessage[]>([])
const teamBlackboardArchiveScope = ref<string | null>(null)
const teamBlackboardArchiveLoading = ref(false)
const teamBlackboardArchiveError = ref<string | null>(null)
const teamArtifacts = ref<AgentTeamArtifact[]>([])
const teamSessionDetail = ref<AgentTeamSession | null>(null)
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
const teamTemplateDraftName = ref('')
const teamTemplateDraftDomain = ref<'product' | 'security' | 'ops' | 'audit' | 'custom'>('custom')
const teamTemplateDraftSaving = ref(false)
const teamTemplateOptions = ref<TeamTemplateOption[]>([])
const selectedTeamTemplateId = ref('')
const isLoadingTeamTemplates = ref(false)
const assistantModelOptions = ref<AssistantModelOption[]>([])
const assistantSelectedModel = ref('')
const isLoadingAssistantModels = ref(false)
const isSubagentPanelOpen = ref(false)
const subagents = computed(() => agentEvents.subagents.value)

const TEAM_ORCHESTRATION_PRESET_METAS: TeamOrchestrationPresetMeta[] = [
  {
    id: 'product_delivery_chain',
    label: '需求到交付',
    description: '产品 -> 架构 -> 研发/测试并行 -> 发布决策，适合 idea 到落地全链路。',
  },
  {
    id: 'security_audit_matrix',
    label: '安全审计矩阵',
    description: '多维安全审计并行 -> 风险收敛 -> 修复验证，适合代码/系统安全评估。',
  },
  {
    id: 'incident_response_flow',
    label: '故障处置链路',
    description: '故障接管 -> 并行根因分析 -> 处置方案 -> 验证复盘，适合线上异常场景。',
  },
]

const TEAM_RECOVERY_PRESETS: TeamRecoveryPreset[] = [
  {
    id: 'conservative',
    label: '保守',
    description: '低重试、长退避、较长人工等待窗口，优先稳定性与可回滚性。',
    max_attempts: 1,
    backoff_ms: 1500,
    human_intervention_timeout_secs: 900,
    max_human_interventions: 4,
    no_human_input_policy: 'conservative',
  },
  {
    id: 'balanced',
    label: '平衡',
    description: '中等重试与等待窗口，在质量、风险和进度间折中。',
    max_attempts: 2,
    backoff_ms: 800,
    human_intervention_timeout_secs: 600,
    max_human_interventions: 3,
    no_human_input_policy: 'balanced',
  },
  {
    id: 'aggressive',
    label: '激进',
    description: '高重试、短退避、短等待窗口，优先推进速度与产出。',
    max_attempts: 3,
    backoff_ms: 400,
    human_intervention_timeout_secs: 300,
    max_human_interventions: 2,
    no_human_input_policy: 'aggressive',
  },
]

const TEAM_TEMPLATE_DOMAIN_OPTIONS = ['product', 'security', 'ops', 'audit', 'custom'] as const

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
const TEAM_TEMPLATE_STORAGE_KEY = 'sentinel:agent:team-template-id'
let unlistenAiConfigUpdated: UnlistenFn | null = null
let unlistenTeamStateChanged: UnlistenFn | null = null
let unlistenTeamRoundCompleted: UnlistenFn | null = null
let unlistenTeamArtifactGenerated: UnlistenFn | null = null
let teamRunStatusPollTimer: ReturnType<typeof setInterval> | null = null
let isPollingTeamRunStatus = false
let teamMainFlowMessageIds = new Set<string>()

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

const formatDurationMs = (value: number | undefined) => {
  const ms = Number(value ?? 0)
  if (!Number.isFinite(ms) || ms <= 0) return '0ms'
  if (ms < 1000) return `${Math.floor(ms)}ms`
  if (ms < 60_000) return `${(ms / 1000).toFixed(1)}s`
  return `${(ms / 60_000).toFixed(1)}m`
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

const formatTeamTemplateTag = (template: AgentTeamTemplate): string => {
  const domain = (template.domain || 'custom').trim() || 'custom'
  return `${domain} · ${template.is_system ? '内置' : '自定义'}`
}

const setSelectedTeamTemplate = (value: string) => {
  const normalized = value || ''
  selectedTeamTemplateId.value = normalized
  try {
    if (normalized) {
      localStorage.setItem(TEAM_TEMPLATE_STORAGE_KEY, normalized)
    } else {
      localStorage.removeItem(TEAM_TEMPLATE_STORAGE_KEY)
    }
  } catch {
    // ignore storage errors
  }
}

const loadTeamTemplateOptions = async (preferredTemplateId = ''): Promise<string> => {
  isLoadingTeamTemplates.value = true
  try {
    let templates = await agentTeamApi.listTemplates()
    if (templates.length === 0) {
      await agentTeamApi.seedBuiltinTemplates()
      templates = await agentTeamApi.listTemplates()
    }

    const options: TeamTemplateOption[] = templates
      .map((template) => ({
        value: template.id,
        label: template.name,
        description: formatTeamTemplateTag(template),
      }))
      .sort((a, b) => a.label.localeCompare(b.label, 'zh-CN'))

    teamTemplateOptions.value = options

    let stored = ''
    try {
      stored = localStorage.getItem(TEAM_TEMPLATE_STORAGE_KEY) || ''
    } catch {
      stored = ''
    }

    const preferred = preferredTemplateId || selectedTeamTemplateId.value || stored
    const resolved = preferred && options.some((item) => item.value === preferred)
      ? preferred
      : (options[0]?.value || '')
    setSelectedTeamTemplate(resolved)
    return resolved
  } catch (e) {
    console.warn('[AgentView] Failed to load team template options:', e)
    teamTemplateOptions.value = []
    setSelectedTeamTemplate('')
    return ''
  } finally {
    isLoadingTeamTemplates.value = false
  }
}

const ensureTeamTemplateIdReady = async (): Promise<string> => {
  const selected = selectedTeamTemplateId.value.trim()
  if (teamTemplateOptions.value.length > 0) {
    if (selected && teamTemplateOptions.value.some((item) => item.value === selected)) {
      return selected
    }
    const fallback = teamTemplateOptions.value[0]?.value || ''
    if (fallback) {
      setSelectedTeamTemplate(fallback)
      return fallback
    }
  }
  return await loadTeamTemplateOptions(selected)
}

const handleTeamTemplateChange = (value: string) => {
  setSelectedTeamTemplate(value || '')
}

function extractTemplateMaxRounds(config: unknown): number | undefined {
  if (typeof config === 'number') {
    const n = Math.max(1, Math.floor(config))
    return Number.isFinite(n) ? n : undefined
  }
  if (config && typeof config === 'object' && !Array.isArray(config)) {
    const obj = config as Record<string, unknown>
    const candidate = Number(obj.max_rounds ?? obj.maxRounds ?? obj.default_rounds ?? obj.rounds)
    if (Number.isFinite(candidate) && candidate > 0) {
      return Math.floor(candidate)
    }
  }
  return undefined
}

function extractTemplateOrchestrationPlan(config: unknown): Record<string, any> | undefined {
  if (!config || typeof config !== 'object' || Array.isArray(config)) return undefined
  const obj = config as Record<string, unknown>
  const raw = obj.orchestration_plan ?? obj.orchestrationPlan ?? obj.plan
  if (!raw || typeof raw !== 'object' || Array.isArray(raw)) return undefined
  const cloned = JSON.parse(JSON.stringify(raw))
  const steps = Array.isArray(cloned?.steps) ? cloned.steps : []
  if (steps.length === 0) return undefined
  return cloned
}

function extractTemplatePlanVersion(config: unknown): number | undefined {
  if (!config || typeof config !== 'object' || Array.isArray(config)) return undefined
  const obj = config as Record<string, unknown>
  const candidate = Number(obj.plan_version ?? obj.planVersion ?? obj.version)
  if (!Number.isFinite(candidate) || candidate <= 0) return undefined
  return Math.floor(candidate)
}

function extractTemplateStateMachine(toolPolicy: unknown): Record<string, any> | undefined {
  if (!toolPolicy || typeof toolPolicy !== 'object' || Array.isArray(toolPolicy)) return undefined
  const policyObj = toolPolicy as Record<string, unknown>
  const raw =
    policyObj.state_machine && typeof policyObj.state_machine === 'object' && !Array.isArray(policyObj.state_machine)
      ? policyObj.state_machine
      : undefined
  if (!raw) return undefined
  return JSON.parse(JSON.stringify(raw as Record<string, unknown>))
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
const TEAM_NON_RUNNING_STATES = new Set(['PENDING', 'SUSPENDED_FOR_HUMAN', 'COMPLETED', 'FAILED', 'ARCHIVED'])
const TEAM_RUN_STATUS_POLL_INTERVAL_MS = 2000
const isTeamRunActive = computed(() => {
  if (!teamModeEnabled.value || !activeTeamSessionId.value) return false
  return !TEAM_NON_RUNNING_STATES.has(teamSessionState.value || '')
})
const isExecuting = computed(() => agentEvents.isExecuting.value || isTeamRunActive.value)
const isStreaming = computed(() => agentEvents.isExecuting.value && !!agentEvents.streamingContent.value)
const streamingContent = computed(() => agentEvents.streamingContent.value)
const contextUsage = computed(() => agentEvents.contextUsage.value)
const teamWorkspaceBadgeCount = computed(
  () => teamBlackboardEntries.value.length + teamArtifacts.value.length,
)
const latestTeamRoundId = computed(() => {
  if (teamRounds.value.length === 0) return null
  return teamRounds.value[teamRounds.value.length - 1]?.id || null
})
const latestTeamDivergence = computed(() => {
  for (let idx = teamRounds.value.length - 1; idx >= 0; idx -= 1) {
    const score = teamRounds.value[idx]?.divergence_score
    if (typeof score === 'number') return score
  }
  return null
})
const teamOrchestrationRuntime = computed<Record<string, any>>(() => {
  const raw = teamSessionDetail.value?.state_machine?.orchestration_runtime
  if (raw && typeof raw === 'object') return raw as Record<string, any>
  return {}
})
const teamMemberNameOptions = computed(() =>
  (teamSessionDetail.value?.members || [])
    .map((member) => member.name)
    .filter((name) => typeof name === 'string' && name.trim().length > 0),
)
const teamOrchestrationPresets = computed(() => TEAM_ORCHESTRATION_PRESET_METAS)
const teamRecoveryPresets = computed(() => TEAM_RECOVERY_PRESETS)
const teamTemplateDomainOptions = computed(() => TEAM_TEMPLATE_DOMAIN_OPTIONS)
const teamSelectedOrchestrationPresetDescription = computed(() => {
  if (!teamSelectedOrchestrationPresetId.value) return ''
  return TEAM_ORCHESTRATION_PRESET_METAS.find((item) => item.id === teamSelectedOrchestrationPresetId.value)?.description || ''
})
const teamSelectedRecoveryPresetDescription = computed(() => {
  const selected = TEAM_RECOVERY_PRESETS.find((item) => item.id === teamSelectedRecoveryPresetId.value)
  return selected?.description || ''
})
const teamCurrentNoHumanInputPolicy = computed<TeamRecoveryPresetId>(() => {
  const fallback = 'balanced'
  const stateMachine = teamSessionDetail.value?.state_machine
  if (!stateMachine || typeof stateMachine !== 'object') return fallback
  const fromIntervention = (stateMachine as any)?.human_intervention?.policy
  const fromRoot = (stateMachine as any)?.no_human_input_policy
  const raw = typeof fromIntervention === 'string' ? fromIntervention : fromRoot
  if (raw === 'conservative' || raw === 'aggressive') return raw
  return fallback
})
const teamCurrentHumanInterventionTimeoutSecs = computed(() => {
  const stateMachine = teamSessionDetail.value?.state_machine
  const fromIntervention = Number((stateMachine as any)?.human_intervention?.timeout_secs)
  if (Number.isFinite(fromIntervention) && fromIntervention > 0) return Math.floor(fromIntervention)
  const fromRoot = Number((stateMachine as any)?.human_intervention_timeout_secs)
  if (Number.isFinite(fromRoot) && fromRoot > 0) return Math.floor(fromRoot)
  return 600
})
const teamCurrentMaxHumanInterventions = computed(() => {
  const stateMachine = teamSessionDetail.value?.state_machine
  const value = Number((stateMachine as any)?.max_human_interventions)
  if (Number.isFinite(value) && value > 0) return Math.floor(value)
  return 3
})
const teamFlattenedStepOptions = computed(() => {
  const options: Array<{ id: string; path: string; label: string }> = []
  const walk = (steps: TeamOrchestrationStep[], prefix: number[]) => {
    steps.forEach((step, idx) => {
      const path = [...prefix, idx]
      const pathLabel = path.map((part) => part + 1).join('.')
      const title = step.name?.trim() || step.phase?.trim() || step.type
      options.push({
        id: step.id,
        path: pathLabel,
        label: `${step.id} (${pathLabel}) · ${title}`,
      })
      if (Array.isArray(step.children) && step.children.length > 0) {
        walk(step.children, path)
      }
    })
  }
  walk(teamOrchestrationDraft.value.steps, [])
  return options
})
const teamStepPathById = computed(() => {
  const pathMap = new Map<string, string>()
  teamFlattenedStepOptions.value.forEach((item) => {
    if (!pathMap.has(item.id)) {
      pathMap.set(item.id, item.path)
    }
  })
  return pathMap
})
const teamLastRuntimeStepPath = computed(() => {
  const lastStepId = teamOrchestrationRuntime.value.last_step_id
  if (typeof lastStepId !== 'string' || !lastStepId.trim()) return '-'
  return teamStepPathById.value.get(lastStepId) || '-'
})
const teamRuntimeSummary = computed(() => {
  const raw = teamOrchestrationRuntime.value.summary
  const totalAttempts = Number(raw?.total_attempts ?? 0)
  const totalSuccess = Number(raw?.total_success ?? 0)
  const totalFailed = Number(raw?.total_failed ?? 0)
  const slowestDurationMs = Number(raw?.slowest_duration_ms ?? 0)
  const slowestStepId = typeof raw?.slowest_step_id === 'string' ? raw.slowest_step_id : ''
  return {
    totalAttempts: Number.isFinite(totalAttempts) ? Math.max(0, Math.floor(totalAttempts)) : 0,
    totalSuccess: Number.isFinite(totalSuccess) ? Math.max(0, Math.floor(totalSuccess)) : 0,
    totalFailed: Number.isFinite(totalFailed) ? Math.max(0, Math.floor(totalFailed)) : 0,
    slowestDurationMs: Number.isFinite(slowestDurationMs) ? Math.max(0, Math.floor(slowestDurationMs)) : 0,
    slowestStepId,
  }
})
const teamRuntimeSuggestedResumeStepId = computed(() => {
  const value = teamOrchestrationRuntime.value.suggested_resume_step_id
  return typeof value === 'string' ? value : ''
})
const teamRuntimeStepStats = computed<TeamRuntimeStepStat[]>(() => {
  const raw = teamOrchestrationRuntime.value.step_stats
  if (!raw || typeof raw !== 'object') return []
  return Object.entries(raw)
    .map(([stepId, value]) => {
      const v = value as Record<string, any>
      const toNum = (n: any) => {
        const parsed = Number(n ?? 0)
        return Number.isFinite(parsed) ? Math.max(0, Math.floor(parsed)) : 0
      }
      return {
        step_id: stepId,
        total_attempts: toNum(v.total_attempts),
        success_count: toNum(v.success_count),
        failure_count: toNum(v.failure_count),
        avg_duration_ms: toNum(v.avg_duration_ms),
        last_duration_ms: toNum(v.last_duration_ms),
        last_status: typeof v.last_status === 'string' ? v.last_status : '',
        last_error: typeof v.last_error === 'string' ? v.last_error : '',
      }
    })
    .sort((a, b) => b.failure_count - a.failure_count || b.avg_duration_ms - a.avg_duration_ms)
})
const teamRuntimeHotspots = computed(() => teamRuntimeStepStats.value.slice(0, 8))
const teamRuntimeFailureModes = computed<TeamRuntimeFailureMode[]>(() => {
  const raw = teamOrchestrationRuntime.value.failure_modes
  if (!raw || typeof raw !== 'object') return []
  return Object.entries(raw)
    .map(([mode, value]) => {
      const v = value as Record<string, any>
      const countRaw = Number(v.count ?? 0)
      const count = Number.isFinite(countRaw) ? Math.max(0, Math.floor(countRaw)) : 0
      return {
        mode,
        count,
        latest_step_id: typeof v.latest_step_id === 'string' ? v.latest_step_id : '',
        latest_error: typeof v.latest_error === 'string' ? v.latest_error : '',
        hint: typeof v.hint === 'string' ? v.hint : '',
      }
    })
    .sort((a, b) => b.count - a.count)
})
const teamRuntimeBackendRecoverySuggestions = computed(() => {
  const raw = teamOrchestrationRuntime.value.recovery_suggestions
  if (!Array.isArray(raw)) return []
  return raw
    .filter((item) => typeof item === 'string')
    .map((item) => String(item).trim())
    .filter((item) => item.length > 0)
})
const teamRuntimeRecoverySuggestions = computed(() => {
  const hints: string[] = [...teamRuntimeBackendRecoverySuggestions.value]
  const seen = new Set(hints)
  const pushHint = (msg: string) => {
    const normalized = msg.trim()
    if (!normalized) return
    if (seen.has(normalized)) return
    seen.add(normalized)
    hints.push(normalized)
  }
  if (teamRuntimeSummary.value.totalFailed > 0 && teamRuntimeSuggestedResumeStepId.value) {
    pushHint(`优先从失败节点 ${teamRuntimeSuggestedResumeStepId.value} 恢复执行。`)
  }
  const frequentFailure = teamRuntimeStepStats.value.find((item) => item.failure_count >= 3)
  if (frequentFailure) {
    pushHint(`节点 ${frequentFailure.step_id} 连续失败较多，建议提高 backoff 或拆分任务。`)
  }
  if (teamRuntimeSummary.value.slowestDurationMs >= 120000 && teamRuntimeSummary.value.slowestStepId) {
    pushHint(`慢节点 ${teamRuntimeSummary.value.slowestStepId} 耗时较长，建议拆分或并行化。`)
  }
  if (hints.length === 0) {
    hints.push('当前执行稳定，可继续按既定编排运行。')
  }
  return hints
})

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
  isTeamWorkspaceActive.value = false
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
    isTeamWorkspaceActive.value = false
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
    isTeamWorkspaceActive.value = false
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
    isTeamWorkspaceActive.value = false
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
    isTeamWorkspaceActive.value = false
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
    isTeamWorkspaceActive.value = false
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

const appendTeamBridgeMessage = (content: string) => {
  const normalized = (content || '').replace(/^\s*\[Team\]\s*/i, '').trim()
  agentEvents.messages.value.push({
    id: crypto.randomUUID(),
    type: 'system',
    content: normalized || content,
    timestamp: Date.now(),
    metadata: {
      kind: 'team_bridge',
    },
  })
}

const mapTeamMessageType = (role: string): AgentMessage['type'] => {
  const normalized = (role || '').toLowerCase()
  if (normalized === 'user') return 'user'
  if (normalized === 'system') return 'system'
  if (normalized === 'assistant') return 'final'
  return 'system'
}

const parseTeamMessageTimestamp = (raw: string): number => {
  const parsed = Date.parse(raw || '')
  return Number.isFinite(parsed) ? parsed : Date.now()
}

const appendTeamMessagesToMainFlow = (messagesResp: AgentTeamMessage[]) => {
  if (!Array.isArray(messagesResp) || messagesResp.length === 0) return
  const sorted = [...messagesResp].sort((a, b) => {
    const ta = parseTeamMessageTimestamp(a.timestamp)
    const tb = parseTeamMessageTimestamp(b.timestamp)
    if (ta !== tb) return ta - tb
    return a.id.localeCompare(b.id)
  })
  for (const msg of sorted) {
    if (!msg?.id || teamMainFlowMessageIds.has(msg.id)) continue
    if (!(msg.content || '').trim()) continue
    teamMainFlowMessageIds.add(msg.id)
    agentEvents.messages.value.push({
      id: `team:${msg.id}`,
      type: mapTeamMessageType(msg.role),
      content: msg.content,
      timestamp: parseTeamMessageTimestamp(msg.timestamp),
      metadata: {
        kind: 'team_member_output',
      },
    })
  }
}

const syncTeamMessagesToMainFlow = async (sessionId?: string | null) => {
  const sid = sessionId || activeTeamSessionId.value
  if (!sid || !teamModeEnabled.value) return
  try {
    const messagesResp = await agentTeamApi.getMessages(sid)
    if (!messagesResp || activeTeamSessionId.value !== sid) return
    teamSessionMessages.value = messagesResp
    appendTeamMessagesToMainFlow(messagesResp)
  } catch (e) {
    console.warn('[AgentView] Failed to sync team messages to main flow:', e)
  }
}

const applyTeamState = (nextState: string) => {
  const normalized = (nextState || '').trim()
  if (!normalized) return false
  const prev = teamSessionState.value
  if (prev === normalized) return false
  teamSessionState.value = normalized
  if (normalized === 'COMPLETED') {
    appendTeamBridgeMessage('[Team] 会话执行完成。')
  } else if (normalized === 'FAILED') {
    appendTeamBridgeMessage('[Team] 会话执行失败。')
  } else if (normalized === 'SUSPENDED_FOR_HUMAN') {
    appendTeamBridgeMessage('[Team] 需要人工介入，请继续输入指导意见。')
  }
  return true
}

const stopTeamRunStatusPolling = () => {
  if (teamRunStatusPollTimer) {
    clearInterval(teamRunStatusPollTimer)
    teamRunStatusPollTimer = null
  }
  isPollingTeamRunStatus = false
}

const pollTeamRunStatusOnce = async () => {
  if (isPollingTeamRunStatus) return
  const sessionId = activeTeamSessionId.value
  if (!teamModeEnabled.value || !sessionId) return
  isPollingTeamRunStatus = true
  try {
    await syncTeamMessagesToMainFlow(sessionId)
    const status = await agentTeamApi.getRunStatus(sessionId)
    if (!status || activeTeamSessionId.value !== sessionId) return
    if (typeof status.state === 'string' && status.state.length > 0) {
      const changed = applyTeamState(status.state)
      if (changed && isTeamWorkspaceActive.value) {
        void loadTeamWorkspaceData()
      }
    }
  } catch (e) {
    console.warn('[AgentView] Failed to poll team run status:', e)
  } finally {
    isPollingTeamRunStatus = false
  }
}

const ensureTeamRunStatusPolling = () => {
  if (!teamModeEnabled.value || !activeTeamSessionId.value) {
    stopTeamRunStatusPolling()
    return
  }
  if (teamRunStatusPollTimer) return
  // Run once immediately to avoid waiting one polling interval.
  void pollTeamRunStatusOnce()
  teamRunStatusPollTimer = setInterval(() => {
    void pollTeamRunStatusOnce()
  }, TEAM_RUN_STATUS_POLL_INTERVAL_MS)
}

const syncActiveTeamSession = async () => {
  if (!conversationId.value) {
    activeTeamSessionId.value = null
    teamSessionState.value = 'PENDING'
    return
  }
  try {
    const sessions = await agentTeamApi.listSessions(conversationId.value, 20, 0)
    const candidate = sessions.find((item) => item.state !== 'ARCHIVED') || null
    activeTeamSessionId.value = candidate?.id || null
    teamSessionState.value = candidate?.state || 'PENDING'
    ensureTeamRunStatusPolling()
    if (candidate?.template_id) {
      setSelectedTeamTemplate(candidate.template_id)
    }
    if (isTeamWorkspaceActive.value) {
      await loadTeamWorkspaceData()
    }
  } catch (e) {
    console.warn('[AgentView] Failed to sync team session:', e)
    activeTeamSessionId.value = null
    teamSessionState.value = 'PENDING'
    stopTeamRunStatusPolling()
  }
}

const handleToggleTeamMode = async (enabled: boolean) => {
  if (!enabled && isTeamRunActive.value) {
    teamModeEnabled.value = true
    appendTeamBridgeMessage('[Team] 正在运行，停止后才能关闭 Team 模式。')
    return
  }
  teamModeEnabled.value = enabled
  if (enabled) {
    await loadTeamTemplateOptions(selectedTeamTemplateId.value)
    await syncActiveTeamSession()
    ensureTeamRunStatusPolling()
    if (isTeamWorkspaceActive.value) {
      await loadTeamWorkspaceData()
    }
  } else {
    stopTeamRunStatusPolling()
    isTeamWorkspaceActive.value = false
  }
}

const createAndStartTeamSession = async (goal: string) => {
  const templateId = await ensureTeamTemplateIdReady()
  if (!templateId) {
    throw new Error('No Team template available')
  }
  const template = await agentTeamApi.getTemplate(templateId)
  if (!template) {
    throw new Error('所选 Team 模板不存在，请重新选择模板。')
  }
  const orchestrationPlan = extractTemplateOrchestrationPlan(template.default_rounds_config)
  if (!orchestrationPlan) {
    throw new Error(`模板「${template.name}」未配置编排流程，请先在模板库编辑并保存工作流。`)
  }
  const planVersion = extractTemplatePlanVersion(template.default_rounds_config)
  const templateStateMachine = extractTemplateStateMachine(template.default_tool_policy)
  const maxRounds = extractTemplateMaxRounds(template.default_rounds_config)
  const session = await agentTeamApi.createSession({
    name: `${goal}`,
    goal,
    template_id: templateId,
    conversation_id: conversationId.value || undefined,
    max_rounds: maxRounds,
    orchestration_plan: orchestrationPlan,
    plan_version: planVersion,
    state_machine: {
      ...(templateStateMachine || {}),
      tool_policy: {
        ...((templateStateMachine as any)?.tool_policy || {}),
        enabled: toolsEnabled.value,
      },
    },
  })
  activeTeamSessionId.value = session.id
  teamSessionState.value = session.state
  ensureTeamRunStatusPolling()
  await agentTeamApi.startRun(session.id)
  await syncTeamMessagesToMainFlow(session.id)
  teamSessionState.value = 'INITIALIZING'
  appendTeamBridgeMessage(`${session.name}`)
  if (isTeamWorkspaceActive.value) {
    await loadTeamWorkspaceData()
  }
  return session
}

const routeTeamMessage = async (content: string) => {
  let currentSession = activeTeamSessionId.value
    ? await agentTeamApi.getSession(activeTeamSessionId.value)
    : null

  if (
    !currentSession ||
    currentSession.state === 'COMPLETED' ||
    currentSession.state === 'FAILED' ||
    currentSession.state === 'ARCHIVED'
  ) {
    await createAndStartTeamSession(content)
    return
  }

  await agentTeamApi.submitMessage({
    session_id: currentSession.id,
    content,
    resume: currentSession.state === 'SUSPENDED_FOR_HUMAN',
  })

  if (currentSession.state === 'PENDING') {
    if (!currentSession.orchestration_plan || !Array.isArray(currentSession.orchestration_plan.steps) || currentSession.orchestration_plan.steps.length === 0) {
      const templateId = currentSession.template_id || selectedTeamTemplateId.value
      if (templateId) {
        const template = await agentTeamApi.getTemplate(templateId)
        const templatePlan = extractTemplateOrchestrationPlan(template?.default_rounds_config)
        if (!templatePlan) {
          throw new Error('当前 Team 会话缺少编排流程，且模板未配置工作流。请到模板库先配置编排。')
        }
        await agentTeamApi.updateSession(currentSession.id, {
          orchestration_plan: templatePlan,
          plan_version: extractTemplatePlanVersion(template?.default_rounds_config),
        })
      } else {
        throw new Error('当前 Team 会话缺少编排流程，请重新选择模板后重试。')
      }
    }
    await agentTeamApi.startRun(currentSession.id)
    await syncTeamMessagesToMainFlow(currentSession.id)
    appendTeamBridgeMessage('[Team] 已启动待运行会话。')
    teamSessionState.value = 'INITIALIZING'
    if (isTeamWorkspaceActive.value) {
      await loadTeamWorkspaceData()
    }
    return
  }

  teamSessionState.value = currentSession.state
  if (currentSession.state === 'SUSPENDED_FOR_HUMAN') {
    appendTeamBridgeMessage('[Team] 已提交人工意见，团队继续执行。')
  } else {
    appendTeamBridgeMessage('[Team] 已追加需求到当前会话。')
  }
  if (isTeamWorkspaceActive.value) {
    await loadTeamWorkspaceData()
  }
}

const generateTeamStepId = (prefix = 'step') => {
  const random = Math.random().toString(36).slice(2, 7)
  return `${prefix}-${Date.now().toString(36)}-${random}`
}

const pickTeamMemberForPreset = (keywords: string[], fallbackIndex = 0) => {
  const options = teamMemberNameOptions.value
  if (options.length === 0) return ''
  const lowered = options.map((name) => name.toLowerCase())
  for (const keyword of keywords) {
    const target = keyword.toLowerCase()
    const hitIndex = lowered.findIndex((item) => item.includes(target))
    if (hitIndex >= 0) {
      return options[hitIndex]
    }
  }
  return options[Math.min(Math.max(fallbackIndex, 0), options.length - 1)] || options[0]
}

const createPresetAgentStep = (input: {
  prefix: string
  name: string
  phase: string
  instruction: string
  member: string
}): TeamOrchestrationStep => ({
  id: generateTeamStepId(input.prefix),
  type: 'agent',
  name: input.name,
  member: input.member,
  phase: input.phase,
  instruction: input.instruction,
  retry: { max_attempts: 1, backoff_ms: 800 },
})

const buildOrchestrationPresetPlan = (presetId: TeamOrchestrationPresetId): TeamOrchestrationPlan => {
  const version = Math.max(1, Number(teamOrchestrationDraft.value.version || 1))
  const pm = pickTeamMemberForPreset(['产品', 'product', 'pm'], 0)
  const architect = pickTeamMemberForPreset(['架构', 'architect'], 1)
  const engineer = pickTeamMemberForPreset(['研发', '开发', 'engineer', 'dev'], 2)
  const qa = pickTeamMemberForPreset(['测试', 'qa', 'quality'], 3)
  const security = pickTeamMemberForPreset(['安全', 'security'], 2)
  const sre = pickTeamMemberForPreset(['sre', '运维', 'ops'], 0)

  if (presetId === 'product_delivery_chain') {
    return {
      version,
      steps: [
        createPresetAgentStep({
          prefix: 'prd',
          name: '需求澄清',
          phase: 'requirements',
          member: pm,
          instruction: '明确目标用户、核心场景、验收标准与边界条件，沉淀为可执行需求。',
        }),
        createPresetAgentStep({
          prefix: 'arch',
          name: '架构设计',
          phase: 'design',
          member: architect || engineer || pm,
          instruction: '输出系统架构、模块边界、关键技术选型与风险控制点。',
        }),
        {
          id: generateTeamStepId('delivery'),
          type: 'parallel',
          name: '并行交付',
          phase: 'implementation_and_validation',
          children: [
            createPresetAgentStep({
              prefix: 'impl',
              name: '研发实现方案',
              phase: 'implementation',
              member: engineer || architect || pm,
              instruction: '拆解实现任务、接口契约与交付顺序，明确依赖与里程碑。',
            }),
            createPresetAgentStep({
              prefix: 'qa',
              name: '测试验证策略',
              phase: 'validation',
              member: qa || engineer || architect,
              instruction: '设计测试范围、关键用例、回归策略与上线前验证清单。',
            }),
          ],
        },
        createPresetAgentStep({
          prefix: 'gate',
          name: '发布决策',
          phase: 'release_gate',
          member: architect || pm || engineer,
          instruction: '综合风险、质量与收益做最终发布建议，并给出发布后观测指标。',
        }),
      ],
    }
  }

  if (presetId === 'security_audit_matrix') {
    return {
      version,
      steps: [
        {
          id: generateTeamStepId('audit'),
          type: 'parallel',
          name: '安全审计并行分析',
          phase: 'security_audit',
          children: [
            createPresetAgentStep({
              prefix: 'auth',
              name: '鉴权与会话审计',
              phase: 'auth_security',
              member: security || engineer || architect,
              instruction: '检查认证、授权、会话管理缺陷并给出风险等级。',
            }),
            createPresetAgentStep({
              prefix: 'deps',
              name: '依赖与供应链审计',
              phase: 'supply_chain_security',
              member: security || engineer || architect,
              instruction: '识别依赖漏洞、供应链风险与版本治理建议。',
            }),
            createPresetAgentStep({
              prefix: 'code',
              name: '代码与配置审计',
              phase: 'code_security',
              member: engineer || security || architect,
              instruction: '检查注入、越权、敏感配置与日志暴露等高风险问题。',
            }),
          ],
        },
        {
          id: generateTeamStepId('remediate'),
          type: 'serial',
          name: '风险收敛与修复',
          phase: 'remediation',
          children: [
            createPresetAgentStep({
              prefix: 'triage',
              name: '风险分级与排期',
              phase: 'triage',
              member: security || pm || architect,
              instruction: '汇总审计发现，按风险与业务影响排序并输出修复优先级。',
            }),
            createPresetAgentStep({
              prefix: 'fix',
              name: '修复方案设计',
              phase: 'fix_plan',
              member: engineer || architect || security,
              instruction: '为高优先问题提供可落地修复策略、代码改造点与回滚方案。',
            }),
            createPresetAgentStep({
              prefix: 'verify',
              name: '修复验证',
              phase: 'retest',
              member: qa || security || engineer,
              instruction: '验证修复有效性并确认未引入回归风险，形成闭环结论。',
            }),
          ],
        },
      ],
    }
  }

  return {
    version,
    steps: [
      createPresetAgentStep({
        prefix: 'takeover',
        name: '故障接管',
        phase: 'incident_intake',
        member: sre || architect || engineer,
        instruction: '明确故障范围、影响用户、时间线与当前止损动作。',
      }),
      {
        id: generateTeamStepId('analysis'),
        type: 'parallel',
        name: '并行根因分析',
        phase: 'incident_analysis',
        children: [
          createPresetAgentStep({
            prefix: 'rootcause',
            name: '技术根因分析',
            phase: 'root_cause',
            member: engineer || architect || sre,
            instruction: '定位技术根因，给出可验证证据与复现路径。',
          }),
          createPresetAgentStep({
            prefix: 'secimpact',
            name: '安全影响评估',
            phase: 'security_impact',
            member: security || engineer || architect,
            instruction: '评估是否存在安全风险扩散、数据泄露或权限滥用。',
          }),
        ],
      },
      createPresetAgentStep({
        prefix: 'plan',
        name: '处置与恢复方案',
        phase: 'mitigation_plan',
        member: architect || engineer || sre,
        instruction: '制定短期止血与长期修复方案，明确执行步骤与责任人。',
      }),
      createPresetAgentStep({
        prefix: 'postmortem',
        name: '验证与复盘',
        phase: 'verification_postmortem',
        member: qa || pm || architect,
        instruction: '验证恢复效果并输出复盘结论、预防措施与后续追踪指标。',
      }),
    ],
  }
}

const getAllTeamAgentSteps = (steps: TeamOrchestrationStep[]): TeamOrchestrationStep[] => {
  const result: TeamOrchestrationStep[] = []
  const walk = (nodes: TeamOrchestrationStep[]) => {
    for (const node of nodes) {
      if (node.type === 'agent') {
        result.push(node)
      } else if (Array.isArray(node.children) && node.children.length > 0) {
        walk(node.children)
      }
    }
  }
  walk(steps)
  return result
}

const inferTeamTemplateDraftName = () => {
  if (teamTemplateDraftName.value.trim()) return
  if (teamSelectedOrchestrationPresetId.value) {
    const preset = TEAM_ORCHESTRATION_PRESET_METAS.find((item) => item.id === teamSelectedOrchestrationPresetId.value)
    if (preset) {
      teamTemplateDraftName.value = `${preset.label}-模板`
      return
    }
  }
  const goal = (teamSessionDetail.value?.goal || '').trim()
  if (goal) {
    teamTemplateDraftName.value = `Team-${goal.slice(0, 20)}${goal.length > 20 ? '...' : ''}`
    return
  }
  teamTemplateDraftName.value = 'Team-编排模板'
}

const normalizeTeamOrchestrationStep = (raw: any): TeamOrchestrationStep => {
  const typeRaw = typeof raw?.type === 'string' ? raw.type : 'agent'
  const type: TeamOrchestrationStepType = typeRaw === 'parallel' || typeRaw === 'serial' ? typeRaw : 'agent'
  const step: TeamOrchestrationStep = {
    id: typeof raw?.id === 'string' && raw.id.trim()
      ? raw.id.trim()
      : generateTeamStepId('step'),
    type,
    name: typeof raw?.name === 'string' ? raw.name : '',
    phase: typeof raw?.phase === 'string' ? raw.phase : '',
    instruction: typeof raw?.instruction === 'string'
      ? raw.instruction
      : (typeof raw?.prompt === 'string' ? raw.prompt : ''),
  }

  if (type === 'agent') {
    step.member = typeof raw?.member === 'string' ? raw.member : ''
    const retryMaxAttempts = Number(raw?.retry?.max_attempts ?? raw?.retry_max_attempts ?? 1)
    const retryBackoffMs = Number(raw?.retry?.backoff_ms ?? raw?.retry_backoff_ms ?? 800)
    step.retry = {
      max_attempts: Number.isFinite(retryMaxAttempts) ? Math.max(1, Math.floor(retryMaxAttempts)) : 1,
      backoff_ms: Number.isFinite(retryBackoffMs) ? Math.max(100, Math.floor(retryBackoffMs)) : 800,
    }
  } else {
    const rawChildren = Array.isArray(raw?.children) ? raw.children : []
    step.children = rawChildren.map((child: any) => normalizeTeamOrchestrationStep(child))
  }

  return step
}

const normalizeTeamOrchestrationPlan = (raw: any): TeamOrchestrationPlan => {
  const versionRaw = Number(raw?.version ?? 1)
  const version = Number.isFinite(versionRaw) ? Math.max(1, Math.floor(versionRaw)) : 1
  const steps = Array.isArray(raw?.steps)
    ? raw.steps.map((step: any) => normalizeTeamOrchestrationStep(step))
    : []
  return { version, steps }
}

const teamOrchestrationPlanToJson = (plan: TeamOrchestrationPlan): any => {
  const mapStep = (step: TeamOrchestrationStep): any => {
    const base: Record<string, any> = {
      id: step.id || generateTeamStepId('step'),
      type: step.type,
    }
    if (step.name && step.name.trim()) {
      base.name = step.name.trim()
    }
    if (step.phase && step.phase.trim()) {
      base.phase = step.phase.trim()
    }
    if (step.instruction && step.instruction.trim()) {
      base.instruction = step.instruction.trim()
    }
    if (step.type === 'agent') {
      base.member = (step.member || '').trim()
      const retryMaxAttempts = Number(step.retry?.max_attempts ?? 1)
      const retryBackoffMs = Number(step.retry?.backoff_ms ?? 800)
      base.retry = {
        max_attempts: Number.isFinite(retryMaxAttempts) ? Math.max(1, Math.floor(retryMaxAttempts)) : 1,
        backoff_ms: Number.isFinite(retryBackoffMs) ? Math.max(100, Math.floor(retryBackoffMs)) : 800,
      }
    } else {
      base.children = Array.isArray(step.children) ? step.children.map(mapStep) : []
    }
    return base
  }

  return {
    version: Number.isFinite(Number(plan.version)) ? Math.max(1, Math.floor(Number(plan.version))) : 1,
    steps: Array.isArray(plan.steps) ? plan.steps.map(mapStep) : [],
  }
}

const updateTeamOrchestrationTextFromDraft = () => {
  const jsonValue = teamOrchestrationPlanToJson(teamOrchestrationDraft.value)
  teamOrchestrationPlanText.value = JSON.stringify(jsonValue, null, 2)
}

const defaultOrchestrationPlan = (): TeamOrchestrationPlan => ({
  version: 1,
  steps: [],
})

const syncTeamOrchestrationEditorFromSession = (force = false) => {
  const plan = teamSessionDetail.value?.orchestration_plan ?? defaultOrchestrationPlan()
  if (teamPlanDirty.value && !force) return
  const normalized = normalizeTeamOrchestrationPlan(plan)
  teamOrchestrationDraft.value = normalized
  teamOrchestrationPlanText.value = JSON.stringify(teamOrchestrationPlanToJson(normalized), null, 2)
  teamPlanDirty.value = false
  teamPlanError.value = null
  if (force || !teamResumeStepId.value.trim()) {
    const lastStepId = teamSessionDetail.value?.state_machine?.orchestration_runtime?.last_step_id
    teamResumeStepId.value = typeof lastStepId === 'string' ? lastStepId : ''
  }
  teamSelectedOrchestrationPresetId.value = null
  teamSelectedRecoveryPresetId.value = teamCurrentNoHumanInputPolicy.value
  if (force || !teamTemplateDraftName.value.trim()) {
    teamTemplateDraftName.value = ''
    inferTeamTemplateDraftName()
  }
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
  const presetPlan = buildOrchestrationPresetPlan(presetId)
  const normalized = normalizeTeamOrchestrationPlan(presetPlan)
  teamOrchestrationDraft.value = normalized
  updateTeamOrchestrationTextFromDraft()
  teamPlanDirty.value = true
  teamPlanError.value = null
  teamSelectedOrchestrationPresetId.value = presetId
  if (!teamTemplateDraftName.value.trim()) {
    inferTeamTemplateDraftName()
  }
  if (presetId === 'product_delivery_chain') {
    teamTemplateDraftDomain.value = 'product'
  } else if (presetId === 'security_audit_matrix') {
    teamTemplateDraftDomain.value = 'security'
  } else {
    teamTemplateDraftDomain.value = 'ops'
  }

  const missingMemberCount = getAllTeamAgentSteps(normalized.steps)
    .filter((step) => !step.member || !step.member.trim())
    .length
  if (missingMemberCount > 0) {
    teamPlanSuccess.value = `已应用预设（${missingMemberCount} 个节点未匹配成员，请手动选择后保存）。`
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

const handleTeamSaveCurrentPlanAsTemplate = async () => {
  teamPlanError.value = null
  teamPlanSuccess.value = null
  inferTeamTemplateDraftName()

  const templateName = teamTemplateDraftName.value.trim()
  if (!templateName) {
    teamPlanError.value = '请输入模板名称。'
    return
  }

  const templateDomain = teamTemplateDraftDomain.value
  if (!TEAM_TEMPLATE_DOMAIN_OPTIONS.includes(templateDomain)) {
    teamPlanError.value = '模板 domain 非法。'
    return
  }

  teamTemplateDraftSaving.value = true
  try {
    const plan = parseTeamOrchestrationPlanInput()
    const steps = Array.isArray(plan?.steps) ? plan.steps : []
    if (steps.length === 0) {
      throw new Error('当前编排为空，无法保存为模板。')
    }

    const sessionMembers = teamSessionDetail.value?.members || []
    let memberPayload: Array<{
      name: string
      responsibility?: string
      system_prompt?: string
      decision_style?: string
      risk_preference?: string
      weight?: number
      tool_policy?: any
      output_schema?: any
      sort_order?: number
    }> = sessionMembers.map((member, index) => ({
      name: member.name,
      responsibility: member.responsibility || undefined,
      system_prompt: member.system_prompt || undefined,
      decision_style: member.decision_style || undefined,
      risk_preference: member.risk_preference || undefined,
      weight: Number.isFinite(Number(member.weight)) ? Number(member.weight) : 1,
      tool_policy: member.tool_policy || undefined,
      output_schema: member.output_schema || undefined,
      sort_order: Number.isFinite(Number(member.sort_order)) ? Number(member.sort_order) : index,
    }))

    if (memberPayload.length === 0) {
      const fallbackMembers = Array.from(
        new Set(
          getAllTeamAgentSteps(teamOrchestrationDraft.value.steps)
            .map((step) => (step.member || '').trim())
            .filter((name) => !!name),
        ),
      )
      memberPayload = fallbackMembers.map((name, index) => ({
        name,
        sort_order: index,
      }))
    }

    if (memberPayload.length === 0) {
      throw new Error('当前会话没有可用成员，至少需要 1 个成员才能保存模板。')
    }

    const recoveryStateMachine = {
      no_human_input_policy: teamCurrentNoHumanInputPolicy.value,
      human_intervention_timeout_secs: teamCurrentHumanInterventionTimeoutSecs.value,
      max_human_interventions: teamCurrentMaxHumanInterventions.value,
      human_intervention: {
        policy: teamCurrentNoHumanInputPolicy.value,
        timeout_secs: teamCurrentHumanInterventionTimeoutSecs.value,
      },
    }

    const maxRounds = Number(teamSessionDetail.value?.max_rounds || 5)
    const normalizedMaxRounds = Number.isFinite(maxRounds) ? Math.max(1, Math.floor(maxRounds)) : 5
    const planVersionRaw = Number(plan?.version ?? teamOrchestrationDraft.value.version ?? 1)
    const planVersion = Number.isFinite(planVersionRaw) ? Math.max(1, Math.floor(planVersionRaw)) : 1

    const created = await agentTeamApi.createTemplate({
      name: templateName,
      description: '由 Team 编排页沉淀，包含编排计划与恢复策略预设。',
      domain: templateDomain,
      default_rounds_config: {
        max_rounds: normalizedMaxRounds,
        orchestration_plan: plan,
        plan_version: planVersion,
      },
      default_tool_policy: {
        state_machine: recoveryStateMachine,
      },
      members: memberPayload,
    })

    teamPlanSuccess.value = `模板已创建：${created.name}`
  } catch (e: any) {
    teamPlanError.value = e?.message || String(e)
  } finally {
    teamTemplateDraftSaving.value = false
  }
}

const ensureTeamContainerChildren = (step: TeamOrchestrationStep): TeamOrchestrationStep[] => {
  if (step.type === 'agent') {
    step.type = 'serial'
    step.member = ''
    step.retry = undefined
  }
  if (!Array.isArray(step.children)) {
    step.children = []
  }
  return step.children
}

const getTeamStepArrayByContainerPath = (containerPath: number[]): TeamOrchestrationStep[] | null => {
  let current = teamOrchestrationDraft.value.steps
  if (containerPath.length === 0) {
    return current
  }
  for (const idx of containerPath) {
    if (!Array.isArray(current) || idx < 0 || idx >= current.length) {
      return null
    }
    const step = current[idx]
    current = ensureTeamContainerChildren(step)
  }
  return current
}

const getTeamStepByPath = (path: number[]): TeamOrchestrationStep | null => {
  if (path.length === 0) return null
  const container = getTeamStepArrayByContainerPath(path.slice(0, -1))
  const index = path[path.length - 1]
  if (!container || index < 0 || index >= container.length) return null
  return container[index]
}

const pathsEqual = (a: number[], b: number[]): boolean =>
  a.length === b.length && a.every((v, idx) => v === b[idx])

const isPathPrefix = (prefix: number[], path: number[]): boolean =>
  prefix.length <= path.length && prefix.every((v, idx) => path[idx] === v)

const removeTeamStepAtPath = (path: number[]): TeamOrchestrationStep | null => {
  if (path.length === 0) return null
  const container = getTeamStepArrayByContainerPath(path.slice(0, -1))
  const index = path[path.length - 1]
  if (!container || index < 0 || index >= container.length) return null
  const [removed] = container.splice(index, 1)
  return removed || null
}

const adjustPathAfterRemoval = (path: number[], removedPath: number[]): number[] | null => {
  if (isPathPrefix(removedPath, path)) {
    return null
  }
  const next = [...path]
  if (
    path.length === removedPath.length &&
    pathsEqual(path.slice(0, -1), removedPath.slice(0, -1)) &&
    removedPath[removedPath.length - 1] < path[path.length - 1]
  ) {
    next[next.length - 1] -= 1
  }
  return next
}

const handleTeamMoveStepByPath = (payload: TeamStepMovePayload) => {
  const sourcePath = payload.sourcePath || []
  const targetPath = payload.targetPath || []
  if (sourcePath.length === 0 || targetPath.length === 0) return
  if (pathsEqual(sourcePath, targetPath) && payload.mode === 'before') return
  if (payload.mode === 'inside' && isPathPrefix(sourcePath, targetPath)) {
    return
  }

  const moved = removeTeamStepAtPath(sourcePath)
  if (!moved) return
  const adjustedTargetPath = adjustPathAfterRemoval(targetPath, sourcePath)
  if (!adjustedTargetPath) {
    markTeamVisualPlanDirty()
    return
  }

  if (payload.mode === 'before') {
    const targetContainer = getTeamStepArrayByContainerPath(adjustedTargetPath.slice(0, -1))
    const targetIndex = adjustedTargetPath[adjustedTargetPath.length - 1]
    if (!targetContainer) return
    const safeIndex = Math.max(0, Math.min(targetIndex, targetContainer.length))
    targetContainer.splice(safeIndex, 0, moved)
  } else {
    const targetStep = getTeamStepByPath(adjustedTargetPath)
    if (!targetStep) return
    const children = ensureTeamContainerChildren(targetStep)
    children.push(moved)
  }

  markTeamVisualPlanDirty()
}

const handleTeamPromoteStep = (path: number[]) => {
  if (path.length < 2) return
  const moveIndex = path[path.length - 1]
  const parentPath = path.slice(0, -1)
  const grandParentPath = path.slice(0, -2)
  const parentIndex = path[path.length - 2]
  const sourceArray = getTeamStepArrayByContainerPath(parentPath)
  const targetArray = getTeamStepArrayByContainerPath(grandParentPath)
  if (!sourceArray || !targetArray) return
  if (moveIndex < 0 || moveIndex >= sourceArray.length) return
  const [moved] = sourceArray.splice(moveIndex, 1)
  targetArray.splice(parentIndex + 1, 0, moved)
  markTeamVisualPlanDirty()
}

const handleTeamNestStep = (path: number[]) => {
  if (path.length < 1) return
  const moveIndex = path[path.length - 1]
  if (moveIndex <= 0) return
  const containerPath = path.slice(0, -1)
  const siblingArray = getTeamStepArrayByContainerPath(containerPath)
  if (!siblingArray) return
  if (moveIndex < 0 || moveIndex >= siblingArray.length) return
  const prevSibling = siblingArray[moveIndex - 1]
  const [moved] = siblingArray.splice(moveIndex, 1)
  const children = ensureTeamContainerChildren(prevSibling)
  children.push(moved)
  markTeamVisualPlanDirty()
}

const parseTeamOrchestrationPlanInput = (): any => {
  const raw = teamOrchestrationPlanText.value.trim()
  if (!raw) {
    throw new Error('编排计划不能为空。')
  }
  const parsed = JSON.parse(raw)
  if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) {
    throw new Error('编排计划必须是 JSON 对象。')
  }
  const normalized = normalizeTeamOrchestrationPlan(parsed)
  teamOrchestrationDraft.value = normalized
  return teamOrchestrationPlanToJson(normalized)
}

const handleTeamSaveOrchestrationPlan = async () => {
  if (!activeTeamSessionId.value) return
  teamPlanSaving.value = true
  teamPlanError.value = null
  teamPlanSuccess.value = null
  try {
    const plan = parseTeamOrchestrationPlanInput()
    const planVersionRaw = Number(plan?.version ?? teamSessionDetail.value?.plan_version ?? 1)
    const planVersion = Number.isFinite(planVersionRaw) ? Math.max(1, Math.floor(planVersionRaw)) : 1
    await agentTeamApi.updateSession(activeTeamSessionId.value, {
      orchestration_plan: plan,
      plan_version: planVersion,
    })
    teamPlanDirty.value = false
    await loadTeamWorkspaceData()
    teamPlanSuccess.value = '编排计划已保存。'
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
    await agentTeamApi.startRun(activeTeamSessionId.value)
    teamSessionState.value = 'INITIALIZING'
    appendTeamBridgeMessage('[Team] 已按当前编排计划启动执行。')
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
    await agentTeamApi.startRun(activeTeamSessionId.value)
    teamSessionState.value = 'INITIALIZING'
    appendTeamBridgeMessage('[Team] 已触发重试运行。')
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
    await agentTeamApi.startRun(activeTeamSessionId.value)
    teamSessionState.value = 'INITIALIZING'
    appendTeamBridgeMessage(`[Team] 已从 step '${stepId}' 发起恢复执行。`)
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

const resetTeamBlackboardArchive = () => {
  teamBlackboardArchiveEntryId.value = null
  teamBlackboardArchiveMessages.value = []
  teamBlackboardArchiveScope.value = null
  teamBlackboardArchiveLoading.value = false
  teamBlackboardArchiveError.value = null
}

const loadTeamWorkspaceData = async () => {
  if (!activeTeamSessionId.value) {
    teamSessionMessages.value = []
    teamRounds.value = []
    teamBlackboardEntries.value = []
    resetTeamBlackboardArchive()
    teamArtifacts.value = []
    teamSessionDetail.value = null
    teamSelectedOrchestrationPresetId.value = null
    teamSelectedRecoveryPresetId.value = 'balanced'
    teamTemplateDraftName.value = ''
    teamTemplateDraftDomain.value = 'custom'
    syncTeamOrchestrationEditorFromSession(true)
    return
  }
  teamWorkspaceLoading.value = true
  try {
    const [sessionResp, messagesResp, roundsResp, blackboardResp, artifactsResp] = await Promise.all([
      agentTeamApi.getSession(activeTeamSessionId.value),
      agentTeamApi.getMessages(activeTeamSessionId.value),
      agentTeamApi.getRounds(activeTeamSessionId.value),
      agentTeamApi.getBlackboard(activeTeamSessionId.value),
      agentTeamApi.listArtifacts(activeTeamSessionId.value),
    ])
    teamSessionDetail.value = sessionResp
    teamSessionMessages.value = messagesResp
    teamRounds.value = roundsResp
    teamBlackboardEntries.value = blackboardResp
    if (
      teamBlackboardArchiveEntryId.value &&
      !blackboardResp.some((entry) => entry.id === teamBlackboardArchiveEntryId.value)
    ) {
      resetTeamBlackboardArchive()
    }
    teamArtifacts.value = artifactsResp
    syncTeamOrchestrationEditorFromSession()
  } catch (e) {
    console.error('[AgentView] Failed to load team workspace data:', e)
  } finally {
    teamWorkspaceLoading.value = false
  }
}

const handleToggleTeamWorkspace = async () => {
  if (!teamModeEnabled.value) return
  if (isTeamWorkspaceActive.value) {
    isTeamWorkspaceActive.value = false
    return
  }
  webExplorerEvents.close()
  todosComposable.close()
  terminalComposable.closeTerminal()
  isHtmlPanelActive.value = false
  isAuditFindingsPanelActive.value = false
  isTeamWorkspaceActive.value = true
  if (!activeTeamSessionId.value) {
    teamWorkspaceTab.value = 'templates'
  }
  await loadTeamWorkspaceData()
}

const handleTeamResolveBlackboardEntry = async (entryId: string) => {
  if (!activeTeamSessionId.value || !entryId) return
  try {
    await agentTeamApi.resolveBlackboardEntry(activeTeamSessionId.value, entryId)
    teamBlackboardEntries.value = await agentTeamApi.getBlackboard(activeTeamSessionId.value)
  } catch (e) {
    console.error('[AgentView] Failed to resolve team blackboard entry:', e)
  }
}

const handleTeamViewBlackboardArchive = async (entryId: string) => {
  if (!activeTeamSessionId.value || !entryId) return

  if (teamBlackboardArchiveEntryId.value === entryId && !teamBlackboardArchiveLoading.value) {
    resetTeamBlackboardArchive()
    return
  }

  teamBlackboardArchiveEntryId.value = entryId
  teamBlackboardArchiveMessages.value = []
  teamBlackboardArchiveScope.value = null
  teamBlackboardArchiveError.value = null
  teamBlackboardArchiveLoading.value = true

  try {
    const archive = await agentTeamApi.getBlackboardEntryArchive(
      activeTeamSessionId.value,
      entryId,
      120,
    ) as AgentTeamBlackboardArchive
    teamBlackboardArchiveMessages.value = Array.isArray(archive.messages) ? archive.messages : []
    teamBlackboardArchiveScope.value = archive.retrieval_scope || null
  } catch (e: any) {
    teamBlackboardArchiveError.value = e?.message || String(e)
  } finally {
    teamBlackboardArchiveLoading.value = false
  }
}

const handleTeamAddBlackboardEntry = async (type: string, title: string, content: string) => {
  if (!activeTeamSessionId.value) return
  try {
    await agentTeamApi.addBlackboardEntry({
      session_id: activeTeamSessionId.value,
      entry_type: type,
      title,
      content,
      contributed_by: '主链路用户',
    })
    teamBlackboardEntries.value = await agentTeamApi.getBlackboard(activeTeamSessionId.value)
  } catch (e) {
    console.error('[AgentView] Failed to add team blackboard entry:', e)
  }
}

const handleTeamAnnotateBlackboardEntry = async (entryId: string, text: string) => {
  if (!activeTeamSessionId.value) return
  try {
    await agentTeamApi.addBlackboardEntry({
      session_id: activeTeamSessionId.value,
      entry_type: 'action_item',
      title: `批注 - ${entryId.slice(0, 8)}`,
      content: text,
      contributed_by: '主链路批注',
    })
    teamBlackboardEntries.value = await agentTeamApi.getBlackboard(activeTeamSessionId.value)
  } catch (e) {
    console.error('[AgentView] Failed to annotate team blackboard entry:', e)
  }
}

const handleTeamTemplatesUpdated = async () => {
  await loadTeamTemplateOptions(selectedTeamTemplateId.value)
  if (teamWorkspaceTab.value === 'templates') {
    await syncActiveTeamSession()
    await loadTeamWorkspaceData()
  }
}

const handleTeamSessionCreated = async (sessionId: string) => {
  activeTeamSessionId.value = sessionId
  await syncActiveTeamSession()
  teamWorkspaceTab.value = 'blackboard'
  await loadTeamWorkspaceData()
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

  if (teamModeEnabled.value && activeTeamSessionId.value) {
    try {
      await agentTeamApi.stopRun(activeTeamSessionId.value)
      applyTeamState('FAILED')
      appendTeamBridgeMessage('[Team] 已停止当前会话运行。')
    } catch (e) {
      console.error('[AgentView] Failed to stop team execution:', e)
      localError.value = t('agent.failedToStopExecution') + ': ' + e
    }
    return
  }

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
  const currentLoadToken = ++historyLoadToken.value
  try {
    const messages = await invoke<any[]>('get_ai_messages_by_conversation', {
      conversationId: convId
    })
    if (currentLoadToken !== historyLoadToken.value || conversationId.value !== convId) {
      console.log('[AgentView] Skip stale conversation history load for:', convId)
      return
    }
    
    console.log('[AgentView] Received messages:', messages)
    
    // Clear current messages
    agentEvents.clearMessages()
    // Restore subagent list from persistent storage
    await loadSubagentRuns(convId, currentLoadToken)
    if (currentLoadToken !== historyLoadToken.value || conversationId.value !== convId) {
      console.log('[AgentView] Skip stale conversation history apply for:', convId)
      return
    }

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

      if (currentLoadToken !== historyLoadToken.value || conversationId.value !== convId) {
        console.log('[AgentView] Skip stale conversation timeline apply for:', convId)
        return
      }

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
  if (conversationId.value !== convId) {
    return
  }
  
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
  if (conversationId.value !== convId) {
    return
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

  if (teamModeEnabled.value) {
    try {
      // Ensure conversation exists for Team session association.
      if (!conversationId.value) {
        const convId = await invoke<string>('create_ai_conversation', {
          request: {
            title: `${t('agent.newConversationTitle')} ${new Date().toLocaleString()}`,
            service_name: 'default'
          }
        })
        conversationId.value = convId
        currentConversationTitle.value = t('agent.newConversationTitle')
        conversationListRef.value?.loadConversations()
        await syncActiveTeamSession()
      }

      let fullTask = task
      if (referencedTraffic.value.length > 0) {
        const trafficContext = buildTrafficContext(referencedTraffic.value)
        fullTask = `${trafficContext}\n\nUser task: ${task}`
      }

      // Clear input and pending artifacts in Team mode.
      inputValue.value = ''
      pendingAttachments.value = []
      pendingDocuments.value = []
      processedDocuments.value = []
      referencedTraffic.value = []

      nextTick(() => {
        messageFlowRef.value?.scrollToBottom()
      })

      emit('submit', fullTask)
      await routeTeamMessage(fullTask)
      emit('complete', {
        mode: 'team',
        session_id: activeTeamSessionId.value,
      })
    } catch (e: any) {
      const errorMsg = e?.toString?.() || String(e)
      localError.value = errorMsg
      emit('error', errorMsg)
    }
    return
  }
  
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
  unlistenTeamRoundCompleted = await listen<AgentTeamRoundEvent>('agent_team:round_completed', () => {
    if (!activeTeamSessionId.value) {
      return
    }
    void syncTeamMessagesToMainFlow(activeTeamSessionId.value)
    if (isTeamWorkspaceActive.value) {
      void loadTeamWorkspaceData()
    }
  })
  unlistenTeamArtifactGenerated = await listen<AgentTeamArtifactEvent>('agent_team:artifact_generated', (event) => {
    if (!activeTeamSessionId.value || event.payload.session_id !== activeTeamSessionId.value) {
      return
    }
    void syncTeamMessagesToMainFlow(event.payload.session_id)
    if (isTeamWorkspaceActive.value) {
      void loadTeamWorkspaceData()
    }
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
  stopTeamRunStatusPolling()
  if (unlistenAiConfigUpdated) {
    unlistenAiConfigUpdated()
    unlistenAiConfigUpdated = null
  }
  if (unlistenTeamStateChanged) {
    unlistenTeamStateChanged()
    unlistenTeamStateChanged = null
  }
  if (unlistenTeamRoundCompleted) {
    unlistenTeamRoundCompleted()
    unlistenTeamRoundCompleted = null
  }
  if (unlistenTeamArtifactGenerated) {
    unlistenTeamArtifactGenerated()
    unlistenTeamArtifactGenerated = null
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
  if (teamModeEnabled.value) {
    await syncActiveTeamSession()
  }
})

watch(activeTeamSessionId, async (newId, oldId) => {
  if (newId !== oldId) {
    teamMainFlowMessageIds = new Set<string>()
  }
  ensureTeamRunStatusPolling()
  if (newId) {
    await syncTeamMessagesToMainFlow(newId)
  }
  if (!isTeamWorkspaceActive.value) return
  await loadTeamWorkspaceData()
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
