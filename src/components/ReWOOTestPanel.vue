<template>
  <div class="rewoo-test-panel p-6 space-y-6">
    <!-- 页面标题 -->    
    <div class="flex justify-between items-center">
      <div>
        <h1 class="text-3xl font-bold">ReWOO 架构测试</h1>
        <p class="text-base-content/70 mt-1">测试 ReWOO (Reasoning without Observation) 引擎的完整执行流程</p>
      </div>
      <div class="flex gap-2">
        <button 
          class="btn btn-outline btn-sm"
          @click="refreshEngineStatus"
          :disabled="loading"
        >
          <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
          </svg>
          刷新状态
        </button>
        <button 
          class="btn btn-error btn-sm"
          @click="clearAllResults"
          :disabled="loading || testResults.length === 0"
        >
          <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
          </svg>
          清除结果
        </button>
      </div>
    </div>

    <!-- 引擎状态卡片 -->
    <div class="card bg-base-100 shadow-xl">
      <div class="card-body">
        <div class="flex justify-between items-center mb-4">
          <h2 class="card-title">引擎状态</h2>
          <div class="badge" :class="engineStatus.active ? 'badge-success' : 'badge-warning'">
            {{ engineStatus.active ? '运行中' : '待机' }}
          </div>
        </div>
        
        <div class="grid grid-cols-1 md:grid-cols-4 gap-4">
          <div class="stat bg-base-200 rounded-lg">
            <div class="stat-title">引擎版本</div>
            <div class="stat-value text-lg">{{ engineStatus.version }}</div>
            <div class="stat-desc">{{ engineStatus.ready ? '就绪' : '未就绪' }}</div>
          </div>
          <div class="stat bg-base-200 rounded-lg">
            <div class="stat-title">运行时间</div>
            <div class="stat-value text-lg">{{ formatUptime(engineStatus.uptime_seconds) }}</div>
            <div class="stat-desc">秒</div>
          </div>
          <div class="stat bg-base-200 rounded-lg">
            <div class="stat-title">总会话数</div>
            <div class="stat-value text-lg">{{ engineStatus.total_sessions }}</div>
            <div class="stat-desc">历史记录</div>
          </div>
          <div class="stat bg-base-200 rounded-lg">
            <div class="stat-title">活跃会话</div>
            <div class="stat-value text-lg text-primary">{{ engineStatus.active_sessions }}</div>
            <div class="stat-desc">当前运行</div>
          </div>
        </div>
      </div>
    </div>

    <!-- 测试配置区域 -->
    <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
      <!-- 预定义测试 -->
      <div class="card bg-base-100 shadow-xl">
        <div class="card-body">
          <h3 class="card-title mb-4">预定义测试</h3>
          
          <div class="space-y-3">
            <div 
              v-for="config in predefinedConfigs" 
              :key="config.name"
              class="border rounded-lg p-4 hover:bg-base-200 transition-colors cursor-pointer"
              :class="{ 'border-primary bg-primary/10': selectedConfig?.name === config.name }"
              @click="selectConfig(config)"
            >
              <div class="flex justify-between items-start mb-2">
                <h4 class="font-semibold">{{ config.name }}</h4>
                <div class="badge badge-sm badge-outline">{{ config.timeout_seconds }}s</div>
              </div>
              <p class="text-sm text-base-content/70 mb-2">{{ config.task }}</p>
              <div class="flex flex-wrap gap-1">
                <div 
                  v-for="tool in config.expected_tools" 
                  :key="tool"
                  class="badge badge-xs badge-primary"
                >
                  {{ tool }}
                </div>
              </div>
            </div>
          </div>
          
          <div class="card-actions justify-end mt-4">
            <button 
              class="btn btn-primary"
              @click="runSelectedTest"
              :disabled="!selectedConfig || loading"
            >
              <span v-if="loading" class="loading loading-spinner loading-sm"></span>
              <svg v-else xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M14.828 14.828a4 4 0 01-5.656 0M9 10h1m4 0h1" />
              </svg>
              {{ loading ? '执行中...' : '运行测试' }}
            </button>
          </div>
        </div>
      </div>

      <!-- 自定义测试 -->
      <div class="card bg-base-100 shadow-xl">
        <div class="card-body">
          <h3 class="card-title mb-4">自定义测试</h3>
          
          <form @submit.prevent="runCustomTest" class="space-y-4">
            <div class="form-control">
              <label class="label">
                <span class="label-text">测试名称</span>
              </label>
              <input 
                type="text" 
                class="input input-bordered" 
                v-model="customTest.name"
                placeholder="输入测试名称"
                required
              >
            </div>
            
            <div class="form-control">
              <label class="label">
                <span class="label-text">任务描述</span>
              </label>
              <textarea 
                class="textarea textarea-bordered h-24" 
                v-model="customTest.task"
                placeholder="描述要测试的任务，例如：搜索关于人工智能的最新信息并总结"
                required
              ></textarea>
            </div>
            
            <div class="form-control">
              <label class="label">
                <span class="label-text">超时时间 (秒)</span>
              </label>
              <input 
                type="number" 
                class="input input-bordered" 
                v-model.number="customTest.timeout_seconds"
                min="10"
                max="300"
                required
              >
            </div>
            
            <div class="form-control">
              <label class="label">
                <span class="label-text">预期工具 (可选)</span>
              </label>
              <div class="flex flex-wrap gap-2 mb-2">
                <div 
                  v-for="tool in customTest.expected_tools" 
                  :key="tool"
                  class="badge badge-primary gap-1"
                >
                  {{ tool }}
                  <button 
                    type="button"
                    class="btn btn-xs btn-circle btn-ghost"
                    @click="removeExpectedTool(tool)"
                  >
                    ×
                  </button>
                </div>
              </div>
              <div class="join">
                <input 
                  type="text" 
                  class="input input-bordered join-item" 
                  v-model="newToolName"
                  placeholder="工具名称"
                  @keyup.enter="addExpectedTool"
                >
                <button 
                  type="button"
                  class="btn btn-outline join-item"
                  @click="addExpectedTool"
                >
                  添加
                </button>
              </div>
            </div>
            
            <div class="card-actions justify-end">
              <button 
                type="submit"
                class="btn btn-secondary"
                :disabled="loading"
              >
                <span v-if="loading" class="loading loading-spinner loading-sm"></span>
                <svg v-else xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z" />
                </svg>
                {{ loading ? '执行中...' : '运行自定义测试' }}
              </button>
            </div>
          </form>
        </div>
      </div>
    </div>

    <!-- 可用工具展示 -->
    <div class="card bg-base-100 shadow-xl">
      <div class="card-body">
        <div class="flex justify-between items-center mb-4">
          <h3 class="card-title">可用工具</h3>
          <button 
            class="btn btn-outline btn-sm"
            @click="refreshAvailableTools"
            :disabled="loading"
          >
            刷新
          </button>
        </div>
        
        <div class="grid grid-cols-2 md:grid-cols-4 lg:grid-cols-6 gap-2">
          <div 
            v-for="tool in availableTools" 
            :key="tool"
            class="badge badge-outline p-3 cursor-pointer hover:badge-primary transition-colors"
            @click="testTool(tool)"
          >
            {{ tool }}
          </div>
        </div>
        
        <div v-if="availableTools.length === 0" class="text-center py-8 text-base-content/60">
          <svg xmlns="http://www.w3.org/2000/svg" class="h-12 w-12 mx-auto mb-2 opacity-50" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13.828 10.172a4 4 0 00-5.656 0l-4 4a4 4 0 105.656 5.656l1.102-1.101m-.758-4.899a4 4 0 005.656 0l4-4a4 4 0 00-5.656-5.656l-1.1 1.1" />
          </svg>
          <p>暂无可用工具</p>
        </div>
      </div>
    </div>

    <!-- 测试结果 -->
    <div class="card bg-base-100 shadow-xl">
      <div class="card-body">
        <div class="flex justify-between items-center mb-4">
          <h3 class="card-title">测试结果</h3>
          <div class="badge badge-info">{{ testResults.length }} 个结果</div>
        </div>
        
        <div v-if="testResults.length > 0" class="space-y-4">
          <div 
            v-for="result in testResults" 
            :key="result.id"
            class="border rounded-lg p-4"
            :class="{
              'border-success bg-success/10': result.success,
              'border-error bg-error/10': !result.success
            }"
          >
            <div class="flex justify-between items-start mb-3">
              <div>
                <h4 class="font-semibold flex items-center gap-2">
                  {{ result.test_name }}
                  <div class="badge badge-sm" :class="result.success ? 'badge-success' : 'badge-error'">
                    {{ result.success ? '成功' : '失败' }}
                  </div>
                </h4>
                <p class="text-sm text-base-content/70">{{ result.task }}</p>
              </div>
              <div class="text-right text-sm text-base-content/60">
                <div>开始: {{ formatTime(result.started_at) }}</div>
                <div v-if="result.completed_at">完成: {{ formatTime(result.completed_at) }}</div>
              </div>
            </div>
            
            <!-- 执行指标 -->
            <div class="grid grid-cols-2 md:grid-cols-4 gap-4 mb-3">
              <div class="stat bg-base-200 rounded p-2">
                <div class="stat-title text-xs">执行时间</div>
                <div class="stat-value text-sm">{{ result.metrics.total_time_ms }}ms</div>
              </div>
              <div class="stat bg-base-200 rounded p-2">
                <div class="stat-title text-xs">工具调用</div>
                <div class="stat-value text-sm">{{ result.metrics.tool_calls }}</div>
              </div>
              <div class="stat bg-base-200 rounded p-2">
                <div class="stat-title text-xs">成功调用</div>
                <div class="stat-value text-sm text-success">{{ result.metrics.successful_tool_calls }}</div>
              </div>
              <div class="stat bg-base-200 rounded p-2">
                <div class="stat-title text-xs">Token消耗</div>
                <div class="stat-value text-sm">{{ result.metrics.total_tokens }}</div>
              </div>
            </div>
            
            <!-- 执行日志 -->
            <div class="collapse collapse-arrow bg-base-200 mb-2">
              <input type="checkbox" /> 
              <div class="collapse-title text-sm font-medium">
                <div class="flex items-center gap-2">
                  <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
                  </svg>
                  执行日志 ({{ result.logs?.length || 0 }} 条)
                </div>
              </div>
              <div class="collapse-content">
                <div class="bg-base-100 rounded p-3 mt-2 max-h-96 overflow-y-auto">
                  <div v-if="result.logs && result.logs.length > 0" class="space-y-2">
                    <div 
                      v-for="(log, index) in result.logs" 
                      :key="index"
                      class="flex items-start gap-3 p-2 rounded border-l-4"
                      :class="{
                        'border-blue-500 bg-blue-50': log.level === 'INFO',
                        'border-green-500 bg-green-50': log.level === 'DEBUG',
                        'border-yellow-500 bg-yellow-50': log.level === 'WARN',
                        'border-red-500 bg-red-50': log.level === 'ERROR'
                      }"
                    >
                      <!-- 组件图标 -->
                      <div class="flex-shrink-0 mt-1">
                        <div 
                          class="w-6 h-6 rounded-full flex items-center justify-center text-xs font-bold text-white"
                          :class="{
                            'bg-purple-500': log.component === 'planner',
                            'bg-blue-500': log.component === 'worker',
                            'bg-green-500': log.component === 'solver',
                            'bg-gray-500': log.component === 'system'
                          }"
                        >
                          {{ log.component === 'planner' ? 'P' : log.component === 'worker' ? 'W' : log.component === 'solver' ? 'S' : 'SYS' }}
                        </div>
                      </div>
                      
                      <!-- 日志内容 -->
                      <div class="flex-1 min-w-0">
                        <div class="flex items-center gap-2 mb-1">
                          <span class="text-xs font-medium text-gray-600">{{ formatTime(log.timestamp) }}</span>
                          <span 
                            class="text-xs px-2 py-1 rounded font-medium"
                            :class="{
                              'bg-blue-100 text-blue-800': log.level === 'INFO',
                              'bg-green-100 text-green-800': log.level === 'DEBUG',
                              'bg-yellow-100 text-yellow-800': log.level === 'WARN',
                              'bg-red-100 text-red-800': log.level === 'ERROR'
                            }"
                          >
                            {{ log.level }}
                          </span>
                          <span class="text-xs px-2 py-1 rounded bg-gray-100 text-gray-700 font-medium">
                            {{ log.component.toUpperCase() }}
                          </span>
                        </div>
                        <p class="text-sm text-gray-800 mb-1">{{ log.message }}</p>
                        <div v-if="log.details" class="mt-2">
                          <details class="text-xs">
                            <summary class="cursor-pointer text-gray-600 hover:text-gray-800">查看详细信息</summary>
                            <pre class="mt-1 p-2 bg-gray-50 rounded text-xs overflow-x-auto">{{ JSON.stringify(log.details, null, 2) }}</pre>
                          </details>
                        </div>
                      </div>
                    </div>
                  </div>
                  <div v-else class="text-center py-4 text-gray-500">
                    <p>暂无执行日志</p>
                  </div>
                </div>
              </div>
            </div>
            
            <!-- 结果内容 -->
            <div class="collapse collapse-arrow bg-base-200">
              <input type="checkbox" /> 
              <div class="collapse-title text-sm font-medium">
                <div class="flex items-center gap-2">
                  <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
                  </svg>
                  {{ result.success ? '执行结果' : '错误信息' }}
                </div>
              </div>
              <div class="collapse-content">
                <div class="bg-base-100 rounded p-3 mt-2">
                  <pre class="text-sm whitespace-pre-wrap">{{ result.success ? result.result : result.error }}</pre>
                </div>
              </div>
            </div>
          </div>
        </div>
        
        <div v-else class="text-center py-8 text-base-content/60">
          <svg xmlns="http://www.w3.org/2000/svg" class="h-12 w-12 mx-auto mb-2 opacity-50" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
          </svg>
          <p>暂无测试结果</p>
          <p class="text-sm">运行测试后将显示结果</p>
        </div>
      </div>
    </div>

    <!-- 工具测试模态框 -->
    <div v-if="showToolTestModal" class="modal modal-open">
      <div class="modal-box">
        <h3 class="font-bold text-lg mb-4">测试工具: {{ selectedTool }}</h3>
        
        <form @submit.prevent="executeToolTest">
          <div class="form-control mb-4">
            <label class="label">
              <span class="label-text">工具参数</span>
            </label>
            <textarea 
              class="textarea textarea-bordered" 
              v-model="toolTestArgs"
              placeholder="输入工具参数，例如：搜索关键词"
              required
            ></textarea>
          </div>
          
          <div v-if="toolTestResult" class="bg-base-200 rounded p-3 mb-4">
            <h4 class="font-semibold mb-2">执行结果:</h4>
            <pre class="text-sm whitespace-pre-wrap">{{ toolTestResult }}</pre>
          </div>
          
          <div class="modal-action">
            <button type="button" class="btn" @click="closeToolTestModal">关闭</button>
            <button type="submit" class="btn btn-primary" :disabled="toolTestLoading">
              <span v-if="toolTestLoading" class="loading loading-spinner loading-sm"></span>
              {{ toolTestLoading ? '执行中...' : '执行测试' }}
            </button>
          </div>
        </form>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useToast } from '../composables/useToast'
