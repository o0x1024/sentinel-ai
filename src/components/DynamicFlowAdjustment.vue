<template>
  <div class="dynamic-flow-adjustment">
    <!-- 调整控制面板 -->
    <div class="card bg-base-100 shadow-xl mb-4">
      <div class="card-body">
        <div class="flex justify-between items-center mb-4">
          <h3 class="card-title">动态流程调整</h3>
          <div class="flex gap-2">
            <button 
              class="btn btn-sm btn-primary"
              @click="showAddStepModal = true"
            >
              <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
              </svg>
              添加步骤
            </button>
            <button 
              class="btn btn-sm btn-outline"
              @click="showAdjustmentHistory = !showAdjustmentHistory"
            >
              <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" />
              </svg>
              调整历史
            </button>
          </div>
        </div>
        
        <!-- 快速调整按钮 -->
        <div class="grid grid-cols-2 md:grid-cols-4 gap-2 mb-4">
          <button 
            class="btn btn-sm btn-outline"
            @click="pauseExecution"
            :disabled="!canPause"
          >
            <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 9v6m4-6v6" />
            </svg>
            暂停执行
          </button>
          
          <button 
            class="btn btn-sm btn-outline"
            @click="resumeExecution"
            :disabled="!canResume"
          >
            <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M14.828 14.828a4 4 0 01-5.656 0M9 10h1m4 0h1" />
            </svg>
            恢复执行
          </button>
          
          <button 
            class="btn btn-sm btn-warning"
            @click="triggerReplan"
            :disabled="!canReplan"
          >
            <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
            </svg>
            重新规划
          </button>
          
          <button 
            class="btn btn-sm btn-error"
            @click="cancelExecution"
            :disabled="!canCancel"
          >
            <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
            </svg>
            取消执行
          </button>
        </div>
        
        <!-- 当前执行状态 -->
        <div class="alert alert-info">
          <svg xmlns="http://www.w3.org/2000/svg" class="stroke-current shrink-0 h-6 w-6" fill="none" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/>
          </svg>
          <div>
            <h3 class="font-bold">当前状态: {{ getStatusText(currentStatus) }}</h3>
            <div class="text-xs">{{ statusDescription }}</div>
          </div>
        </div>
      </div>
    </div>
    
    <!-- 步骤管理 -->
    <div class="card bg-base-100 shadow-xl mb-4">
      <div class="card-body">
        <h4 class="card-title text-lg mb-4">执行步骤管理</h4>
        
        <div class="space-y-3">
          <div 
            v-for="(step, index) in executionSteps" 
            :key="step.id"
            class="step-item border rounded-lg p-3"
            :class="getStepClass(step)"
          >
            <div class="flex justify-between items-start">
              <div class="flex-1">
                <div class="flex items-center gap-2 mb-2">
                  <div class="badge badge-sm" :class="getStepStatusBadge(step.status)">
                    {{ getStepStatusText(step.status) }}
                  </div>
                  <div v-if="step.architecture" class="badge badge-sm badge-outline" :class="getArchitectureBadge(step.architecture)">
                    {{ getArchitectureText(step.architecture) }}
                  </div>
                  <span class="font-medium">{{ step.name }}</span>
                  <span class="text-sm text-base-content/60">#{{ index + 1 }}</span>
                </div>
                
                <p class="text-sm text-base-content/70 mb-2">{{ step.description }}</p>
                
                <div class="flex items-center gap-4 text-xs text-base-content/60">
                  <span>类型: {{ getStepTypeText(step.type) }}</span>
                  <span v-if="step.estimatedDuration">预计: {{ formatDuration(step.estimatedDuration) }}</span>
                  <span v-if="step.actualDuration">实际: {{ formatDuration(step.actualDuration) }}</span>
                </div>
                
                <!-- 进度条 -->
                <div v-if="step.status === 'running' && step.progress !== undefined" class="mt-2">
                  <div class="flex justify-between text-xs mb-1">
                    <span>执行进度</span>
                    <span>{{ Math.round(step.progress) }}%</span>
                  </div>
                  <progress class="progress progress-primary w-full h-2" :value="step.progress" max="100"></progress>
                </div>
              </div>
              
              <!-- 操作按钮 -->
              <div class="dropdown dropdown-end">
                <div tabindex="0" role="button" class="btn btn-sm btn-ghost">
                  <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 5v.01M12 12v.01M12 19v.01M12 6a1 1 0 110-2 1 1 0 010 2zm0 7a1 1 0 110-2 1 1 0 010 2zm0 7a1 1 0 110-2 1 1 0 010 2z" />
                  </svg>
                </div>
                <ul tabindex="0" class="dropdown-content z-[1] menu p-2 shadow bg-base-100 rounded-box w-40">
                  <li v-if="canModifyStep(step)">
                    <a @click="editStep(step)">编辑步骤</a>
                  </li>
                  <li v-if="canSkipStep(step)">
                    <a @click="skipStep(step)">跳过步骤</a>
                  </li>
                  <li v-if="canRetryStep(step)">
                    <a @click="retryStep(step)">重试步骤</a>
                  </li>
                  <li v-if="canRemoveStep(step)">
                    <a @click="removeStep(step)" class="text-error">删除步骤</a>
                  </li>
                </ul>
              </div>
            </div>
            
            <!-- 错误信息 -->
            <div v-if="step.error" class="mt-2 p-2 bg-error/10 border border-error/20 rounded text-sm">
              <div class="font-medium text-error">错误信息:</div>
              <div class="text-error/80">{{ step.error.message }}</div>
            </div>
          </div>
        </div>
      </div>
    </div>
    
    <!-- 调整历史 -->
    <div v-if="showAdjustmentHistory" class="card bg-base-100 shadow-xl mb-4">
      <div class="card-body">
        <h4 class="card-title text-lg mb-4">调整历史</h4>
        
        <div class="max-h-60 overflow-y-auto">
          <div 
            v-for="adjustment in adjustmentHistory" 
            :key="adjustment.id"
            class="border-l-4 border-primary pl-4 py-2 mb-3"
          >
            <div class="flex justify-between items-start">
              <div>
                <div class="font-medium">{{ getAdjustmentTypeText(adjustment.type) }}</div>
                <div class="text-sm text-base-content/70">{{ adjustment.reason }}</div>
                <div class="text-xs text-base-content/60 mt-1">
                  {{ formatTime(adjustment.timestamp) }} - {{ adjustment.appliedBy }}
                </div>
              </div>
              <div class="badge badge-sm badge-outline">{{ adjustment.type }}</div>
            </div>
          </div>
        </div>
      </div>
    </div>
    
    <!-- 添加步骤模态框 -->
    <div v-if="showAddStepModal" class="modal modal-open">
      <div class="modal-box">
        <h3 class="font-bold text-lg mb-4">添加新步骤</h3>
        
        <form @submit.prevent="addNewStep">
          <div class="form-control mb-4">
            <label class="label">
              <span class="label-text">步骤名称</span>
            </label>
            <input 
              v-model="newStep.name" 
              type="text" 
              class="input input-bordered" 
              placeholder="输入步骤名称"
              required
            >
          </div>
          
          <div class="form-control mb-4">
            <label class="label">
              <span class="label-text">步骤描述</span>
            </label>
            <textarea 
              v-model="newStep.description" 
              class="textarea textarea-bordered" 
              placeholder="输入步骤描述"
              rows="3"
            ></textarea>
          </div>
          
          <div class="form-control mb-4">
            <label class="label">
              <span class="label-text">步骤类型</span>
            </label>
            <select v-model="newStep.type" class="select select-bordered">
              <!-- 通用步骤类型 -->
              <option value="tool_call">工具调用</option>
              <option value="condition">条件判断</option>
              <option value="parallel">并行执行</option>
              <option value="loop">循环执行</option>
              <option value="human_input">人工输入</option>
              
              <!-- ReWOO 特定步骤 -->
              <template v-if="architecture === 'rewoo'">
                <option value="rewoo_plan">ReWOO 规划</option>
                <option value="rewoo_worker">ReWOO 工作器</option>
                <option value="rewoo_solver">ReWOO 求解器</option>
              </template>
              
              <!-- LLMCompiler 特定步骤 -->
              <template v-if="architecture === 'llm_compiler'">
                <option value="llm_dag_build">DAG 构建</option>
                <option value="llm_parallel_exec">并行执行</option>
                <option value="llm_joiner_decision">连接器决策</option>
              </template>
            </select>
          </div>
          
          <div class="form-control mb-4">
            <label class="label">
              <span class="label-text">插入位置</span>
            </label>
            <select v-model="newStep.insertPosition" class="select select-bordered">
              <option value="end">末尾</option>
              <option value="current">当前步骤后</option>
              <option value="beginning">开头</option>
            </select>
          </div>
          
          <div class="modal-action">
            <button type="button" class="btn btn-ghost" @click="showAddStepModal = false">
              取消
            </button>
            <button type="submit" class="btn btn-primary">
              添加步骤
            </button>
          </div>
        </form>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { DetailedExecutionStatus } from '../types/enhanced-execution'
