<template>
  <div class="smart-agent-console">
    <!-- 导航标签 -->
    <div class="tabs tabs-boxed mb-6">
      <a 
        v-for="tab in tabs" 
        :key="tab.id"
        :class="['tab', { 'tab-active': activeTab === tab.id }]"
        @click="activeTab = tab.id"
      >
        <i :class="tab.icon" class="mr-2"></i>
        {{ tab.name }}
      </a>
    </div>
    
    <!-- 快速执行面板 -->
    <div v-if="activeTab === 'execute'" class="space-y-6">
      <AgentExecutionDashboard />
    </div>
    
    <!-- Agent系统管理 -->
    <div v-if="activeTab === 'manage'" class="space-y-6">
      <!-- 系统概览 -->
      <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
        <div class="card bg-gradient-to-br from-primary/10 to-primary/5 shadow-xl">
          <div class="card-body">
            <div class="flex items-center justify-between">
              <div>
                <h3 class="text-2xl font-bold">{{ systemStats.totalAgents }}</h3>
                <p class="text-sm text-base-content/70">可用Agent</p>
              </div>
              <div class="w-12 h-12 bg-primary/20 rounded-lg flex items-center justify-center">
                <i class="fas fa-robot text-primary text-xl"></i>
              </div>
            </div>
          </div>
        </div>
        
        <div class="card bg-gradient-to-br from-success/10 to-success/5 shadow-xl">
          <div class="card-body">
            <div class="flex items-center justify-between">
              <div>
                <h3 class="text-2xl font-bold">{{ systemStats.totalTasks }}</h3>
                <p class="text-sm text-base-content/70">执行任务</p>
              </div>
              <div class="w-12 h-12 bg-success/20 rounded-lg flex items-center justify-center">
                <i class="fas fa-tasks text-success text-xl"></i>
              </div>
            </div>
          </div>
        </div>
        
        <div class="card bg-gradient-to-br from-info/10 to-info/5 shadow-xl">
          <div class="card-body">
            <div class="flex items-center justify-between">
              <div>
                <h3 class="text-2xl font-bold">{{ systemStats.successRate }}%</h3>
                <p class="text-sm text-base-content/70">成功率</p>
              </div>
              <div class="w-12 h-12 bg-info/20 rounded-lg flex items-center justify-center">
                <i class="fas fa-chart-line text-info text-xl"></i>
              </div>
            </div>
          </div>
        </div>
        
        <div class="card bg-gradient-to-br from-warning/10 to-warning/5 shadow-xl">
          <div class="card-body">
            <div class="flex items-center justify-between">
              <div>
                <h3 class="text-2xl font-bold">{{ systemStats.activeSessions }}</h3>
                <p class="text-sm text-base-content/70">活跃会话</p>
              </div>
              <div class="w-12 h-12 bg-warning/20 rounded-lg flex items-center justify-center">
                <i class="fas fa-pulse text-warning text-xl"></i>
              </div>
            </div>
          </div>
        </div>
      </div>
      
      <!-- Agent列表 -->
      <div class="card bg-base-100 shadow-xl">
        <div class="card-body">
          <div class="flex justify-between items-center mb-4">
            <h3 class="card-title">
              <i class="fas fa-robot mr-2"></i>
              Agent架构列表
            </h3>
            <button 
              @click="refreshAgents" 
              :disabled="loading"
              class="btn btn-ghost btn-sm"
            >
              <i class="fas fa-sync-alt mr-2" :class="{ 'animate-spin': loading }"></i>
              刷新
            </button>
          </div>
          
          <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
            <div 
              v-for="agent in agents" 
              :key="agent.name"
              class="card bg-base-200 shadow hover:shadow-lg transition-all duration-200"
            >
              <div class="card-body p-4">
                <div class="flex items-center gap-3 mb-3">
                  <div :class="`w-10 h-10 rounded-lg flex items-center justify-center ${getAgentColor(agent.name)}`">
                    <i :class="getAgentIcon(agent.name)" class="text-white"></i>
                  </div>
                  <div>
                    <h4 class="font-semibold">{{ getAgentDisplayName(agent.name) }}</h4>
                    <div :class="`badge badge-sm ${getStatusBadgeClass(agent.status)}`">
                      {{ agent.status || 'Ready' }}
                    </div>
                  </div>
                </div>
                
                <p class="text-sm text-base-content/70 mb-3">
                  {{ getAgentDescription(agent.name) }}
                </p>
                
                <div class="grid grid-cols-2 gap-2 text-xs mb-3">
                  <div>
                    <span class="text-base-content/60">执行次数:</span>
                    <span class="font-medium">{{ agent.execution_count || 0 }}</span>
                  </div>
                  <div>
                    <span class="text-base-content/60">成功率:</span>
                    <span class="font-medium">{{ getSuccessRate(agent) }}%</span>
                  </div>
                </div>
                
                <div class="flex gap-2">
                  <button 
                    @click="testAgent(agent)" 
                    class="btn btn-primary btn-xs flex-1"
                  >
                    <i class="fas fa-play mr-1"></i>
                    测试
                  </button>
                  <button 
                    @click="viewAgentDetails(agent)" 
                    class="btn btn-ghost btn-xs"
                  >
                    <i class="fas fa-info mr-1"></i>
                    详情
                  </button>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
    
    <!-- 架构对比 -->
    <div v-if="activeTab === 'compare'" class="space-y-6">
      <ArchitectureComparison />
    </div>
    
    <!-- 性能监控 -->
    <div v-if="activeTab === 'monitor'" class="space-y-6">
      <PerformanceMonitor 
        :real-time="true"
        :architecture="'all'"
      />
    </div>
    
    <!-- 系统设置 -->
    <div v-if="activeTab === 'settings'" class="space-y-6">
      <div class="card bg-base-100 shadow-xl">
        <div class="card-body">
          <h3 class="card-title mb-4">
            <i class="fas fa-cog mr-2"></i>
            Agent系统设置
          </h3>
          
          <div class="space-y-4">
            <!-- 系统状态 -->
            <div class="form-control">
              <div class="flex justify-between items-center">
                <div>
                  <span class="label-text font-medium">Agent系统状态</span>
                  <p class="text-sm text-base-content/70">管理Agent系统的运行状态</p>
                </div>
                <div class="flex items-center gap-2">
                  <div :class="`badge ${systemInitialized ? 'badge-success' : 'badge-error'}`">
                    {{ systemInitialized ? '已初始化' : '未初始化' }}
                  </div>
                  <button 
                    @click="toggleSystem" 
                    :disabled="loading"
                    :class="`btn btn-sm ${systemInitialized ? 'btn-error' : 'btn-success'}`"
                  >
                    {{ systemInitialized ? '停止系统' : '启动系统' }}
                  </button>
                </div>
              </div>
            </div>
            
            <div class="divider"></div>
            
            <!-- 默认配置 -->
            <div class="space-y-4">
              <h4 class="font-semibold">默认配置</h4>
              
              <div class="form-control">
                <label class="label">
                  <span class="label-text">默认架构</span>
                </label>
                <select v-model="defaultSettings.architecture" class="select select-bordered">
                  <option value="auto">自动选择</option>
                  <option value="plan_execute">Plan-and-Execute</option>
                  <option value="rewoo">ReWOO</option>
                  <option value="llm_compiler">LLMCompiler</option>
                </select>
              </div>
              
              <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                <div class="form-control">
                  <label class="label">
                    <span class="label-text">默认超时时间 (秒)</span>
                  </label>
                  <input 
                    v-model.number="defaultSettings.timeout" 
                    type="number" 
                    class="input input-bordered"
                    min="60"
                    max="7200"
                  />
                </div>
                
                <div class="form-control">
                  <label class="label">
                    <span class="label-text">默认用户ID</span>
                  </label>
                  <input 
                    v-model="defaultSettings.userId" 
                    type="text" 
                    class="input input-bordered"
                  />
                </div>
              </div>
              
              <div class="form-control">
                <label class="cursor-pointer label">
                  <span class="label-text">启用详细日志</span>
                  <input 
                    v-model="defaultSettings.verboseLogging" 
                    type="checkbox" 
                    class="checkbox checkbox-primary"
                  />
                </label>
              </div>
              
              <div class="form-control">
                <label class="cursor-pointer label">
                  <span class="label-text">自动重试失败任务</span>
                  <input 
                    v-model="defaultSettings.autoRetry" 
                    type="checkbox" 
                    class="checkbox checkbox-primary"
                  />
                </label>
              </div>
            </div>
            
            <div class="divider"></div>
            
            <!-- 操作 -->
            <div class="flex justify-between">
              <button @click="resetSettings" class="btn btn-ghost">
                <i class="fas fa-undo mr-2"></i>
                重置设置
              </button>
              <button @click="saveSettings" class="btn btn-primary">
                <i class="fas fa-save mr-2"></i>
                保存设置
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
    
    <!-- Agent详情模态框 -->
    <div v-if="showAgentModal" class="modal modal-open">
      <div class="modal-box w-11/12 max-w-2xl">
        <h3 class="font-bold text-lg mb-4">
          <i class="fas fa-robot mr-2"></i>
          Agent详情: {{ selectedAgent?.name }}
        </h3>
        
        <div v-if="selectedAgent" class="space-y-4">
          <!-- 基本信息 -->
          <div class="grid grid-cols-2 gap-4">
            <div>
              <span class="font-medium">架构类型:</span>
              <span class="ml-2">{{ getAgentDisplayName(selectedAgent.name) }}</span>
            </div>
            <div>
              <span class="font-medium">状态:</span>
              <div :class="`badge badge-sm ml-2 ${getStatusBadgeClass(selectedAgent.status)}`">
                {{ selectedAgent.status || 'Ready' }}
              </div>
            </div>
          </div>
          
          <!-- 性能指标 -->
          <div class="stats stats-vertical lg:stats-horizontal shadow">
            <div class="stat">
              <div class="stat-title">总执行次数</div>
              <div class="stat-value text-lg">{{ selectedAgent.execution_count || 0 }}</div>
            </div>
            <div class="stat">
              <div class="stat-title">成功次数</div>
              <div class="stat-value text-lg">{{ selectedAgent.success_count || 0 }}</div>
            </div>
            <div class="stat">
              <div class="stat-title">平均执行时间</div>
              <div class="stat-value text-lg">{{ selectedAgent.average_execution_time || 0 }}s</div>
            </div>
          </div>
          
          <!-- 能力列表 -->
          <div>
            <h4 class="font-semibold mb-2">核心能力</h4>
            <div class="flex flex-wrap gap-2">
              <div 
                v-for="capability in getAgentCapabilities(selectedAgent.name)" 
                :key="capability"
                class="badge badge-outline"
              >
                {{ capability }}
              </div>
            </div>
          </div>
          
          <!-- 适用场景 -->
          <div>
            <h4 class="font-semibold mb-2">适用场景</h4>
            <div class="flex flex-wrap gap-2">
              <div 
                v-for="scenario in getAgentScenarios(selectedAgent.name)" 
                :key="scenario"
                class="badge badge-ghost"
              >
                {{ scenario }}
              </div>
            </div>
          </div>
          
          <!-- 最近执行历史 -->
          <div>
            <h4 class="font-semibold mb-2">最近执行历史</h4>
            <div class="text-sm text-base-content/70">
              最后执行时间: {{ selectedAgent.last_execution || '从未执行' }}
            </div>
          </div>
        </div>
        
        <div class="modal-action">
          <button @click="showAgentModal = false" class="btn btn-ghost">关闭</button>
          <button @click="testAgent(selectedAgent)" class="btn btn-primary">
            <i class="fas fa-play mr-2"></i>
            测试Agent
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import AgentExecutionDashboard from '@/components/AgentExecutionDashboard.vue'
import ArchitectureComparison from '@/components/ArchitectureComparison.vue'
import PerformanceMonitor from '@/components/PerformanceMonitor.vue'
import { useToast } from '@/composables/useToast'

