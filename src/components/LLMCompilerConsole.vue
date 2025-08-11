<template>
  <div class="llm-compiler-console">
    <!-- 控制台头部 -->
    <div class="card bg-base-100 shadow-xl mb-6">
      <div class="card-body">
        <div class="flex justify-between items-center mb-4">
          <div>
            <h1 class="card-title text-2xl">LLMCompiler 控制台</h1>
            <p class="text-base-content/70">并行任务编译与执行引擎</p>
          </div>
          <div class="flex gap-2">
            <div class="badge" :class="systemStatus.active ? 'badge-success' : 'badge-warning'">
              {{ systemStatus.active ? '系统运行中' : '系统待机' }}
            </div>
            <button 
              class="btn btn-primary btn-sm"
              @click="startNewSession"
              :disabled="systemStatus.active"
            >
              <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M14.828 14.828a4 4 0 01-5.656 0M9 10h1m4 0h1" />
              </svg>
              启动新会话
            </button>
            <button 
              class="btn btn-outline btn-sm"
              @click="showSettings = !showSettings"
            >
              <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
              </svg>
              设置
            </button>
          </div>
        </div>
        
        <!-- 系统概览 -->
        <div class="grid grid-cols-1 md:grid-cols-5 gap-4">
          <div class="stat bg-base-200 rounded-lg">
            <div class="stat-title">活跃会话</div>
            <div class="stat-value text-lg">{{ systemOverview.activeSessions }}</div>
          </div>
          <div class="stat bg-base-200 rounded-lg">
            <div class="stat-title">并发任务</div>
            <div class="stat-value text-lg text-info">{{ systemOverview.concurrentTasks }}</div>
          </div>
          <div class="stat bg-base-200 rounded-lg">
            <div class="stat-title">今日完成</div>
            <div class="stat-value text-lg text-success">{{ systemOverview.completedToday }}</div>
          </div>
          <div class="stat bg-base-200 rounded-lg">
            <div class="stat-title">系统负载</div>
            <div class="stat-value text-lg" :class="getLoadClass(systemOverview.systemLoad)">{{ Math.round(systemOverview.systemLoad * 100) }}%</div>
          </div>
          <div class="stat bg-base-200 rounded-lg">
            <div class="stat-title">平均响应</div>
            <div class="stat-value text-lg">{{ formatDuration(systemOverview.avgResponseTime) }}</div>
          </div>
        </div>
      </div>
    </div>
    
    <!-- 主要内容区域 -->
    <div class="grid grid-cols-1 xl:grid-cols-3 gap-6">
      <!-- 左侧：会话管理 -->
      <div class="xl:col-span-1">
        <div class="card bg-base-100 shadow-xl">
          <div class="card-body">
            <div class="flex justify-between items-center mb-4">
              <h3 class="card-title">会话管理</h3>
              <button 
                class="btn btn-sm btn-outline"
                @click="refreshSessions"
                :disabled="refreshingSessions"
              >
                <span v-if="refreshingSessions" class="loading loading-spinner loading-sm"></span>
                刷新
              </button>
            </div>
            
            <!-- 会话列表 -->
            <div class="space-y-2 max-h-96 overflow-y-auto">
              <div 
                v-for="session in sessions" 
                :key="session.id"
                class="p-3 rounded-lg border cursor-pointer transition-all duration-200"
                :class="[
                  selectedSessionId === session.id ? 'border-primary bg-primary/10' : 'border-base-300 hover:border-base-400',
                  session.status === 'active' ? 'border-l-4 border-l-success' : session.status === 'error' ? 'border-l-4 border-l-error' : ''
                ]"
                @click="selectSession(session.id)"
              >
                <div class="flex justify-between items-start mb-2">
                  <div>
                    <div class="font-semibold text-sm">{{ session.name }}</div>
                    <div class="text-xs text-base-content/60">{{ session.id.substring(0, 8) }}</div>
                  </div>
                  <div class="badge badge-sm" :class="getSessionStatusBadge(session.status)">{{ session.status }}</div>
                </div>
                
                <div class="text-xs text-base-content/70 mb-2">{{ session.description }}</div>
                
                <div class="flex justify-between items-center text-xs">
                  <span>{{ formatTime(session.createdAt) }}</span>
                  <span>{{ session.taskCount }} 任务</span>
                </div>
                
                <!-- 进度条 -->
                <div class="mt-2">
                  <progress 
                    class="progress progress-primary w-full h-1" 
                    :value="session.progress" 
                    max="100"
                  ></progress>
                </div>
              </div>
            </div>
            
            <!-- 空状态 -->
            <div v-if="sessions.length === 0" class="text-center py-8 text-base-content/60">
              <svg xmlns="http://www.w3.org/2000/svg" class="h-12 w-12 mx-auto mb-2 opacity-50" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z" />
              </svg>
              <p>暂无活跃会话</p>
              <p class="text-sm">点击"启动新会话"开始</p>
            </div>
          </div>
        </div>
      </div>
      
      <!-- 右侧：主要视图 -->
      <div class="xl:col-span-2">
        <!-- 视图切换标签 -->
        <div class="tabs tabs-boxed mb-4">
          <a 
            class="tab"
            :class="{ 'tab-active': activeView === 'dag' }"
            @click="activeView = 'dag'"
          >
            <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4 mr-2" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13.828 10.172a4 4 0 00-5.656 0l-4 4a4 4 0 105.656 5.656l1.102-1.101m-.758-4.899a4 4 0 005.656 0l4-4a4 4 0 00-5.656-5.656l-1.1 1.1" />
            </svg>
            DAG 可视化
          </a>
          <a 
            class="tab"
            :class="{ 'tab-active': activeView === 'monitor' }"
            @click="activeView = 'monitor'"
          >
            <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4 mr-2" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z" />
            </svg>
            执行监控
          </a>
          <a 
            class="tab"
            :class="{ 'tab-active': activeView === 'logs' }"
            @click="activeView = 'logs'"
          >
            <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4 mr-2" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
            </svg>
            系统日志
          </a>
        </div>
        
        <!-- 视图内容 -->
        <div class="view-content">
          <!-- DAG 可视化视图 -->
          <div v-if="activeView === 'dag'">
            <LLMCompilerDAGView 
              :session-id="selectedSessionId"
              @dag-started="handleDAGStarted"
              @dag-paused="handleDAGPaused"
              @dag-stopped="handleDAGStopped"
              @task-cancelled="handleTaskCancelled"
              @node-selected="handleNodeSelected"
            />
          </div>
          
          <!-- 执行监控视图 -->
          <div v-if="activeView === 'monitor'">
            <LLMCompilerExecutionMonitor 
              :session-id="selectedSessionId"
              @task-cancelled="handleTaskCancelled"
              @task-retried="handleTaskRetried"
              @alert-dismissed="handleAlertDismissed"
            />
          </div>
          
          <!-- 系统日志视图 -->
          <div v-if="activeView === 'logs'">
            <div class="card bg-base-100 shadow-xl">
              <div class="card-body">
                <div class="flex justify-between items-center mb-4">
                  <h3 class="card-title">系统日志</h3>
                  <div class="flex gap-2">
                    <!-- 日志级别过滤 -->
                    <div class="dropdown dropdown-end">
                      <div tabindex="0" role="button" class="btn btn-sm btn-outline">
                        <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 4a1 1 0 011-1h16a1 1 0 011 1v2.586a1 1 0 01-.293.707l-6.414 6.414a1 1 0 00-.293.707V17l-4 4v-6.586a1 1 0 00-.293-.707L3.293 7.293A1 1 0 013 6.586V4z" />
                        </svg>
                        {{ logLevelFilter }}
                      </div>
                      <ul tabindex="0" class="dropdown-content z-[1] menu p-2 shadow bg-base-100 rounded-box w-32">
                        <li><a @click="logLevelFilter = 'ALL'">全部</a></li>
                        <li><a @click="logLevelFilter = 'ERROR'">错误</a></li>
                        <li><a @click="logLevelFilter = 'WARN'">警告</a></li>
                        <li><a @click="logLevelFilter = 'INFO'">信息</a></li>
                        <li><a @click="logLevelFilter = 'DEBUG'">调试</a></li>
                      </ul>
                    </div>
                    
                    <button 
                      class="btn btn-sm btn-outline"
                      @click="clearLogs"
                    >
                      清空日志
                    </button>
                    
                    <button 
                      class="btn btn-sm btn-outline"
                      @click="exportLogs"
                    >
                      导出日志
                    </button>
                  </div>
                </div>
                
                <!-- 日志内容 -->
                <div class="h-96 overflow-y-auto bg-base-200 rounded-lg p-4 font-mono text-sm" ref="logContainer">
                  <div 
                    v-for="log in filteredLogs" 
                    :key="log.id"
                    class="mb-1 flex"
                    :class="getLogClass(log.level)"
                  >
                    <span class="text-base-content/60 mr-2">{{ formatLogTime(log.timestamp) }}</span>
                    <span class="font-semibold mr-2" :class="getLogLevelClass(log.level)">[{{ log.level }}]</span>
                    <span class="text-base-content/80 mr-2">{{ log.component }}:</span>
                    <span>{{ log.message }}</span>
                  </div>
                  
                  <!-- 空状态 -->
                  <div v-if="filteredLogs.length === 0" class="text-center py-8 text-base-content/60">
                    <p>暂无日志记录</p>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
    
    <!-- 设置模态框 -->
    <dialog ref="settingsModal" class="modal" :class="{ 'modal-open': showSettings }">
      <div class="modal-box">
        <form method="dialog">
          <button class="btn btn-sm btn-circle btn-ghost absolute right-2 top-2" @click="showSettings = false">✕</button>
        </form>
        
        <h3 class="font-bold text-lg mb-4">LLMCompiler 设置</h3>
        
        <div class="space-y-4">
          <!-- 并发设置 -->
          <div>
            <label class="label">
              <span class="label-text">最大并发任务数</span>
            </label>
            <input 
              type="range" 
              min="1" 
              max="16" 
              v-model="settings.maxConcurrentTasks" 
              class="range range-primary" 
            />
            <div class="w-full flex justify-between text-xs px-2">
              <span>1</span>
              <span>4</span>
              <span>8</span>
              <span>12</span>
              <span>16</span>
            </div>
            <div class="text-sm text-base-content/70 mt-1">当前值: {{ settings.maxConcurrentTasks }}</div>
          </div>
          
          <!-- 超时设置 -->
          <div>
            <label class="label">
              <span class="label-text">任务超时时间 (秒)</span>
            </label>
            <input 
              type="number" 
              min="10" 
              max="3600" 
              v-model="settings.taskTimeout" 
              class="input input-bordered w-full" 
            />
          </div>
          
          <!-- 重试设置 -->
          <div>
            <label class="label">
              <span class="label-text">最大重试次数</span>
            </label>
            <input 
              type="number" 
              min="0" 
              max="10" 
              v-model="settings.maxRetries" 
              class="input input-bordered w-full" 
            />
          </div>
          
          <!-- 日志设置 -->
          <div>
            <label class="label">
              <span class="label-text">日志级别</span>
            </label>
            <select v-model="settings.logLevel" class="select select-bordered w-full">
              <option value="DEBUG">调试 (DEBUG)</option>
              <option value="INFO">信息 (INFO)</option>
              <option value="WARN">警告 (WARN)</option>
              <option value="ERROR">错误 (ERROR)</option>
            </select>
          </div>
          
          <!-- 自动刷新 -->
          <div class="form-control">
            <label class="label cursor-pointer">
              <span class="label-text">启用自动刷新</span>
              <input type="checkbox" v-model="settings.autoRefresh" class="checkbox checkbox-primary" />
            </label>
          </div>
          
          <div v-if="settings.autoRefresh">
            <label class="label">
              <span class="label-text">刷新间隔 (秒)</span>
            </label>
            <input 
              type="number" 
              min="1" 
              max="60" 
              v-model="settings.refreshInterval" 
              class="input input-bordered w-full" 
            />
          </div>
        </div>
        
        <div class="modal-action">
          <button class="btn btn-primary" @click="saveSettings">保存设置</button>
          <button class="btn btn-outline" @click="resetSettings">重置默认</button>
        </div>
      </div>
    </dialog>
    
    <!-- 新会话模态框 -->
    <dialog ref="newSessionModal" class="modal" :class="{ 'modal-open': showNewSession }">
      <div class="modal-box">
        <form method="dialog">
          <button class="btn btn-sm btn-circle btn-ghost absolute right-2 top-2" @click="showNewSession = false">✕</button>
        </form>
        
        <h3 class="font-bold text-lg mb-4">创建新会话</h3>
        
        <div class="space-y-4">
          <div>
            <label class="label">
              <span class="label-text">会话名称</span>
            </label>
            <input 
              type="text" 
              v-model="newSessionForm.name" 
              placeholder="输入会话名称" 
              class="input input-bordered w-full" 
            />
          </div>
          
          <div>
            <label class="label">
              <span class="label-text">会话描述</span>
            </label>
            <textarea 
              v-model="newSessionForm.description" 
              placeholder="输入会话描述" 
              class="textarea textarea-bordered w-full" 
              rows="3"
            ></textarea>
          </div>
          
          <div>
            <label class="label">
              <span class="label-text">初始任务</span>
            </label>
            <textarea 
              v-model="newSessionForm.initialTask" 
              placeholder="输入要执行的任务描述" 
              class="textarea textarea-bordered w-full" 
              rows="4"
            ></textarea>
          </div>
        </div>
        
        <div class="modal-action">
          <button 
            class="btn btn-primary" 
            @click="createSession"
            :disabled="!newSessionForm.name || !newSessionForm.initialTask"
          >
            创建会话
          </button>
          <button class="btn btn-outline" @click="showNewSession = false">取消</button>
        </div>
      </div>
    </dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted, onUnmounted } from 'vue'
