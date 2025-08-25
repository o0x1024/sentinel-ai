<template>
  <div class="agent-execution-dashboard">
    <!-- 页面标题 -->
    <div class="mb-6">
      <h1 class="text-3xl font-bold mb-2">智能Agent执行控制台</h1>
      <p class="text-base-content/70">
        场景驱动的智能Agent系统，为不同任务场景提供最优解决方案
      </p>
    </div>
    
    <!-- 执行流程步骤指示器 -->
    <div class="steps w-full mb-8">
      <div :class="['step', currentStep >= 1 ? 'step-primary' : '']">
        <i class="fas fa-magic"></i>
        <span class="text-xs">场景选择</span>
      </div>
      <div :class="['step', currentStep >= 2 ? 'step-primary' : '']">
        <i class="fas fa-cog"></i>
        <span class="text-xs">参数配置</span>
      </div>
      <div :class="['step', currentStep >= 3 ? 'step-primary' : '']">
        <i class="fas fa-play"></i>
        <span class="text-xs">执行监控</span>
      </div>
      <div :class="['step', currentStep >= 4 ? 'step-primary' : '']">
        <i class="fas fa-check-circle"></i>
        <span class="text-xs">结果查看</span>
      </div>
    </div>
    
    <!-- 步骤1: 场景选择 -->
    <div v-if="currentStep === 1" class="space-y-6">
      <ScenarioSelector 
        v-model="selectedScenario"
        @confirm="onScenarioConfirmed"
      />
    </div>
    
    <!-- 步骤2: 参数配置 -->
    <div v-if="currentStep === 2" class="space-y-6">
      <div class="card bg-base-100 shadow-xl">
        <div class="card-body">
          <h3 class="card-title mb-4">
            <i class="fas fa-cog mr-2"></i>
            任务配置
          </h3>
          
          <!-- 场景信息回显 -->
          <div class="alert alert-info mb-4">
            <i class="fas fa-info-circle"></i>
            <div>
              <h4 class="font-bold">已选择场景: {{ getScenarioDisplayName(scenarioConfig?.scenario) }}</h4>
              <div class="text-sm">推荐架构: {{ getArchitectureName(scenarioConfig?.architecture) }}</div>
            </div>
          </div>
          
          <!-- 任务配置表单 -->
          <div class="space-y-4">
            <div class="form-control">
              <label class="label">
                <span class="label-text">任务描述 *</span>
              </label>
              <textarea 
                v-model="taskConfig.description" 
                class="textarea textarea-bordered h-24" 
                placeholder="请详细描述您要执行的任务..."
                required
              ></textarea>
            </div>
            
            <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
              <div class="form-control">
                <label class="label">
                  <span class="label-text">目标</span>
                </label>
                <input 
                  v-model="taskConfig.target" 
                  type="text" 
                  class="input input-bordered" 
                  placeholder="example.com 或 192.168.1.1"
                />
              </div>
              
              <div class="form-control">
                <label class="label">
                  <span class="label-text">优先级</span>
                </label>
                <select v-model="taskConfig.priority" class="select select-bordered">
                  <option value="normal">普通</option>
                  <option value="high">高</option>
                  <option value="critical">紧急</option>
                  <option value="low">低</option>
                </select>
              </div>
            </div>
            
            <div class="form-control">
              <label class="label">
                <span class="label-text">用户ID</span>
              </label>
              <input 
                v-model="taskConfig.userId" 
                type="text" 
                class="input input-bordered" 
                placeholder="admin"
              />
            </div>
            
            <!-- 高级配置 -->
            <div class="collapse collapse-arrow border border-base-300">
              <input type="checkbox" />
              <div class="collapse-title text-sm font-medium">
                <i class="fas fa-sliders-h mr-2"></i>
                高级配置
              </div>
              <div class="collapse-content">
                <div class="space-y-4 pt-2">
                  <div class="form-control">
                    <label class="label">
                      <span class="label-text">强制使用架构</span>
                    </label>
                    <select v-model="taskConfig.forceArchitecture" class="select select-bordered">
                      <option value="">使用推荐架构</option>
                      <option value="plan_execute">Plan-and-Execute</option>
                      <option value="rewoo">ReWOO</option>
                      <option value="llm_compiler">LLMCompiler</option>
                    </select>
                  </div>
                  
                  <div class="form-control">
                    <label class="label">
                      <span class="label-text">超时时间 (秒)</span>
                    </label>
                    <input 
                      v-model.number="taskConfig.timeout" 
                      type="number" 
                      class="input input-bordered" 
                      min="60"
                      max="7200"
                    />
                  </div>
                  
                  <div class="form-control">
                    <label class="label">
                      <span class="label-text">额外参数 (JSON格式)</span>
                    </label>
                    <textarea 
                      v-model="taskConfig.extraParams" 
                      class="textarea textarea-bordered h-20 font-mono text-sm" 
                      placeholder='{"key": "value"}'
                    ></textarea>
                  </div>
                </div>
              </div>
            </div>
          </div>
          
          <!-- 配置验证结果 -->
          <div v-if="configValidation.message" :class="[
            'alert mt-4',
            configValidation.valid ? 'alert-success' : 'alert-error'
          ]">
            <i :class="[
              'fas',
              configValidation.valid ? 'fa-check-circle' : 'fa-exclamation-circle'
            ]"></i>
            <span>{{ configValidation.message }}</span>
          </div>
          
          <!-- 操作按钮 -->
          <div class="flex justify-between mt-6">
            <button @click="goToPreviousStep" class="btn btn-ghost">
              <i class="fas fa-arrow-left mr-2"></i>
              返回场景选择
            </button>
            <button 
              @click="startExecution" 
              :disabled="!isConfigValid"
              class="btn btn-primary"
            >
              <i class="fas fa-play mr-2"></i>
              开始执行
            </button>
          </div>
        </div>
      </div>
    </div>
    
    <!-- 步骤3: 执行监控 -->
    <div v-if="currentStep === 3" class="space-y-6">
      <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <!-- 执行状态卡片 -->
        <div class="card bg-base-100 shadow-xl">
          <div class="card-body">
            <h3 class="card-title mb-4">
              <i class="fas fa-tasks mr-2"></i>
              执行状态
            </h3>
            
            <div class="space-y-4">
              <div class="flex justify-between items-center">
                <span class="text-base-content/70">会话ID:</span>
                <span class="font-mono text-sm">{{ executionStatus.sessionId || 'N/A' }}</span>
              </div>
              
              <div class="flex justify-between items-center">
                <span class="text-base-content/70">当前状态:</span>
                <div :class="[
                  'badge',
                  getStatusBadgeClass(executionStatus.status)
                ]">
                  <i :class="getStatusIcon(executionStatus.status)" class="mr-1"></i>
                  {{ executionStatus.status || 'Unknown' }}
                </div>
              </div>
              
              <div class="flex justify-between items-center">
                <span class="text-base-content/70">使用架构:</span>
                <span class="font-semibold">{{ getArchitectureName(executionStatus.architecture) }}</span>
              </div>
              
              <div class="flex justify-between items-center">
                <span class="text-base-content/70">已执行时间:</span>
                <span>{{ formatDuration(executionStatus.elapsedTime) }}</span>
              </div>
              
              <div class="space-y-2">
                <div class="flex justify-between text-sm">
                  <span>执行进度</span>
                  <span>{{ executionStatus.progress || 0 }}%</span>
                </div>
                <div class="progress progress-primary w-full">
                  <div 
                    class="progress-bar" 
                    :style="`width: ${executionStatus.progress || 0}%`"
                  ></div>
                </div>
              </div>
            </div>
            
            <!-- 控制按钮 -->
            <div class="flex gap-2 mt-4">
              <button 
                @click="pauseExecution" 
                :disabled="!canPause"
                class="btn btn-warning btn-sm"
              >
                <i class="fas fa-pause mr-1"></i>
                暂停
              </button>
              <button 
                @click="cancelExecution" 
                :disabled="!canCancel"
                class="btn btn-error btn-sm"
              >
                <i class="fas fa-stop mr-1"></i>
                取消
              </button>
            </div>
          </div>
        </div>
        
        <!-- 实时日志 -->
        <div class="card bg-base-100 shadow-xl">
          <div class="card-body">
            <h3 class="card-title mb-4">
              <i class="fas fa-terminal mr-2"></i>
              执行日志
            </h3>
            
            <div class="console-log-container h-64 overflow-y-auto text-xs font-mono bg-black text-white rounded-lg p-4">
              <div v-for="(log, index) in executionLogs" :key="index" :class="[
                'whitespace-pre-wrap leading-relaxed',
                getLogClass(log.level)
              ]">
                <span class="text-gray-400">[{{ formatTime(log.timestamp) }}]</span>
                <span :class="getLogLevelClass(log.level)">[{{ log.level.toUpperCase() }}]</span>
                <span class="text-white">{{ log.message }}</span>
              </div>
              <div v-if="executionLogs.length === 0" class="text-gray-500 italic">
                等待执行开始...
              </div>
            </div>
            
            <div class="flex justify-between items-center mt-2">
              <button @click="clearLogs" class="btn btn-ghost btn-xs">清空日志</button>
              <button @click="exportLogs" class="btn btn-ghost btn-xs">导出日志</button>
            </div>
          </div>
        </div>
      </div>
      
      <!-- 实时性能监控 -->
      <div class="card bg-base-100 shadow-xl">
        <div class="card-body">
          <h3 class="card-title mb-4">
            <i class="fas fa-chart-line mr-2"></i>
            性能监控
          </h3>
          
          <div class="grid grid-cols-2 md:grid-cols-4 gap-4">
            <div class="stat">
              <div class="stat-title">CPU使用率</div>
              <div class="stat-value text-lg">{{ performanceMetrics.cpuUsage }}%</div>
              <div class="stat-desc">当前值</div>
            </div>
            
            <div class="stat">
              <div class="stat-title">内存使用</div>
              <div class="stat-value text-lg">{{ performanceMetrics.memoryUsage }}MB</div>
              <div class="stat-desc">当前值</div>
            </div>
            
            <div class="stat">
              <div class="stat-title">网络IO</div>
              <div class="stat-value text-lg">{{ performanceMetrics.networkIO }}</div>
              <div class="stat-desc">请求数</div>
            </div>
            
            <div class="stat">
              <div class="stat-title">Token消耗</div>
              <div class="stat-value text-lg">{{ performanceMetrics.tokenUsage }}</div>
              <div class="stat-desc">总计</div>
            </div>
          </div>
        </div>
      </div>
    </div>
    
    <!-- 步骤4: 结果查看 -->
    <div v-if="currentStep === 4" class="space-y-6">
      <div class="card bg-base-100 shadow-xl">
        <div class="card-body">
          <h3 class="card-title mb-4">
            <i class="fas fa-flag-checkered mr-2"></i>
            执行结果
          </h3>
          
          <!-- 结果概要 -->
          <div :class="[
            'alert mb-4',
            executionResult.success ? 'alert-success' : 'alert-error'
          ]">
            <i :class="[
              'fas text-2xl',
              executionResult.success ? 'fa-check-circle' : 'fa-times-circle'
            ]"></i>
            <div>
              <h4 class="font-bold">
                {{ executionResult.success ? '执行成功' : '执行失败' }}
              </h4>
              <div class="text-sm">
                耗时: {{ formatDuration(executionResult.executionTime) }} | 
                架构: {{ getArchitectureName(executionResult.architecture) }}
              </div>
            </div>
          </div>
          
          <!-- 结果数据 -->
          <div v-if="executionResult.data" class="space-y-4">
            <h4 class="font-semibold">结果数据</h4>
            <div class="console-log-container max-h-60 overflow-y-auto">
              <pre class="text-green-400"><code>{{ formatJSON(executionResult.data) }}</code></pre>
            </div>
          </div>
          
          <!-- 错误信息 -->
          <div v-if="!executionResult.success && executionResult.error" class="space-y-4">
            <h4 class="font-semibold text-error">错误信息</h4>
            <div class="alert alert-error">
              <i class="fas fa-exclamation-triangle"></i>
              <span>{{ executionResult.error }}</span>
            </div>
          </div>
          
          <!-- 生成的工作产品 -->
          <div v-if="executionResult.artifacts && executionResult.artifacts.length > 0" class="space-y-4">
            <h4 class="font-semibold">生成的工作产品</h4>
            <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
              <div 
                v-for="artifact in executionResult.artifacts" 
                :key="artifact.name"
                class="card bg-base-200 shadow"
              >
                <div class="card-body p-4">
                  <h5 class="card-title text-sm">{{ artifact.name }}</h5>
                  <p class="text-xs text-base-content/70">{{ artifact.artifact_type }}</p>
                  <div class="card-actions">
                    <button @click="viewArtifact(artifact)" class="btn btn-primary btn-xs">
                      <i class="fas fa-eye mr-1"></i>
                      查看
                    </button>
                    <button @click="downloadArtifact(artifact)" class="btn btn-ghost btn-xs">
                      <i class="fas fa-download mr-1"></i>
                      下载
                    </button>
                  </div>
                </div>
              </div>
            </div>
          </div>
          
          <!-- 操作按钮 -->
          <div class="flex justify-between mt-6">
            <button @click="startNewExecution" class="btn btn-primary">
              <i class="fas fa-plus mr-2"></i>
              开始新任务
            </button>
            <div class="space-x-2">
              <button @click="exportResult" class="btn btn-ghost">
                <i class="fas fa-download mr-2"></i>
                导出结果
              </button>
              <button @click="shareResult" class="btn btn-ghost">
                <i class="fas fa-share mr-2"></i>
                分享结果
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
    
    <!-- 加载状态 -->
    <div v-if="loading" class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
      <div class="card bg-base-100 shadow-xl">
        <div class="card-body text-center">
          <div class="loading loading-spinner loading-lg mb-4"></div>
          <h3 class="text-lg font-semibold">{{ loadingMessage }}</h3>
          <p class="text-base-content/70">{{ loadingDetail }}</p>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import ScenarioSelector from './ScenarioSelector.vue'