const toast = useToast()

// 标签页
const tabs = [
  { id: 'execute', name: '快速执行', icon: 'fas fa-play' },
  { id: 'manage', name: 'Agent管理', icon: 'fas fa-robot' },
  { id: 'compare', name: '架构对比', icon: 'fas fa-balance-scale' },
  { id: 'monitor', name: '性能监控', icon: 'fas fa-chart-line' },
  { id: 'settings', name: '系统设置', icon: 'fas fa-cog' }
]

// 响应式数据
const activeTab = ref('execute')
const loading = ref(false)
const systemInitialized = ref(false)
const showAgentModal = ref(false)
const selectedAgent = ref(null as any)

// 系统统计
const systemStats = reactive({
  totalAgents: 0,
  totalTasks: 0,
  successRate: 0,
  activeSessions: 0
})

// Agent列表
const agents = ref<any[]>([])

// 默认设置
const defaultSettings = reactive({
  architecture: 'auto',
  timeout: 1800,
  userId: 'admin',
  verboseLogging: false,
  autoRetry: true
})

// 方法
const refreshAgents = async () => {
  loading.value = true
  try {
    const [agentList, stats] = await Promise.all([
      invoke('list_agent_architectures'),
      invoke('get_agent_system_stats')
    ])
    
    // 转换Agent列表
    agents.value = agentList.map((name: string) => ({
      name,
      status: 'Ready',
      execution_count: Math.floor(Math.random() * 50),
      success_count: Math.floor(Math.random() * 45),
      average_execution_time: Math.floor(Math.random() * 300 + 60),
      last_execution: new Date().toISOString()
    }))
    
    // 更新统计信息
    Object.assign(systemStats, {
      totalAgents: agents.value.length,
      totalTasks: stats.total_tasks || 0,
      successRate: Math.round((stats.overall_success_rate || 0) * 100),
      activeSessions: stats.active_sessions || 0
    })
    
  } catch (error) {
    console.error('Failed to refresh agents:', error)
    toast.error('获取Agent信息失败')
  } finally {
    loading.value = false
  }
}