import LLMCompilerDAGView from './LLMCompilerDAGView.vue'
import LLMCompilerExecutionMonitor from './LLMCompilerExecutionMonitor.vue'

// 类型定义
interface SystemStatus {
  active: boolean
  version: string
  uptime: number
}

interface SystemOverview {
  activeSessions: number
  concurrentTasks: number
  completedToday: number
  systemLoad: number
  avgResponseTime: number
}

interface Session {
  id: string
  name: string
  description: string
  status: 'active' | 'paused' | 'completed' | 'error'
  createdAt: Date
  taskCount: number
  progress: number
}

interface LogEntry {
  id: string
  timestamp: Date
  level: 'DEBUG' | 'INFO' | 'WARN' | 'ERROR'
  component: string
  message: string
}

interface Settings {
  maxConcurrentTasks: number
  taskTimeout: number
  maxRetries: number
  logLevel: string
  autoRefresh: boolean
  refreshInterval: number
}

interface NewSessionForm {
  name: string
  description: string
  initialTask: string
}

// 响应式数据
const systemStatus = ref<SystemStatus>({
  active: false,
  version: '1.0.0',
  uptime: 0
})

const systemOverview = ref<SystemOverview>({
  activeSessions: 0,
  concurrentTasks: 0,
  completedToday: 0,
  systemLoad: 0,
  avgResponseTime: 0
})