// 响应式数据
const currentStep = ref(1)
const selectedScenario = ref('')
const scenarioConfig = ref(null as any)
const loading = ref(false)
const loadingMessage = ref('')
const loadingDetail = ref('')

// 任务配置
const taskConfig = reactive({
  description: '',
  target: '',
  priority: 'normal',
  userId: 'admin',
  forceArchitecture: '',
  timeout: 1800,
  extraParams: '{}'
})

// 配置验证
const configValidation = reactive({
  valid: false,
  message: ''
})

// 执行状态
const executionStatus = reactive({
  sessionId: '',
  status: '',
  architecture: '',
  progress: 0,
  elapsedTime: 0,
  startTime: null as Date | null
})

// 执行日志
const executionLogs = ref<Array<{
  level: string
  message: string
  timestamp: Date
}>>([])

// 性能指标
const performanceMetrics = reactive({
  cpuUsage: 0,
  memoryUsage: 0,
  networkIO: 0,
  tokenUsage: 0
})

// 执行结果
const executionResult = reactive({
  success: false,
  data: null as any,
  error: '',
  executionTime: 0,
  architecture: '',
  artifacts: [] as any[]
})

// 定时器
let statusCheckInterval: NodeJS.Timeout | null = null
let performanceInterval: NodeJS.Timeout | null = null

