<template>
  <div class="agent-view h-full flex bg-gradient-to-br from-base-100 to-base-200 overflow-hidden relative">
    <!-- Backdrop -->
    <div 
      v-if="showConversations"
      class="conversation-backdrop absolute inset-0 bg-black/20 z-40 transition-opacity"
      @click="closeAllOverlays()"
    ></div>

    <!-- Conversation List Drawer -->
    <Transition name="slide-drawer">
      <div 
        v-if="showConversations"
        ref="conversationDrawerRef"
        class="conversation-drawer absolute left-0 top-0 bottom-0 w-80 bg-base-100 shadow-2xl z-50 overflow-hidden"
      >
        <ConversationList 
          ref="conversationListRef"
          :current-conversation-id="conversationId"
          @select="handleSelectConversation"
          @create="handleCreateConversation"
          @close="closeConversationDrawer()"
        />
      </div>
    </Transition>

    <!-- Main content area -->
    <div class="flex-1 flex flex-col overflow-hidden min-h-0">
      <!-- {{ t('agent.conversationHeader') }} -->
      <div class="conversation-header px-4 py-2 border-b border-base-300 flex items-center justify-between bg-base-100/50">
        <div class="flex items-center gap-2">
          <button 
            @click="toggleConversationDrawer()"
            class="btn btn-sm btn-ghost"
            :title="`${t('agent.switchConversationList')} (Ctrl/Cmd+Shift+B)`"
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
            v-if="hasConnectedBrowserShellSessions"
            @click="handleToggleBrowserShell()"
            class="btn btn-sm gap-1"
            :class="activeRightPanel === 'browser-shell' ? 'btn-primary' : 'btn-ghost text-primary'"
            :title="browserShellPanelTitle"
          >
            <i class="fas fa-window-maximize"></i>
            <span>{{ browserShellDisplayName }}</span>
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
            :title="`${t('agent.newConversation')} (Ctrl/Cmd+Shift+N)`"
          >
            <i class="fas fa-plus"></i>
            <span>{{ t('agent.newConversation') }}</span>
          </button>
        </div>
      </div>
      <div class="border-b border-base-300 bg-base-100/40 px-4 py-2">
        <div class="flex flex-col gap-2 xl:flex-row xl:items-center">
          <div class="text-xs font-medium text-base-content/70">
            当前会话工作目录
          </div>
          <input
            v-model.trim="conversationWorkingDirectoryOverride"
            type="text"
            class="input input-sm flex-1 font-mono"
            :placeholder="conversationWorkingDirectoryPlaceholder"
          />
          <button
            class="btn btn-xs btn-outline"
            :disabled="!conversationWorkingDirectoryOverride"
            @click="clearConversationWorkingDirectoryOverride"
          >
            继承默认
          </button>
          <div class="text-xs text-base-content/60 break-all">
            生效目录: {{ effectiveConversationWorkingDirectoryLabel }}
          </div>
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
            v-if="currentBrowserShellSessionId && isCurrentBoundBrowserShellConnected"
            class="mx-4 mt-2 rounded-lg border border-primary/25 bg-primary/10 px-3 py-2"
          >
            <div class="flex flex-col gap-2 lg:flex-row lg:items-center lg:justify-between">
              <div class="min-w-0">
                <div class="text-sm font-medium text-primary">当前对话已绑定第三方{{ browserShellDisplayName }}</div>
                <div class="text-xs text-base-content/70 break-all">
                  session: {{ currentBrowserShellSessionId }}
                </div>
                <div class="mt-1 text-xs text-base-content/60">
                  本次对话会优先使用 <code>browser_shell</code> 操作这个第三方网页终端，而不是普通 DOM 浏览器动作。
                </div>
                <div class="mt-1 text-xs">
                  <span :class="currentBrowserShellDirectWriteEnabled ? 'text-warning' : 'text-base-content/60'">
                    {{ currentBrowserShellDirectWriteEnabled ? '已授权 AI 直接写入，无需逐条审批。' : '当前仍需逐条审批写入请求。' }}
                  </span>
                </div>
              </div>
              <div class="flex flex-wrap gap-2">
                <button
                  class="btn btn-xs"
                  :class="currentBrowserShellDirectWriteEnabled ? 'btn-warning' : 'btn-outline btn-warning'"
                  @click="toggleBrowserShellDirectWrite()"
                >
                  <i class="fas fa-bolt mr-1"></i>
                  {{ currentBrowserShellDirectWriteEnabled ? '关闭 AI 直写' : '授权 AI 直写' }}
                </button>
                <button
                  class="btn btn-xs btn-outline btn-primary"
                  @click="activateRightPanel('browser-shell')"
                >
                  <i class="fas fa-terminal mr-1"></i>
                  打开{{ browserShellDisplayName }}
                </button>
                <button
                  class="btn btn-xs btn-outline"
                  @click="clearBoundBrowserShellSession"
                >
                  <i class="fas fa-link-slash mr-1"></i>
                  解除绑定
                </button>
              </div>
            </div>
          </div>
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
            @open-tool-config="toggleToolConfigDrawer"
          />
        </div>
        
        <!-- Right: Side Panel (Task, HTML, Terminal, or Team) -->
        <div 
          v-if="activeRightPanel"
          class="sidebar-container flex-shrink-0 border-l border-base-300 flex flex-col overflow-hidden bg-base-100 relative"
          :style="{ width: activeRightPanel === 'work-config' ? `${toolConfigDrawerWidth}px` : `${sidebarWidth}px` }"
        >
            <!-- Resize Handle -->
            <div 
              class="resize-handle absolute left-0 top-0 bottom-0 w-1 cursor-col-resize hover:bg-primary/50 transition-colors z-10"
              @mousedown="activeRightPanel === 'work-config' ? startToolConfigDrawerResize($event) : startResize($event)"
            ></div>

            <AssistantWorkConfigPanel
              v-if="activeRightPanel === 'work-config'"
              :available-models="assistantModelOptions"
              :context-mode="assistantSessionSettings.contextMode"
              :execution-mode="assistantExecutionMode"
              :model-loading="isLoadingAssistantModels"
              :parallel-judge-model="assistantParallelJudgeModel"
              :parallel-selected-models="assistantParallelSelectedModels"
              :profile-id="assistantSessionSettings.profileId"
              :profile-loading="isLoadingAssistantProfiles"
              :profile-options="assistantProfileOptions"
              :run-mode="assistantSessionSettings.runMode"
              :selected-model="assistantSelectedModel"
              :tool-config="toolConfig"
              @update:context-mode="handleAssistantContextModeChange"
              @update:execution-mode="setAssistantExecutionMode"
              @update:model="handleAssistantModelSelection"
              @update:parallel-judge-model="setAssistantParallelJudgeModel"
              @update:parallel-models="setAssistantParallelSelectedModels"
              @update:profile-id="handleAssistantProfileChange"
              @update:run-mode="handleAssistantRunModeChange"
              @update:tool-config="handleToolConfigUpdate"
              @close="closeToolConfigDrawer()"
            />
            
            <TeamWorkspacePanel
              v-else-if="activeRightPanel === 'team' && !activeTeamV4RunId"
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

            <TeamV4WorkspacePanel
              v-else-if="activeRightPanel === 'team'"
              :agents="teamV4Agents"
              :events="teamV4Events"
              :harness-runs="teamV4HarnessRuns"
              :loading="teamV4WorkspaceLoading"
              :memories="teamV4Memories"
              :run="teamV4Run"
              :tasks="teamV4Tasks"
              @cancel-harness="cancelTeamV4HarnessRun"
              @resume-harness="resumeTeamV4HarnessRun"
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
              v-else-if="isViewActive && activeRightPanel === 'terminal'"
              class="h-full border-0 rounded-none bg-transparent"
              :working-directory="effectiveConversationWorkingDirectory || undefined"
              @close="handleCloseTerminal"
            />
            <BrowserShellBridgePanel
              v-else-if="activeRightPanel === 'browser-shell' && hasConnectedBrowserShellSessions"
              class="h-full overflow-y-auto border-0 rounded-none bg-transparent p-4"
              :auto-authorize-direct-write-on-bind="true"
              :show-tool-test-actions="false"
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
        <button @click="clearError" class="error-close bg-transparent border-none text-error cursor-pointer text-xl leading-none px-1 hover:text-base-content" :title="t('agent.close')">×</button>
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
import { ref, computed, watch, nextTick, onMounted, onUnmounted, type Ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import type { AgentMessage } from '@/types/agent'
import type {
  AgentTeamMessage,
  AgentTeamSession,
  TeamBlackboardEntry,
  TeamTask,
} from '@/types/agentTeam'
import type {
  TeamV4Agent,
  TeamV4Event,
  TeamV4HarnessRun,
  TeamV4Memory,
  TeamV4Run,
  TeamV4Task,
} from '@/types/teamRuntime'
import { agentTeamApi } from '@/api/agentTeam'
import { teamRuntimeApi } from '@/api/teamRuntime'
import { useAgentEvents } from '@/composables/useAgentEvents'
import { useAgentTasks } from '@/composables/useAgentTasks'
import { useBrowserShell } from '@/composables/useBrowserShell'
import { useTerminal } from '@/composables/useTerminal'
import { useAgentSessionManager } from '@/composables/useAgentSessionManager'
import AskUserQuestionModal from './AskUserQuestionModal.vue'
import MessageFlow from './MessageFlow.vue'
import TaskPanel from './TaskPanel.vue'
import HtmlPanel from './HtmlPanel.vue'
import SubagentPanel from './SubagentPanel.vue'
import SubagentDetailModal from './SubagentDetailModal.vue'
import BrowserShellBridgePanel from '@/components/Tools/BrowserShellBridgePanel.vue'
import InteractiveTerminal from '@/components/Tools/InteractiveTerminal.vue'
import InputAreaComponent from '@/components/InputAreaComponent.vue'
import ConversationList from './ConversationList.vue'
import AssistantWorkConfigPanel from './AssistantWorkConfigPanel.vue'
import TeamWorkspacePanel from './TeamWorkspacePanel.vue'
import TeamV4WorkspacePanel from './TeamV4WorkspacePanel.vue'
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
import { useAgentBrowserShellAvailability } from './useAgentBrowserShellAvailability'
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

interface AgentRuntimeSettings {
  working_directory?: string | null
}

const props = withDefaults(defineProps<{
  executionId?: string
  showTasks?: boolean
  selectedRole?: any
  focusedMemoryId?: string | null
  focusedMessageId?: string | null
  active?: boolean
}>(), {
  showTasks: true,
  focusedMemoryId: null,
  focusedMessageId: null,
  active: true,
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

// i18n
const { t, locale } = useI18n()

const isViewActive = computed(() => props.active)
const isChineseUi = computed(() => locale.value.toLowerCase().startsWith('zh'))
const browserShellDisplayName = computed(() => (isChineseUi.value ? '浏览器 Shell' : 'Browser Shell'))
const browserShellPanelTitle = computed(() => (isChineseUi.value ? '打开浏览器 Shell 面板' : 'Open Browser Shell Bridge'))

// Refs
const messageFlowRef = ref<InstanceType<typeof MessageFlow> | null>(null)
const conversationListRef = ref<InstanceType<typeof ConversationList> | null>(null)
const conversationDrawerRef = ref<HTMLElement | null>(null)
const inputAreaRef = ref<InstanceType<typeof InputAreaComponent> | null>(null)
const inputValue = ref('')
const localError = ref<string | null>(null)
const submitInFlight = ref(false)
const conversationId = ref<string | null>(props.executionId ?? null)
const showConversations = ref(false) // Default hidden
const lastOverlayTrigger = ref<HTMLElement | null>(null)
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
const agentDefaultWorkingDirectory = ref('')

const {
  defaultAssistantProfileId,
  defaultTeamProfileId,
  getAssistantProfileOption,
  getTeamProfileOption,
  loadDefaultAssistantProfile,
  loadDefaultTeamProfile,
  isLoadingAssistantProfiles,
  loadAssistantProfiles,
  loadTeamProfiles,
  profileOptions: assistantProfileOptions,
  teamProfileOptions,
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
  setWorkingDirectoryOverride,
  teamModeEnabled,
  tenthManEnabled,
  toConversationBinding,
  webSearchEnabled,
} = useAssistantSessionSettings()
const conversationWorkingDirectoryOverride = computed({
  get: () => assistantSessionSettings.value.workingDirectoryOverride,
  set: (value: string) => {
    setWorkingDirectoryOverride(value)
  },
})
const effectiveConversationWorkingDirectory = computed(() => {
  const overrideValue = conversationWorkingDirectoryOverride.value.trim()
  if (overrideValue) return overrideValue
  return agentDefaultWorkingDirectory.value.trim()
})
const conversationWorkingDirectoryPlaceholder = computed(() => {
  const inherited = agentDefaultWorkingDirectory.value.trim()
  return inherited ? `继承默认: ${inherited}` : '未设置时继承 Agent 默认工作目录'
})
const effectiveConversationWorkingDirectoryLabel = computed(() => {
  const resolved = effectiveConversationWorkingDirectory.value.trim()
  return resolved || '未配置'
})
const clearConversationWorkingDirectoryOverride = () => {
  setWorkingDirectoryOverride('')
}
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
const activeTeamV4RunId = ref<string | null>(null)
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
const teamV4Agents = ref<TeamV4Agent[]>([])
const teamV4Events = ref<TeamV4Event[]>([])
const teamV4HarnessRuns = ref<TeamV4HarnessRun[]>([])
const teamV4Memories = ref<TeamV4Memory[]>([])
const teamV4Run = ref<TeamV4Run | null>(null)
const teamV4Tasks = ref<TeamV4Task[]>([])
const teamV4WorkspaceLoading = ref(false)
const isSubagentPanelOpen = ref(false)
const subagents = computed(() => agentEvents.subagents.value)
const {
  assistantDefaultMaxContextTokens,
  assistantExecutionMode,
  assistantGlobalDefaultModel,
  assistantModelOptions,
  assistantParallelJudgeModel,
  assistantParallelSelectedModels,
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
  setAssistantExecutionMode,
  setAssistantParallelJudgeModel,
  setAssistantParallelSelectedModels,
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
  if (activeTeamV4RunId.value) {
    return ['draft', 'planning', 'running', 'waiting_human'].includes(
      String(teamV4Run.value?.state || '').trim().toLowerCase(),
    )
  }
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
    activeTeamV4RunId.value ||
    teamSessionDetail.value ||
    teamSessionMessages.value.length ||
    teamTasks.value.length ||
    teamBlackboardEntries.value.length ||
    teamV4Agents.value.length ||
    teamV4Events.value.length ||
    teamV4HarnessRuns.value.length ||
    teamV4Memories.value.length ||
    teamV4Run.value ||
    teamV4Tasks.value.length,
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
const browserShellComposable = useBrowserShell()
const currentBrowserShellSessionId = computed(() => browserShellComposable.currentSessionId.value)
const currentBrowserShellDirectWriteEnabled = computed(() =>
  browserShellComposable.directWriteEnabled.value,
)
const buildCurrentConversationBinding = () => toConversationBinding({
  browserShellDirectWriteEnabled: currentBrowserShellDirectWriteEnabled.value,
  browserShellSessionId: currentBrowserShellSessionId.value,
  selectedModel: assistantSelectedModel.value,
  toolsEnabled: toolsEnabled.value,
  toolConfig: toolConfig.value,
})
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
  handleToggleBrowserShell,
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
  parallelTaskSources: computed(() => agentEvents.parallelTaskSources.value),
  propsShowTasks: props.showTasks,
  resetAgentError: () => {
    agentEvents.resetError()
  },
  resolveAgentName,
  selectedTeamTaskAssigneeId: computed(() => normalizeOptionalText(selectedTeamTask.value?.assignee_agent_id) || null),
  teamWorkspaceAvailable,
  terminalClose: () => {
    if (!isViewActive.value) return
    terminalComposable.closeTerminal()
  },
  terminalHasHistory: computed(() => terminalComposable.hasHistory.value),
  terminalIsActive: computed(() => isViewActive.value && terminalComposable.isTerminalActive.value),
  terminalOpen: () => {
    if (!isViewActive.value) return
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
  lastOverlayTrigger.value = document.activeElement instanceof HTMLElement ? document.activeElement : null
  showConversations.value = false
  activateRightPanel('work-config')
}

const toggleToolConfigDrawer = () => {
  lastOverlayTrigger.value = document.activeElement instanceof HTMLElement ? document.activeElement : null
  showConversations.value = false
  if (activeRightPanel.value === 'work-config') {
    closeToolConfigDrawer()
    return
  }
  activateRightPanel('work-config')
}

const closeConversationDrawer = () => {
  showConversations.value = false
}

const closeToolConfigDrawer = () => {
  deactivateRightPanel('work-config')
}

const closeAllOverlays = () => {
  showConversations.value = false
}

const toggleConversationDrawer = () => {
  if (showConversations.value) {
    closeConversationDrawer()
    return
  }
  lastOverlayTrigger.value = document.activeElement instanceof HTMLElement ? document.activeElement : null
  if (activeRightPanel.value === 'work-config') {
    closeToolConfigDrawer()
  }
  showConversations.value = true
}

const restoreOverlayTriggerFocus = () => {
  nextTick(() => {
    lastOverlayTrigger.value?.focus()
    lastOverlayTrigger.value = null
  })
}

const isEditableTarget = (target: EventTarget | null) => {
  if (!(target instanceof HTMLElement)) return false
  const tagName = target.tagName.toLowerCase()
  return tagName === 'input' || tagName === 'textarea' || tagName === 'select' || target.isContentEditable
}

const handleGlobalKeydown = (event: KeyboardEvent) => {
  if (event.key === 'Escape') {
    if (activeRightPanel.value === 'work-config') {
      event.preventDefault()
      closeToolConfigDrawer()
      return
    }
    if (showConversations.value) {
      event.preventDefault()
      closeConversationDrawer()
      return
    }
  }

  if (isEditableTarget(event.target) || !(event.metaKey || event.ctrlKey) || !event.shiftKey) {
    return
  }

  const key = event.key.toLowerCase()
  if (key === 'b') {
    event.preventDefault()
    toggleConversationDrawer()
    return
  }
  if (key === 'n') {
    event.preventDefault()
    void handleCreateConversation()
    return
  }
  if (key === 't') {
    event.preventDefault()
    handleToggleTasks()
    return
  }
  if (key === 'o') {
    event.preventDefault()
    toggleToolConfigDrawer()
    return
  }
  if (key === 'i') {
    event.preventDefault()
    inputAreaRef.value?.focusInput()
  }
}

const clearBoundBrowserShellSession = () => {
  browserShellComposable.clearSession()
}

const toggleBrowserShellDirectWrite = () => {
  browserShellComposable.setDirectWriteEnabled(!browserShellComposable.directWriteEnabled.value)
}

const {
  hasConnectedBrowserShellSessions,
  isCurrentBoundBrowserShellConnected,
} = useAgentBrowserShellAvailability({
  currentBrowserShellSessionId,
  clearBoundBrowserShellSession,
})

watch(hasConnectedBrowserShellSessions, (hasConnected) => {
  if (!hasConnected && activeRightPanel.value === 'browser-shell') {
    deactivateRightPanel('browser-shell')
  }
})

watch(showConversations, (open) => {
  if (open) {
    nextTick(() => {
      conversationListRef.value?.focusSearch?.()
    })
    return
  }

  restoreOverlayTriggerFocus()
})

const handleParallelTaskSourceFocus = (event: Event) => {
  const sourceKey = String((event as CustomEvent)?.detail?.sourceKey || '').trim()
  if (!sourceKey) return
  handleTaskSourceChange(sourceKey)
  activateRightPanel('tasks')
  taskComposable.open()
}

onMounted(() => {
  window.addEventListener('agent:parallel-task-source-focus', handleParallelTaskSourceFocus)
  window.addEventListener('keydown', handleGlobalKeydown)
  void invoke<AgentRuntimeSettings>('get_agent_config')
    .then((config) => {
      agentDefaultWorkingDirectory.value = String(config?.working_directory || '').trim()
    })
    .catch((error) => {
      console.warn('[AgentView] Failed to load default working directory:', error)
    })
})

onUnmounted(() => {
  window.removeEventListener('agent:parallel-task-source-focus', handleParallelTaskSourceFocus)
  window.removeEventListener('keydown', handleGlobalKeydown)
})

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
  buildCurrentConversationBinding,
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
  bindBrowserShellSession: (sessionId) => {
    browserShellComposable.bindSession(sessionId)
  },
  setBrowserShellDirectWriteEnabled: (enabled) => {
    browserShellComposable.setDirectWriteEnabled(enabled)
  },
  conversationId,
  currentBrowserShellDirectWriteEnabled,
  currentBrowserShellSessionId,
  activeTeamSessionId,
  assistantGlobalDefaultModel,
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

const loadTeamV4WorkspaceData = async (runId = activeTeamV4RunId.value) => {
  const normalizedRunId = String(runId || '').trim()
  if (!normalizedRunId) return
  teamV4WorkspaceLoading.value = true
  try {
    const [run, agents, tasks, events, memories, harnessRuns] = await Promise.all([
      teamRuntimeApi.getRun(normalizedRunId),
      teamRuntimeApi.listAgents(normalizedRunId),
      teamRuntimeApi.listTasks(normalizedRunId),
      teamRuntimeApi.listEvents(normalizedRunId, 0, 500),
      teamRuntimeApi.listMemories(normalizedRunId),
      teamRuntimeApi.listHarnessRuns(normalizedRunId),
    ])
    teamV4Run.value = run
    teamV4Agents.value = agents
    teamV4Tasks.value = tasks
    teamV4Events.value = events
    teamV4Memories.value = memories
    teamV4HarnessRuns.value = harnessRuns
  } finally {
    teamV4WorkspaceLoading.value = false
  }
}

const cancelTeamV4HarnessRun = async (harnessRunId: string) => {
  await teamRuntimeApi.cancelHarnessRun(harnessRunId)
  await loadTeamV4WorkspaceData()
}

const resumeTeamV4HarnessRun = async (harnessRunId: string) => {
  await teamRuntimeApi.resumeHarnessRun(harnessRunId, 600)
  await loadTeamV4WorkspaceData()
}

const persistTeamV4Message = async (message: AgentMessage, role: 'user' | 'assistant') => {
  const targetConversationId = conversationId.value
  if (!targetConversationId) return
  await invoke('save_ai_message', {
    request: {
      id: message.id,
      conversation_id: targetConversationId,
      role,
      content: message.content,
      metadata: message.metadata ?? null,
      architecture_type: 'team_v4',
      architecture_meta: JSON.stringify({
        team_run_id: message.metadata?.team_session_id ?? null,
        role,
      }),
      structured_data: null,
    },
  })
}

const startTeamV4AssistantRun = async (goal: string) => {
  await flushPendingToolConfigSave()
  await Promise.all([
    loadTeamProfiles(),
    loadDefaultTeamProfile(),
  ])
  const selectedAssistantProfile = getAssistantProfileOption(assistantSessionSettings.value.profileId)
  const selectedTeamProfileId =
    selectedAssistantProfile?.defaultTeamProfileId?.trim()
    || defaultTeamProfileId.value.trim()
    || teamProfileOptions.value[0]?.id
    || null
  const selectedTeamProfile = selectedTeamProfileId
    ? getTeamProfileOption(selectedTeamProfileId)
    : null
  if (!selectedTeamProfile) {
    throw new Error('Team mode requires a Team Profile.')
  }
  const teamToolPolicyMatrix = selectedTeamProfile.toolPolicyMatrix
  const teamMemoryPolicy = {
    ...selectedTeamProfile.memoryPolicy,
    ragEnabled: ragEnabled.value,
  }
  const teamHarnessPolicy = selectedTeamProfile.harnessPolicy
  const teamConcurrencyPolicy = selectedTeamProfile.concurrencyPolicy
  const teamSafetyPolicy = selectedTeamProfile.safetyPolicy
  const bootstrap = await teamRuntimeApi.startAssistantRun({
    conversationId: conversationId.value,
    profileId: assistantSessionSettings.value.profileId,
    teamProfileId: selectedTeamProfile.id,
    orchestratorProfileId: selectedTeamProfile.orchestratorProfileId,
    specialistProfileIds: selectedTeamProfile.specialistProfileIds,
    monitorProfileId: selectedTeamProfile.monitorProfileId,
    goal,
    model: selectedTeamProfile.defaultModel || null,
    contextMode: selectedTeamProfile.contextMode,
    toolPolicyMatrix: teamToolPolicyMatrix,
    memoryPolicy: teamMemoryPolicy,
    harnessPolicy: teamHarnessPolicy,
    concurrencyPolicy: teamConcurrencyPolicy,
    safetyPolicy: teamSafetyPolicy,
  })

  activeTeamV4RunId.value = bootstrap.run.id
  teamV4Run.value = bootstrap.run
  teamV4Agents.value = [bootstrap.orchestrator, bootstrap.monitor, ...bootstrap.specialists]
  teamV4Tasks.value = bootstrap.specialistAssignments.map((assignment) => assignment.task)
  teamV4Events.value = bootstrap.events
  teamV4Memories.value = []
  teamV4HarnessRuns.value = bootstrap.specialistAssignments.map((assignment) => assignment.harnessRun)
  teamSessionState.value = 'EXECUTING'
  isTeamWorkspaceActive.value = true
  activateRightPanel('team')

  const now = Date.now()
  const userMessage: AgentMessage = {
    id: crypto.randomUUID(),
    type: 'user',
    content: goal,
    timestamp: now,
    metadata: {
      kind: 'team_v4_user_goal',
      team_session_id: bootstrap.run.id,
      team_task_record_id: bootstrap.rootTask.id,
    },
  }
  const summaryContent = [
    `Team v4 run started: ${bootstrap.run.id}`,
    `Orchestrator created ${bootstrap.specialistAssignments.length} task assignment(s) for ${bootstrap.specialists.length} Specialist(s).`,
    'Monitor is attached to collect signals, evaluate quality, and trigger retry recommendations.',
    `Harness lease active until ${bootstrap.specialistAssignments[0]?.harnessRun.lease_expires_at || 'unknown'}.`,
  ].join('\n')
  const assistantMessage: AgentMessage = {
    id: crypto.randomUUID(),
    type: 'planning',
    content: summaryContent,
    timestamp: now + 1,
    metadata: {
      kind: 'team_v4_orchestrator_bootstrap',
      team_member_id: bootstrap.orchestrator.id,
      team_member_name: bootstrap.orchestrator.name,
      team_member_role: 'orchestrator',
      team_session_id: bootstrap.run.id,
      team_task_record_id: bootstrap.rootTask.id,
      team_task_key: bootstrap.rootTask.task_key,
      team_task_title: bootstrap.rootTask.title,
      team_sequence: bootstrap.events[bootstrap.events.length - 1]?.sequence,
    },
  }
  agentEvents.messages.value.push(userMessage, assistantMessage)
  nextTick(() => {
    scrollMessageViewportToBottom()
  })
  void Promise.all([
    persistTeamV4Message(userMessage, 'user'),
    persistTeamV4Message(assistantMessage, 'assistant'),
    loadTeamV4WorkspaceData(bootstrap.run.id),
  ]).catch((error) => {
    console.warn('[AgentView] Team v4 bootstrap side effects failed:', error)
  })
  return bootstrap
}

const stopTeamV4Run = async () => {
  const runId = activeTeamV4RunId.value
  if (!runId) return
  await teamRuntimeApi.appendEvent(runId, {
    actorId: null,
    taskId: teamV4Tasks.value[0]?.id || null,
    eventType: 'run_cancelled_by_user',
    visibility: 'user',
    payload: {
      reason: 'user_stop',
    },
  })
  await teamRuntimeApi.updateRunState(runId, 'cancelled')
  teamSessionState.value = 'FAILED'
  await loadTeamV4WorkspaceData(runId)
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
  activeTeamV4RunId,
  agentMessages: agentEvents.messages,
  agentStreamingContent: agentEvents.streamingContent,
  agentSubagents: agentEvents.subagents,
  assistantModelOptions,
  assistantContextMode,
  assistantExecutionMode,
  assistantParallelJudgeModel,
  assistantParallelSelectedModels,
  assistantSelectedModel,
  buildCurrentConversationBinding,
  effectiveWorkingDirectory: effectiveConversationWorkingDirectory,
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
  forceTaskCompletionContract: false,
  getFailedToClearConversationLabel: () => t('agent.failedToClearConversation'),
  getFailedToStopExecutionLabel: () => t('agent.failedToStopExecution'),
  getNewConversationTitle: () => `${t('agent.newConversationTitle')} ${new Date().toLocaleString()}`,
  getToolCallCompletedLabel: () => t('agent.toolCallCompleted'),
  getUnnamedConversationTitle: () => t('agent.newConversationTitle'),
  getAssistantProfileOption,
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
  refreshTeamV4Workspace: loadTeamV4WorkspaceData,
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
  startTeamV4AssistantRun,
  startTeamExecutionRun,
  stopAgentExecutionState: () => {
    agentEvents.stopExecution()
  },
  stopTeamV4Run,
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
    currentBrowserShellDirectWriteEnabled,
    currentBrowserShellSessionId,
    conversationWorkingDirectoryOverride,
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
  buildCurrentConversationBinding,
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