const sessions = ref<Session[]>([])
const logs = ref<LogEntry[]>([])

const settings = reactive<Settings>({
  maxConcurrentTasks: 8,
  taskTimeout: 300,
  maxRetries: 3,
  logLevel: 'INFO',
  autoRefresh: true,
  refreshInterval: 5
})

const newSessionForm = reactive<NewSessionForm>({
  name: '',
  description: '',
  initialTask: ''
})

// UI状态
const activeView = ref('dag')
const selectedSessionId = ref<string | null>(null)
const refreshingSessions = ref(false)
const showSettings = ref(false)
const showNewSession = ref(false)
const logLevelFilter = ref('ALL')

// 模态框引用
const settingsModal = ref<HTMLDialogElement>()
const newSessionModal = ref<HTMLDialogElement>()
const logContainer = ref<HTMLElement>()

// 计算属性
const filteredLogs = computed(() => {
  if (logLevelFilter.value === 'ALL') {
    return logs.value
  }
  return logs.value.filter(log => log.level === logLevelFilter.value)
})

// 方法
const startNewSession = () => {
  showNewSession.value = true
}

const createSession = async () => {
  try {
    const newSession: Session = {
      id: `session_${Date.now()}`,
      name: newSessionForm.name,
      description: newSessionForm.description,
      status: 'active',
      createdAt: new Date(),
      taskCount: 0,
      progress: 0
    }
    
    sessions.value.unshift(newSession)
    selectedSessionId.value = newSession.id
    systemStatus.value.active = true
    
    // 重置表单
    newSessionForm.name = ''
    newSessionForm.description = ''
    newSessionForm.initialTask = ''
    
    showNewSession.value = false
    
    // 添加日志
    addLog('INFO', 'SessionManager', `新会话已创建: ${newSession.name}`)
    
  } catch (error) {
    console.error('创建会话失败:', error)
    addLog('ERROR', 'SessionManager', `创建会话失败: ${error}`)
  }
}