import type {
  EngineStatusInfo,
  TestConfig,
  TestResult,
  CustomTestConfig,
  defaultReWOOConfig
} from '../types/rewoo-test'

const toast = useToast()

// 响应式数据
const loading = ref(false)
const engineStatus = ref<EngineStatusInfo>({
  active: false,
  ready: false,
  version: '1.0.0',
  uptime_seconds: 0,
  total_sessions: 0,
  active_sessions: 0
})

const predefinedConfigs = ref<TestConfig[]>([])
const selectedConfig = ref<TestConfig | null>(null)
const availableTools = ref<string[]>([])
const testResults = ref<TestResult[]>([])

// 自定义测试
const customTest = reactive<CustomTestConfig>({
  name: '1',
  task: '请扫描一下mgtv.com有哪些子域名',
  timeout_seconds: 300,
  expected_tools: [],
  rewoo_config: {
    planner: {
      model_name: 'deepseek-chat',
      temperature: 0.0,
      max_tokens: 4000,
      max_steps: 10
    },
    worker: {
      timeout_seconds: 300,
      max_retries: 3,
      enable_parallel: false
    },
    solver: {
      model_name: 'deepseek-chat',
      temperature: 0.0,
      max_tokens: 2000
    }
  }
})

const newToolName = ref('')