import type { FlowAdjustment } from '../types/enhanced-execution'

// 步骤状态类型
type StepStatus = 'pending' | 'running' | 'completed' | 'failed' | 'skipped' | 'cancelled'

// 步骤类型
interface ExecutionStep {
  id: string
  name: string
  description: string
  type: 'tool_call' | 'condition' | 'parallel' | 'loop' | 'human_input' | 'rewoo_plan' | 'rewoo_worker' | 'rewoo_solver' | 'llm_dag_build' | 'llm_parallel_exec' | 'llm_joiner_decision'
  status: StepStatus
  progress?: number
  estimatedDuration?: number
  actualDuration?: number
  dependencies: string[]
  architecture?: 'plan_execute' | 'rewoo' | 'llm_compiler'
  error?: {
    message: string
    code?: string
    details?: any
  }
  metadata?: Record<string, any>
}

// Props
interface Props {
  sessionId: string
  currentStatus: DetailedExecutionStatus
  executionSteps: ExecutionStep[]
  allowAdjustments?: boolean
  architecture?: 'plan_execute' | 'rewoo' | 'llm_compiler'
}

const props = withDefaults(defineProps<Props>(), {
  allowAdjustments: true,
  architecture: 'plan_execute'
})

// Emits
const emit = defineEmits<{
  pauseExecution: []
  resumeExecution: []
  cancelExecution: []
  triggerReplan: []
  addStep: [step: ExecutionStep, position: string]
  removeStep: [stepId: string]
  modifyStep: [stepId: string, changes: Partial<ExecutionStep>]
  skipStep: [stepId: string]
  retryStep: [stepId: string]
  adjustmentApplied: [adjustment: FlowAdjustment]
}>()

