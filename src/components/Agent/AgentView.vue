<template>
  <div
    class="agent-view h-full flex bg-gradient-to-br from-base-100 to-base-200 overflow-hidden relative"
  >
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
      <div
        class="conversation-header grid grid-cols-[minmax(0,1fr)_minmax(14rem,32rem)_minmax(0,1fr)] items-center gap-3 border-b border-base-300 bg-base-100/50 px-4 py-2"
      >
        <div class="flex min-w-0 items-center gap-2">
          <button
            @click="toggleConversationDrawer()"
            class="btn btn-sm btn-ghost shrink-0"
            :title="`${t('agent.switchConversationList')} (Ctrl/Cmd+Shift+B)`"
          >
            <i class="fas fa-bars"></i>
          </button>
          <span class="truncate text-sm font-medium text-base-content/70">
            {{ currentConversationTitle }}
          </span>
          <span
            v-if="conversationExecutionState"
            class="badge badge-sm shrink-0"
            :class="conversationExecutionStateBadgeClass"
          >
            {{ conversationExecutionStateBadgeText }}
          </span>
        </div>
        <div
          class="group relative w-full justify-self-center"
          :title="conversationWorkingDirectoryTooltip"
        >
          <input
            :value="conversationWorkingDirectoryInputValue"
            type="text"
            class="input input-sm h-8 w-full border-transparent bg-transparent pr-20 text-center font-mono text-xs shadow-none placeholder:text-base-content/65 hover:border-base-300 hover:bg-base-100/70 hover:text-left focus:border-base-300 focus:bg-base-100/70 focus:text-left focus:outline-none"
            :readonly="agentExecutionMode === 'docker'"
            :placeholder="conversationWorkingDirectoryPlaceholder"
            :title="conversationWorkingDirectoryTooltip"
            aria-label="当前会话工作目录"
            @input="handleConversationWorkingDirectoryInput"
          />
          <div
            class="absolute right-0 top-0 flex h-8 opacity-0 transition-opacity group-hover:opacity-100 group-focus-within:opacity-100"
          >
            <button
              class="btn btn-sm btn-outline h-8 min-h-8 rounded-none border-r-0 px-3"
              :disabled="!canOpenConversationWorkingDirectory"
              :title="openConversationWorkingDirectoryTitle"
              aria-label="打开当前会话工作目录"
              @click="openConversationWorkingDirectory"
            >
              <i class="fas fa-up-right-from-square"></i>
            </button>
            <button
              v-if="agentExecutionMode !== 'docker'"
              class="btn btn-sm btn-outline h-8 min-h-8 rounded-l-none px-3"
              title="选择当前会话工作目录"
              aria-label="选择当前会话工作目录"
              @click="selectConversationWorkingDirectory"
            >
              <i class="fas fa-folder-open"></i>
            </button>
          </div>
        </div>
        <div class="flex shrink-0 items-center justify-self-end gap-2">
          <button
            @click="handleToggleTasks()"
            class="btn btn-sm gap-1"
            :class="activeRightPanel === 'tasks' ? 'btn-primary' : 'btn-ghost text-primary'"
            :title="activeRightPanel === 'tasks' ? t('agent.tasksPanelOpen') : t('agent.viewTasks')"
          >
            <i class="fas fa-tasks"></i>
            <span>{{ t('agent.tasks') }}</span>
            <span v-if="taskBadgeCount > 0" class="badge badge-xs badge-primary">{{
              taskBadgeCount
            }}</span>
          </button>
          <button
            @click="handleToggleHarness()"
            class="btn btn-sm gap-1"
            :class="activeRightPanel === 'harness' ? 'btn-primary' : 'btn-ghost text-primary'"
            title="Harness"
          >
            <i class="fas fa-shield-alt"></i>
            <span>Harness</span>
          </button>
          <button
            v-if="hasHtmlPanelContent"
            @click="handleToggleHtmlPanel()"
            class="btn btn-sm gap-1"
            :class="activeRightPanel === 'html' ? 'btn-primary' : 'btn-ghost text-primary'"
            :title="
              activeRightPanel === 'html' ? t('agent.htmlPanelOpen') : t('agent.viewHtmlPanel')
            "
          >
            <i class="fas fa-code"></i>
            <span>{{ t('agent.htmlPanel') }}</span>
          </button>
          <button
            @click="handleToggleTerminal()"
            class="btn btn-sm gap-1"
            :class="activeRightPanel === 'terminal' ? 'btn-primary' : 'btn-ghost text-primary'"
            :title="
              activeRightPanel === 'terminal'
                ? t('agent.terminalPanelOpen')
                : t('agent.viewTerminal')
            "
          >
            <i class="fas fa-terminal"></i>
            <span>{{ t('agent.terminal') }}</span>
          </button>
          <button
            @click="
              activeRightPanel === 'workspace-files'
                ? deactivateRightPanel('workspace-files')
                : activateRightPanel('workspace-files')
            "
            class="btn btn-sm gap-1"
            :class="
              activeRightPanel === 'workspace-files' ? 'btn-primary' : 'btn-ghost text-primary'
            "
            title="工作目录面板"
          >
            <i class="fas fa-folder-tree"></i>
            <span>工作目录</span>
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
            <span v-if="teamWorkspaceBadgeCount > 0" class="badge badge-xs badge-primary">{{
              teamWorkspaceBadgeCount
            }}</span>
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
                <div class="text-sm font-medium text-primary">
                  当前对话已绑定第三方{{ browserShellDisplayName }}
                </div>
                <div class="text-xs text-base-content/70 break-all">
                  session: {{ currentBrowserShellSessionId }}
                </div>
                <div class="mt-1 text-xs text-base-content/60">
                  本次对话会优先使用 <code>browser_shell</code> 操作这个第三方网页终端，而不是普通
                  DOM 浏览器动作。
                </div>
                <div class="mt-1 text-xs">
                  <span
                    :class="
                      currentBrowserShellDirectWriteEnabled
                        ? 'text-warning'
                        : 'text-base-content/60'
                    "
                  >
                    {{
                      currentBrowserShellDirectWriteEnabled
                        ? '已授权 AI 直接写入，无需逐条审批。'
                        : '当前仍需逐条审批写入请求。'
                    }}
                  </span>
                </div>
              </div>
              <div class="flex flex-wrap gap-2">
                <button
                  class="btn btn-xs"
                  :class="
                    currentBrowserShellDirectWriteEnabled
                      ? 'btn-warning'
                      : 'btn-outline btn-warning'
                  "
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
                <button class="btn btn-xs btn-outline" @click="clearBoundBrowserShellSession">
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
                  <span v-if="focusedMemoryMessageId" class="ml-2"
                    >message: {{ focusedMemoryMessageId }}</span
                  >
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
                <button class="btn btn-xs btn-outline" @click="clearFocusedLocation">
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
              :context-compression="contextCompression"
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
            :selected-agent="selectedAssistantAgentOptionId"
            :agent-loading="isLoadingAssistantAgentOptions"
            @send-message="handleSubmit"
            @interrupt-message="handleInterruptSubmit"
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

        <!-- Right: Side Panel -->
        <div
          v-if="activeRightPanel"
          class="sidebar-container flex-shrink-0 border-l border-base-300 flex flex-col overflow-hidden bg-base-100 relative"
          :style="{
            width:
              activeRightPanel === 'work-config'
                ? `${toolConfigDrawerWidth}px`
                : `${sidebarWidth}px`,
          }"
        >
          <!-- Resize Handle -->
          <div
            class="resize-handle absolute left-0 top-0 bottom-0 w-1 cursor-col-resize hover:bg-primary/50 transition-colors z-10"
            @mousedown="
              activeRightPanel === 'work-config'
                ? startToolConfigDrawerResize($event)
                : startResize($event)
            "
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
            :selected-model="assistantSelectedModel"
            :tool-config="toolConfig"
            @update:context-mode="handleAssistantContextModeChange"
            @update:execution-mode="setAssistantExecutionMode"
            @update:model="handleAssistantModelSelection"
            @update:parallel-judge-model="setAssistantParallelJudgeModel"
            @update:parallel-models="setAssistantParallelSelectedModels"
            @update:profile-id="handleAssistantProfileChange"
            @update:tool-config="handleToolConfigUpdate"
            @close="closeToolConfigDrawer()"
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
          <AgentHarnessPanel
            v-else-if="activeRightPanel === 'harness'"
            :active="activeRightPanel === 'harness'"
            :conversation-id="conversationId"
            :team-agents="teamV4Agents"
            :team-events="teamV4Events"
            :team-harness-runs="teamV4HarnessRuns"
            :team-tasks="teamV4Tasks"
            @close="handleCloseHarness"
          />
          <HtmlPanel
            v-else-if="activeRightPanel === 'html'"
            :html-content="htmlPanelContent"
            :is-active="activeRightPanel === 'html'"
            class="h-full p-4 overflow-y-auto border-0 bg-transparent"
            @close="handleCloseHtmlPanel"
          />
          <WorkspaceFilesPanel
            v-else-if="activeRightPanel === 'workspace-files'"
            :conversation-id="conversationId"
            :working-directory="workspaceFilesStorageDirectory"
            :display-working-directory="displayedConversationWorkingDirectory"
            :initial-relative-path="workspaceFilesInitialRelativePath"
            @close="deactivateRightPanel('workspace-files')"
          />
          <InteractiveTerminal
            v-else-if="isViewActive && activeRightPanel === 'terminal'"
            class="h-full border-0 rounded-none bg-transparent"
            :working-directory="effectiveConversationWorkingDirectory || undefined"
            :execution-id="conversationId"
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
        <button
          @click="clearError"
          class="error-close bg-transparent border-none text-error cursor-pointer text-xl leading-none px-1 hover:text-base-content"
          :title="t('agent.close')"
        >
          ×
        </button>
      </div>
    </div>

    <!-- Subagent Detail Modal -->
    <SubagentDetailModal
      :visible="showSubagentDetailModal"
      :subagent="selectedSubagent"
      :message-state="selectedSubagentMessageState"
      :load-messages="subagentMessageStore.loadMessages"
      @close="showSubagentDetailModal = false"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, defineAsyncComponent, watch, nextTick, onMounted, onUnmounted, type Ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import type { AgentMessage } from '@/types/agent'
