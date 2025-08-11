<template>
  <div class="llm-compiler-test-page">
    <!-- 页面头部 -->
    <div class="card bg-base-100 shadow-xl mb-6">
      <div class="card-body">
        <div class="flex justify-between items-center">
          <div>
            <h1 class="text-3xl font-bold">LLMCompiler 引擎测试</h1>
            <p class="text-base-content/70 mt-2">测试和监控 LLMCompiler 引擎的各项功能</p>
          </div>
          <div class="flex gap-2">
            <div class="badge" :class="systemStatus.online ? 'badge-success' : 'badge-error'">
              {{ systemStatus.online ? '系统在线' : '系统离线' }}
            </div>
            <button 
              class="btn btn-primary btn-sm"
              @click="runQuickTest"
              :disabled="quickTestRunning"
            >
              <span v-if="quickTestRunning" class="loading loading-spinner loading-sm"></span>
              {{ quickTestRunning ? '测试中...' : '快速测试' }}
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- 测试控制面板 -->
    <div class="card bg-base-100 shadow-xl mb-6">
      <div class="card-body">
        <h2 class="card-title mb-4">测试控制面板</h2>
        
        <!-- 测试输入区域 -->
        <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
          <!-- 查询输入 -->
          <div>
            <label class="label">
              <span class="label-text font-semibold">测试查询</span>
            </label>
            <textarea 
              v-model="testQuery"
              class="textarea textarea-bordered w-full h-32"
              placeholder="输入要测试的查询内容..."
            ></textarea>
            
            <!-- 预设查询 -->
            <div class="mt-2">
              <label class="label">
                <span class="label-text text-sm">预设查询</span>
              </label>
              <div class="flex flex-wrap gap-2">
                <button 
                  v-for="preset in presetQueries" 
                  :key="preset.id"
                  class="btn btn-xs btn-outline"
                  @click="testQuery = preset.query"
                >
                  {{ preset.name }}
                </button>
              </div>
            </div>
          </div>
          
          <!-- 测试配置 -->
          <div>
            <label class="label">
              <span class="label-text font-semibold">测试配置</span>
            </label>
            
            <div class="space-y-3">
              <!-- 并发度设置 -->
              <div class="form-control">
                <label class="label">
                  <span class="label-text text-sm">最大并发度</span>
                  <span class="label-text-alt">{{ testConfig.maxConcurrency }}</span>
                </label>
                <input 
                  type="range" 
                  min="1" 
                  max="10" 
                  v-model="testConfig.maxConcurrency" 
                  class="range range-primary range-sm"
                />
              </div>
              
              <!-- 超时设置 -->
              <div class="form-control">
                <label class="label">
                  <span class="label-text text-sm">超时时间 (秒)</span>
                </label>
                <input 
                  type="number" 
                  v-model="testConfig.timeout" 
                  class="input input-bordered input-sm"
                  min="10" 
                  max="300"
                />
              </div>
              
              <!-- 调试模式 -->
              <div class="form-control">
                <label class="cursor-pointer label">
                  <span class="label-text text-sm">调试模式</span>
                  <input 
                    type="checkbox" 
                    v-model="testConfig.debugMode" 
                    class="checkbox checkbox-primary checkbox-sm"
                  />
                </label>
              </div>
              
              <!-- 详细日志 -->
              <div class="form-control">
                <label class="cursor-pointer label">
                  <span class="label-text text-sm">详细日志</span>
                  <input 
                    type="checkbox" 
                    v-model="testConfig.verboseLogging" 
                    class="checkbox checkbox-primary checkbox-sm"
                  />
                </label>
              </div>
            </div>
          </div>
        </div>
        
        <!-- 操作按钮 -->
        <div class="flex gap-2 mt-6">
          <button 
            class="btn btn-primary"
            @click="executeTest"
            :disabled="!testQuery.trim() || testExecuting"
          >
            <span v-if="testExecuting" class="loading loading-spinner loading-sm"></span>
            <svg v-else xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M14.828 14.828a4 4 0 01-5.656 0M9 10h1m4 0h1" />
            </svg>
            {{ testExecuting ? '执行中...' : '执行测试' }}
          </button>
          
          <button 
            class="btn btn-warning"
            @click="stopTest"
            :disabled="!testExecuting"
          >
            <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 10l4 4 4-4" />
            </svg>
            停止测试
          </button>
          
          <button 
            class="btn btn-outline"
            @click="clearResults"
          >
            <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
            </svg>
            清空结果
          </button>
          
          <button 
            class="btn btn-outline"
            @click="exportResults"
            :disabled="testResults.length === 0"
          >
            <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 10v6m0 0l-3-3m3 3l3-3m2 8H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
            </svg>
            导出结果
          </button>
        </div>
      </div>
    </div>

    <!-- 测试结果展示 -->
    <div v-if="currentTestResult" class="card bg-base-100 shadow-xl mb-6">
      <div class="card-body">
        <div class="flex justify-between items-center mb-4">
          <h2 class="card-title">当前测试结果</h2>
          <div class="badge" :class="getResultStatusBadge(currentTestResult.status)">
            {{ currentTestResult.status }}
          </div>
        </div>
        
        <!-- 执行统计 -->
        <div class="grid grid-cols-1 md:grid-cols-4 gap-4 mb-4">
          <div class="stat bg-base-200 rounded-lg">
            <div class="stat-title">执行时间</div>
            <div class="stat-value text-lg">{{ formatDuration(currentTestResult.executionTime) }}</div>
          </div>
          <div class="stat bg-base-200 rounded-lg">
            <div class="stat-title">任务总数</div>
            <div class="stat-value text-lg">{{ currentTestResult.totalTasks }}</div>
          </div>
          <div class="stat bg-base-200 rounded-lg">
            <div class="stat-title">成功率</div>
            <div class="stat-value text-lg text-success">{{ currentTestResult.successRate }}%</div>
          </div>
          <div class="stat bg-base-200 rounded-lg">
            <div class="stat-title">平均并发</div>
            <div class="stat-value text-lg">{{ currentTestResult.avgConcurrency.toFixed(1) }}</div>
          </div>
        </div>
        
        <!-- 响应内容 -->
        <div class="collapse collapse-arrow bg-base-200">
          <input type="checkbox" checked /> 
          <div class="collapse-title font-medium">AI 响应内容</div>
          <div class="collapse-content">
            <div class="prose max-w-none">
              <div v-html="formatResponse(currentTestResult.response)"></div>
            </div>
          </div>
        </div>
        
        <!-- 详细日志 -->
        <div v-if="testConfig.verboseLogging && currentTestResult.logs" class="collapse collapse-arrow bg-base-200 mt-2">
          <input type="checkbox" /> 
          <div class="collapse-title font-medium">详细执行日志</div>
          <div class="collapse-content">
            <pre class="text-xs bg-base-300 p-4 rounded overflow-x-auto">{{ currentTestResult.logs }}</pre>
          </div>
        </div>
      </div>
    </div>

    <!-- 组件集成区域 -->
    <div class="tabs tabs-lifted mb-6">
      <input type="radio" name="component_tabs" role="tab" class="tab" aria-label="控制台" v-model="activeTab" value="console" />
      <div role="tabpanel" class="tab-content bg-base-100 border-base-300 rounded-box p-6">
        <LLMCompilerConsole v-if="activeTab === 'console'" />
      </div>

      <input type="radio" name="component_tabs" role="tab" class="tab" aria-label="DAG视图" v-model="activeTab" value="dag" />
      <div role="tabpanel" class="tab-content bg-base-100 border-base-300 rounded-box p-6">
        <LLMCompilerDAGView v-if="activeTab === 'dag'" />
      </div>

      <input type="radio" name="component_tabs" role="tab" class="tab" aria-label="执行监控" v-model="activeTab" value="monitor" />
      <div role="tabpanel" class="tab-content bg-base-100 border-base-300 rounded-box p-6">
        <LLMCompilerExecutionMonitor v-if="activeTab === 'monitor'" />
      </div>
    </div>

    <!-- 历史测试记录 -->
    <div class="card bg-base-100 shadow-xl">
      <div class="card-body">
        <div class="flex justify-between items-center mb-4">
          <h2 class="card-title">测试历史</h2>
          <div class="flex gap-2">
            <button 
              class="btn btn-sm btn-outline"
              @click="refreshHistory"
            >
              刷新
            </button>
            <button 
              class="btn btn-sm btn-outline"
              @click="clearHistory"
              :disabled="testResults.length === 0"
            >
              清空历史
            </button>
          </div>
        </div>
        
        <div class="overflow-x-auto">
          <table class="table table-zebra">
            <thead>
              <tr>
                <th>时间</th>
                <th>查询</th>
                <th>状态</th>
                <th>执行时间</th>
                <th>任务数</th>
                <th>成功率</th>
                <th>操作</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="result in testResults" :key="result.id">
                <td class="text-sm">{{ formatTime(result.timestamp) }}</td>
                <td class="max-w-xs truncate">{{ result.query }}</td>
                <td>
                  <div class="badge badge-sm" :class="getResultStatusBadge(result.status)">
                    {{ result.status }}
                  </div>
                </td>
                <td>{{ formatDuration(result.executionTime) }}</td>
                <td>{{ result.totalTasks }}</td>
                <td>{{ result.successRate }}%</td>
                <td>
                  <div class="flex gap-1">
                    <button 
                      class="btn btn-xs btn-outline"
                      @click="viewResult(result)"
                    >
                      查看
                    </button>
                    <button 
                      class="btn btn-xs btn-outline"
                      @click="rerunTest(result)"
                    >
                      重跑
                    </button>
                  </div>
                </td>
              </tr>
            </tbody>
          </table>
          
          <!-- 空状态 -->
          <div v-if="testResults.length === 0" class="text-center py-8 text-base-content/60">
            <svg xmlns="http://www.w3.org/2000/svg" class="h-12 w-12 mx-auto mb-2 opacity-50" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5H7a2 2 0 00-2 2v10a2 2 0 002 2h8a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2" />
            </svg>
            <p>暂无测试记录</p>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted, onUnmounted } from 'vue'