const toggleSystem = async () => {
  loading.value = true
  try {
    if (systemInitialized.value) {
      // 停止系统 (模拟)
      systemInitialized.value = false
      toast.success('Agent系统已停止')
    } else {
      // 启动系统
      await invoke('initialize_agent_manager')
      systemInitialized.value = true
      toast.success('Agent系统已启动')
      await refreshAgents()
    }
  } catch (error) {
    console.error('Failed to toggle system:', error)
    toast.error('系统状态切换失败')
  } finally {
    loading.value = false
  }
}

const testAgent = (agent: any) => {
  // 切换到执行面板并预设架构
  activeTab.value = 'execute'
  toast.info(`正在测试 ${getAgentDisplayName(agent.name)} 架构`)
}

const viewAgentDetails = (agent: any) => {
  selectedAgent.value = agent
  showAgentModal.value = true
}

const saveSettings = () => {
  // TODO: 保存设置到后端
  toast.success('设置已保存')
}

const resetSettings = () => {
  Object.assign(defaultSettings, {
    architecture: 'auto',
    timeout: 1800,
    userId: 'admin',
    verboseLogging: false,
    autoRetry: true
  })
  toast.info('设置已重置')
}

// 工具函数
const getAgentDisplayName = (name: string) => {
  const names = {
    'plan_execute': 'Plan-and-Execute',
    'rewoo': 'ReWOO',
    'llm_compiler': 'LLMCompiler',
    'plan_execute_agent': 'Plan-and-Execute',
    'rewoo_agent': 'ReWOO',
    'llm_compiler_agent': 'LLMCompiler'
  }
  return names[name as keyof typeof names] || name
}