import type {
  TeamV4Agent,
  TeamV4Event,
  TeamV4HarnessRun,
  TeamV4Memory,
  TeamV4Run,
  TeamV4Task,
} from '@/types/teamRuntime'
import { teamRuntimeApi } from '@/api/teamRuntime'
import { useAgentEvents } from '@/composables/useAgentEvents'
import { useAgentTasks } from '@/composables/useAgentTasks'
import { useBrowserShell } from '@/composables/useBrowserShell'
import { useTerminal } from '@/composables/useTerminal'
import { useAgentSessionManager } from '@/composables/useAgentSessionManager'
import MessageFlow from './MessageFlow.vue'
import SubagentPanel from './SubagentPanel.vue'
import InputAreaComponent from '@/components/InputAreaComponent.vue'
import ConversationList from './ConversationList.vue'
import {
  type AgentExecutionFinishedEvent,
  getExecutionStateBadgeClass,
  getExecutionStateLabelKey,
  type PersistedAgentExecutionState,
} from './executionState'
import { useAgentConversationFlow } from './useAgentConversationFlow'
import { useAgentModelAndToolConfig } from './useAgentModelAndToolConfig'
import { useAgentPanels } from './useAgentPanels'
import { normalizeHarnessMaxContinuations, useAssistantProfiles } from './assistantProfiles'
import { useAssistantSessionSettings } from './useAssistantSessionSettings'
import { buildNewAssistantConversationBinding } from './agentConversationBindingBuilder'
import { useAssistantAgentSwitchOptions } from './agentProfileSwitchSupport'
import type { ReferencedAsset, ReferencedTraffic, TrafficSendType } from './agentDraftTypes'
import { useAgentDraftArtifacts } from './useAgentDraftArtifacts'
import { type UiToolConfigPayload } from './toolConfigRuntime'
import { mapPersistedAgentTasks } from './agentTaskHistorySupport'
import { useAgentMessageFocus } from './useAgentMessageFocus'
import { useAgentConversationBinding } from './useAgentConversationBinding'
import { useAgentBrowserShellAvailability } from './useAgentBrowserShellAvailability'
import { useAgentViewLifecycle } from './useAgentViewLifecycle'
import { useAgentSubagents } from './useAgentSubagents'
import { useSubagentMessageStore } from './useSubagentMessageStore'
import { useAgentViewEffects } from './useAgentViewEffects'
import { isVisionModelUnsupportedError } from './agentVisionErrorSupport'
import { persistTeamV4Message } from './teamV4MessagePersistence'
import { AI_CONFIG_UPDATED_EVENT } from '@/services/aiConfigEvents'