// 计算属性
const isConfigValid = computed(() => {
  return taskConfig.description.trim().length > 0
})

const canPause = computed(() => {
  return executionStatus.status === 'Running' || executionStatus.status === 'Executing'
})

const canCancel = computed(() => {
  return ['Running', 'Executing', 'Planning', 'Paused'].includes(executionStatus.status)
})

// 方法
const onScenarioConfirmed = (config: any) => {
  scenarioConfig.value = config
  currentStep.value = 2
  
  // 预填配置
  if (config.scenario !== 'custom') {
    taskConfig.description = `执行${getScenarioDisplayName(config.scenario)}任务`
  } else {
    taskConfig.description = config.customConfig.description
  }
}

const goToPreviousStep = () => {
  if (currentStep.value > 1) {
    currentStep.value--
  }
}

const startExecution = async () => {
  if (!isConfigValid.value) return
  
  loading.value = true
  loadingMessage.value = '正在初始化Agent系统...'
  
  try {
    // 首先初始化Agent管理器
    await invoke('initialize_agent_manager')
    
    loadingMessage.value = '正在分发任务...'
    
    // 准备执行请求
    const executionRequest = {
      user_input: taskConfig.description,
      target: taskConfig.target || null,
      context: {
        scenario: scenarioConfig.value?.scenario,
        ...parseExtraParams()
      },
      conversation_id: null,
      user_id: taskConfig.userId,
      architecture: taskConfig.forceArchitecture || scenarioConfig.value?.architecture,
      priority: taskConfig.priority
    }
    
    // 分发任务
    const response = await invoke('dispatch_multi_agent_task', { request: executionRequest })
    
    executionStatus.sessionId = response.session_id
    executionStatus.architecture = response.selected_architecture
    executionStatus.status = 'Running'
    executionStatus.startTime = new Date()
    executionStatus.elapsedTime = 0
    
    // 重置执行结果
    Object.assign(executionResult, {
      success: false,
      data: null,
      error: '',
      executionTime: 0,
      architecture: response.selected_architecture,
      artifacts: []
    })
    
    currentStep.value = 3
    
    // 开始状态监控
    startStatusMonitoring()
    startPerformanceMonitoring()
    
    addLog('info', `任务已开始执行，会话ID: ${executionStatus.sessionId}`)
    addLog('info', `开始时间: ${executionStatus.startTime.toLocaleString()}`)
    addLog('info', `使用架构: ${response.selected_architecture}`)
    
  } catch (error) {
    console.error('Failed to start execution:', error)
    addLog('error', `执行失败: ${error}`)
  } finally {
    loading.value = false
  }
}

