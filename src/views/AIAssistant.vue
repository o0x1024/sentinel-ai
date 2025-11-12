<template>
  <div class="ai-assistant-view page-content-full h-full flex flex-col bg-base-100 overflow-hidden">
    <!-- 头部控制栏 -->
    <div class="navbar bg-base-200 shadow-sm border-b border-base-300 flex-shrink-0">
      <div class="navbar-start">
        <h1 class="text-xl font-bold flex items-center gap-2">
          <i class="fas fa-brain text-primary"></i>
          {{ t('aiAssistant.title', 'AI智能助手') }}
        </h1>
      </div>
      <div class="navbar-center">
        <div class="stats shadow">
          <div class="stat place-items-center py-2 px-4">
            <div class="stat-title text-xs">{{ t('aiAssistant.activeAgents', '活跃Agent') }}</div>
            <div class="stat-value text-sm text-primary">{{ activeAgentsCount }}</div>
          </div>
          <div class="stat place-items-center py-2 px-4">
            <div class="stat-title text-xs">{{ t('aiAssistant.totalTasks', '总任务数') }}</div>
            <div class="stat-value text-sm text-secondary">{{ totalTasksCount }}</div>
          </div>
        </div>
      </div>
      <div class="navbar-end">
        <div class="flex items-center gap-2">
          <!-- 角色选择器 -->
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
                    <div class="badge badge-xs badge-ghost">默认</div>
                    <span>{{ t('aiAssistant.defaultRole', '默认助手') }}</span>
                  </div>
                </a>
              </li>
              <div class="divider my-1"></div>
              <li v-for="role in roles" :key="role.id" @click="handleSelectRole(role)">
                <a class="flex items-center justify-between gap-3" :class="{ 'active': selectedRole?.id === role.id }">
                  <div class="flex items-center gap-2">
                    <div class="badge badge-xs badge-primary">角色</div>
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
          <!-- Agent选择器 -->
          <div class="dropdown dropdown-end">
            <div tabindex="0" role="button" class="btn btn-sm btn-outline gap-2">
              <i class="fas fa-robot"></i>
              {{ selectedAgent ? selectedAgent.name : t('aiAssistant.selectAgent', '选择Agent') }}
              <i class="fas fa-chevron-down text-xs"></i>
            </div>
            <ul tabindex="0" class="dropdown-content z-[1000] menu p-2 shadow bg-base-100 rounded-box w-72 md:w-80">
              <li><span class="menu-title">{{ t('aiAssistant.availableAgents', '可用Agent') }}</span></li>
              <li v-for="agent in availableAgents" :key="agent.id" @click="selectAgent(agent)">
                <a class="flex items-center justify-between gap-3">
                  <div class="flex items-center gap-2">
                    <div class="badge badge-xs" :class="agent.status === 'active' ? 'badge-success' : 'badge-ghost'">
                      {{ agent.status }}
                    </div>
                    <span class="truncate">{{ agent.name }}</span>
                  </div>
                  <div class="text-xs text-base-content/60">{{ agent.type }}</div>
                </a>
              </li>
              <li v-if="availableAgents.length === 0">
                <span class="text-base-content/50 text-sm">{{ t('aiAssistant.noAgents', '暂无可用Agent') }}</span>
              </li>
            </ul>
          </div>
          <!-- 会话管理 -->
          <div class="flex items-center gap-1 ml-2">
            <button @click="handleCreateConversation" class="btn btn-xs btn-ghost" :disabled="isLoadingConversations">
              <i class="fas fa-plus"></i>
            </button>
            <button @click="openConversationsDrawer" class="btn btn-xs btn-ghost" :disabled="isLoadingConversations">
              <i class="fas fa-list"></i>
            </button>
            <button v-if="currentConversationId" @click="handleClearConversation" class="btn btn-xs btn-ghost text-warning">
              <i class="fas fa-broom"></i>
            </button>
            <div v-if="currentConversationId" class="ml-1 text-xs text-base-content/60 truncate max-w-[12rem]" :title="currentConversationTitle">
              会话 ({{ currentConversationTitle }})
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- 主内容区 -->
    <div class="flex-1 overflow-hidden min-h-0">
      <!-- 聊天区域 -->
      <div class="h-full flex flex-col">
        <AIChat 
          ref="aiChatRef"
          :selected-agent="selectedAgent"
          :selected-role="selectedRole"
          @execution-started="handleExecutionStarted"
          @execution-progress="handleExecutionProgress"
          @execution-completed="handleExecutionCompleted"
        />
      </div>
    </div>

    <!-- 会话列表抽屉 -->
    <div class="fixed inset-0 z-50 mt-16" v-if="showConversationsList">
      <div class="absolute inset-0 bg-black bg-opacity-50" @click="showConversationsList = false"></div>
      <div class="absolute right-0 top-0 h-[calc(100vh-4rem)] w-80 bg-base-200 shadow-xl transform transition-transform duration-300 ease-in-out">
        <div class="h-full p-4 overflow-y-auto">
          <div class="flex items-center justify-between mb-4">
            <h3 class="font-bold text-lg flex items-center gap-2">
              <i class="fas fa-comments text-primary"></i>
              会话历史
            </h3>
            <button @click="showConversationsList = false" class="btn btn-ghost btn-sm btn-circle">
              <i class="fas fa-times"></i>
            </button>
          </div>
          <div class="flex gap-2 mb-4">
            <button @click="handleLoadConversations" class="btn btn-outline btn-sm flex-1" :disabled="isLoadingConversations">
              <i class="fas fa-sync" :class="{ 'animate-spin': isLoadingConversations }"></i>
              刷新
            </button>
            <button @click="handleCreateConversation" class="btn btn-primary btn-sm" :disabled="isLoadingConversations">
              <i class="fas fa-plus"></i>
              新建
            </button>
          </div>
          <div v-if="drawerConversations.length === 0" class="text-center text-base-content/60 py-8">
            <i class="fas fa-comments text-4xl opacity-30 mb-4"></i>
            <p>暂无会话记录</p>
          </div>
          <div v-else class="space-y-3 max-h-[calc(100vh-200px)] overflow-y-auto">
            <div 
              v-for="conv in drawerConversations" 
              :key="conv.id"
              class="card bg-base-100 shadow-sm hover:shadow-md transition-all duration-200 cursor-pointer"
              :class="{ 'ring-2 ring-primary': conv.id === currentConversationId }"
              @click="handleSwitchConversation(conv.id)"
            >
              <div class="card-body p-3">
                <div class="flex items-start justify-between">
                  <div class="flex-1 min-w-0">
                    <h4 class="font-medium text-sm truncate mb-1">
                      {{ conv.title || '无标题会话' }}
                    </h4>
                    <div class="text-xs text-base-content/60 space-y-1">
                      <div class="flex items-center gap-2">
                        <i class="fas fa-clock"></i>
                        {{ new Date(conv.created_at).toLocaleString() }}
                      </div>
                      <div class="flex items-center gap-2">
                        <i class="fas fa-comment-dots"></i>
                        {{ conv.total_messages }} 条消息
                      </div>
                    </div>
                  </div>
                  <div class="flex flex-col gap-1">
                    <button 
                      v-if="conv.id === currentConversationId" 
                      class="badge badge-primary badge-xs"
                    >
                      当前
                    </button>
                    <button 
                      @click.stop="handleDeleteConversation(conv.id)" 
                      class="btn btn-ghost btn-xs text-error hover:bg-error hover:text-error-content"
                      title="删除会话"
                    >
                      <i class="fas fa-trash"></i>
                    </button>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- 角色管理弹窗 -->
    <RoleManagement v-if="showRoleManagement" @close="showRoleManagement = false" />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, nextTick } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import AIChat from '@/components/AIChat.vue'