interface AgentRuntimeSettings {
  shell?: {
    docker_config?: {
      volumes?: Record<string, string> | null
    } | null
  } | null
  terminal?: {
    default_execution_mode?: 'docker' | 'host' | null
  } | null
  working_directory?: string | null
}

const DEFAULT_DOCKER_WORKING_DIRECTORY = '/workspace'
const CONTAINER_CONTEXT_DIR = '/workspace/context'
const EXECUTION_DIR_PREFIX = 'session_'

function dockerSessionWorkingDir(executionId?: string | null): string {
  const raw = (executionId || '').trim()
  if (!raw) return DEFAULT_DOCKER_WORKING_DIRECTORY
  const sanitized = raw.replace(/[^a-zA-Z0-9\-_]/g, '').slice(0, 12)
  if (!sanitized) return DEFAULT_DOCKER_WORKING_DIRECTORY
  return `${CONTAINER_CONTEXT_DIR}/${EXECUTION_DIR_PREFIX}${sanitized}`
}

const props = withDefaults(
  defineProps<{
    executionId?: string
    showTasks?: boolean
    selectedRole?: any
    focusedMemoryId?: string | null
    focusedMessageId?: string | null
    active?: boolean
  }>(),
  {
    showTasks: true,
    focusedMemoryId: null,
    focusedMessageId: null,
    active: true,
  }
)