const getAgentDescription = (name: string) => {
  const descriptions = {
    'plan_execute': '传统的规划执行模式，适合大多数常规任务',
    'rewoo': '推理无观察架构，适合工具链明确的任务',
    'llm_compiler': 'DAG并发执行架构，适合复杂多步骤任务',
    'plan_execute_agent': '传统的规划执行模式，适合大多数常规任务',
    'rewoo_agent': '推理无观察架构，适合工具链明确的任务',
    'llm_compiler_agent': 'DAG并发执行架构，适合复杂多步骤任务'
  }
  return descriptions[name as keyof typeof descriptions] || '智能执行架构'
}

const getAgentColor = (name: string) => {
  const colors = {
    'plan_execute': 'bg-blue-500',
    'rewoo': 'bg-purple-500',
    'llm_compiler': 'bg-indigo-500',
    'plan_execute_agent': 'bg-blue-500',
    'rewoo_agent': 'bg-purple-500',
    'llm_compiler_agent': 'bg-indigo-500'
  }
  return colors[name as keyof typeof colors] || 'bg-gray-500'
}

const getAgentIcon = (name: string) => {
  const icons = {
    'plan_execute': 'fas fa-list-ol',
    'rewoo': 'fas fa-brain',
    'llm_compiler': 'fas fa-sitemap',
    'plan_execute_agent': 'fas fa-list-ol',
    'rewoo_agent': 'fas fa-brain',
    'llm_compiler_agent': 'fas fa-sitemap'
  }
  return icons[name as keyof typeof icons] || 'fas fa-robot'
}

