<template>
  <div class="ai-assistant-view page-content-full h-full flex flex-col bg-base-100 overflow-hidden">
    <!-- {{ t('aiAssistant.headerControlBar') }} -->
    <div class="navbar bg-base-200 shadow-sm border-b border-base-300 flex-shrink-0">
      <div class="navbar-start">
        <h1 class="text-xl font-bold flex items-center gap-2">
          <i class="fas fa-robot text-primary"></i>
          {{ t('aiAssistant.title', 'AI智能助手') }}
        </h1>
      </div>
      <div class="navbar-end">
        <div class="flex items-center gap-2">
          <!-- {{ t('aiAssistant.roleSelector') }} -->
          <div class="dropdown dropdown-end">
            <div tabindex="0" role="button" class="btn btn-sm btn-outline gap-2">
              <i class="fas fa-user-tie"></i>
              {{ selectedRole ? selectedRole.title : t('aiAssistant.selectRole', '选择角色') }}
              <i class="fas fa-chevron-down text-xs"></i>
            </div>
            <ul tabindex="0" class="dropdown-content z-[1000] menu p-2 shadow bg-base-100 rounded-box w-72 md:w-80">
              <li><span class="menu-title">{{ t('aiAssistant.availableRoles', '可用角色') }}</span></li>
              <li @click="handleSelectRole(null)">
                <a class="flex items-center justify-between gap-3" :class="{ 'active': !selectedRole }">
                  <div class="flex items-center gap-2">
                    <div class="badge badge-xs badge-ghost">{{ t('aiAssistant.defaultBadge') }}</div>
                    <span>{{ t('aiAssistant.defaultRole', '默认助手') }}</span>
                  </div>
                </a>
              </li>
              <div class="divider my-1"></div>
              <li v-for="role in roles" :key="role.id" @click="handleSelectRole(role)">
                <a class="flex items-center justify-between gap-3" :class="{ 'active': selectedRole?.id === role.id }">
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
    </div>

    <!-- {{ t('aiAssistant.mainContentArea') }} -->
    <div class="flex-1 overflow-hidden min-h-0 flex flex-col">
      <AgentTabs @new-tab="handleNewTab" />
      
      <div class="flex-1 relative overflow-hidden">
        <template v-for="session in sessions" :key="session.id">
          <AgentView 
            v-show="session.id === activeSessionId"
            :ref="el => setAgentRef(el, session.id)"
            :execution-id="session.id"
            :show-todos="true"
            :selected-role="selectedRole"
            class="absolute inset-0"
            @submit="handleAgentSubmit"
            @complete="handleAgentComplete"
            @error="handleAgentError"
          />
        </template>
        
        <!-- Empty State -->
        <div v-if="sessions.length === 0" class="flex flex-col items-center justify-center h-full text-base-content/40 gap-4">
          <i class="fas fa-robot text-6xl"></i>
          <p>{{ t('aiAssistant.noActiveSessions', '暂无活跃对话，请开启新标签页') }}</p>
          <button class="btn btn-primary btn-sm" @click="handleNewTab">
            <i class="fas fa-plus"></i>
            {{ t('aiAssistant.startNewConversation', '开启新对话') }}
          </button>
        </div>
      </div>
    </div>

    <!-- {{ t('aiAssistant.roleManagementModal') }} -->
    <RoleManagement v-if="showRoleManagement" @close="showRoleManagement = false" />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, onActivated, nextTick, shallowRef } from 'vue'
import { useI18n } from 'vue-i18n'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { invoke } from '@tauri-apps/api/core'
import RoleManagement from '@/components/RoleManagement.vue'
import { AgentView, AgentTabs } from '@/components/Agent'
import { useRoleManagement } from '@/composables/useRoleManagement'
import { useAgentSessionManager } from '@/composables/useAgentSessionManager'

// {{ t('aiAssistant.trafficReferenceType') }}
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

defineOptions({
  name: 'AIAssistant'
});

const { t } = useI18n()

// 角色管理
const {
  roles,
  selectedRole,
  isLoading: isLoadingRoles,
  loadRoles,
  selectRole,
} = useRoleManagement()

// 角色管理状态
const showRoleManagement = ref(false)

// --- 会话管理 ---
const { sessions, activeSessionId, addSession } = useAgentSessionManager()
const agentViewRefs = ref<Record<string, any>>({})

const setAgentRef = (el: any, id: string) => {
  if (el) {
    agentViewRefs.value[id] = el
  } else {
    delete agentViewRefs.value[id]
  }
}

const activeAgentView = computed(() => {
  if (!activeSessionId.value) return null
  return agentViewRefs.value[activeSessionId.value]
})

const handleNewTab = async () => {
  try {
    const convId = await invoke<string>('create_ai_conversation', {
      request: {
        title: `${t('agent.newConversationTitle')} ${new Date().toLocaleString()}`,
        service_name: 'default'
      }
    })
    addSession(convId, t('agent.newConversationTitle'))
  } catch (e) {
    console.error('Failed to create new conversation for tab:', e)
  }
}

// 流量事件监听器
let unlistenTraffic: UnlistenFn | null = null

const handleSelectRole = async (role: any) => {
  try {
    await selectRole(role)
  } catch (error) {
    console.error('Failed to select role:', error)
  }
}

// --- 事件处理 ---
const handleAgentSubmit = (task: string) => {
  console.log('Agent task submitted:', task)
}

const handleAgentComplete = async (result: any) => {
  console.log('Agent task completed:', result)
}

const handleAgentError = (error: string) => {
  console.error('Agent task error:', error)
}

// 初始化
onMounted(async () => {
  try {
    // 加载角色列表
    await loadRoles()

    // 如果没有任何会话，尝试加载最近的一个或创建一个
    if (sessions.value.length === 0) {
      const conversations = await invoke<any[]>('get_ai_conversations')
      if (conversations && conversations.length > 0) {
        const latest = conversations.sort((a, b) => 
          new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime()
        )[0]
        addSession(latest.id, latest.title || t('agent.unnamedConversation'))
      } else {
        await handleNewTab()
      }
    }

    // 监听流量发送到助手事件
    unlistenTraffic = await listen<{ requests: ReferencedTraffic[], type?: 'request' | 'response' | 'both' }>('traffic:send-to-assistant', (event) => {
      console.log('AIAssistant: Received traffic data:', event.payload)
      if (event.payload?.requests && activeAgentView.value?.addReferencedTraffic) {
        const type = event.payload.type || 'both'
        activeAgentView.value.addReferencedTraffic(event.payload.requests, type)
      }
    })
  } catch (error) {
    console.error('Failed to initialize AI Assistant:', error)
  }
})

// 清理
onUnmounted(() => {
  if (unlistenTraffic) {
    unlistenTraffic()
    unlistenTraffic = null
  }
})

// 激活时聚焦
onActivated(() => {
  nextTick(() => {
    activeAgentView.value?.focusInput()
  })
})
</script>

<style scoped>
.ai-assistant-view {
  font-family: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
}

/* 自定义滚动条 */
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

/* 状态指示器动画 */
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