const AgentHarnessPanel = defineAsyncComponent(() => import('./AgentHarnessPanel.vue'))
const AssistantWorkConfigPanel = defineAsyncComponent(() => import('./AssistantWorkConfigPanel.vue'))
const BrowserShellBridgePanel = defineAsyncComponent(() => import('@/components/Tools/BrowserShellBridgePanel.vue'))
const HtmlPanel = defineAsyncComponent(() => import('./HtmlPanel.vue'))
const InteractiveTerminal = defineAsyncComponent(() => import('@/components/Tools/InteractiveTerminal.vue'))
const SubagentDetailModal = defineAsyncComponent(() => import('./SubagentDetailModal.vue'))
const TaskPanel = defineAsyncComponent(() => import('./TaskPanel.vue'))
const TeamV4WorkspacePanel = defineAsyncComponent(() => import('./TeamV4WorkspacePanel.vue'))
const WorkspaceFilesPanel = defineAsyncComponent(() => import('./WorkspaceFilesPanel.vue'))

const emit = defineEmits<{
  (e: 'submit', task: string): void
  (e: 'complete', result: any): void
  (e: 'error', error: string): void
  (
    e: 'conversation-changed',
    payload: {
      previousConversationId: string | null
      conversationId: string
      title: string | null
    }
  ): void
  (e: 'memory-message-focused', payload: { memoryId: string; messageId: string }): void
}>()

// i18n
const { t, locale } = useI18n()

const isViewActive = computed(() => props.active)
const isChineseUi = computed(() => locale.value.toLowerCase().startsWith('zh'))
const browserShellDisplayName = computed(() =>
  isChineseUi.value ? '浏览器 Shell' : 'Browser Shell'
)
const browserShellPanelTitle = computed(() =>
  isChineseUi.value ? '打开浏览器 Shell 面板' : 'Open Browser Shell Bridge'
)

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
const agentDockerWorkingDirectory = ref(DEFAULT_DOCKER_WORKING_DIRECTORY)
const agentDockerStorageDirectory = ref('')
const agentExecutionMode = ref<'docker' | 'host'>('host')

const applyAgentRuntimeSettings = (config?: AgentRuntimeSettings | null) => {
  agentDefaultWorkingDirectory.value = String(config?.working_directory || '').trim()
  agentExecutionMode.value = config?.terminal?.default_execution_mode === 'docker' ? 'docker' : 'host'
  const dockerVolumeEntries = Object.entries(config?.shell?.docker_config?.volumes || {})
  const dockerWorkspaceVolume = dockerVolumeEntries.find(
    ([, containerPath]) => containerPath.trim() === DEFAULT_DOCKER_WORKING_DIRECTORY,
  )
  agentDockerWorkingDirectory.value = String(
    dockerWorkspaceVolume?.[1] || DEFAULT_DOCKER_WORKING_DIRECTORY,
  ).trim()
  agentDockerStorageDirectory.value = String(
    dockerWorkspaceVolume?.[0] || '',
  ).trim()
}

const loadAgentRuntimeSettings = () => {
  void invoke<AgentRuntimeSettings>('get_agent_config')
    .then(applyAgentRuntimeSettings)
    .catch(error => {
      console.warn('[AgentView] Failed to load runtime settings:', error)
    })
}

const {
  defaultAssistantProfileId,
  defaultTeamProfileId,
  getAssistantProfileOption,
  getTeamProfileOption,
  loadDefaultAssistantProfile,
  loadDefaultTeamProfile,
  isLoadingAssistantProfiles,
  isLoadingTeamProfiles,
  loadAssistantProfiles,
  loadTeamProfiles,
  profileOptions: assistantProfileOptions,
  teamProfileOptions,
} = useAssistantProfiles()

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
  setTeamProfileId,
  setWorkingDirectoryOverride,
  teamModeEnabled,
  tenthManEnabled,
  toConversationBinding,
  webSearchEnabled,
} = useAssistantSessionSettings()
const { assistantAgentOptions, isLoadingAssistantAgentOptions, selectedAssistantAgentOptionId } =
  useAssistantAgentSwitchOptions({
    assistantProfileOptions,
    assistantSessionSettings,
    defaultTeamProfileId,
    isLoadingAssistantProfiles,
    isLoadingTeamProfiles,
    teamProfileOptions,
  })
