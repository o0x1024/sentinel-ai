<template>
  <div class="rewoo-control-panel">
    <!-- ReWOO引擎状态 -->
    <div class="card bg-base-100 shadow-xl mb-6">
      <div class="card-body">
        <div class="flex justify-between items-center mb-4">
          <h2 class="card-title">ReWOO 引擎控制面板</h2>
          <div class="badge" :class="engineStatus.active ? 'badge-success' : 'badge-warning'">
            {{ engineStatus.active ? '运行中' : '待机' }}
          </div>
        </div>
        
        <!-- 引擎统计 -->
        <div class="grid grid-cols-1 md:grid-cols-4 gap-4 mb-4">
          <div class="stat bg-base-200 rounded-lg">
            <div class="stat-title">执行会话</div>
            <div class="stat-value text-lg">{{ statistics.totalSessions }}</div>
            <div class="stat-desc">总计</div>
          </div>
          <div class="stat bg-base-200 rounded-lg">
            <div class="stat-title">Token效率</div>
            <div class="stat-value text-lg text-success">{{ statistics.tokenEfficiency }}%</div>
            <div class="stat-desc">相比传统模式</div>
          </div>
          <div class="stat bg-base-200 rounded-lg">
            <div class="stat-title">平均执行时间</div>
            <div class="stat-value text-lg">{{ formatDuration(statistics.avgExecutionTime) }}</div>
            <div class="stat-desc">毫秒</div>
          </div>
          <div class="stat bg-base-200 rounded-lg">
            <div class="stat-title">成功率</div>
            <div class="stat-value text-lg text-primary">{{ statistics.successRate }}%</div>
            <div class="stat-desc">最近100次</div>
          </div>
        </div>
        
        <!-- 快速操作 -->
        <div class="flex gap-2">
          <button 
            class="btn btn-primary btn-sm"
            @click="startReWOOSession"
            :disabled="!engineStatus.ready"
          >
            <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M14.828 14.828a4 4 0 01-5.656 0M9 10h1m4 0h1" />
            </svg>
            启动ReWOO会话
          </button>
          <button 
            class="btn btn-outline btn-sm"
            @click="showEngineConfig = !showEngineConfig"
          >
            配置引擎
          </button>
          <button 
            class="btn btn-ghost btn-sm"
            @click="refreshStatistics"
          >
            刷新统计
          </button>
        </div>
      </div>
    </div>
    
    <!-- 变量管理 -->
    <div class="card bg-base-100 shadow-xl mb-6">
      <div class="card-body">
        <div class="flex justify-between items-center mb-4">
          <h3 class="card-title">变量替换管理</h3>
          <button 
            class="btn btn-sm btn-outline"
            @click="showAddVariableModal = true"
          >
            <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
            </svg>
            添加变量
          </button>
        </div>
        
        <!-- 变量列表 -->
        <div class="space-y-2">
          <div 
            v-for="variable in variables" 
            :key="variable.id"
            class="flex items-center justify-between p-3 bg-base-200 rounded-lg"
          >
            <div class="flex-1">
              <div class="flex items-center gap-2">
                <code class="text-sm font-mono bg-base-300 px-2 py-1 rounded">{{ variable.name }}</code>
                <div class="badge badge-sm" :class="getVariableTypeBadge(variable.type)">{{ variable.type }}</div>
              </div>
              <div class="text-sm text-base-content/70 mt-1">{{ variable.description }}</div>
            </div>
            <div class="flex items-center gap-2">
              <div class="text-sm font-mono">{{ formatVariableValue(variable.value) }}</div>
              <div class="dropdown dropdown-end">
                <div tabindex="0" role="button" class="btn btn-sm btn-ghost">
                  <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 5v.01M12 12v.01M12 19v.01M12 6a1 1 0 110-2 1 1 0 010 2zm0 7a1 1 0 110-2 1 1 0 010 2zm0 7a1 1 0 110-2 1 1 0 010 2z" />
                  </svg>
                </div>
                <ul tabindex="0" class="dropdown-content z-[1] menu p-2 shadow bg-base-100 rounded-box w-32">
                  <li><a @click="editVariable(variable)">编辑</a></li>
                  <li><a @click="deleteVariable(variable)" class="text-error">删除</a></li>
                </ul>
              </div>
            </div>
          </div>
        </div>
        
        <!-- 空状态 -->
        <div v-if="variables.length === 0" class="text-center py-8 text-base-content/60">
          <svg xmlns="http://www.w3.org/2000/svg" class="h-12 w-12 mx-auto mb-2 opacity-50" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 7h.01M7 3h5c.512 0 1.024.195 1.414.586l7 7a2 2 0 010 2.828l-7 7a2 2 0 01-2.828 0l-7-7A1.994 1.994 0 013 12V7a4 4 0 014-4z" />
          </svg>
          <p>暂无变量定义</p>
          <p class="text-sm">点击上方按钮添加变量</p>
        </div>
      </div>
    </div>
    
    <!-- 工具链执行状态 -->
    <div class="card bg-base-100 shadow-xl mb-6">
      <div class="card-body">
        <h3 class="card-title mb-4">工具链执行状态</h3>
        
        <!-- 工具链流程图 -->
        <div class="bg-base-200 rounded-lg p-4 mb-4">
          <div class="flex items-center justify-center min-h-[200px]">
            <div v-if="!currentToolChain" class="text-center text-base-content/60">
              <svg xmlns="http://www.w3.org/2000/svg" class="h-12 w-12 mx-auto mb-2 opacity-50" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13.828 10.172a4 4 0 00-5.656 0l-4 4a4 4 0 105.656 5.656l1.102-1.101m-.758-4.899a4 4 0 005.656 0l4-4a4 4 0 00-5.656-5.656l-1.1 1.1" />
              </svg>
              <p>暂无活动工具链</p>
            </div>
            
            <!-- 工具链可视化 -->
            <div v-else class="w-full">
              <div class="flex items-center justify-between mb-4">
                <h4 class="font-semibold">{{ currentToolChain.name }}</h4>
                <div class="badge" :class="getToolChainStatusBadge(currentToolChain.status)">{{ currentToolChain.status }}</div>
              </div>
              
              <!-- 工具步骤 -->
              <div class="flex items-center gap-2 overflow-x-auto pb-2">
                <div 
                  v-for="(tool, index) in currentToolChain.tools" 
                  :key="tool.id"
                  class="flex items-center gap-2"
                >
                  <div 
                    class="flex flex-col items-center p-3 rounded-lg border-2 min-w-[100px]"
                    :class="getToolStatusClass(tool.status)"
                  >
                    <div class="w-3 h-3 rounded-full mb-1" :class="getToolStatusIndicator(tool.status)"></div>
                    <div class="text-sm font-medium text-center">{{ tool.name }}</div>
                    <div class="text-xs text-center text-base-content/60">{{ tool.type }}</div>
                    <div v-if="tool.progress !== undefined" class="w-full mt-2">
                      <progress class="progress progress-primary w-full h-1" :value="tool.progress" max="100"></progress>
                    </div>
                  </div>
                  
                  <!-- 箭头 -->
                  <div v-if="index < currentToolChain.tools.length - 1" class="text-base-content/40">
                    <svg xmlns="http://www.w3.org/2000/svg" class="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
                    </svg>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
        
        <!-- 工具执行日志 -->
        <div class="collapse collapse-arrow bg-base-200">
          <input type="checkbox" /> 
          <div class="collapse-title text-lg font-medium">
            执行日志 ({{ toolExecutionLogs.length }} 条)
          </div>
          <div class="collapse-content">
            <div class="max-h-60 overflow-y-auto space-y-2">
              <div 
                v-for="log in toolExecutionLogs" 
                :key="log.id"
                class="p-2 bg-base-100 rounded border-l-4"
                :class="getLogBorderClass(log.level)"
              >
                <div class="flex justify-between items-start">
                  <div class="flex-1">
                    <div class="flex items-center gap-2 mb-1">
                      <div class="badge badge-xs" :class="getLogLevelBadge(log.level)">{{ log.level }}</div>
                      <span class="text-sm font-medium">{{ log.toolName }}</span>
                    </div>
                    <div class="text-sm text-base-content/80">{{ log.message }}</div>
                  </div>
                  <div class="text-xs text-base-content/60">{{ formatTime(log.timestamp) }}</div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
    
    <!-- Solver结果展示 -->
    <div class="card bg-base-100 shadow-xl">
      <div class="card-body">
        <h3 class="card-title mb-4">Solver 结果展示</h3>
        
        <div v-if="solverResult">
          <!-- 结果摘要 -->
          <div class="grid grid-cols-1 md:grid-cols-3 gap-4 mb-4">
            <div class="stat bg-base-200 rounded-lg">
              <div class="stat-title">处理时间</div>
              <div class="stat-value text-lg">{{ formatDuration(solverResult.processingTime) }}</div>
            </div>
            <div class="stat bg-base-200 rounded-lg">
              <div class="stat-title">置信度</div>
              <div class="stat-value text-lg">{{ Math.round(solverResult.confidence * 100) }}%</div>
            </div>
            <div class="stat bg-base-200 rounded-lg">
              <div class="stat-title">使用工具</div>
              <div class="stat-value text-lg">{{ solverResult.toolsUsed }}</div>
            </div>
          </div>
          
          <!-- 结果内容 -->
          <div class="bg-base-200 rounded-lg p-4">
            <h4 class="font-semibold mb-2">最终结果</h4>
            <div class="prose max-w-none">
              <div v-html="formatSolverResult(solverResult.content)"></div>
            </div>
          </div>
          
          <!-- 工具输出汇总 -->
          <div class="mt-4">
            <div class="collapse collapse-arrow bg-base-200">
              <input type="checkbox" /> 
              <div class="collapse-title text-lg font-medium">
                工具输出汇总 ({{ solverResult.toolOutputs?.length || 0 }} 个工具)
              </div>
              <div class="collapse-content">
                <div class="space-y-3">
                  <div 
                    v-for="output in solverResult.toolOutputs" 
                    :key="output.toolId"
                    class="border rounded-lg p-3"
                  >
                    <div class="flex justify-between items-center mb-2">
                      <span class="font-medium">{{ output.toolName }}</span>
                      <div class="badge badge-sm" :class="output.success ? 'badge-success' : 'badge-error'">
                        {{ output.success ? '成功' : '失败' }}
                      </div>
                    </div>
                    <div class="text-sm text-base-content/80">
                      <pre class="whitespace-pre-wrap">{{ output.result }}</pre>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
        
        <!-- 空状态 -->
        <div v-else class="text-center py-8 text-base-content/60">
          <svg xmlns="http://www.w3.org/2000/svg" class="h-12 w-12 mx-auto mb-2 opacity-50" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.663 17h4.673M12 3v1m6.364 1.636l-.707.707M21 12h-1M4 12H3m3.343-5.657l-.707-.707m2.828 9.9a5 5 0 117.072 0l-.548.547A3.374 3.374 0 0014 18.469V19a2 2 0 11-4 0v-.531c0-.895-.356-1.754-.988-2.386l-.548-.547z" />
          </svg>
          <p>暂无Solver结果</p>
          <p class="text-sm">启动ReWOO会话后将显示结果</p>
        </div>
      </div>
    </div>
    
    <!-- 添加变量模态框 -->
    <div v-if="showAddVariableModal" class="modal modal-open">
      <div class="modal-box">
        <h3 class="font-bold text-lg mb-4">添加变量</h3>
        
        <form @submit.prevent="addVariable">
          <div class="form-control mb-4">
            <label class="label">
              <span class="label-text">变量名称</span>
            </label>
            <input 
              type="text" 
              class="input input-bordered" 
              v-model="newVariable.name"
              placeholder="例如: #E1, #E2"
              required
            >
          </div>
          
          <div class="form-control mb-4">
            <label class="label">
              <span class="label-text">变量类型</span>
            </label>
            <select class="select select-bordered" v-model="newVariable.type">
              <option value="string">字符串</option>
              <option value="number">数字</option>
              <option value="boolean">布尔值</option>
              <option value="object">对象</option>
              <option value="array">数组</option>
            </select>
          </div>
          
          <div class="form-control mb-4">
            <label class="label">
              <span class="label-text">描述</span>
            </label>
            <textarea 
              class="textarea textarea-bordered" 
              v-model="newVariable.description"
              placeholder="变量用途描述"
            ></textarea>
          </div>
          
          <div class="form-control mb-6">
            <label class="label">
              <span class="label-text">初始值</span>
            </label>
            <input 
              type="text" 
              class="input input-bordered" 
              v-model="newVariable.value"
              placeholder="变量初始值"
            >
          </div>
          
          <div class="modal-action">
            <button type="button" class="btn" @click="showAddVariableModal = false">取消</button>
            <button type="submit" class="btn btn-primary">添加</button>
          </div>
        </form>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted, onUnmounted } from 'vue'