// 工具测试
const showToolTestModal = ref(false)
const selectedTool = ref('')
const toolTestArgs = ref('')
const toolTestResult = ref('')
const toolTestLoading = ref(false)

// 方法
const refreshEngineStatus = async () => {
  try {
    const status = await invoke<EngineStatusInfo>('get_rewoo_engine_status')
    engineStatus.value = status
  } catch (error) {
    console.error('获取引擎状态失败:', error)
    toast.error('获取引擎状态失败')
  }
}

const loadPredefinedConfigs = async () => {
  try {
    const configs = await invoke<TestConfig[]>('get_predefined_test_configs')
    predefinedConfigs.value = configs
  } catch (error) {
    console.error('加载预定义配置失败:', error)
    toast.error('加载预定义配置失败')
  }
}

const refreshAvailableTools = async () => {
  try {
    const tools = await invoke<string[]>('get_available_tools')
    availableTools.value = tools
  } catch (error) {
    console.error('获取可用工具失败:', error)
    toast.error('获取可用工具失败')
  }
}

const loadTestResults = async () => {
  try {
    const results = await invoke<TestResult[]>('get_all_test_results')
    testResults.value = results
  } catch (error) {
    console.error('加载测试结果失败:', error)
    toast.error('加载测试结果失败')
  }
}