const assistantHarnessMaxContinuations = computed(() =>
  normalizeHarnessMaxContinuations(assistantSessionSettings.value.harnessMaxContinuations)
)
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
const displayedConversationWorkingDirectory = computed(() => {
  if (agentExecutionMode.value === 'docker') {
    const convId = conversationId.value
    if (convId) return dockerSessionWorkingDir(convId)
    return agentDockerWorkingDirectory.value.trim()
  }
  return effectiveConversationWorkingDirectory.value.trim()
})
const conversationWorkingDirectoryInputValue = computed(() => {
  if (agentExecutionMode.value === 'docker') {
    return displayedConversationWorkingDirectory.value
  }
  return conversationWorkingDirectoryOverride.value
})
const workspaceFilesStorageDirectory = computed(() => {
  if (agentExecutionMode.value === 'docker') {
    return agentDockerStorageDirectory.value.trim()
  }
  return effectiveConversationWorkingDirectory.value.trim()
})
const workspaceFilesInitialRelativePath = computed(() => {
  if (agentExecutionMode.value !== 'docker') return ''
  const convId = conversationId.value
  if (!convId) return ''
  const sanitized = convId.replace(/[^a-zA-Z0-9\-_]/g, '').slice(0, 12)
  return sanitized ? `context/${EXECUTION_DIR_PREFIX}${sanitized}` : ''
})
const conversationWorkingDirectoryPlaceholder = computed(() => {
  const inherited = displayedConversationWorkingDirectory.value.trim()
  return inherited || '未配置工作目录'
})
const effectiveConversationWorkingDirectoryLabel = computed(() => {
  const resolved = displayedConversationWorkingDirectory.value.trim()
  return resolved || '未配置'
})
const conversationWorkingDirectoryTooltip = computed(() => {
  const overrideValue = conversationWorkingDirectoryOverride.value.trim()
  const resolved = effectiveConversationWorkingDirectoryLabel.value
  if (agentExecutionMode.value === 'docker') {
    return `Docker 工作目录: ${resolved}`
  }
  return overrideValue ? `当前会话工作目录: ${resolved}` : `继承默认工作目录: ${resolved}`
})
const handleConversationWorkingDirectoryInput = (event: Event) => {
  if (agentExecutionMode.value === 'docker') return
  setWorkingDirectoryOverride((event.target as HTMLInputElement).value)
}
const canOpenConversationWorkingDirectory = computed(
  () => agentExecutionMode.value !== 'docker' && effectiveConversationWorkingDirectory.value.trim().length > 0
)
const openConversationWorkingDirectoryTitle = computed(() => {
  if (agentExecutionMode.value === 'docker') {
    return `Docker 工作目录: ${displayedConversationWorkingDirectory.value || '未配置'}`
  }
  const directory = effectiveConversationWorkingDirectory.value.trim()
  return directory ? `打开工作目录: ${directory}` : '未配置工作目录'
})
const openConversationWorkingDirectory = async () => {
  if (agentExecutionMode.value === 'docker') {
    localError.value = 'Docker 工作目录不能通过宿主机文件管理器直接打开。'
    return
  }
  const directory = effectiveConversationWorkingDirectory.value.trim()
  if (!directory) {
    localError.value = '未配置工作目录，无法打开。'
    return
  }

  try {
    await invoke('plugin:opener|open_path', { path: directory, with: null })
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error)
    localError.value = `打开工作目录失败: ${message}`
  }
}
const selectConversationWorkingDirectory = async () => {
  const { open } = await import('@tauri-apps/plugin-dialog')
  const selected = await open({
    directory: true,
    multiple: false,
    title: '选择当前会话工作目录',
  })
  if (typeof selected === 'string' && selected.trim()) {
    setWorkingDirectoryOverride(selected)
  }
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
const activeTeamV4RunId = ref<string | null>(null)
const isTeamWorkspaceActive = ref(false)
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
const agentEvents = useAgentEvents(
  computed(() => conversationId.value || ''),
  {
    suppressUserMessages: computed(() => teamModeEnabled.value),
    defaultMaxContextTokens: assistantDefaultMaxContextTokens,
    subagentParentExecutionMatcher: () => false,
  }
)
const messages = computed(() => agentEvents.messages.value)
const visibleMessages = messages
const isTeamRunActive = computed(() => {
  if (activeTeamV4RunId.value) {
    return ['draft', 'planning', 'running', 'waiting_human'].includes(
      String(teamV4Run.value?.state || '')
        .trim()
        .toLowerCase()
    )
  }
  return false
})

const teamWorkspaceAvailable = computed(() =>
  Boolean(
    teamModeEnabled.value ||
      activeTeamV4RunId.value ||
      teamV4Agents.value.length ||
      teamV4Events.value.length ||
      teamV4HarnessRuns.value.length ||
      teamV4Memories.value.length ||
      teamV4Run.value ||
      teamV4Tasks.value.length
  )
)
const teamWorkspaceBadgeCount = computed(
  () =>
    teamV4Tasks.value.filter((task) => ['failed', 'cancelled'].includes(String(task.status || '').trim().toLowerCase()))
      .length
)
const isExecuting = computed(() => agentEvents.isExecuting.value || isTeamRunActive.value)
const isStreaming = computed(
  () => agentEvents.isExecuting.value && !!agentEvents.streamingContent.value
)
const streamingContent = computed(() => agentEvents.streamingContent.value)
const contextUsage = computed(() => agentEvents.contextUsage.value)
const contextCompression = computed(() => agentEvents.contextCompression.value)
const focusTeamTaskInWorkspace = (taskId: string) => {
  const normalizedTaskId = String(taskId || '').trim()
  if (!normalizedTaskId) return
  activateRightPanel('team')
  isTeamWorkspaceActive.value = true
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
  emitMemoryMessageFocused: payload => emit('memory-message-focused', payload),
  focusTeamTaskInWorkspace,
})

