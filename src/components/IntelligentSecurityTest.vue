<template>
  <div class="container mx-auto p-6">
    <!-- 页面标题 -->
    <div class="mb-8">
      <h1 class="text-3xl font-bold text-base-content mb-2">
        🤖 智能安全测试
      </h1>
      <p class="text-base-content/70">
        基于 LLM + MCP + Agent + 内置工具的智能化安全测试平台
      </p>
    </div>

    <!-- 测试配置卡片 -->
    <div class="card bg-base-100 shadow-xl mb-6">
      <div class="card-body">
        <h2 class="card-title text-xl mb-4">
          <i class="fas fa-cog mr-2"></i>
          测试配置
        </h2>
        
        <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
          <!-- 目标配置 -->
          <div class="form-control">
            <label class="label">
              <span class="label-text font-medium">测试目标</span>
            </label>
            <input 
              v-model="testConfig.target" 
              type="text" 
              placeholder="example.com 或 192.168.1.1" 
              class="input input-bordered w-full"
              :disabled="isRunning"
            />
          </div>

          <!-- 测试类型 -->
          <div class="form-control">
            <label class="label">
              <span class="label-text font-medium">测试类型</span>
            </label>
            <select 
              v-model="testConfig.testType" 
              class="select select-bordered w-full"
              :disabled="isRunning"
            >
              <option value="comprehensive">全面测试</option>
              <option value="reconnaissance">信息收集</option>
              <option value="vulnerability_scan">漏洞扫描</option>
              <option value="custom">自定义</option>
            </select>
          </div>

          <!-- 风险等级 -->
          <div class="form-control">
            <label class="label">
              <span class="label-text font-medium">风险容忍度</span>
            </label>
            <select 
              v-model="testConfig.riskTolerance" 
              class="select select-bordered w-full"
              :disabled="isRunning"
            >
              <option value="Conservative">保守</option>
              <option value="Balanced">平衡</option>
              <option value="Aggressive">激进</option>
            </select>
          </div>

          <!-- LLM 模型 -->
          <div class="form-control">
            <label class="label">
              <span class="label-text font-medium">LLM 模型</span>
            </label>
            <select 
              v-model="testConfig.llmModel" 
              class="select select-bordered w-full"
              :disabled="isRunning"
            >
              <option value="gpt-4">GPT-4</option>
              <option value="gpt-3.5-turbo">GPT-3.5 Turbo</option>
              <option value="claude-3">Claude-3</option>
            </select>
          </div>
        </div>

        <!-- 工具选择 -->
        <div class="mt-6">
          <h3 class="text-lg font-medium mb-3">启用的工具</h3>
          <div class="grid grid-cols-2 md:grid-cols-4 gap-3">
            <label v-for="tool in availableTools" :key="tool.name" class="cursor-pointer">
              <input 
                type="checkbox" 
                :value="tool.name" 
                v-model="testConfig.enabledTools" 
                class="checkbox checkbox-primary mr-2"
                :disabled="isRunning"
              />
              <span class="label-text">{{ tool.label }}</span>
            </label>
          </div>
        </div>

        <!-- 操作按钮 -->
        <div class="card-actions justify-end mt-6">
          <button 
            @click="startTest" 
            class="btn btn-primary"
            :disabled="!testConfig.target || isRunning"
          >
            <i class="fas fa-play mr-2"></i>
            {{ isRunning ? '测试进行中...' : '开始测试' }}
          </button>
          
          <button 
            v-if="isRunning" 
            @click="stopTest" 
            class="btn btn-error"
          >
            <i class="fas fa-stop mr-2"></i>
            停止测试
          </button>
        </div>
      </div>
    </div>

    <!-- 实时状态 -->
    <div v-if="isRunning || testResult" class="card bg-base-100 shadow-xl mb-6">
      <div class="card-body">
        <h2 class="card-title text-xl mb-4">
          <i class="fas fa-chart-line mr-2"></i>
          测试状态
        </h2>

        <!-- 进度条 -->
        <div v-if="isRunning" class="mb-4">
          <div class="flex justify-between text-sm mb-2">
            <span>当前阶段: {{ currentStage }}</span>
            <span>{{ Math.round(progress) }}%</span>
          </div>
          <progress class="progress progress-primary w-full" :value="progress" max="100"></progress>
        </div>

        <!-- 实时日志 -->
        <div class="bg-base-200 rounded-lg p-4 max-h-60 overflow-y-auto">
          <div v-for="(log, index) in logs" :key="index" class="text-sm mb-1">
            <span class="text-base-content/50">{{ log.timestamp }}</span>
            <span :class="getLogClass(log.level)" class="ml-2">
              {{ log.message }}
            </span>
          </div>
        </div>
      </div>
    </div>

    <!-- 测试结果 -->
    <div v-if="testResult" class="card bg-base-100 shadow-xl">
      <div class="card-body">
        <h2 class="card-title text-xl mb-4">
          <i class="fas fa-chart-bar mr-2"></i>
          测试结果
        </h2>

        <!-- 结果摘要 -->
        <div class="grid grid-cols-1 md:grid-cols-4 gap-4 mb-6">
          <div class="stat bg-base-200 rounded-lg">
            <div class="stat-title">测试状态</div>
            <div class="stat-value text-lg" :class="getStatusClass(testResult.status)">
              {{ getStatusText(testResult.status) }}
            </div>
          </div>
          
          <div class="stat bg-base-200 rounded-lg">
            <div class="stat-title">置信度</div>
            <div class="stat-value text-lg text-primary">
              {{ Math.round(testResult.confidence_score * 100) }}%
            </div>
          </div>
          
          <div class="stat bg-base-200 rounded-lg">
            <div class="stat-title">执行时间</div>
            <div class="stat-value text-lg text-info">
              {{ formatDuration(testResult.execution_time) }}
            </div>
          </div>
          
          <div class="stat bg-base-200 rounded-lg">
            <div class="stat-title">发现问题</div>
            <div class="stat-value text-lg text-warning">
              {{ testResult.vulnerabilities_count || 0 }}
            </div>
          </div>
        </div>

        <!-- 详细结果 -->
        <div class="tabs tabs-bordered mb-4">
          <a 
            v-for="tab in resultTabs" 
            :key="tab.key"
            class="tab"
            :class="{ 'tab-active': activeTab === tab.key }"
            @click="activeTab = tab.key"
          >
            <i :class="tab.icon" class="mr-2"></i>
            {{ tab.label }}
          </a>
        </div>

        <!-- 标签页内容 -->
        <div class="tab-content">
          <!-- LLM 分析结果 -->
          <div v-if="activeTab === 'llm'" class="space-y-4">
            <div class="bg-base-200 rounded-lg p-4">
              <h4 class="font-medium mb-2">🧠 LLM 智能分析</h4>
              <div class="prose max-w-none">
                <div v-html="formatLLMAnalysis(testResult.llm_analysis)"></div>
              </div>
            </div>
          </div>

          <!-- MCP 工具结果 -->
          <div v-if="activeTab === 'mcp'" class="space-y-4">
            <div v-for="(result, tool) in testResult.mcp_results" :key="tool" class="bg-base-200 rounded-lg p-4">
              <h4 class="font-medium mb-2">
                🔧 {{ tool }}
              </h4>
              <pre class="text-sm overflow-x-auto">{{ JSON.stringify(result, null, 2) }}</pre>
            </div>
          </div>

          <!-- 内置工具结果 -->
          <div v-if="activeTab === 'builtin'" class="space-y-4">
            <div v-for="(result, tool) in testResult.builtin_results" :key="tool" class="bg-base-200 rounded-lg p-4">
              <h4 class="font-medium mb-2">
                🛠️ {{ tool }}
              </h4>
              <pre class="text-sm overflow-x-auto">{{ JSON.stringify(result, null, 2) }}</pre>
            </div>
          </div>

          <!-- 漏洞详情 -->
          <div v-if="activeTab === 'vulnerabilities'" class="space-y-4">
            <div v-for="vuln in testResult.vulnerabilities" :key="vuln.id" class="alert" :class="getVulnAlertClass(vuln.severity)">
              <div>
                <h4 class="font-medium">{{ vuln.title }}</h4>
                <p class="text-sm mt-1">{{ vuln.description }}</p>
                <div class="mt-2">
                  <span class="badge" :class="getVulnBadgeClass(vuln.severity)">{{ vuln.severity }}</span>
                  <span class="badge badge-outline ml-2">{{ vuln.category }}</span>
                </div>
              </div>
            </div>
          </div>

          <!-- 综合报告 -->
          <div v-if="activeTab === 'report'" class="space-y-4">
            <div class="bg-base-200 rounded-lg p-4">
              <h4 class="font-medium mb-2">📊 综合安全评估报告</h4>
              <div class="prose max-w-none">
                <div v-html="formatReport(testResult.comprehensive_report)"></div>
              </div>
            </div>
          </div>
        </div>

        <!-- 操作按钮 -->
        <div class="card-actions justify-end mt-6">
          <button @click="downloadReport" class="btn btn-outline">
            <i class="fas fa-download mr-2"></i>
            下载报告
          </button>
          
          <button @click="shareResult" class="btn btn-outline">
            <i class="fas fa-share mr-2"></i>
            分享结果
          </button>
          
          <button @click="startNewTest" class="btn btn-primary">
            <i class="fas fa-plus mr-2"></i>
            新建测试
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useToast } from '@/composables/useToast'

