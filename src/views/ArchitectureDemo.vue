<template>
  <div class="architecture-demo">
    <!-- 页面标题 -->
    <div class="mb-6">
      <h1 class="text-3xl font-bold mb-2">智能体架构演示</h1>
      <p class="text-base-content/70">
        体验不同的智能体执行架构，了解它们的特点和适用场景
      </p>
    </div>
    
    <!-- 导航标签 -->
    <div class="tabs tabs-boxed mb-6">
      <a 
        v-for="tab in tabs" 
        :key="tab.id"
        :class="['tab', { 'tab-active': activeTab === tab.id }]"
        @click="activeTab = tab.id"
      >
        {{ tab.name }}
      </a>
    </div>
    
    <!-- 架构选择 -->
    <div v-if="activeTab === 'selector'" class="space-y-6">
      <ArchitectureSelector 
        v-model="selectedArchitecture"
        @change="onArchitectureChange"
      />
      
      <!-- 选择结果 -->
      <div v-if="selectedArchitecture" class="alert alert-success">
        <svg xmlns="http://www.w3.org/2000/svg" class="stroke-current shrink-0 h-6 w-6" fill="none" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"/>
        </svg>
        <div>
          <h3 class="font-bold">架构已选择</h3>
          <div class="text-xs">当前选择: {{ getArchitectureName(selectedArchitecture) }}</div>
        </div>
      </div>
    </div>
    
    <!-- 架构配置 -->
    <div v-if="activeTab === 'config'" class="space-y-6">
      <div class="alert alert-info mb-4">
        <svg xmlns="http://www.w3.org/2000/svg" class="stroke-current shrink-0 h-6 w-6" fill="none" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/>
        </svg>
        <div>
          <h3 class="font-bold">配置 {{ getArchitectureName(selectedArchitecture) }} 架构</h3>
          <div class="text-xs">调整参数以优化执行性能</div>
        </div>
      </div>
      
      <ArchitectureConfig 
        :architecture="selectedArchitecture"
        :initial-config="architectureConfigs[selectedArchitecture]"
        @config-change="onConfigChange"
        @save="onConfigSave"
      />
    </div>
    
    <!-- 架构对比 -->
    <div v-if="activeTab === 'comparison'">
      <ArchitectureComparison />
    </div>
    
    <!-- 实时演示 -->
    <div v-if="activeTab === 'demo'" class="space-y-6">
      <!-- 控制面板 -->
      <div class="card bg-base-100 shadow-xl">
        <div class="card-body">
          <h3 class="card-title mb-4">实时演示控制</h3>
          
          <div class="grid grid-cols-1 md:grid-cols-3 gap-4 mb-4">
            <div class="form-control">
              <label class="label">
                <span class="label-text">演示架构</span>
              </label>
              <select v-model="demoArchitecture" class="select select-bordered">
                <option value="plan_execute">Plan-and-Execute</option>
                <option value="rewoo">ReWOO</option>
                <option value="llm_compiler">LLMCompiler</option>
              </select>
            </div>
            
            <div class="form-control">
              <label class="label">
                <span class="label-text">演示任务</span>
              </label>
              <select v-model="demoTask" class="select select-bordered">
                <option value="simple">简单查询任务</option>
                <option value="complex">复杂分析任务</option>
                <option value="parallel">并行处理任务</option>
              </select>
            </div>
            
            <div class="form-control">
              <label class="label">
                <span class="label-text">执行模式</span>
              </label>
              <select v-model="demoMode" class="select select-bordered">
                <option value="simulation">模拟执行</option>
                <option value="real">真实执行</option>
              </select>
            </div>
          </div>
          
          <div class="flex gap-2">
            <button 
              class="btn btn-primary"
              @click="startDemo"
              :disabled="isDemoRunning"
            >
              <svg v-if="!isDemoRunning" xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M14.828 14.828a4 4 0 01-5.656 0M9 10h1m4 0h1" />
              </svg>
              <span class="loading loading-spinner loading-sm" v-else></span>
              {{ isDemoRunning ? '执行中...' : '开始演示' }}
            </button>
            
            <button 
              class="btn btn-outline"
              @click="stopDemo"
              :disabled="!isDemoRunning"
            >
              <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
              </svg>
              停止演示
            </button>
            
            <button 
              class="btn btn-ghost"
              @click="resetDemo"
            >
              <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
              </svg>
              重置
            </button>
          </div>
        </div>
      </div>
      
      <!-- 架构特定演示组件 -->
      <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <!-- ReWOO 演示 -->
        <div v-if="demoArchitecture === 'rewoo'">
          <ReWOOControlPanel 
            :session-id="demoSessionId"
            :engine-status="demoStatus"
            :variables="demoVariables"
            :tools="demoTools"
            :solver-result="demoSolverResult"
          />
        </div>
        
        <!-- LLMCompiler 演示 -->
        <div v-if="demoArchitecture === 'llm_compiler'" class="space-y-4">
          <LLMCompilerDAGView 
            :session-id="demoSessionId"
            :dag-status="demoDAGStatus"
            :tasks="demoTasks"
            :execution-flow="demoExecutionFlow"
          />
          
          <LLMCompilerExecutionMonitor 
            :session-id="demoSessionId"
            :execution-status="demoExecutionStatus"
            :performance-metrics="demoPerformanceMetrics"
            :task-details="demoTaskDetails"
          />
        </div>
        
        <!-- Plan-and-Execute 演示 -->
        <div v-if="demoArchitecture === 'plan_execute'">
          <EnhancedStateMachine 
            :session-id="demoSessionId"
            :current-status="demoStatus"
            :execution-data="demoExecutionData"
            :allow-transitions="true"
          />
        </div>
        
        <!-- 通用流程图 -->
        <div>
          <FlowchartVisualization 
            :session-id="demoSessionId"
            :nodes="demoNodes"
            :connections="demoConnections"
            :current-step="demoCurrentStep"
            :layout="'hierarchical'"
          />
        </div>
      </div>
      
      <!-- 演示日志 -->
      <div class="card bg-base-100 shadow-xl">
        <div class="card-body">
          <h4 class="card-title text-lg mb-4">演示日志</h4>
          <div class="mockup-code max-h-60 overflow-y-auto">
            <pre v-for="(log, index) in demoLogs" :key="index"><code>{{ log }}</code></pre>
          </div>
        </div>
      </div>
    </div>
    
    <!-- 性能监控 -->
    <div v-if="activeTab === 'performance'">
      <PerformanceMonitor 
        :session-id="demoSessionId"
        :architecture="selectedArchitecture"
        :real-time="true"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted, onUnmounted } from 'vue'