const scrollMessageViewportToBottom = () => {
  messageFlowRef.value?.scrollToBottom()
}

const handleToggleTeamMode = async (enabled: boolean) => {
  teamModeEnabled.value = enabled
  if (enabled) return
  isTeamWorkspaceActive.value = false
}

const handleToggleTeamWorkspace = () => {
  if (!teamWorkspaceAvailable.value) return
  if (activeRightPanel.value === 'team') {
    deactivateRightPanel('team')
    return
  }
  activateRightPanel('team')
  isTeamWorkspaceActive.value = true
}
const { handleViewSubagentDetails, loadSubagentRuns, selectedSubagent, showSubagentDetailModal } =
  useAgentSubagents({
    conversationId,
    historyLoadToken,
    subagents: agentEvents.subagents,
  })
const subagentMessageStore = useSubagentMessageStore({
  parentExecutionId: conversationId,
  subagents: agentEvents.subagents,
})
const selectedSubagentId = computed(() => selectedSubagent.value?.id || null)
const selectedSubagentMessageState = subagentMessageStore.selectedState(selectedSubagentId)

const taskComposable = useAgentTasks()
const terminalComposable = useTerminal()
const browserShellComposable = useBrowserShell()
const currentBrowserShellSessionId = computed(() => browserShellComposable.currentSessionId.value)
const currentBrowserShellDirectWriteEnabled = computed(
  () => browserShellComposable.directWriteEnabled.value
)
const buildCurrentConversationBinding = () =>
  toConversationBinding({
    browserShellDirectWriteEnabled: currentBrowserShellDirectWriteEnabled.value,
    browserShellSessionId: currentBrowserShellSessionId.value,
    selectedModel: assistantSelectedModel.value,
    toolsEnabled: toolsEnabled.value,
    toolConfig: toolConfig.value,
  })