// 类型定义
interface EngineStatus {
  active: boolean
  ready: boolean
  version: string
  uptime: number
}

interface Statistics {
  totalSessions: number
  tokenEfficiency: number
  avgExecutionTime: number
  successRate: number
}

interface Variable {
  id: string
  name: string
  type: 'string' | 'number' | 'boolean' | 'object' | 'array'
  value: any
  description: string
  createdAt: Date
}

interface ToolChain {
  id: string
  name: string
  status: 'pending' | 'running' | 'completed' | 'failed'
  tools: Tool[]
}

interface Tool {
  id: string
  name: string
  type: string
  status: 'pending' | 'running' | 'completed' | 'failed'
  progress?: number
}

interface SolverResult {
  content: string
  confidence: number
  processingTime: number
  toolsUsed: number
  toolOutputs: ToolOutput[]
}

interface ToolOutput {
  toolId: string
  toolName: string
  result: string
  success: boolean
}

interface ExecutionLog {
  id: string
  level: 'info' | 'warn' | 'error' | 'debug'
  toolName: string
  message: string
  timestamp: Date
}

// Props
interface Props {
  sessionId?: string
}

const props = defineProps<Props>()

// Emits
const emit = defineEmits<{
  sessionStarted: [sessionId: string]
  variableAdded: [variable: Variable]
  variableUpdated: [variable: Variable]
  variableDeleted: [variableId: string]
}>()

