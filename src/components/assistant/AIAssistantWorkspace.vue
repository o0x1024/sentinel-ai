<template>
  <div class="ai-assistant-workspace h-full flex flex-col bg-base-100 overflow-hidden">
    <div class="flex-1 overflow-hidden min-h-0 flex flex-col">
      <div class="flex items-center bg-base-300/30 px-2 pt-2 gap-2 border-b border-base-300">
        <div class="flex-1 min-w-0">
          <AgentTabs @new-tab="handleNewTab" />
        </div>

        <div class="flex-shrink-0 pb-2 flex items-center gap-2">
          <button
            class="btn btn-sm btn-outline gap-2"
            :title="t('aiAssistant.turnLogsTitle', 'Turn 日志')"
            @click="showTurnLogsModal = true"
          >
            <i class="fas fa-scroll"></i>
            <span class="hidden md:inline">{{ t('aiAssistant.turnLogsButton', 'Turn 日志') }}</span>
          </button>
          <button
            class="btn btn-sm btn-outline gap-2"
            :title="t('aiAssistant.manageForcedRules', '管理强制规则')"
            @click="showForcedRulesModal = true"
          >
            <i class="fas fa-file-signature"></i>
            <span class="hidden md:inline">{{ t('aiAssistant.forcedRules', '强制规则') }}</span>
          </button>
          <div class="dropdown dropdown-end">
            <div tabindex="0" role="button" class="btn btn-sm btn-outline gap-2">
              <i class="fas fa-user-tie"></i>
              {{ selectedRole ? selectedRole.title : t('aiAssistant.selectRole', '选择角色') }}
              <i class="fas fa-chevron-down text-xs"></i>
            </div>
            <ul tabindex="0" class="dropdown-content z-[1000] menu p-2 shadow bg-base-100 rounded-box w-72 md:w-80">
              <li><span class="menu-title">{{ t('aiAssistant.availableRoles', '可用角色') }}</span></li>
              <li @click="handleSelectRole(null)">
                <a class="flex items-center justify-between gap-3" :class="{ active: !selectedRole }">
                  <div class="flex items-center gap-2">
                    <div class="badge badge-xs badge-ghost">{{ t('aiAssistant.defaultBadge') }}</div>
                    <span>{{ t('aiAssistant.defaultRole', '默认助手') }}</span>
                  </div>
                </a>
              </li>
              <div class="divider my-1"></div>
              <li v-for="role in roles" :key="role.id" @click="handleSelectRole(role)">
                <a class="flex items-center justify-between gap-3" :class="{ active: selectedRole?.id === role.id }">
                  <div class="flex items-center gap-2">
                    <div class="badge badge-xs badge-primary">{{ t('aiAssistant.roleBadge') }}</div>
                    <span class="truncate">{{ role.title }}</span>
                  </div>
                  <div class="text-xs text-base-content/60 truncate max-w-20" :title="role.description">
                    {{ role.description }}
                  </div>
                </a>
              </li>
              <div class="divider my-1"></div>
              <li @click="showRoleManagement = true">
                <a class="flex items-center gap-2 text-primary">
                  <i class="fas fa-cog"></i>
                  <span>{{ t('aiAssistant.manageRoles', '管理角色') }}</span>
                </a>
              </li>
              <li v-if="roles.length === 0 && !isLoadingRoles">
                <span class="text-base-content/50 text-sm">{{ t('aiAssistant.noRoles', '暂无自定义角色') }}</span>
              </li>
            </ul>
          </div>
        </div>
      </div>

      <div class="flex-1 relative overflow-hidden">
        <AgentView
          v-for="session in sessions"
          :key="session.id"
          :ref="(el) => setAgentViewRef(session.id, el)"
          v-show="activeSessionId === session.id"
          :execution-id="session.id"
          :focused-memory-id="focusedMemoryConversationId === session.id ? focusedMemoryId : null"
          :focused-message-id="focusedMemoryConversationId === session.id ? focusedMessageId : null"
          :show-tasks="true"
          :selected-role="selectedRole"
          class="absolute inset-0"
          @submit="handleAgentSubmit"
          @complete="handleAgentComplete"
          @error="handleAgentError"
          @conversation-changed="(payload) => handleAgentConversationChanged(session.id, payload)"
          @memory-message-focused="handleMemoryMessageFocused"
        />

        <div
          v-if="isBootstrapping && sessions.length === 0"
          class="flex flex-col items-center justify-center h-full text-base-content/50 gap-4"
        >
          <span class="loading loading-spinner loading-lg text-primary"></span>
          <p>{{ t('aiAssistant.loadingSessions', '正在恢复历史会话...') }}</p>
        </div>

        <div v-else-if="sessions.length === 0" class="flex flex-col items-center justify-center h-full text-base-content/40 gap-4">
          <i class="fas fa-robot text-6xl"></i>
          <p>{{ t('aiAssistant.noActiveSessions', '暂无活跃对话，请开启新标签页') }}</p>
          <button class="btn btn-primary btn-sm" @click="handleNewTab">
            <i class="fas fa-plus"></i>
            {{ t('aiAssistant.startNewConversation', '开启新对话') }}
          </button>
        </div>
      </div>
    </div>

    <RoleManagement v-if="showRoleManagement" @close="showRoleManagement = false" />
    <UserForcedRulesModal
      v-if="showForcedRulesModal"
      @close="showForcedRulesModal = false"
      @saved="handleForcedRulesSaved"
    />
    <TurnLogsModal
      v-model="showTurnLogsModal"
      :active-session-id="activeSessionId"
      :conversation-options="sessions"
      @open-conversation="handleOpenConversationFromLog"
    />
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onActivated, onMounted, onUnmounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { invoke } from '@tauri-apps/api/core'
import { useRoute, useRouter } from 'vue-router'
import RoleManagement from '@/components/RoleManagement.vue'
import UserForcedRulesModal from '@/components/UserForcedRulesModal.vue'
import TurnLogsModal from '@/components/Agent/TurnLogsModal.vue'
import { AgentTabs, AgentView } from '@/components/Agent'
import type { AiTurnLogSummaryEntry } from '@/api/aiLogs'
import { useRoleManagement } from '@/composables/useRoleManagement'
import { useAgentSessionManager } from '@/composables/useAgentSessionManager'
import { dialog } from '@/composables/useDialog'
import type { AiConversationSummary } from '@/components/Agent/conversationTypes'
import { pickLatestConversation } from '@/components/Agent/agentConversationSessionSupport'
import { buildFocusedMessageQuery, readFocusLocationState } from '@/components/Agent/focusLocationSupport'
import { isAssistantPresentationTarget, type AssistantPresentationTarget } from '@/services/assistantPresentation'

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