const startStatusMonitoring = () => {
  statusCheckInterval = setInterval(async () => {
    try {
      const status = await invoke('get_agent_task_status', { 
        sessionId: executionStatus.sessionId 
      })
      
      if (status) {
        const oldStatus = executionStatus.status
        executionStatus.status = status
        
        if (oldStatus !== status) {
          addLog('info', `状态变更: ${oldStatus} -> ${status}`)
        }
        
        // 模拟进度更新
        if (status === 'Running' || status === 'Executing') {
          executionStatus.progress = Math.min(executionStatus.progress + Math.random() * 10, 95)
        } else if (status === 'Completed') {
          executionStatus.progress = 100
          onExecutionCompleted()
        } else if (status === 'Failed') {
          onExecutionFailed()
        }
      }
      
      // 获取实时日志
      await fetchExecutionLogs()
      
      // 更新已执行时间
      if (executionStatus.startTime) {
        executionStatus.elapsedTime = Date.now() - executionStatus.startTime.getTime()
      }
      
    } catch (error) {
      console.error('Failed to check status:', error)
    }
  }, 2000)
}

const startPerformanceMonitoring = () => {
  performanceInterval = setInterval(() => {
    // 模拟性能数据
    performanceMetrics.cpuUsage = Math.floor(Math.random() * 30 + 20)
    performanceMetrics.memoryUsage = Math.floor(Math.random() * 200 + 300)
    performanceMetrics.networkIO = Math.floor(Math.random() * 10 + 5)
    performanceMetrics.tokenUsage += Math.floor(Math.random() * 50 + 10)
  }, 3000)
}