// 响应式数据
const engineStatus = ref<EngineStatus>({
  active: false,
  ready: true,
  version: '1.0.0',
  uptime: 0
})

const statistics = ref<Statistics>({
  totalSessions: 0,
  tokenEfficiency: 0,
  avgExecutionTime: 0,
  successRate: 0
})

const variables = ref<Variable[]>([])
const currentToolChain = ref<ToolChain | null>(null)
const solverResult = ref<SolverResult | null>(null)
const toolExecutionLogs = ref<ExecutionLog[]>([])

// 模态框状态
const showEngineConfig = ref(false)
const showAddVariableModal = ref(false)

// 新变量表单
const newVariable = reactive({
  name: '',
  type: 'string' as Variable['type'],
  value: '',
  description: ''
})

// 方法
const startReWOOSession = async () => {
  try {
    // 调用后端API启动ReWOO会话
    const sessionId = `rewoo_${Date.now()}`
    emit('sessionStarted', sessionId)
    
    // 更新引擎状态
    engineStatus.value.active = true
  } catch (error) {
    console.error('启动ReWOO会话失败:', error)
  }
}

const refreshStatistics = async () => {
  try {
    // 调用后端API获取统计数据
    // const stats = await api.getReWOOStatistics()
    // statistics.value = stats
  } catch (error) {
    console.error('刷新统计失败:', error)
  }
}