// 响应式数据
const showAdjustmentHistory = ref(false)
const showAddStepModal = ref(false)
const adjustmentHistory = ref<FlowAdjustment[]>([])

// 新步骤表单
const newStep = ref({
  name: '',
  description: '',
  type: 'tool_call' as const,
  insertPosition: 'end'
})

// 计算属性
const statusDescription = computed(() => {
  const descriptions: Record<string, string> = {
    'initialized': '系统已初始化，等待开始执行',
    'planning_started': '正在分析任务并生成执行计划',
    'step_executing': '正在执行当前步骤',
    'monitoring_started': '正在监控执行进度',
    'replan_triggered': '检测到问题，正在重新规划',
    'paused': '执行已暂停，等待用户操作',
    'completed': '所有步骤已成功完成',
    'failed': '执行过程中发生错误',
    'requires_intervention': '需要用户干预才能继续'
  }
  return descriptions[props.currentStatus] || '未知状态'
})

const canPause = computed(() => {
  return ['step_executing', 'monitoring_started'].includes(props.currentStatus)
})

const canResume = computed(() => {
  return props.currentStatus === 'paused'
})

const canReplan = computed(() => {
  return ['step_executing', 'paused', 'failed'].includes(props.currentStatus)
})

const canCancel = computed(() => {
  return !['completed', 'cancelled', 'failed'].includes(props.currentStatus)
})

