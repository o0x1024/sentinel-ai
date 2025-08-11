<template>
  <div class="enhanced-state-machine">
    <!-- 状态机控制面板 -->
    <div class="card bg-base-100 shadow-xl mb-6">
      <div class="card-body">
        <h2 class="card-title">状态机控制面板</h2>
        
        <!-- 当前状态显示 -->
        <div class="grid grid-cols-1 md:grid-cols-3 gap-4 mb-4">
          <div class="stat bg-base-200 rounded-lg">
            <div class="stat-title">当前状态</div>
            <div class="stat-value text-lg" :class="getStatusClass(currentState)">
              {{ getStatusText(currentState) }}
            </div>
            <div class="stat-desc">{{ formatTime(lastTransitionTime) }}</div>
          </div>
          
          <div class="stat bg-base-200 rounded-lg">
            <div class="stat-title">状态持续时间</div>
            <div class="stat-value text-lg">{{ formatDuration(stateDuration) }}</div>
            <div class="stat-desc">自上次转换</div>
          </div>
          
          <div class="stat bg-base-200 rounded-lg">
            <div class="stat-title">转换次数</div>
            <div class="stat-value text-lg">{{ transitionCount }}</div>
            <div class="stat-desc">总计</div>
          </div>
        </div>
        
        <!-- 状态转换控制 -->
        <div class="flex flex-wrap gap-2 mb-4">
          <button 
            v-for="action in availableActions" 
            :key="action.name"
            class="btn btn-sm"
            :class="action.class"
            @click="triggerTransition(action.event)"
            :disabled="!action.enabled"
          >
            {{ action.label }}
          </button>
        </div>
        
        <!-- 状态历史 -->
        <div class="collapse collapse-arrow bg-base-200">
          <input type="checkbox" /> 
          <div class="collapse-title text-xl font-medium">
            状态历史 ({{ stateHistory.length }} 条记录)
          </div>
          <div class="collapse-content">
            <div class="max-h-60 overflow-y-auto">
              <div 
                v-for="(entry, index) in stateHistory.slice().reverse()" 
                :key="index"
                class="flex justify-between items-center py-2 border-b border-base-300 last:border-b-0"
              >
                <div>
                  <span class="font-medium">{{ getStatusText(entry.metadata?.fromState || entry.state) }}</span>
                  <span class="mx-2">→</span>
                  <span class="font-medium">{{ getStatusText(entry.metadata?.toState || entry.state) }}</span>
                </div>
                <div class="text-sm text-base-content/70">
                  {{ formatTime(entry.timestamp) }}
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
    
    <!-- 状态转换图 -->
    <div class="card bg-base-100 shadow-xl mb-6">
      <div class="card-body">
        <h2 class="card-title">状态转换图</h2>
        <div class="state-diagram" ref="stateDiagram">
          <svg width="100%" height="400" viewBox="0 0 800 400">
            <!-- 状态节点 -->
            <g v-for="(state, index) in stateNodes" :key="state.id">
              <circle
                :cx="state.x"
                :cy="state.y"
                :r="state.radius"
                :class="getStateNodeClass(state.id)"
                class="state-node"
              />
              <text
                :x="state.x"
                :y="state.y + 5"
                text-anchor="middle"
                class="text-sm font-medium fill-current"
              >
                {{ getStatusText(state.id) }}
              </text>
            </g>
            
            <!-- 转换箭头 -->
            <g v-for="transition in stateTransitions" :key="transition.id">
              <defs>
                <marker
                  :id="`arrowhead-${transition.id}`"
                  markerWidth="10"
                  markerHeight="7"
                  refX="9"
                  refY="3.5"
                  orient="auto"
                >
                  <polygon
                    points="0 0, 10 3.5, 0 7"
                    class="fill-current text-base-content/60"
                  />
                </marker>
              </defs>
              <path
                :d="transition.path"
                class="stroke-current text-base-content/60 fill-none stroke-2"
                :marker-end="`url(#arrowhead-${transition.id})`"
              />
            </g>
          </svg>
        </div>
      </div>
    </div>
    
    <!-- 实时监控 -->
    <div class="card bg-base-100 shadow-xl">
      <div class="card-body">
        <h2 class="card-title">实时监控</h2>
        
        <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
          <!-- 性能指标 -->
          <div>
            <h3 class="text-lg font-semibold mb-2">性能指标</h3>
            <div class="space-y-2">
              <div class="flex justify-between">
                <span>执行时间:</span>
                <span class="font-mono">{{ formatDuration(metrics.executionTime) }}</span>
              </div>
              <div class="flex justify-between">
                <span>内存使用:</span>
                <span class="font-mono">{{ formatBytes(metrics.memoryUsage) }}</span>
              </div>
              <div class="flex justify-between">
                <span>CPU使用率:</span>
                <span class="font-mono">{{ metrics.cpuUsage.toFixed(1) }}%</span>
              </div>
            </div>
          </div>
          
          <!-- 警告和错误 -->
          <div>
            <h3 class="text-lg font-semibold mb-2">警告和错误</h3>
            <div class="max-h-32 overflow-y-auto">
              <div 
                v-for="(warning, index) in warnings" 
                :key="index"
                class="alert alert-warning alert-sm mb-1"
              >
                <svg xmlns="http://www.w3.org/2000/svg" class="stroke-current shrink-0 h-4 w-4" fill="none" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L3.732 16.5c-.77.833.192 2.5 1.732 2.5z" />
                </svg>
                <span class="text-xs">{{ warning.message }}</span>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from 'vue'