const addVariable = () => {
  const variable: Variable = {
    id: `var_${Date.now()}`,
    name: newVariable.name,
    type: newVariable.type,
    value: parseVariableValue(newVariable.value, newVariable.type),
    description: newVariable.description,
    createdAt: new Date()
  }
  
  variables.value.push(variable)
  emit('variableAdded', variable)
  
  // 重置表单
  Object.assign(newVariable, {
    name: '',
    type: 'string',
    value: '',
    description: ''
  })
  
  showAddVariableModal.value = false
}

const editVariable = (variable: Variable) => {
  // 实现编辑变量逻辑
}

const deleteVariable = (variable: Variable) => {
  const index = variables.value.findIndex(v => v.id === variable.id)
  if (index > -1) {
    variables.value.splice(index, 1)
    emit('variableDeleted', variable.id)
  }
}

const parseVariableValue = (value: string, type: Variable['type']) => {
  try {
    switch (type) {
      case 'number':
        return Number(value)
      case 'boolean':
        return value.toLowerCase() === 'true'
      case 'object':
      case 'array':
        return JSON.parse(value)
      default:
        return value
    }
  } catch {
    return value
  }
}

// 格式化方法
const formatDuration = (ms: number) => {
  if (ms < 1000) return `${ms}ms`
  if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`
  return `${(ms / 60000).toFixed(1)}m`
}

const formatTime = (date: Date) => {
  return date.toLocaleTimeString()
}

const formatVariableValue = (value: any) => {
  if (typeof value === 'object') {
    return JSON.stringify(value, null, 2).substring(0, 50) + '...'
  }
  return String(value).substring(0, 50)
}

const formatSolverResult = (content: string) => {
  // 简单的Markdown渲染
  return content.replace(/\n/g, '<br>')
}

// 样式类方法
const getVariableTypeBadge = (type: string) => {
  const badges = {
    string: 'badge-primary',
    number: 'badge-secondary',
    boolean: 'badge-accent',
    object: 'badge-info',
    array: 'badge-warning'
  }
  return badges[type as keyof typeof badges] || 'badge-ghost'
}

const getToolChainStatusBadge = (status: string) => {
  const badges = {
    pending: 'badge-warning',
    running: 'badge-info',
    completed: 'badge-success',
    failed: 'badge-error'
  }
  return badges[status as keyof typeof badges] || 'badge-ghost'
}

const getToolStatusClass = (status: string) => {
  const classes = {
    pending: 'border-gray-300 bg-gray-50',
    running: 'border-blue-300 bg-blue-50',
    completed: 'border-green-300 bg-green-50',
    failed: 'border-red-300 bg-red-50'
  }
  return classes[status as keyof typeof classes] || 'border-gray-300 bg-gray-50'
}

const getToolStatusIndicator = (status: string) => {
  const indicators = {
    pending: 'bg-gray-400',
    running: 'bg-blue-400',
    completed: 'bg-green-400',
    failed: 'bg-red-400'
  }
  return indicators[status as keyof typeof indicators] || 'bg-gray-400'
}

const getLogBorderClass = (level: string) => {
  const classes = {
    info: 'border-blue-400',
    warn: 'border-yellow-400',
    error: 'border-red-400',
    debug: 'border-gray-400'
  }
  return classes[level as keyof typeof classes] || 'border-gray-400'
}

const getLogLevelBadge = (level: string) => {
  const badges = {
    info: 'badge-info',
    warn: 'badge-warning',
    error: 'badge-error',
    debug: 'badge-ghost'
  }
  return badges[level as keyof typeof badges] || 'badge-ghost'
}

// 生命周期
onMounted(() => {
  refreshStatistics()
})
</script>

<style scoped>
.rewoo-control-panel {
  @apply space-y-6;
}

.prose {
  @apply text-base-content;
}

.prose pre {
  @apply bg-base-300 text-base-content;
}
</style>