type PendingTrafficReference = {
  requests: ReferencedTraffic[]
  type: 'request' | 'response' | 'both'
}

const props = withDefaults(
  defineProps<{
    presentationTarget: AssistantPresentationTarget
    active?: boolean
  }>(),
  {
    active: true,
  },
)

defineOptions({
  name: 'AIAssistantWorkspace',
})

const { t } = useI18n()
const route = useRoute()
const router = useRouter()
const {
  roles,
  selectedRole,
  isLoading: isLoadingRoles,
  loadRoles,
  selectRole,
} = useRoleManagement()
const showRoleManagement = ref(false)
const showForcedRulesModal = ref(false)
const showTurnLogsModal = ref(false)
const isBootstrapping = ref(true)
const {
  sessions,
  activeSessionId,
  preferredBootstrapSessionId,
  addSession,
  replaceSession,
  syncSessionsWithConversations,
} = useAgentSessionManager()
const agentViewRefs = ref<Record<string, any>>({})
const focusLocation = computed(() => readFocusLocationState(route.query))
const focusedMemoryConversationId = computed(() => focusLocation.value.conversationId)
const focusedMemoryId = computed(() => focusLocation.value.memoryId)
const focusedMessageId = computed(() => focusLocation.value.focusedMessageId)
const pendingTrafficReferences = ref<PendingTrafficReference[]>([])
const pendingAssetReferences = ref<ReferencedAsset[][]>([])

let unlistenTraffic: UnlistenFn | null = null
let unlistenAssets: UnlistenFn | null = null

const setAgentViewRef = (sessionId: string, el: any | null) => {
  if (el) {
    agentViewRefs.value[sessionId] = el
    void nextTick(flushPendingReferences)
    return
  }

  delete agentViewRefs.value[sessionId]
}

