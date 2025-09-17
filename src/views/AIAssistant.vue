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
          
          <!-- 设置按钮 -->
          <button class="btn btn-sm btn-ghost btn-circle" @click="openSettings" :title="t('common.settings', '设置')">
            <i class="fas fa-cog"></i>
          </button>
        </div>
      </div>
    </div>

    <!-- 主内容区 -->
    <div class="flex-1 overflow-hidden min-h-0">
      <!-- 聊天区域 -->
      <div class="h-full flex flex-col">
        <AIChat 
          :selected-agent="selectedAgent"
          @execution-started="handleExecutionStarted"
          @execution-progress="handleExecutionProgress"
          @execution-completed="handleExecutionCompleted"
        />
      </div>
    </div>

    <!-- 设置模态框 -->
    <div v-if="showSettings" class="modal modal-open">
      <div class="modal-box max-w-4xl">
        <h3 class="font-bold text-lg mb-4">{{ t('aiAssistant.settings.title', 'AI助手设置') }}</h3>
        
        <div class="tabs tabs-bordered mb-4">
          <a class="tab" :class="{ 'tab-active': settingsTab === 'general' }" @click="settingsTab = 'general'">
            {{ t('aiAssistant.settings.general', '通用设置') }}
          </a>
          
          <a class="tab" :class="{ 'tab-active': settingsTab === 'agents' }" @click="settingsTab = 'agents'">
            {{ t('aiAssistant.settings.agents', 'Agent管理') }}
          </a>
        </div>

        <div v-if="settingsTab === 'general'" class="space-y-4">
          
          
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ t('aiAssistant.settings.maxConcurrentTasks', '最大并发任务') }}</span>
            </label>
            <input v-model.number="maxConcurrentTasks" type="number" class="input input-bordered" min="1" max="10">
          </div>
        </div>

        

        <div v-else-if="settingsTab === 'agents'" class="space-y-4">
          <div class="flex justify-between items-center">
            <h4 class="font-semibold">{{ t('aiAssistant.settings.customAgents', '自定义Agent') }}</h4>
            <button class="btn btn-sm btn-primary" @click="addAgent"><i class="fas fa-plus"></i>{{ t('common.add', '添加') }}</button>
          </div>
          <div class="space-y-2 max-h-80 overflow-y-auto pr-1">
            <div v-for="(agent, idx) in editingAgents" :key="agent.id || idx" class="bg-base-200 rounded-lg p-3 grid grid-cols-6 gap-2 items-center">
              <input v-model="agent.name" class="input input-sm input-bordered col-span-1" :placeholder="t('common.name', '名称')" />
              <input v-model="agent.description" class="input input-sm input-bordered col-span-2" :placeholder="t('common.description', '描述')" />
              <input v-model="agent.type" class="input input-sm input-bordered col-span-1" placeholder="Type" />
              <select v-model="agent.status" class="select select-sm select-bordered col-span-1">
                <option value="active">active</option>
                <option value="idle">idle</option>
              </select>
              <div class="col-span-1 flex items-center gap-2 justify-end">
                <input v-model.number="agent.tasks_completed" type="number" class="input input-sm input-bordered w-24" min="0" />
                <button class="btn btn-sm btn-error" @click="removeAgent(idx)"><i class="fas fa-trash"></i></button>
              </div>
            </div>
          </div>
        </div>

        <div class="modal-action">
          <button class="btn btn-primary" @click="saveSettings">{{ t('common.save', '保存') }}</button>
          <button class="btn" @click="closeSettings">{{ t('common.cancel', '取消') }}</button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import AIChat from '@/components/AIChat.vue'

const { t } = useI18n()

interface AiAssistantSettings {
  default_architecture: string
  max_concurrent_tasks: number
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



const selectAgent = (agent) => {
  selectedAgent.value = agent
  // 通知EnhancedAIChat组件Agent变更
}

const openSettings = () => {
  showSettings.value = true
  // 打开时带入可编辑副本
  editingAgents.value = JSON.parse(JSON.stringify(availableAgents.value))
}

const closeSettings = () => {
  showSettings.value = false
}

const saveSettings = async () => {
  try {
    // 保存基础设置（保留并兼容后端，不再暴露架构UI）
    await invoke('save_ai_assistant_settings', { settings: { default_architecture: 'auto', max_concurrent_tasks: maxConcurrentTasks.value, auto_execute: false, notification_enabled: true } })
    // 保存场景Agent（逐个 upsert）
    for (const a of editingAgents.value) {
      const profile = {
        id: a.id,
        name: a.name,
        description: a.description,
        enabled: a.status === 'active',
        version: '1.0.0',
        engine: (a.type === 'rewoo' || a.type === 'llm-compiler' || a.type === 'plan-execute') ? a.type : 'auto',
        llm: { default: { provider: 'auto', model: 'auto', temperature: null, max_tokens: null } },
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString()
      }
      await invoke('save_scenario_agent', { profile })
    }
    closeSettings()
  } catch (error) {
    console.error('Failed to save settings:', error)
  }
}

const getStatusBadgeClass = (status) => {
  switch (status) {
    case 'running': return 'badge-primary'
    case 'completed': return 'badge-success'
    case 'failed': return 'badge-error'
    case 'paused': return 'badge-warning'
    default: return 'badge-ghost'
  }
}

// 架构徽章函数已移除

// 架构开关已移除

const addAgent = () => {
  editingAgents.value.push({
    id: `custom-${Date.now()}`,
    name: '',
    description: '',
    type: 'General',
    status: 'idle',
    tasks_completed: 0
  })
}

const removeAgent = (index: number) => {
  editingAgents.value.splice(index, 1)
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
      defaultArchitecture.value = settings.default_architecture || 'plan-execute'
      maxConcurrentTasks.value = (settings.max_concurrent_tasks as number) || 5
      // 暂时先设置，等加载架构后再修正
    }
    // 架构加载与偏好已移除
    
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