const buildNewConversationBinding = () => {
  return buildNewAssistantConversationBinding({
    defaultTeamProfileId: defaultTeamProfileId.value,
    getAssistantProfileOption,
    profileId: assistantSessionSettings.value.profileId,
    runMode: assistantSessionSettings.value.runMode,
    teamProfileId: assistantSessionSettings.value.teamProfileId,
    teamProfileOptions: teamProfileOptions.value,
    workingDirectoryOverride: conversationWorkingDirectoryOverride.value,
  })
}
const {
  activeRightPanel,
  activateRightPanel,
  clearError,
  deactivateRightPanel,
  error,
  handleCloseHarness,
  handleCloseHtmlPanel,
  handleCloseTasks,
  handleCloseTerminal,
  handleRenderHtml,
  handleTaskSourceChange,
  handleToggleHarness,
  handleToggleBrowserShell,
  handleToggleHtmlPanel,
  handleToggleTasks,
  handleToggleTerminal,
  hasHtmlPanelContent,
  htmlPanelContent,
  loadSidebarWidth,
  loadToolConfigDrawerWidth,
  pruneTasksForCurrentContextAfter,
  selectedTaskSourceKey,
  sidebarWidth,
  startToolConfigDrawerResize,
  startResize,
  taskBadgeCount,
  taskSourceOptions,
  tasks,
  toolConfigDrawerWidth,
} = useAgentPanels({
  agentError: computed(() => agentEvents.error.value),
  clearTasksForExecution: taskComposable.clearTasksForExecution,
  conversationId,
  getConversationIdForExecution: taskComposable.getConversationIdForExecution,
  getTasksForExecution: taskComposable.getTasksForExecution,
  isTeamWorkspaceActive,
  isTaskPanelActive: computed(() => taskComposable.isTaskPanelActive.value),
  localError,
  parallelTaskSources: computed(() => agentEvents.parallelTaskSources.value),
  pruneTasksForExecutionAfter: async (executionId, timestampMs) => {
    const remaining = await invoke<any[]>('prune_agent_tasks_after', {
      executionId,
      afterTimestampMs: timestampMs,
    })
    return mapPersistedAgentTasks(remaining)
  },
  propsShowTasks: props.showTasks,
  resetAgentError: () => {
    agentEvents.resetError()
  },
  setTasksForExecution: taskComposable.setTasksForExecution,
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
  lastOverlayTrigger.value =
    document.activeElement instanceof HTMLElement ? document.activeElement : null
  showConversations.value = false
  activateRightPanel('work-config')
}

const toggleToolConfigDrawer = () => {
  lastOverlayTrigger.value =
    document.activeElement instanceof HTMLElement ? document.activeElement : null
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
  lastOverlayTrigger.value =
    document.activeElement instanceof HTMLElement ? document.activeElement : null
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
  return (
    tagName === 'input' ||
    tagName === 'textarea' ||
    tagName === 'select' ||
    target.isContentEditable
  )
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

const { hasConnectedBrowserShellSessions, isCurrentBoundBrowserShellConnected } =
  useAgentBrowserShellAvailability({
    currentBrowserShellSessionId,
    clearBoundBrowserShellSession,
  })

watch(hasConnectedBrowserShellSessions, hasConnected => {
  if (!hasConnected && activeRightPanel.value === 'browser-shell') {
    deactivateRightPanel('browser-shell')
  }
})

watch(showConversations, open => {
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
  window.addEventListener(AI_CONFIG_UPDATED_EVENT, loadAgentRuntimeSettings)
  loadAgentRuntimeSettings()
})

onUnmounted(() => {
  window.removeEventListener('agent:parallel-task-source-focus', handleParallelTaskSourceFocus)
  window.removeEventListener('keydown', handleGlobalKeydown)
  window.removeEventListener(AI_CONFIG_UPDATED_EVENT, loadAgentRuntimeSettings)
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
  const persistedTasks = await invoke<any[]>('get_agent_tasks_for_conversation', {
    conversationId: targetConversationId,
  })
  const rowsByExecutionId = new Map<string, any[]>()
  for (const row of persistedTasks || []) {
    const executionId = String(row?.execution_id || '').trim()
    if (!executionId) continue
    const rows = rowsByExecutionId.get(executionId) || []
    rows.push(row)
    rowsByExecutionId.set(executionId, rows)
  }
  for (const [executionId, rows] of rowsByExecutionId.entries()) {
    taskComposable.setTasksForExecution(
      executionId,
      mapPersistedAgentTasks(rows),
      targetConversationId
    )
  }
}

const handleAssistantModelSelection = (value: string | null) => {
  handleAssistantModelChange(value || '')
}
const {
  assistantProfileRegistryReady,
  handleAssistantContextModeChange,
  handleAssistantProfileChange,
  loadConversationBinding,
  schedulePersistConversationBinding,
} = useAgentConversationBinding({
  bindBrowserShellSession: sessionId => {
    browserShellComposable.bindSession(sessionId)
  },
  setBrowserShellDirectWriteEnabled: enabled => {
    browserShellComposable.setDirectWriteEnabled(enabled)
  },
  conversationId,
  currentBrowserShellDirectWriteEnabled,
  currentBrowserShellSessionId,
  assistantGlobalDefaultModel,
  assistantSelectedModel,
  defaultAssistantProfileId,
  defaultTeamProfileId,
  defaultToolConfig,
  toolConfig,
  toolsEnabled,
  applyConversationBinding,
  applyProfilePreset,
  getAssistantProfileOption,
  getTeamProfileOption,
  handleToggleTeamMode,
  resetSessionSettings,
  setAssistantSelectedModel,
  setContextMode,
  setProfileId,
  setRunMode,
  setTeamProfileId,
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
  await teamRuntimeApi.finishHarnessRun(harnessRunId, {
    status: 'cancelled',
    error: 'Cancelled from Team workspace.',
  })
  await loadTeamV4WorkspaceData()
}

const resumeTeamV4HarnessRun = async (harnessRunId: string) => {
  await teamRuntimeApi.resumeHarnessRun(harnessRunId, 600)
  await loadTeamV4WorkspaceData()
}

const startTeamV4AssistantRun = async (goal: string) => {
  await flushPendingToolConfigSave()
  await Promise.all([loadTeamProfiles(), loadDefaultTeamProfile()])
  const selectedTeamProfileId =
    assistantSessionSettings.value.teamProfileId.trim() ||
    defaultTeamProfileId.value.trim() ||
    teamProfileOptions.value[0]?.id ||
    null
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
  teamV4Tasks.value = bootstrap.specialistAssignments.map(assignment => assignment.task)
  teamV4Events.value = bootstrap.events
  teamV4Memories.value = []
  teamV4HarnessRuns.value = bootstrap.specialistAssignments.map(assignment => assignment.harnessRun)
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
    persistTeamV4Message({
      conversationId: conversationId.value,
      message: userMessage,
      role: 'user',
    }),
    persistTeamV4Message({
      conversationId: conversationId.value,
      message: assistantMessage,
      role: 'assistant',
    }),
    loadTeamV4WorkspaceData(bootstrap.run.id),
  ]).catch(error => {
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
  await Promise.all(
    teamV4HarnessRuns.value
      .filter(harness => ['queued', 'running', 'paused', 'expired'].includes(harness.status))
      .map(harness =>
        teamRuntimeApi.finishHarnessRun(harness.id, {
          status: 'cancelled',
          error: 'Team run stopped by user.',
        })
      )
  )
  await teamRuntimeApi.updateRunState(runId, 'cancelled')
  await loadTeamV4WorkspaceData(runId)
}

const {
  handleClearConversation,
  handleConversationExecutionStateUpdate,
  handleCreateConversation: handleCreateConversationFlow,
  handleEditMessage,
  handleInterruptSubmit,
  handleResendMessage,
  handleSelectConversation: handleSelectConversationFlow,
  handleStop,
  handleSubmit,
  loadConversationHistory,
  loadLatestConversation,
} = useAgentConversationFlow({
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
  buildNewConversationBinding,
  effectiveWorkingDirectory: effectiveConversationWorkingDirectory,
  buildToolConfig: () => toolConfig.value as unknown as UiToolConfigPayload,
  clearAgentMessages: () => {
    agentEvents.clearMessages()
  },
  clearDraftArtifacts,
  closeConversationDrawer: () => {
    showConversations.value = false
  },
  conversationExecutionState,
  conversationId,
  currentExecutionId: agentEvents.currentExecutionId,
  currentConversationTitle,
  emitComplete: payload => {
    emit('complete', payload)
  },
  emitError: message => {
    emit('error', message)
  },
  emitSubmit: task => {
    emit('submit', task)
  },
  executionIdProp: props.executionId,
  forceTaskPlanContract: false,
  harnessMaxContinuations: assistantHarnessMaxContinuations,
  getFailedToClearConversationLabel: () => t('agent.failedToClearConversation'),
  getFailedToStopExecutionLabel: () => t('agent.failedToStopExecution'),
  getNewConversationTitle: () =>
    `${t('agent.newConversationTitle')} ${new Date().toLocaleString()}`,
  getToolCallCompletedLabel: () => t('agent.toolCallCompleted'),
  getUnnamedConversationTitle: () => t('agent.newConversationTitle'),
  getAssistantProfileOption,
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
  pruneTasksForCurrentContextAfter,
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
  scrollMessageViewportToBottom,
  setPendingDocumentAttachments: documents => {
    agentEvents.setPendingDocumentAttachments(documents)
  },
  startTeamV4AssistantRun,
  stopAgentExecutionState: () => {
    agentEvents.stopExecution()
  },
  stopTeamV4Run,
  submitInFlight,
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
  assistantProfileRegistryReady,
  conversationExecutionState,
  conversationId,
  executionId: props.executionId,
  loadAssistantModelOptions,
  loadAssistantProfiles,
  loadDefaultAssistantProfile,
  loadDefaultTeamProfile,
  loadConversationHistory,
  loadLatestConversation,
  loadSidebarWidth,
  loadToolConfigDrawerWidth,
  loadTeamProfiles,
  loadToolConfig,
  focusInput: () => inputAreaRef.value?.focusInput(),
  handleConversationExecutionStateUpdate,
  preconnectTerminal: () => terminalComposable.preconnect(),
  scrollMessageViewportToBottom,
})

watch(
  [assistantProfileRegistryReady, conversationId],
  ([ready, value]) => {
    if (!ready) return
    void loadConversationBinding(value)
  },
  { immediate: true }
)

watch(
  [
    conversationId,
    ragEnabled,
    webSearchEnabled,
    tenthManEnabled,
    teamModeEnabled,
    () => assistantSessionSettings.value.teamProfileId,
    assistantSelectedModel,
    currentBrowserShellDirectWriteEnabled,
    currentBrowserShellSessionId,
    conversationWorkingDirectoryOverride,
    toolsEnabled,
    toolConfig,
  ],
  () => {
    schedulePersistConversationBinding()
  }
)

watch(error, value => {
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