const getActiveAgentViewRef = () => {
  if (!activeSessionId.value) return null
  return agentViewRefs.value[activeSessionId.value] ?? null
}

function queueTrafficReferences(requests: ReferencedTraffic[], type: 'request' | 'response' | 'both') {
  pendingTrafficReferences.value.push({
    requests,
    type,
  })
  void nextTick(flushPendingReferences)
}

function queueAssetReferences(assets: ReferencedAsset[]) {
  pendingAssetReferences.value.push(assets)
  void nextTick(flushPendingReferences)
}

function flushPendingReferences() {
  const activeAgentViewRef = getActiveAgentViewRef()
  if (!activeAgentViewRef) {
    return
  }

  if (pendingTrafficReferences.value.length > 0 && activeAgentViewRef.addReferencedTraffic) {
    for (const payload of pendingTrafficReferences.value) {
      activeAgentViewRef.addReferencedTraffic(payload.requests, payload.type)
    }
    pendingTrafficReferences.value = []
  }

  if (pendingAssetReferences.value.length > 0 && activeAgentViewRef.addReferencedAssets) {
    for (const payload of pendingAssetReferences.value) {
      activeAgentViewRef.addReferencedAssets(payload)
    }
    pendingAssetReferences.value = []
  }
}

async function focusActiveAgentInput() {
  await nextTick()
  flushPendingReferences()
  getActiveAgentViewRef()?.focusInput?.()
}

const handleNewTab = async () => {
  try {
    const convId = await invoke<string>('create_ai_conversation', {
      request: {
        title: `${t('agent.newConversationTitle')} ${new Date().toLocaleString()}`,
        service_name: 'default',
      },
    })
    addSession(convId, t('agent.newConversationTitle'))
    await focusActiveAgentInput()
  } catch (e) {
    console.error('Failed to create new conversation for tab:', e)
  }
}

const handleSelectRole = async (role: any) => {
  try {
    await selectRole(role)
  } catch (error) {
    console.error('Failed to select role:', error)
  }
}

const handleForcedRulesSaved = () => {
  dialog.toast.success(t('aiAssistant.forcedRulesSaved', '强制规则已保存'))
}

const openConversationById = async (conversationId: string, fallbackTitle?: string) => {
  const normalizedConversationId = String(conversationId || '').trim()
  if (!normalizedConversationId) return false

  const existing = sessions.value.find(session => session.id === normalizedConversationId)
  if (existing) {
    addSession(existing.id, existing.title)
    await focusActiveAgentInput()
    return true
  }

  try {
    const conversations = await invoke<AiConversationSummary[]>('get_ai_conversations')
    const matched = Array.isArray(conversations)
      ? conversations.find(item => String(item?.id || '').trim() === normalizedConversationId)
      : null

    if (!matched) return false

    addSession(
      normalizedConversationId,
      matched.title || fallbackTitle || t('agent.unnamedConversation'),
    )
    await focusActiveAgentInput()
    return true
  } catch (error) {
    console.error('Failed to open conversation by route:', error)
    return false
  }
}

const handleOpenConversationFromLog = async (entry: AiTurnLogSummaryEntry) => {
  const conversationId = String(entry.conversation_id || '').trim()
  if (!conversationId) return

  await openConversationById(
    conversationId,
    String(entry.user_request_preview || '').trim().slice(0, 40) || t('agent.unnamedConversation'),
  )
}

const syncConversationFromRoute = async () => {
  const conversationId = typeof route.query.conversationId === 'string'
    ? route.query.conversationId
    : typeof route.query.conversation_id === 'string'
      ? route.query.conversation_id
      : ''

  if (!conversationId) return false

  return await openConversationById(
    conversationId,
    `${t('agent.unnamedConversation')} ${String(conversationId).slice(0, 8)}`,
  )
}

const syncConversationFromLastSession = async () => {
  const conversationId = String(preferredBootstrapSessionId.value || '').trim()
  if (!conversationId) return false

  return await openConversationById(
    conversationId,
    `${t('agent.unnamedConversation')} ${conversationId.slice(0, 8)}`,
  )
}

const handleAgentSubmit = (task: string) => {
  console.log('Agent task submitted:', task)
}

const handleAgentComplete = async (result: any) => {
  console.log('Agent task completed:', result)
}