const toast = useToast()

// 响应式数据
const isRunning = ref(false)
const progress = ref(0)
const currentStage = ref('')
const logs = ref<Array<{timestamp: string, level: string, message: string}>>([])
const testResult = ref<any>(null)
const activeTab = ref('llm')

// 测试配置
const testConfig = reactive({
  target: '',
  testType: 'comprehensive',
  riskTolerance: 'Balanced',
  llmModel: 'gpt-4',
  enabledTools: ['subfinder', 'nmap', 'nuclei', 'shodan']
})

// 可用工具
const availableTools = ref([
  { name: 'subfinder', label: 'Subfinder (子域名发现)' },
  { name: 'nmap', label: 'Nmap (端口扫描)' },
  { name: 'nuclei', label: 'Nuclei (漏洞扫描)' },
  { name: 'shodan', label: 'Shodan (网络空间搜索)' },
  { name: 'whois', label: 'WHOIS (域名信息)' },
  { name: 'web_scanner', label: 'Web Scanner (Web应用扫描)' }
])

// 结果标签页
const resultTabs = [
  { key: 'llm', label: 'LLM 分析', icon: 'fas fa-brain' },
  { key: 'mcp', label: 'MCP 工具', icon: 'fas fa-plug' },
  { key: 'builtin', label: '内置工具', icon: 'fas fa-tools' },
  { key: 'vulnerabilities', label: '漏洞详情', icon: 'fas fa-bug' },
  { key: 'report', label: '综合报告', icon: 'fas fa-file-alt' }
]