import LLMCompilerConsole from '../components/LLMCompilerConsole.vue'
import LLMCompilerDAGView from '../components/LLMCompilerDAGView.vue'
import LLMCompilerExecutionMonitor from '../components/LLMCompilerExecutionMonitor.vue'

// 类型定义
interface TestResult {
  id: number
  timestamp: number
  query: string
  status: 'success' | 'failed' | 'error'
  executionTime: number
  totalTasks: number
  successRate: number
  avgConcurrency: number
  response: string
  logs: string | null
}

interface TestConfig {
  maxConcurrency: number
  timeout: number
  debugMode: boolean
  verboseLogging: boolean
}

interface SystemStatus {
  online: boolean
  lastCheck: number
}

// 模拟invoke函数
const invoke = async (command: string, args?: any): Promise<any> => {
  // 模拟API调用
  await new Promise(resolve => setTimeout(resolve, 1000 + Math.random() * 2000))
  
  switch (command) {
    case 'execute_llm_compiler_workflow':
      return {
        success: Math.random() > 0.2,
        total_tasks: Math.floor(Math.random() * 10) + 1,
        success_rate: Math.floor(Math.random() * 100),
        avg_concurrency: Math.random() * 5,
        response: `模拟响应: ${args.query}\n\n执行结果:\n- 任务已完成\n- 分析报告已生成\n- 建议采取相应措施`,
        logs: args.config.verboseLogging ? '详细执行日志...' : null
      }
    case 'get_llm_compiler_status':
      return { online: Math.random() > 0.1 }
    case 'get_llm_compiler_test_history':
      return []
    default:
      return {}
  }
}