const handleAgentError = (error: string) => {
  console.error('Agent task error:', error)
}

const handleAgentConversationChanged = (
  tabSessionId: string,
  payload: {
    previousConversationId: string | null
    conversationId: string
    title: string | null
  },
) => {
  const nextConversationId = String(payload.conversationId || '').trim()
  if (!nextConversationId) return

  const currentTabSessionId = String(payload.previousConversationId || tabSessionId || '').trim()
  if (!currentTabSessionId) return

  replaceSession(
    currentTabSessionId,
    nextConversationId,
    payload.title || undefined,
  )
}

const handleMemoryMessageFocused = ({ memoryId, messageId }: { memoryId: string; messageId: string }) => {
  const nextQuery = buildFocusedMessageQuery(route.query, { memoryId, messageId })
  if (JSON.stringify(nextQuery) === JSON.stringify(route.query)) return
  void router.replace({ query: nextQuery })
}

onMounted(async () => {
  let roleLoadPromise: Promise<void> | null = null

  try {
    roleLoadPromise = loadRoles()

    const conversations = await invoke<AiConversationSummary[]>('get_ai_conversations')
    syncSessionsWithConversations(conversations || [])

    const openedFromRoute = await syncConversationFromRoute()
    const openedFromLastSession = openedFromRoute
      ? false
      : await syncConversationFromLastSession()

    if (!openedFromRoute && !openedFromLastSession && sessions.value.length === 0) {
      const latest = pickLatestConversation(conversations || [])
      if (latest) {
        addSession(latest.id, latest.title || t('agent.unnamedConversation'))
      } else {
        await handleNewTab()
      }
    }

    unlistenTraffic = await listen<{ requests: ReferencedTraffic[]; type?: 'request' | 'response' | 'both' }>(
      'traffic:send-to-assistant',
      (event) => {
        if (!isAssistantPresentationTarget(props.presentationTarget) || !event.payload?.requests?.length) {
          return
        }

        queueTrafficReferences(event.payload.requests, event.payload.type || 'both')
      },
    )

    unlistenAssets = await listen<{ assets: ReferencedAsset[] }>('asset:send-to-assistant', (event) => {
      if (!isAssistantPresentationTarget(props.presentationTarget) || !event.payload?.assets?.length) {
        return
      }

      queueAssetReferences(event.payload.assets)
    })
  } catch (error) {
    console.error('Failed to initialize AI Assistant:', error)
  } finally {
    isBootstrapping.value = false
    if (roleLoadPromise) {
      await roleLoadPromise
    }
    void focusActiveAgentInput()
  }
})

onUnmounted(() => {
  if (unlistenTraffic) {
    unlistenTraffic()
    unlistenTraffic = null
  }
  if (unlistenAssets) {
    unlistenAssets()
    unlistenAssets = null
  }
})

onActivated(() => {
  if (props.active) {
    void focusActiveAgentInput()
  }
})

watch(
  () => props.active,
  (active) => {
    if (active) {
      void focusActiveAgentInput()
      return
    }

    pendingTrafficReferences.value = []
    pendingAssetReferences.value = []
  },
)

watch(
  () => [route.query.conversationId, route.query.conversation_id],
  async ([conversationId, conversationIdSnake]) => {
    const nextConversationId = typeof conversationId === 'string'
      ? conversationId
      : typeof conversationIdSnake === 'string'
        ? conversationIdSnake
        : ''

    if (!nextConversationId) return
    await openConversationById(nextConversationId)
  },
)

watch(activeSessionId, () => {
  if (props.active) {
    void focusActiveAgentInput()
  }
})
</script>

<style scoped>
.ai-assistant-workspace {
  font-family: var(--app-font-sans);
}

::-webkit-scrollbar {
  width: 6px;
}

::-webkit-scrollbar-track {
  background: transparent;
}

::-webkit-scrollbar-thumb {
  background: hsl(var(--bc) / 0.2);
  border-radius: 3px;
}

::-webkit-scrollbar-thumb:hover {
  background: hsl(var(--bc) / 0.3);
}

.badge-success {
  animation: pulse 2s infinite;
}

@keyframes pulse {
  0%, 100% {
    opacity: 1;
  }
  50% {
    opacity: 0.7;
  }
}
</style>