import { DetailedExecutionStatus } from '../types/enhanced-execution'
import type { 
  StateTransitionEvent, 
  ExecutionWarning,
  StateHistoryEntry 
} from '../types/enhanced-execution'

// Props
interface Props {
  sessionId?: string
  initialState?: DetailedExecutionStatus
  autoUpdate?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  initialState: DetailedExecutionStatus.INITIALIZED,
  autoUpdate: true
})

// Emits
const emit = defineEmits<{
  stateChanged: [state: DetailedExecutionStatus]
  transitionTriggered: [event: StateTransitionEvent]
  error: [error: Error]
}>()

// 响应式数据
const currentState = ref<DetailedExecutionStatus>(props.initialState)
const lastTransitionTime = ref<Date>(new Date())
const transitionCount = ref(0)
const stateHistory = ref<StateHistoryEntry[]>([])
const warnings = ref<ExecutionWarning[]>([])
const metrics = ref({
  executionTime: 0,
  memoryUsage: 0,
  cpuUsage: 0
})

// 状态持续时间
const stateDuration = computed(() => {
  return Date.now() - lastTransitionTime.value.getTime()
})

// 可用操作
const availableActions = computed(() => {
  const actions = []
  
  switch (currentState.value) {
    case DetailedExecutionStatus.INITIALIZED:
      actions.push({
        name: 'start_planning',
        label: '开始规划',
        event: 'START_PLANNING',
        class: 'btn-primary',
        enabled: true
      })
      break
      
    case DetailedExecutionStatus.PLANNING_STARTED:
      actions.push(
        {
          name: 'complete_planning',
          label: '完成规划',
          event: 'PLANNING_COMPLETED',
          class: 'btn-success',
          enabled: true
        },
        {
          name: 'pause',
          label: '暂停',
          event: 'PAUSE',
          class: 'btn-warning',
          enabled: true
        }
      )
      break
      
    case DetailedExecutionStatus.STEP_EXECUTING:
      actions.push(
        {
          name: 'pause',
          label: '暂停',
          event: 'PAUSE',
          class: 'btn-warning',
          enabled: true
        },
        {
          name: 'stop',
          label: '停止',
          event: 'STOP',
          class: 'btn-error',
          enabled: true
        }
      )
      break
      
    case DetailedExecutionStatus.PAUSED:
      actions.push(
        {
          name: 'resume',
          label: '恢复',
          event: 'RESUME',
          class: 'btn-success',
          enabled: true
        },
        {
          name: 'stop',
          label: '停止',
          event: 'STOP',
          class: 'btn-error',
          enabled: true
        }
      )
      break
      
    case DetailedExecutionStatus.REQUIRES_INTERVENTION:
      actions.push({
        name: 'intervene',
        label: '人工干预',
        event: 'USER_INTERVENTION',
        class: 'btn-info',
        enabled: true
      })
      break
  }
  
  return actions
})