const onExecutionCompleted = () => {
  stopMonitoring()
  
  // 计算执行时间
  const endTime = new Date()
  const executionTime = executionStatus.startTime ? 
    endTime.getTime() - executionStatus.startTime.getTime() : 
    executionStatus.elapsedTime
  
  // 模拟成功结果
  Object.assign(executionResult, {
    success: true,
    data: {
      result: '任务执行成功',
      metrics: { ...performanceMetrics },
      summary: '所有步骤都已成功完成'
    },
    executionTime: executionTime,
    architecture: executionStatus.architecture,
    artifacts: [
      {
        name: '执行报告',
        artifact_type: 'ScanReport',
        data: { 
          report: 'detailed_report_data',
          startTime: executionStatus.startTime?.toISOString(),
          endTime: endTime.toISOString(),
          duration: executionTime,
          metrics: performanceMetrics
        }
      }
    ]
  })
  
  currentStep.value = 4
  addLog('info', `任务执行完成，总耗时: ${formatDuration(executionTime)}`)
  addLog('info', `结束时间: ${endTime.toLocaleString()}`)
}

const onExecutionFailed = () => {
  stopMonitoring()
  
  // 计算执行时间
  const endTime = new Date()
  const executionTime = executionStatus.startTime ? 
    endTime.getTime() - executionStatus.startTime.getTime() : 
    executionStatus.elapsedTime
  
  Object.assign(executionResult, {
    success: false,
    error: '任务执行过程中遇到错误',
    executionTime: executionTime,
    architecture: executionStatus.architecture
  })
  
  currentStep.value = 4
  addLog('error', `任务执行失败，总耗时: ${formatDuration(executionTime)}`)
  addLog('error', `结束时间: ${endTime.toLocaleString()}`)
}