const selectConfig = (config: TestConfig) => {
  selectedConfig.value = config
}

const runSelectedTest = async () => {
  if (!selectedConfig.value) return
  
  loading.value = true
  try {
    const testId = await invoke<string>('execute_rewoo_test', {
      testConfig: selectedConfig.value
    })
    
    toast.success(`测试已启动，ID: ${testId}`)
    
    // 等待一段时间后刷新结果
    setTimeout(() => {
      loadTestResults()
    }, 2000)
    
  } catch (error) {
    console.error('运行测试失败:', error)
    toast.error('运行测试失败: ' + error)
  } finally {
    loading.value = false
  }
}

const runCustomTest = async () => {
  loading.value = true
  try {
    const testId = await invoke<string>('execute_rewoo_test', {
      testConfig: customTest
    })
    
    toast.success(`自定义测试已启动，ID: ${testId}`)
    
    // 等待一段时间后刷新结果
    setTimeout(() => {
      loadTestResults()
    }, 2000)
    
    // 重置表单
    customTest.name = ''
    customTest.task = ''
    customTest.timeout_seconds = 60
    customTest.expected_tools = []
    
  } catch (error) {
    console.error('运行自定义测试失败:', error)
    toast.error('运行自定义测试失败: ' + error)
  } finally {
    loading.value = false
  }
}