// 状态节点配置
const stateNodes = computed(() => {
  return [
    { id: DetailedExecutionStatus.INITIALIZED, x: 100, y: 100, radius: 30 },
    { id: DetailedExecutionStatus.PLANNING_STARTED, x: 300, y: 100, radius: 30 },
    { id: DetailedExecutionStatus.STEP_EXECUTING, x: 500, y: 100, radius: 30 },
    { id: DetailedExecutionStatus.MONITORING_STARTED, x: 700, y: 100, radius: 30 },
    { id: DetailedExecutionStatus.REPLAN_TRIGGERED, x: 500, y: 250, radius: 30 },
    { id: DetailedExecutionStatus.PAUSED, x: 300, y: 250, radius: 30 },
    { id: DetailedExecutionStatus.COMPLETED, x: 700, y: 250, radius: 30 },
    { id: DetailedExecutionStatus.FAILED, x: 100, y: 250, radius: 30 },
    { id: DetailedExecutionStatus.REQUIRES_INTERVENTION, x: 100, y: 350, radius: 30 },
    // ReWOO 特定状态节点
    { id: 'rewoo_planning', x: 150, y: 50, radius: 25 },
    { id: 'rewoo_worker_executing', x: 350, y: 50, radius: 25 },
    { id: 'rewoo_solver_processing', x: 550, y: 50, radius: 25 },
    { id: 'rewoo_variable_resolving', x: 750, y: 50, radius: 25 },
    // LLMCompiler 特定状态节点
    { id: 'dag_building', x: 150, y: 300, radius: 25 },
    { id: 'parallel_executing', x: 350, y: 300, radius: 25 },
    { id: 'dependency_resolving', x: 550, y: 300, radius: 25 },
    { id: 'joiner_deciding', x: 750, y: 300, radius: 25 },
    { id: 'task_fetching', x: 450, y: 350, radius: 25 }
  ]
})

// 状态转换路径
const stateTransitions = computed(() => {
  return [
    { id: 1, path: 'M 130 100 L 270 100' }, // INITIALIZED -> PLANNING_STARTED
    { id: 2, path: 'M 330 100 L 470 100' }, // PLANNING_STARTED -> STEP_EXECUTING
    { id: 3, path: 'M 530 100 L 670 100' }, // STEP_EXECUTING -> MONITORING
    { id: 4, path: 'M 500 130 L 500 220' }, // STEP_EXECUTING -> REPLANNING
    { id: 5, path: 'M 300 130 L 300 220' }, // PLANNING_STARTED -> PAUSED
    { id: 6, path: 'M 700 130 L 700 220' }, // MONITORING -> COMPLETED
    { id: 7, path: 'M 470 250 L 330 250' }, // REPLANNING -> PAUSED
    { id: 8, path: 'M 130 250 L 130 320' }  // FAILED -> REQUIRES_INTERVENTION
  ]
})

// 方法
const getStatusClass = (status: DetailedExecutionStatus): string => {
  const statusMap: Record<string, string> = {
    'initialized': 'text-info',
    'planning_started': 'text-warning',
    'step_executing': 'text-primary',
    'monitoring': 'text-info',
    'replanning': 'text-warning',
    'completed': 'text-success',
    'failed': 'text-error',
    'paused': 'text-warning',
    'requires_intervention': 'text-error',
    // ReWOO 特定状态样式
    'rewoo_planning': 'text-purple-500',
    'rewoo_worker_executing': 'text-blue-500',
    'rewoo_solver_processing': 'text-green-500',
    'rewoo_variable_resolving': 'text-yellow-500',
    // LLMCompiler 特定状态样式
    'dag_building': 'text-indigo-500',
    'parallel_executing': 'text-pink-500',
    'dependency_resolving': 'text-teal-500',
    'joiner_deciding': 'text-orange-500',
    'task_fetching': 'text-cyan-500'
  }
  return statusMap[status] || 'text-base-content'
}

const getStatusText = (status: DetailedExecutionStatus): string => {
  const statusMap: Record<string, string> = {
    'initialized': '已初始化',
    'planning_started': '规划中',
    'planning_completed': '规划完成',
    'step_executing': '执行中',
    'step_completed': '步骤完成',
    'tool_calling': '工具调用',
    'tool_completed': '工具完成',
    'monitoring': '监控中',
    'replanning': '重规划中',
    'completed': '已完成',
    'failed': '失败',
    'paused': '已暂停',
    'resumed': '已恢复',
    'cancelled': '已取消',
    'requires_intervention': '需要干预',
    // ReWOO 特定状态
    'rewoo_planning': 'ReWOO规划',
    'rewoo_worker_executing': 'ReWOO工作器执行',
    'rewoo_solver_processing': 'ReWOO求解器处理',
    'rewoo_variable_resolving': 'ReWOO变量解析',
    // LLMCompiler 特定状态
    'dag_building': 'DAG构建',
    'parallel_executing': '并行执行',
    'dependency_resolving': '依赖解析',
    'joiner_deciding': '连接器决策',
    'task_fetching': '任务获取'
  }
  return statusMap[status] || status
}