const stopMonitoring = () => {
  if (statusCheckInterval) {
    clearInterval(statusCheckInterval)
    statusCheckInterval = null
  }
  
  if (performanceInterval) {
    clearInterval(performanceInterval)
    performanceInterval = null
  }
}

const pauseExecution = async () => {
  try {
    // TODO: 实现暂停功能
    addLog('info', '暂停功能开发中...')
  } catch (error) {
    addLog('error', `暂停失败: ${error}`)
  }
}

const cancelExecution = async () => {
  try {
    await invoke('cancel_agent_task', { sessionId: executionStatus.sessionId })
    
    // 计算执行时间
    const endTime = new Date()
    const executionTime = executionStatus.startTime ? 
      endTime.getTime() - executionStatus.startTime.getTime() : 
      executionStatus.elapsedTime
    
    executionStatus.status = 'Cancelled'
    
    // 更新执行结果
    Object.assign(executionResult, {
      success: false,
      error: '任务被用户取消',
      executionTime: executionTime,
      architecture: executionStatus.architecture
    })
    
    stopMonitoring()
    currentStep.value = 4
    addLog('info', `任务已取消，总耗时: ${formatDuration(executionTime)}`)
    addLog('info', `取消时间: ${endTime.toLocaleString()}`)
  } catch (error) {
    addLog('error', `取消失败: ${error}`)
  }
}

const startNewExecution = () => {
  // 重置状态
  currentStep.value = 1
  selectedScenario.value = ''
  scenarioConfig.value = null
  
  Object.assign(taskConfig, {
    description: '',
    target: '',
    priority: 'normal',
    userId: 'admin',
    forceArchitecture: '',
    timeout: 1800,
    extraParams: '{}'
  })
  
  Object.assign(executionStatus, {
    sessionId: '',
    status: '',
    architecture: '',
    progress: 0,
    elapsedTime: 0,
    startTime: null
  })
  
  Object.assign(executionResult, {
    success: false,
    data: null,
    error: '',
    executionTime: 0,
    architecture: '',
    artifacts: []
  })
  
  executionLogs.value = []
  
  Object.assign(performanceMetrics, {
    cpuUsage: 0,
    memoryUsage: 0,
    networkIO: 0,
    tokenUsage: 0
  })
}

// 工具函数
const getScenarioDisplayName = (scenario: string) => {
  const names = {
    'security_scan': '安全扫描',
    'batch_analysis': '批量分析', 
    'api_integration': 'API集成',
    'complex_task': '复杂任务',
    'data_analysis': '数据分析',
    'custom': '自定义场景'
  }
  return names[scenario as keyof typeof names] || scenario
}

const getArchitectureName = (architecture: string) => {
  const names = {
    'plan_execute': 'Plan-and-Execute',
    'rewoo': 'ReWOO', 
    'llm_compiler': 'LLMCompiler'
  }
  return names[architecture as keyof typeof names] || architecture
}

const getStatusBadgeClass = (status: string) => {
  const classes = {
    'Created': 'badge-info',
    'Planning': 'badge-warning',
    'Running': 'badge-info',
    'Executing': 'badge-info',
    'Completed': 'badge-success',
    'Failed': 'badge-error',
    'Cancelled': 'badge-neutral',
    'Paused': 'badge-warning'
  }
  return classes[status as keyof typeof classes] || 'badge-neutral'
}

const getStatusIcon = (status: string) => {
  const icons = {
    'Created': 'fas fa-plus-circle',
    'Planning': 'fas fa-brain',
    'Running': 'fas fa-spinner fa-spin',
    'Executing': 'fas fa-cogs fa-spin',
    'Completed': 'fas fa-check-circle',
    'Failed': 'fas fa-times-circle',
    'Cancelled': 'fas fa-ban',
    'Paused': 'fas fa-pause-circle'
  }
  return icons[status as keyof typeof icons] || 'fas fa-question-circle'
}