// 方法
const getStatusText = (status: DetailedExecutionStatus): string => {
  const statusMap: Record<string, string> = {
    'initialized': '已初始化',
    'planning_started': '规划中',
    'step_executing': '执行中',
    'monitoring_started': '监控中',
    'replan_triggered': '重规划中',
    'paused': '已暂停',
    'completed': '已完成',
    'failed': '失败',
    'requires_intervention': '需要干预'
  }
  return statusMap[status] || status
}

const getStepClass = (step: ExecutionStep): string => {
  const statusClasses = {
    'pending': 'border-gray-300 bg-gray-50',
    'running': 'border-blue-400 bg-blue-50',
    'completed': 'border-green-400 bg-green-50',
    'failed': 'border-red-400 bg-red-50',
    'skipped': 'border-yellow-400 bg-yellow-50',
    'cancelled': 'border-gray-400 bg-gray-100'
  }
  return statusClasses[step.status] || 'border-gray-300'
}

const getStepStatusBadge = (status: StepStatus): string => {
  const badgeClasses = {
    'pending': 'badge-ghost',
    'running': 'badge-info',
    'completed': 'badge-success',
    'failed': 'badge-error',
    'skipped': 'badge-warning',
    'cancelled': 'badge-neutral'
  }
  return badgeClasses[status] || 'badge-ghost'
}

const getStepStatusText = (status: StepStatus): string => {
  const statusTexts = {
    'pending': '待执行',
    'running': '执行中',
    'completed': '已完成',
    'failed': '失败',
    'skipped': '已跳过',
    'cancelled': '已取消'
  }
  return statusTexts[status] || status
}

const getStepTypeText = (type: string): string => {
  const typeTexts = {
    'tool_call': '工具调用',
    'condition': '条件判断',
    'parallel': '并行执行',
    'loop': '循环执行',
    'human_input': '人工输入',
    // ReWOO 特定类型
    'rewoo_plan': 'ReWOO 规划',
    'rewoo_worker': 'ReWOO 工作器',
    'rewoo_solver': 'ReWOO 求解器',
    // LLMCompiler 特定类型
    'llm_dag_build': 'DAG 构建',
    'llm_parallel_exec': '并行执行',
    'llm_joiner_decision': '连接器决策'
  }
  return typeTexts[type as keyof typeof typeTexts] || type
}

const getAdjustmentTypeText = (type: string): string => {
  const typeTexts = {
    'add_step': '添加步骤',
    'remove_step': '删除步骤',
    'modify_step': '修改步骤',
    'change_order': '调整顺序',
    'add_condition': '添加条件',
    'modify_condition': '修改条件'
  }
  return typeTexts[type as keyof typeof typeTexts] || type
}

const getArchitectureText = (architecture: string): string => {
  const architectureTexts = {
    'plan_execute': 'Plan-Execute',
    'rewoo': 'ReWOO',
    'llm_compiler': 'LLMCompiler'
  }
  return architectureTexts[architecture as keyof typeof architectureTexts] || architecture
}

const getArchitectureBadge = (architecture: string): string => {
  const badgeClasses = {
    'plan_execute': 'badge-info',
    'rewoo': 'badge-secondary',
    'llm_compiler': 'badge-accent'
  }
  return badgeClasses[architecture as keyof typeof badgeClasses] || 'badge-ghost'
}

const formatDuration = (ms: number): string => {
  const seconds = Math.floor(ms / 1000)
  const minutes = Math.floor(seconds / 60)
  const hours = Math.floor(minutes / 60)
  
  if (hours > 0) {
    return `${hours}h ${minutes % 60}m`
  } else if (minutes > 0) {
    return `${minutes}m ${seconds % 60}s`
  } else {
    return `${seconds}s`
  }
}

const formatTime = (date: Date): string => {
  return date.toLocaleTimeString()
}

const canModifyStep = (step: ExecutionStep): boolean => {
  return props.allowAdjustments && ['pending', 'failed'].includes(step.status)
}

const canSkipStep = (step: ExecutionStep): boolean => {
  return props.allowAdjustments && ['pending', 'failed'].includes(step.status)
}

const canRetryStep = (step: ExecutionStep): boolean => {
  return props.allowAdjustments && step.status === 'failed'
}

const canRemoveStep = (step: ExecutionStep): boolean => {
  return props.allowAdjustments && ['pending', 'failed', 'skipped'].includes(step.status)
}