import ArchitectureSelector from '../components/ArchitectureSelector.vue'
import ArchitectureConfig from '../components/ArchitectureConfig.vue'
import ArchitectureComparison from '../components/ArchitectureComparison.vue'
import ReWOOControlPanel from '../components/ReWOOControlPanel.vue'
import LLMCompilerDAGView from '../components/LLMCompilerDAGView.vue'
import LLMCompilerExecutionMonitor from '../components/LLMCompilerExecutionMonitor.vue'
import EnhancedStateMachine from '../components/EnhancedStateMachine.vue'
import FlowchartVisualization from '../components/FlowchartVisualization.vue'
import PerformanceMonitor from '../components/PerformanceMonitor.vue'
import { DetailedExecutionStatus } from '../types/enhanced-execution'

// 标签页
const tabs = [
  { id: 'selector', name: '架构选择' },
  { id: 'config', name: '架构配置' },
  { id: 'comparison', name: '架构对比' },
  { id: 'demo', name: '实时演示' },
  { id: 'performance', name: '性能监控' }
]

// 响应式数据
const activeTab = ref('selector')
const selectedArchitecture = ref('plan_execute')
const architectureConfigs = reactive({
  plan_execute: {},
  rewoo: {},
  llm_compiler: {}
})

// 演示相关数据
const demoArchitecture = ref('plan_execute')
const demoTask = ref('simple')
const demoMode = ref('simulation')
const isDemoRunning = ref(false)
const demoSessionId = ref('demo_session_001')
const demoStatus = ref<DetailedExecutionStatus>('initialized')
const demoLogs = ref<string[]>([])