const addLog = (level: string, message: string) => {
  executionLogs.value.push({
    level,
    message,
    timestamp: new Date()
  })
  
  // 保持日志数量在合理范围内
  if (executionLogs.value.length > 100) {
    executionLogs.value = executionLogs.value.slice(-50)
  }
}

// 获取执行日志
const fetchExecutionLogs = async () => {
  if (!executionStatus.sessionId) return
  
  try {
    const logs = await invoke('get_agent_task_logs', { 
      sessionId: executionStatus.sessionId 
    })
    
    if (logs && Array.isArray(logs)) {
      // 清空现有日志，替换为后端日志
      executionLogs.value = logs.map((log: any) => ({
        level: log.level.toLowerCase(),
        message: log.message,
        timestamp: new Date(log.timestamp)
      }))
    }
  } catch (error) {
    console.error('Failed to fetch execution logs:', error)
    // 如果获取失败，添加一条错误日志
    addLog('error', '无法获取执行日志')
  }
}

const getLogClass = (level: string) => {
  return ''
}

const getLogLevelClass = (level: string) => {
  const classes = {
    'debug': 'text-gray-500',
    'info': 'text-cyan-400',
    'warn': 'text-yellow-400',
    'error': 'text-red-400'
  }
  return classes[level as keyof typeof classes] || 'text-white'
}

const formatTime = (timestamp: Date) => {
  return timestamp.toLocaleTimeString()
}

const formatDuration = (ms: number) => {
  const seconds = Math.floor(ms / 1000)
  const minutes = Math.floor(seconds / 60)
  const hours = Math.floor(minutes / 60)
  
  if (hours > 0) {
    return `${hours}小时${minutes % 60}分${seconds % 60}秒`
  } else if (minutes > 0) {
    return `${minutes}分${seconds % 60}秒`
  } else {
    return `${seconds}秒`
  }
}

const formatJSON = (data: any) => {
  return JSON.stringify(data, null, 2)
}

const parseExtraParams = () => {
  try {
    return JSON.parse(taskConfig.extraParams || '{}')
  } catch {
    return {}
  }
}

const clearLogs = () => {
  executionLogs.value = []
}