import RoleManagement from '@/components/RoleManagement.vue'
import { useRoleManagement } from '@/composables/useRoleManagement'


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
  createRole,
  updateRole,
  deleteRole,
  selectRole,
} = useRoleManagement()

// Persist selected agent locally
const SELECTED_AGENT_KEY = 'ai:selectedAgentId'

interface AiAssistantSettings {
  auto_execute?: boolean
  notification_enabled?: boolean
}

interface CustomAgent {
  id: string
  name: string
  description: string
  type: string
  status: string
  tasks_completed: number
}

interface AgentStats {
  active_count: number
  total_tasks: number
}

// 状态数据
const activeAgentsCount = ref(0)
const totalTasksCount = ref(0)
const selectedAgent = ref(null)
const showSettings = ref(false)
const settingsTab = ref('general')
const defaultArchitecture = ref('plan-execute')
const maxConcurrentTasks = ref(5)

// 当前执行信息
const currentExecution = ref(null)

// 架构选择已移除

// 可用Agent（从后端加载/自定义）
const availableAgents = ref<CustomAgent[]>([])
const editingAgents = ref<CustomAgent[]>([])

// 角色管理状态
const showRoleManagement = ref(false)

// --- 会话管理 from AIChat ---
const aiChatRef = ref<any>(null)
const showConversationsList = ref(false)
const conversationList = computed(() => aiChatRef.value?.conversations?.value || [])
const currentConversationId = computed(() => aiChatRef.value?.currentConversationId?.value || null)
const isLoadingConversations = computed(() => aiChatRef.value?.isLoadingConversations?.value || false)
const currentConversationTitle = computed(() => aiChatRef.value?.getCurrentConversationTitle?.() || '新会话')