const addExpectedTool = () => {
  if (newToolName.value.trim() && !customTest.expected_tools.includes(newToolName.value.trim())) {
    customTest.expected_tools.push(newToolName.value.trim())
    newToolName.value = ''
  }
}

const removeExpectedTool = (tool: string) => {
  const index = customTest.expected_tools.indexOf(tool)
  if (index > -1) {
    customTest.expected_tools.splice(index, 1)
  }
}

const testTool = (tool: string) => {
  selectedTool.value = tool
  toolTestArgs.value = ''
  toolTestResult.value = ''
  showToolTestModal.value = true
}

const executeToolTest = async () => {
  toolTestLoading.value = true
  try {
    const result = await invoke<string>('simulate_tool_execution', {
      toolName: selectedTool.value,
      args: toolTestArgs.value
    })
    toolTestResult.value = result
  } catch (error) {
    console.error('工具测试失败:', error)
    toolTestResult.value = '错误: ' + error
  } finally {
    toolTestLoading.value = false
  }
}

const closeToolTestModal = () => {
  showToolTestModal.value = false
  selectedTool.value = ''
  toolTestArgs.value = ''
  toolTestResult.value = ''
}

const clearAllResults = async () => {
  try {
    await invoke<void>('clear_test_results')
    testResults.value = []
    toast.success('测试结果已清除')
  } catch (error) {
    console.error('清除结果失败:', error)
    toast.error('清除结果失败: ' + error)
  }
}

// 工具函数
const formatUptime = (seconds: number) => {
  if (seconds < 60) return `${seconds}s`
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m ${seconds % 60}s`
  return `${Math.floor(seconds / 3600)}h ${Math.floor((seconds % 3600) / 60)}m`
}

const formatTime = (timestamp: any) => {
  if (typeof timestamp === 'object' && timestamp.secs_since_epoch) {
    return new Date(timestamp.secs_since_epoch * 1000).toLocaleString()
  }
  return new Date(timestamp).toLocaleString()
}

// 生命周期
onMounted(async () => {
  await Promise.all([
    refreshEngineStatus(),
    loadPredefinedConfigs(),
    refreshAvailableTools(),
    loadTestResults()
  ])
})
</script>

<style scoped>
.rewoo-test-panel {
  min-height: 100vh;
  background: linear-gradient(135deg, hsl(var(--b1)) 0%, hsl(var(--b2)) 100%);
}

.stat {
  padding: 1rem;
}

.stat-title {
  font-size: 0.75rem;
  opacity: 0.7;
}

.stat-value {
  font-size: 1.25rem;
  font-weight: bold;
}

.badge {
  transition: all 0.2s ease;
}

.badge:hover {
  transform: translateY(-1px);
}

.card {
  transition: all 0.3s ease;
}

.card:hover {
  transform: translateY(-2px);
}

.collapse-title {
  font-size: 0.875rem;
}

pre {
  font-family: 'Monaco', 'Menlo', 'Ubuntu Mono', monospace;
  font-size: 0.75rem;
  line-height: 1.4;
}
</style>