const exportLogs = () => {
  const logText = executionLogs.value
    .map(log => `[${formatTime(log.timestamp)}] [${log.level.toUpperCase()}] ${log.message}`)
    .join('\n')
  
  const blob = new Blob([logText], { type: 'text/plain' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = `execution-logs-${executionStatus.sessionId}.txt`
  a.click()
  URL.revokeObjectURL(url)
}

const exportResult = () => {
  const resultData = {
    sessionId: executionStatus.sessionId,
    scenario: scenarioConfig.value?.scenario,
    architecture: executionResult.architecture,
    success: executionResult.success,
    executionTime: executionResult.executionTime,
    data: executionResult.data,
    error: executionResult.error,
    artifacts: executionResult.artifacts,
    logs: executionLogs.value
  }
  
  const blob = new Blob([JSON.stringify(resultData, null, 2)], { type: 'application/json' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = `execution-result-${executionStatus.sessionId}.json`
  a.click()
  URL.revokeObjectURL(url)
}

const shareResult = () => {
  // TODO: 实现结果分享功能
  addLog('info', '分享功能开发中...')
}

const viewArtifact = (artifact: any) => {
  // 创建查看工作产品的模态框或新窗口
  const content = JSON.stringify(artifact.data || artifact, null, 2)
  
  // 创建一个新的窗口来显示内容
  const newWindow = window.open('', '_blank', 'width=800,height=600,resizable=yes,scrollbars=yes')
  if (newWindow) {
    newWindow.document.write(`
      <!DOCTYPE html>
      <html>
        <head>
          <title>查看工作产品: ${artifact.name}</title>
          <style>
            body {
              font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
              margin: 20px;
              background-color: #f5f5f5;
            }
            .container {
              background: white;
              padding: 20px;
              border-radius: 8px;
              box-shadow: 0 2px 10px rgba(0,0,0,0.1);
            }
            h1 {
              color: #333;
              border-bottom: 2px solid #007acc;
              padding-bottom: 10px;
            }
            .metadata {
              background: #f8f9fa;
              padding: 15px;
              border-radius: 5px;
              margin-bottom: 20px;
              border-left: 4px solid #007acc;
            }
            pre {
              background: #2d3748;
              color: #e2e8f0;
              padding: 20px;
              border-radius: 5px;
              overflow: auto;
              font-size: 14px;
              line-height: 1.5;
            }
            .close-btn {
              background: #007acc;
              color: white;
              border: none;
              padding: 10px 20px;
              border-radius: 5px;
              cursor: pointer;
              margin-top: 20px;
            }
            .close-btn:hover {
              background: #005999;
            }
          </style>
        </head>
        <body>
          <div class="container">
            <h1>🔍 ${artifact.name}</h1>
            <div class="metadata">
              <strong>类型:</strong> ${artifact.artifact_type || '未知'}<br>
              <strong>生成时间:</strong> ${new Date().toLocaleString()}<br>
              <strong>大小:</strong> ${new Blob([content]).size} 字节
            </div>
            <h3>内容:</h3>
            <pre>${content}</pre>
            <button class="close-btn" onclick="window.close()">关闭窗口</button>
          </div>
        </body>
      </html>
    `)
    newWindow.document.close()
  } else {
    // 如果弹窗被阻止，使用alert显示
    alert(`工作产品内容:\n\n${content}`)
  }
  
  addLog('info', `查看工作产品: ${artifact.name}`)
}

const downloadArtifact = (artifact: any) => {
  try {
    // 准备下载内容
    let content: string
    let filename: string
    let mimeType: string
    
    if (artifact.artifact_type === 'ScanReport') {
      // 扫描报告格式化为JSON
      content = JSON.stringify(artifact.data || artifact, null, 2)
      filename = `${artifact.name.replace(/[^a-zA-Z0-9\u4e00-\u9fa5]/g, '_')}_${Date.now()}.json`
      mimeType = 'application/json'
    } else if (artifact.artifact_type === 'TextReport') {
      // 文本报告
      content = typeof artifact.data === 'string' ? artifact.data : JSON.stringify(artifact.data, null, 2)
      filename = `${artifact.name.replace(/[^a-zA-Z0-9\u4e00-\u9fa5]/g, '_')}_${Date.now()}.txt`
      mimeType = 'text/plain'
    } else {
      // 其他类型默认为JSON
      content = JSON.stringify(artifact.data || artifact, null, 2)
      filename = `${artifact.name.replace(/[^a-zA-Z0-9\u4e00-\u9fa5]/g, '_')}_${Date.now()}.json`
      mimeType = 'application/json'
    }
    
    // 创建Blob并下载
    const blob = new Blob([content], { type: `${mimeType};charset=utf-8` })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = filename
    a.style.display = 'none'
    document.body.appendChild(a)
    a.click()
    document.body.removeChild(a)
    URL.revokeObjectURL(url)
    
    addLog('info', `已下载工作产品: ${filename}`)
  } catch (error) {
    console.error('Download failed:', error)
    addLog('error', `下载失败: ${error}`)
  }
}

// 生命周期
onMounted(() => {
  addLog('info', '智能Agent执行控制台已启动')
})

onUnmounted(() => {
  stopMonitoring()
})
</script>

<style scoped>
.steps .step {
  @apply text-xs;
}

.steps .step:before {
  @apply w-6 h-6;
}

.console-log-container {
  background-color: #1a1a1a;
  border: 1px solid #333;
  scrollbar-width: thin;
  scrollbar-color: #666 #333;
}

.console-log-container::-webkit-scrollbar {
  width: 8px;
}

.console-log-container::-webkit-scrollbar-track {
  background: #333;
  border-radius: 4px;
}

.console-log-container::-webkit-scrollbar-thumb {
  background: #666;
  border-radius: 4px;
}

.console-log-container::-webkit-scrollbar-thumb:hover {
  background: #888;
}

.progress-bar {
  @apply bg-primary h-full rounded;
  transition: width 0.3s ease;
}

.stat {
  @apply text-center;
}

.stat-title {
  @apply text-xs text-base-content/60;
}

.stat-value {
  @apply font-bold text-primary;
}

.stat-desc {
  @apply text-xs text-base-content/50;
}
</style>