// 执行控制方法
const pauseExecution = () => {
  emit('pauseExecution')
  recordAdjustment({
    type: 'modify_step',
    target: 'execution',
    data: { action: 'pause' },
    reason: '用户手动暂停执行',
    appliedBy: 'user'
  })
}

const resumeExecution = () => {
  emit('resumeExecution')
  recordAdjustment({
    type: 'modify_step',
    target: 'execution',
    data: { action: 'resume' },
    reason: '用户手动恢复执行',
    appliedBy: 'user'
  })
}

const cancelExecution = () => {
  emit('cancelExecution')
  recordAdjustment({
    type: 'modify_step',
    target: 'execution',
    data: { action: 'cancel' },
    reason: '用户手动取消执行',
    appliedBy: 'user'
  })
}

const triggerReplan = () => {
  emit('triggerReplan')
  recordAdjustment({
    type: 'modify_step',
    target: 'execution',
    data: { action: 'replan' },
    reason: '用户手动触发重规划',
    appliedBy: 'user'
  })
}

// 步骤管理方法
const addNewStep = () => {
  const step: ExecutionStep = {
    id: `step_${Date.now()}`,
    name: newStep.value.name,
    description: newStep.value.description,
    type: newStep.value.type,
    status: 'pending',
    dependencies: [],
    architecture: props.architecture
  }
  
  emit('addStep', step, newStep.value.insertPosition)
  
  recordAdjustment({
    type: 'add_step',
    target: step.id,
    data: step,
    reason: `用户添加新步骤: ${step.name}`,
    appliedBy: 'user'
  })
  
  // 重置表单
  newStep.value = {
    name: '',
    description: '',
    type: 'tool_call',
    insertPosition: 'end'
  }
  
  showAddStepModal.value = false
}

const editStep = (step: ExecutionStep) => {
  // 这里可以打开编辑模态框
  console.log('编辑步骤:', step)
}

const skipStep = (step: ExecutionStep) => {
  emit('skipStep', step.id)
  recordAdjustment({
    type: 'modify_step',
    target: step.id,
    data: { status: 'skipped' },
    reason: `用户跳过步骤: ${step.name}`,
    appliedBy: 'user'
  })
}

const retryStep = (step: ExecutionStep) => {
  emit('retryStep', step.id)
  recordAdjustment({
    type: 'modify_step',
    target: step.id,
    data: { status: 'pending', error: null },
    reason: `用户重试步骤: ${step.name}`,
    appliedBy: 'user'
  })
}

const removeStep = (step: ExecutionStep) => {
  emit('removeStep', step.id)
  recordAdjustment({
    type: 'remove_step',
    target: step.id,
    data: step,
    reason: `用户删除步骤: ${step.name}`,
    appliedBy: 'user'
  })
}

const recordAdjustment = (adjustmentData: Omit<FlowAdjustment, 'id' | 'timestamp'>) => {
  const adjustment: FlowAdjustment = {
    id: `adj_${Date.now()}`,
    timestamp: new Date(),
    ...adjustmentData
  }
  
  adjustmentHistory.value.unshift(adjustment)
  
  // 限制历史记录数量
  if (adjustmentHistory.value.length > 50) {
    adjustmentHistory.value = adjustmentHistory.value.slice(0, 50)
  }
  
  emit('adjustmentApplied', adjustment)
}

// 监听步骤变化
watch(() => props.executionSteps, (newSteps, oldSteps) => {
  if (oldSteps && newSteps.length !== oldSteps.length) {
    // 检测步骤数量变化
    if (newSteps.length > oldSteps.length) {
      const newStep = newSteps.find(step => !oldSteps.some(old => old.id === step.id))
      if (newStep) {
        recordAdjustment({
          type: 'add_step',
          target: newStep.id,
          data: newStep,
          reason: '系统自动添加步骤',
          appliedBy: 'system'
        })
      }
    }
  }
}, { deep: true })
</script>

<style scoped>
.dynamic-flow-adjustment {
  @apply space-y-4;
}

.step-item {
  transition: all 0.2s ease;
}

.step-item:hover {
  @apply shadow-md;
}

.modal {
  @apply z-50;
}
</style>