// 响应式数据
const activeTab = ref('console')
const testQuery = ref('')
const testExecuting = ref(false)
const quickTestRunning = ref(false)
const currentTestResult = ref<TestResult | null>(null)
const testResults = ref<TestResult[]>([])

// 系统状态
const systemStatus = reactive<SystemStatus>({
  online: true,
  lastCheck: Date.now()
})

// 测试配置
const testConfig = reactive<TestConfig>({
  maxConcurrency: 3,
  timeout: 60,
  debugMode: false,
  verboseLogging: false
})

// 预设查询
const presetQueries = [
  {
    id: 1,
    name: '简单查询',
    query: '分析当前系统的安全状态'
  },
  {
    id: 2,
    name: '复杂分析',
    query: '执行全面的漏洞扫描并生成详细报告，包括风险评估和修复建议'
  },
  {
    id: 3,
    name: '性能测试',
    query: '测试系统在高并发情况下的性能表现，分析资源使用情况'
  },
  {
    id: 4,
    name: '多任务协调',
    query: '同时执行端口扫描、服务识别和漏洞检测，并整合结果'
  }
]

// 执行测试
const executeTest = async () => {
  if (!testQuery.value.trim()) return
  
  testExecuting.value = true
  const startTime = Date.now()
  
  try {
    const result = await invoke('execute_llm_compiler_workflow', {
      query: testQuery.value,
      config: testConfig
    })
    
    const testResult: TestResult = {
      id: Date.now(),
      timestamp: startTime,
      query: testQuery.value,
      status: result.success ? 'success' : 'failed',
      executionTime: Date.now() - startTime,
      totalTasks: result.total_tasks || 0,
      successRate: result.success_rate || 0,
      avgConcurrency: result.avg_concurrency || 0,
      response: result.response || '',
      logs: testConfig.verboseLogging ? result.logs : null
    }
    
    currentTestResult.value = testResult
    testResults.value.unshift(testResult)
    
    // 限制历史记录数量
    if (testResults.value.length > 50) {
      testResults.value = testResults.value.slice(0, 50)
    }
    
  } catch (error: any) {
    console.error('测试执行失败:', error)
    
    const testResult: TestResult = {
      id: Date.now(),
      timestamp: startTime,
      query: testQuery.value,
      status: 'error',
      executionTime: Date.now() - startTime,
      totalTasks: 0,
      successRate: 0,
      avgConcurrency: 0,
      response: `执行错误: ${error?.message || error}`,
      logs: null
    }
    
    currentTestResult.value = testResult
    testResults.value.unshift(testResult)
  } finally {
    testExecuting.value = false
  }
}