// 为抽屉显示准备一次性快照，避免 ref 访问时机问题
const drawerConversations = ref<any[]>([])
const snapshotConversations = () => {
  const list: any = aiChatRef.value?.conversations?.value ?? []
  try {
    drawerConversations.value = Array.from(list as any)
  } catch {
    drawerConversations.value = []
  }
}
const fetchConversationsSnapshot = async () => {
  try {
    const result = await invoke('get_ai_conversations')
    drawerConversations.value = Array.isArray(result) ? (result as any[]) : []
  } catch (e) {
    console.error('Failed to fetch conversations snapshot:', e)
    drawerConversations.value = []
  }
}

const openConversationsDrawer = async () => {
  await fetchConversationsSnapshot()
  showConversationsList.value = true
}

const handleCreateConversation = async () => {
  try {
    await aiChatRef.value?.createNewConversation?.()
    showConversationsList.value = false
  } catch (e) {
    console.error('Failed to create conversation:', e)
  }
}

const handleLoadConversations = async () => {
  await fetchConversationsSnapshot()
}

const handleSwitchConversation = async (id: string) => {
  try {
    await aiChatRef.value?.switchToConversation?.(id)
    showConversationsList.value = false
  } catch (e) {
    console.error('Failed to switch conversation:', e)
  }
}

const handleDeleteConversation = async (id: string) => {
  try {
    await aiChatRef.value?.deleteConversation?.(id)
    await fetchConversationsSnapshot()
  } catch (e) {
    console.error('Failed to delete conversation:', e)
  }
}

const handleClearConversation = () => {
  try {
    aiChatRef.value?.clearCurrentConversation?.()
  } catch (e) {
    console.error('Failed to clear conversation:', e)
  }
}



const selectAgent = (agent) => {
  selectedAgent.value = agent
  try {
    localStorage.setItem(SELECTED_AGENT_KEY, agent?.id || '')
  } catch (e) {
    console.error('Failed to persist selected agent:', e)
  }
}

const handleSelectRole = async (role) => {
  try {
    await selectRole(role)
  } catch (error) {
    console.error('Failed to select role:', error)
    // TODO: 显示错误提示
  }
}

const restoreSelectedAgent = () => {
  try {
    const savedId = localStorage.getItem(SELECTED_AGENT_KEY)
    if (!savedId) return
    const found = availableAgents.value.find(a => a.id === savedId)
    if (found) {
      selectedAgent.value = found as any
    }
  } catch (e) {
    console.error('Failed to restore selected agent:', e)
  }
}


// 事件处理
const handleExecutionStarted = (execution) => {
  currentExecution.value = execution
  activeAgentsCount.value++
}

// 架构变更已移除

const handleExecutionProgress = (progress) => {
  if (currentExecution.value) {
    currentExecution.value.progress = progress
  }
}

const handleExecutionCompleted = (result) => {
  if (currentExecution.value) {
    currentExecution.value.status = result.success ? 'completed' : 'failed'
    currentExecution.value.progress = 100
  }
  activeAgentsCount.value = Math.max(0, activeAgentsCount.value - 1)
  totalTasksCount.value++
  
  // 3秒后清除执行状态
  setTimeout(() => {
    if (currentExecution.value?.status === 'completed' || currentExecution.value?.status === 'failed') {
      currentExecution.value = null
    }
  }, 3000)
}

// 初始化
onMounted(async () => {
  try {
    // 加载设置
    const settings = await invoke<AiAssistantSettings>('get_ai_assistant_settings')
    if (settings) {

    }
    
    // 加载Agent状态
    const agentStats = await invoke<AgentStats>('get_agent_statistics')
    if (agentStats) {
      activeAgentsCount.value = agentStats.active_count || 0
      totalTasksCount.value = agentStats.total_tasks || 0
    }

    // 加载场景Agent
    const agents = await invoke<any[]>('list_scenario_agents').catch(() => [])
    availableAgents.value = Array.isArray(agents) ? agents.map(a => ({
      id: a.id,
      name: a.name,
      description: a.description,
      type: a.engine,
      status: a.enabled ? 'active' : 'idle',
      tasks_completed: 0,
    })) : []

    // 尝试还原已选择的Agent
    restoreSelectedAgent()

    // 加载角色列表
    await loadRoles()
  } catch (error) {
    console.error('Failed to initialize AI Assistant:', error)
  }
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

/* 动画效果 */
.transition-colors {
  transition: background-color 0.2s ease;
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