// 轮询定时器
let pollingTimer: NodeJS.Timeout | null = null

// 开始测试
const startTest = async () => {
  try {
    isRunning.value = true
    progress.value = 0
    currentStage.value = '初始化...'
    logs.value = []
    testResult.value = null
    
    addLog('info', '开始智能安全测试...')
    
    // 调用后端 API 开始测试
    const sessionId = await invoke('start_intelligent_security_test', {
      config: {
        target: testConfig.target,
        test_type: testConfig.testType,
        risk_tolerance: testConfig.riskTolerance,
        llm_model: testConfig.llmModel,
        enabled_tools: testConfig.enabledTools
      }
    })
    
    addLog('success', `测试会话已创建: ${sessionId}`)
    
    // 开始轮询状态
    startPolling(sessionId as string)
    
  } catch (error: any) {
    console.error('启动测试失败:', error)
    toast.error(`启动测试失败: ${error.message}`)
    isRunning.value = false
  }
}

// 停止测试
const stopTest = async () => {
  try {
    await invoke('stop_intelligent_security_test')
    addLog('warning', '测试已被用户停止')
    isRunning.value = false
    stopPolling()
  } catch (error: any) {
    console.error('停止测试失败:', error)
    toast.error(`停止测试失败: ${error.message}`)
  }
}

// 开始轮询
const startPolling = (sessionId: string) => {
  pollingTimer = setInterval(async () => {
    try {
      const status = await invoke('get_test_status', { sessionId })
      updateStatus(status as any)
    } catch (error) {
      console.error('获取状态失败:', error)
    }
  }, 2000) // 每2秒轮询一次
}