// 停止测试
const stopTest = async () => {
  try {
    await invoke('stop_llm_compiler_execution')
    testExecuting.value = false
  } catch (error) {
    console.error('停止测试失败:', error)
  }
}

// 快速测试
const runQuickTest = async () => {
  quickTestRunning.value = true
  const originalQuery = testQuery.value
  
  testQuery.value = '执行系统健康检查'
  await executeTest()
  
  testQuery.value = originalQuery
  quickTestRunning.value = false
}

// 清空结果
const clearResults = () => {
  currentTestResult.value = null
  testResults.value = []
}

// 导出结果
const exportResults = () => {
  const data = JSON.stringify(testResults.value, null, 2)
  const blob = new Blob([data], { type: 'application/json' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = `llm-compiler-test-results-${new Date().toISOString().split('T')[0]}.json`
  a.click()
  URL.revokeObjectURL(url)
}

// 查看结果
const viewResult = (result: TestResult) => {
  currentTestResult.value = result
}

// 重新运行测试
const rerunTest = (result: TestResult) => {
  testQuery.value = result.query
  executeTest()
}

// 刷新历史
const refreshHistory = async () => {
  try {
    const history = await invoke('get_llm_compiler_test_history')
    testResults.value = history || []
  } catch (error) {
    console.error('刷新历史失败:', error)
  }
}

// 清空历史
const clearHistory = async () => {
  try {
    await invoke('clear_llm_compiler_test_history')
    testResults.value = []
    currentTestResult.value = null
  } catch (error) {
    console.error('清空历史失败:', error)
  }
}

// 工具函数
const formatDuration = (ms: number): string => {
  if (ms < 1000) return `${ms}ms`
  if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`
  return `${Math.floor(ms / 60000)}m ${Math.floor((ms % 60000) / 1000)}s`
}

const formatTime = (timestamp: number): string => {
  return new Date(timestamp).toLocaleString('zh-CN')
}

const formatResponse = (response: string): string => {
  if (!response) return ''
  return response.replace(/\n/g, '<br>').replace(/\*\*(.*?)\*\*/g, '<strong>$1</strong>')
}

const getResultStatusBadge = (status: string): string => {
  switch (status) {
    case 'success': return 'badge-success'
    case 'failed': return 'badge-warning'
    case 'error': return 'badge-error'
    default: return 'badge-neutral'
  }
}

// 系统状态检查
const checkSystemStatus = async () => {
  try {
    const status = await invoke('get_llm_compiler_status')
    systemStatus.online = status.online
    systemStatus.lastCheck = Date.now()
  } catch (error) {
    systemStatus.online = false
    systemStatus.lastCheck = Date.now()
  }
}

// 生命周期
let statusCheckInterval: ReturnType<typeof setInterval> | undefined

onMounted(() => {
  checkSystemStatus()
  refreshHistory()
  
  // 定期检查系统状态
  statusCheckInterval = setInterval(checkSystemStatus, 30000)
})

onUnmounted(() => {
  if (statusCheckInterval) {
    clearInterval(statusCheckInterval)
  }
})
</script>

<style scoped>
.llm-compiler-test-page {
  @apply p-6 max-w-7xl mx-auto;
}

.tab-content {
  min-height: 500px;
}

.prose {
  @apply text-base-content;
}

.prose strong {
  @apply font-semibold text-primary;
}
</style>