// 演示数据
const demoVariables = ref({})
const demoTools = ref([])
const demoSolverResult = ref(null)
const demoDAGStatus = ref({})
const demoTasks = ref([])
const demoExecutionFlow = ref([])
const demoExecutionStatus = ref({})
const demoPerformanceMetrics = ref({})
const demoTaskDetails = ref([])
const demoExecutionData = ref({})
const demoNodes = ref([])
const demoConnections = ref([])
const demoCurrentStep = ref('')

// 定时器
let demoTimer: NodeJS.Timeout | null = null

// 方法
const getArchitectureName = (archId: string): string => {
  const names = {
    plan_execute: 'Plan-and-Execute',
    rewoo: 'ReWOO',
    llm_compiler: 'LLMCompiler'
  }
  return names[archId as keyof typeof names] || archId
}

const onArchitectureChange = (architecture: string) => {
  console.log('架构已切换:', architecture)
  addDemoLog(`架构切换到: ${getArchitectureName(architecture)}`)
}

const onConfigChange = (config: any) => {
  architectureConfigs[selectedArchitecture.value as keyof typeof architectureConfigs] = config
  console.log('配置已更新:', config)
}

const onConfigSave = (config: any) => {
  console.log('配置已保存:', config)
  addDemoLog(`${getArchitectureName(selectedArchitecture.value)} 配置已保存`)
}

const startDemo = () => {
  isDemoRunning.value = true
  addDemoLog(`开始 ${getArchitectureName(demoArchitecture.value)} 架构演示`)
  addDemoLog(`任务类型: ${demoTask.value}, 执行模式: ${demoMode.value}`)
  
  // 模拟演示过程
  simulateDemo()
}

const stopDemo = () => {
  isDemoRunning.value = false
  if (demoTimer) {
    clearInterval(demoTimer)
    demoTimer = null
  }
  addDemoLog('演示已停止')
}

const resetDemo = () => {
  stopDemo()
  demoLogs.value = []
  demoStatus.value = 'initialized'
  addDemoLog('演示已重置')
}

const simulateDemo = () => {
  const steps = {
    plan_execute: [
      'initialized',
      'planning_started', 
      'step_executing',
      'monitoring_started',
      'completed'
    ],
    rewoo: [
      'rewoo_planning',
      'rewoo_worker_executing',
      'rewoo_solver_processing',
      'completed'
    ],
    llm_compiler: [
      'llm_dag_building',
      'llm_parallel_executing',
      'llm_joiner_deciding',
      'completed'
    ]
  }
  
  const currentSteps = steps[demoArchitecture.value as keyof typeof steps]
  let stepIndex = 0
  
  demoTimer = setInterval(() => {
    if (stepIndex < currentSteps.length) {
      demoStatus.value = currentSteps[stepIndex] as DetailedExecutionStatus
      addDemoLog(`状态更新: ${currentSteps[stepIndex]}`)
      stepIndex++
    } else {
      stopDemo()
    }
  }, 2000)
}

const addDemoLog = (message: string) => {
  const timestamp = new Date().toLocaleTimeString()
  demoLogs.value.unshift(`[${timestamp}] ${message}`)
  
  // 限制日志数量
  if (demoLogs.value.length > 50) {
    demoLogs.value = demoLogs.value.slice(0, 50)
  }
}

// 生命周期
onMounted(() => {
  addDemoLog('架构演示页面已加载')
})

onUnmounted(() => {
  if (demoTimer) {
    clearInterval(demoTimer)
  }
})
</script>

<style scoped>
.architecture-demo {
  @apply p-6 max-w-7xl mx-auto;
}

.mockup-code {
  @apply text-xs;
}

.tabs {
  @apply justify-center;
}
</style>