// 停止轮询
const stopPolling = () => {
  if (pollingTimer) {
    clearInterval(pollingTimer)
    pollingTimer = null
  }
}

// 更新状态
const updateStatus = (status: any) => {
  progress.value = status.progress || 0
  currentStage.value = status.current_stage || ''
  
  // 添加新日志
  if (status.new_logs) {
    status.new_logs.forEach((log: any) => {
      addLog(log.level, log.message)
    })
  }
  
  // 检查是否完成
  if (status.status === 'completed' || status.status === 'failed') {
    isRunning.value = false
    stopPolling()
    
    if (status.result) {
      testResult.value = status.result
      addLog('success', '测试完成！')
      toast.success('智能安全测试已完成')
    } else {
      addLog('error', '测试失败')
      toast.error('测试执行失败')
    }
  }
}

// 添加日志
const addLog = (level: string, message: string) => {
  logs.value.push({
    timestamp: new Date().toLocaleTimeString(),
    level,
    message
  })
  
  // 限制日志数量
  if (logs.value.length > 100) {
    logs.value = logs.value.slice(-100)
  }
}

// 工具函数
const getLogClass = (level: string) => {
  const classes = {
    info: 'text-info',
    success: 'text-success',
    warning: 'text-warning',
    error: 'text-error'
  }
  return classes[level as keyof typeof classes] || 'text-base-content'
}

const getStatusClass = (status: string) => {
  const classes = {
    completed: 'text-success',
    failed: 'text-error',
    running: 'text-info'
  }
  return classes[status as keyof typeof classes] || 'text-base-content'
}

const getStatusText = (status: string) => {
  const texts = {
    completed: '已完成',
    failed: '失败',
    running: '进行中'
  }
  return texts[status as keyof typeof texts] || status
}

const formatDuration = (seconds: number) => {
  const minutes = Math.floor(seconds / 60)
  const remainingSeconds = seconds % 60
  return `${minutes}:${remainingSeconds.toString().padStart(2, '0')}`
}

const getVulnAlertClass = (severity: string) => {
  const classes = {
    critical: 'alert-error',
    high: 'alert-warning',
    medium: 'alert-info',
    low: 'alert-success'
  }
  return classes[severity as keyof typeof classes] || 'alert-info'
}

const getVulnBadgeClass = (severity: string) => {
  const classes = {
    critical: 'badge-error',
    high: 'badge-warning',
    medium: 'badge-info',
    low: 'badge-success'
  }
  return classes[severity as keyof typeof classes] || 'badge-info'
}

const formatLLMAnalysis = (analysis: string) => {
  // 简单的 Markdown 到 HTML 转换
  return analysis
    ?.replace(/\*\*(.*?)\*\*/g, '<strong>$1</strong>')
    ?.replace(/\*(.*?)\*/g, '<em>$1</em>')
    ?.replace(/\n/g, '<br>')
    || '暂无分析结果'
}

const formatReport = (report: any) => {
  if (typeof report === 'string') {
    return formatLLMAnalysis(report)
  }
  return JSON.stringify(report, null, 2)
}

// 操作函数
const downloadReport = () => {
  const data = JSON.stringify(testResult.value, null, 2)
  const blob = new Blob([data], { type: 'application/json' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = `security_test_report_${testConfig.target}_${new Date().toISOString().split('T')[0]}.json`
  a.click()
  URL.revokeObjectURL(url)
}

const shareResult = () => {
  // 实现分享功能
  toast.info('分享功能开发中...')
}

const startNewTest = () => {
  testResult.value = null
  testConfig.target = ''
  logs.value = []
  progress.value = 0
}

// 生命周期
onMounted(() => {
  // 组件挂载时的初始化
})

onUnmounted(() => {
  // 清理定时器
  stopPolling()
})
</script>

<style scoped>
.tab-content {
  min-height: 300px;
}

.prose {
  max-width: none;
}

.stat {
  padding: 1rem;
}

pre {
  background: rgba(0, 0, 0, 0.1);
  padding: 1rem;
  border-radius: 0.5rem;
  font-size: 0.875rem;
  line-height: 1.4;
}
</style>