const getStateNodeClass = (stateId: DetailedExecutionStatus): string => {
  const baseClass = 'fill-current stroke-current stroke-2'
  if (stateId === currentState.value) {
    return `${baseClass} text-primary`
  }
  return `${baseClass} text-base-content/30`
}

const formatTime = (time: Date): string => {
  return time.toLocaleTimeString()
}

const formatDuration = (ms: number): string => {
  const seconds = Math.floor(ms / 1000)
  const minutes = Math.floor(seconds / 60)
  const hours = Math.floor(minutes / 60)
  
  if (hours > 0) {
    return `${hours}h ${minutes % 60}m ${seconds % 60}s`
  } else if (minutes > 0) {
    return `${minutes}m ${seconds % 60}s`
  } else {
    return `${seconds}s`
  }
}

const formatBytes = (bytes: number): string => {
  const sizes = ['B', 'KB', 'MB', 'GB']
  if (bytes === 0) return '0 B'
  const i = Math.floor(Math.log(bytes) / Math.log(1024))
  return `${(bytes / Math.pow(1024, i)).toFixed(1)} ${sizes[i]}`
}

const triggerTransition = (event: string) => {
  const transitionEvent: StateTransitionEvent = {
    sessionId: props.sessionId || '',
    from: currentState.value,
    to: currentState.value, // 这里应该根据状态机逻辑计算
    timestamp: new Date(),
    metadata: {
      event,
      metrics: metrics.value
    }
  }
  
  emit('transitionTriggered', transitionEvent)
}

const updateState = (newState: DetailedExecutionStatus) => {
  const oldState = currentState.value
  currentState.value = newState
  lastTransitionTime.value = new Date()
  transitionCount.value++
  
  // 添加到历史记录
  stateHistory.value.push({
    state: newState,
    timestamp: new Date(),
    duration: stateDuration.value,
    metadata: {
      fromState: oldState,
      toState: newState
    }
  })
  
  // 限制历史记录数量
  if (stateHistory.value.length > 100) {
    stateHistory.value = stateHistory.value.slice(-100)
  }
  
  emit('stateChanged', newState)
}

// 模拟实时更新
let updateInterval: NodeJS.Timeout | null = null

const startRealTimeUpdates = () => {
  if (!props.autoUpdate) return
  
  updateInterval = setInterval(() => {
    // 模拟性能指标更新
    metrics.value = {
      executionTime: metrics.value.executionTime + 1000,
      memoryUsage: Math.random() * 1024 * 1024 * 100, // 0-100MB
      cpuUsage: Math.random() * 100
    }
    
    // 模拟警告生成
    if (Math.random() < 0.1) { // 10% 概率生成警告
      warnings.value.push({
        id: Date.now().toString(),
        type: 'performance',
        severity: 'medium',
        message: `性能警告: CPU使用率 ${metrics.value.cpuUsage.toFixed(1)}%`,
        timestamp: new Date(),
        metadata: {
          resolved: false
        }
      })
      
      // 限制警告数量
      if (warnings.value.length > 10) {
        warnings.value = warnings.value.slice(-10)
      }
    }
  }, 1000)
}

const stopRealTimeUpdates = () => {
  if (updateInterval) {
    clearInterval(updateInterval)
    updateInterval = null
  }
}

// 生命周期
onMounted(() => {
  startRealTimeUpdates()
})

onUnmounted(() => {
  stopRealTimeUpdates()
})

// 监听状态变化
watch(() => props.initialState, (newState) => {
  updateState(newState)
})

// 暴露方法给父组件
defineExpose({
  updateState,
  getCurrentState: () => currentState.value,
  getStateHistory: () => stateHistory.value,
  triggerTransition
})
</script>

<style scoped>
.enhanced-state-machine {
  @apply space-y-6;
}

.state-node {
  transition: all 0.3s ease;
}

.state-node:hover {
  @apply scale-110;
}

.state-diagram {
  @apply w-full h-full;
}
</style>