const selectSession = (sessionId: string) => {
  selectedSessionId.value = sessionId
}

const refreshSessions = async () => {
  refreshingSessions.value = true
  try {
    // 调用后端API刷新会话数据
    await new Promise(resolve => setTimeout(resolve, 1000)) // 模拟API调用
  } finally {
    refreshingSessions.value = false
  }
}

const saveSettings = () => {
  // 保存设置到本地存储或后端
  localStorage.setItem('llmcompiler_settings', JSON.stringify(settings))
  showSettings.value = false
  addLog('INFO', 'Settings', '设置已保存')
}

const resetSettings = () => {
  Object.assign(settings, {
    maxConcurrentTasks: 8,
    taskTimeout: 300,
    maxRetries: 3,
    logLevel: 'INFO',
    autoRefresh: true,
    refreshInterval: 5
  })
}

const clearLogs = () => {
  logs.value = []
}

const exportLogs = () => {
  const logText = logs.value.map(log => 
    `${formatLogTime(log.timestamp)} [${log.level}] ${log.component}: ${log.message}`
  ).join('\n')
  
  const blob = new Blob([logText], { type: 'text/plain' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = `llmcompiler_logs_${new Date().toISOString().split('T')[0]}.txt`
  a.click()
  URL.revokeObjectURL(url)
}

const addLog = (level: 'DEBUG' | 'INFO' | 'WARN' | 'ERROR', component: string, message: string) => {
  const log: LogEntry = {
    id: `log_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
    timestamp: new Date(),
    level,
    component,
    message
  }
  
  logs.value.unshift(log)
  
  // 限制日志数量
  if (logs.value.length > 1000) {
    logs.value = logs.value.slice(0, 1000)
  }
}

// 事件处理
const handleDAGStarted = (sessionId: string) => {
  addLog('INFO', 'DAG', `DAG执行已启动: ${sessionId}`)
}

const handleDAGPaused = () => {
  addLog('WARN', 'DAG', 'DAG执行已暂停')
}

const handleDAGStopped = () => {
  addLog('INFO', 'DAG', 'DAG执行已停止')
}

const handleTaskCancelled = (taskId: string) => {
  addLog('WARN', 'TaskManager', `任务已取消: ${taskId}`)
}

const handleTaskRetried = (taskId: string) => {
  addLog('INFO', 'TaskManager', `任务重试: ${taskId}`)
}

const handleNodeSelected = (node: any) => {
  addLog('DEBUG', 'DAG', `节点已选择: ${node.name}`)
}

const handleAlertDismissed = (alertId: string) => {
  addLog('DEBUG', 'AlertManager', `警告已忽略: ${alertId}`)
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

const formatLogTime = (date: Date) => {
  return date.toLocaleTimeString('zh-CN', { 
    hour12: false,
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit'
  } as Intl.DateTimeFormatOptions)
}

// 样式类方法
const getLoadClass = (load: number) => {
  if (load < 0.5) return 'text-success'
  if (load < 0.8) return 'text-warning'
  return 'text-error'
}

const getSessionStatusBadge = (status: string) => {
  const badges = {
    active: 'badge-success',
    paused: 'badge-warning',
    completed: 'badge-info',
    error: 'badge-error'
  }
  return badges[status as keyof typeof badges] || 'badge-ghost'
}

const getLogClass = (level: string) => {
  const classes = {
    DEBUG: 'text-base-content/60',
    INFO: 'text-base-content',
    WARN: 'text-warning',
    ERROR: 'text-error'
  }
  return classes[level as keyof typeof classes] || 'text-base-content'
}

const getLogLevelClass = (level: string) => {
  const classes = {
    DEBUG: 'text-base-content/60',
    INFO: 'text-info',
    WARN: 'text-warning',
    ERROR: 'text-error'
  }
  return classes[level as keyof typeof classes] || 'text-base-content'
}

// 模拟数据生成（开发阶段）
const generateMockData = () => {
  // 生成模拟会话
  const mockSessions: Session[] = [
    {
      id: 'session_001',
      name: '数据分析任务',
      description: '分析用户行为数据并生成报告',
      status: 'active',
      createdAt: new Date(Date.now() - 300000),
      taskCount: 5,
      progress: 60
    },
    {
      id: 'session_002',
      name: 'API集成测试',
      description: '测试第三方API集成功能',
      status: 'completed',
      createdAt: new Date(Date.now() - 600000),
      taskCount: 3,
      progress: 100
    }
  ]
  
  sessions.value = mockSessions
  selectedSessionId.value = mockSessions[0].id
  
  // 更新系统概览
  systemOverview.value = {
    activeSessions: mockSessions.filter(s => s.status === 'active').length,
    concurrentTasks: 4,
    completedToday: 12,
    systemLoad: 0.65,
    avgResponseTime: 1250
  }
  
  // 生成模拟日志
  const mockLogs: LogEntry[] = [
    {
      id: 'log_001',
      timestamp: new Date(Date.now() - 60000),
      level: 'INFO',
      component: 'SessionManager',
      message: '会话 session_001 已启动'
    },
    {
      id: 'log_002',
      timestamp: new Date(Date.now() - 30000),
      level: 'DEBUG',
      component: 'TaskExecutor',
      message: '任务 task_001 开始执行'
    },
    {
      id: 'log_003',
      timestamp: new Date(Date.now() - 15000),
      level: 'WARN',
      component: 'APIClient',
      message: 'API响应时间较长: 3.2s'
    }
  ]
  
  logs.value = mockLogs
}

// 加载设置
const loadSettings = () => {
  const savedSettings = localStorage.getItem('llmcompiler_settings')
  if (savedSettings) {
    Object.assign(settings, JSON.parse(savedSettings))
  }
}

// 自动刷新
let refreshInterval: ReturnType<typeof setInterval> | null = null

const startAutoRefresh = () => {
  if (settings.autoRefresh && !refreshInterval) {
    refreshInterval = setInterval(() => {
      // 刷新数据
      refreshSessions()
    }, settings.refreshInterval * 1000)
  }
}

const stopAutoRefresh = () => {
  if (refreshInterval) {
    clearInterval(refreshInterval)
    refreshInterval = null
  }
}

// 生命周期
onMounted(() => {
  // 加载设置
  loadSettings()
  
  // 生成模拟数据
  generateMockData()
  
  // 启动自动刷新
  startAutoRefresh()
  
  // 添加启动日志
  addLog('INFO', 'System', 'LLMCompiler控制台已启动')
})

onUnmounted(() => {
  stopAutoRefresh()
})
</script>

<style scoped>
.llm-compiler-console {
  @apply space-y-6;
}

.view-content {
  @apply min-h-[600px];
}

.session-item {
  @apply transition-all duration-200;
}

.session-item:hover {
  @apply shadow-sm;
}

.log-container {
  @apply font-mono text-sm;
}

.tabs {
  @apply mb-4;
}

.tab {
  @apply transition-all duration-200;
}
</style>