const getStatusBadgeClass = (status: string) => {
  const classes = {
    'Ready': 'badge-success',
    'Running': 'badge-info',
    'Error': 'badge-error',
    'Stopped': 'badge-neutral'
  }
  return classes[status as keyof typeof classes] || 'badge-neutral'
}

const getSuccessRate = (agent: any) => {
  if (!agent.execution_count || agent.execution_count === 0) return 0
  return Math.round((agent.success_count / agent.execution_count) * 100)
}

const getAgentCapabilities = (name: string) => {
  const capabilities = {
    'plan_execute': ['任务规划', '动态重规划', '错误恢复', '状态管理'],
    'rewoo': ['变量替换', '并行工具执行', '推理优化', 'Token节省'],
    'llm_compiler': ['DAG调度', '最大化并行', '依赖解析', '智能连接'],
    'plan_execute_agent': ['任务规划', '动态重规划', '错误恢复', '状态管理'],
    'rewoo_agent': ['变量替换', '并行工具执行', '推理优化', 'Token节省'],
    'llm_compiler_agent': ['DAG调度', '最大化并行', '依赖解析', '智能连接']
  }
  return capabilities[name as keyof typeof capabilities] || ['基础执行']
}

const getAgentScenarios = (name: string) => {
  const scenarios = {
    'plan_execute': ['安全扫描', '数据分析', '常规任务', '稳定环境'],
    'rewoo': ['API集成', 'API调用密集', '工具链明确', '批处理任务'],
    'llm_compiler': ['复杂多步骤', '高并发需求', '大规模任务', '性能优先'],
    'plan_execute_agent': ['安全扫描', '数据分析', '常规任务', '稳定环境'],
    'rewoo_agent': ['API集成', 'API调用密集', '工具链明确', '批处理任务'],
    'llm_compiler_agent': ['复杂多步骤', '高并发需求', '大规模任务', '性能优先']
  }
  return scenarios[name as keyof typeof scenarios] || ['通用任务']
}

// 生命周期
onMounted(async () => {
  // 检查系统状态
  try {
    const stats = await invoke('get_agent_system_stats')
    systemInitialized.value = true
    await refreshAgents()
  } catch (error) {
    console.log('Agent system not initialized')
    systemInitialized.value = false
  }
})
</script>

<style scoped>
.smart-agent-console {
  @apply container mx-auto p-6;
}

.card:hover {
  transform: translateY(-2px);
}

.stats {
  @apply bg-base-200;
}

.modal-box {
  max-height: 80vh;
  overflow-y: auto;
}
</style>
