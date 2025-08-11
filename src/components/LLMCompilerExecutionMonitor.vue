<template>
  <div class="llm-compiler-execution-monitor">
    <!-- 执行状态总览 -->
    <div class="card bg-base-100 shadow-xl mb-6">
      <div class="card-body">
        <div class="flex justify-between items-center mb-4">
          <h2 class="card-title">LLMCompiler 执行监控</h2>
          <div class="flex gap-2">
            <div class="badge" :class="executionStatus.active ? 'badge-success' : 'badge-warning'">
              {{ executionStatus.active ? '执行中' : '待机' }}
            </div>
            <button 
              class="btn btn-sm btn-outline"
              @click="refreshMonitor"
              :disabled="refreshing"
            >
              <span v-if="refreshing" class="loading loading-spinner loading-sm"></span>
              刷新监控
            </button>
          </div>
        </div>
        
        <!-- 执行统计 -->
        <div class="grid grid-cols-1 md:grid-cols-6 gap-4">
          <div class="stat bg-base-200 rounded-lg">
            <div class="stat-title">总任务数</div>
            <div class="stat-value text-lg">{{ executionStats.totalTasks }}</div>
          </div>
          <div class="stat bg-base-200 rounded-lg">
            <div class="stat-title">并发执行</div>
            <div class="stat-value text-lg text-info">{{ executionStats.concurrentTasks }}</div>
          </div>
          <div class="stat bg-base-200 rounded-lg">
            <div class="stat-title">已完成</div>
            <div class="stat-value text-lg text-success">{{ executionStats.completedTasks }}</div>
          </div>
          <div class="stat bg-base-200 rounded-lg">
            <div class="stat-title">失败任务</div>
            <div class="stat-value text-lg text-error">{{ executionStats.failedTasks }}</div>
          </div>
          <div class="stat bg-base-200 rounded-lg">
            <div class="stat-title">执行效率</div>
            <div class="stat-value text-lg text-primary">{{ executionStats.efficiency }}%</div>
          </div>
          <div class="stat bg-base-200 rounded-lg">
            <div class="stat-title">平均延迟</div>
            <div class="stat-value text-lg">{{ formatDuration(executionStats.avgLatency) }}</div>
          </div>
        </div>
      </div>
    </div>
    
    <!-- 实时执行流 -->
    <div class="grid grid-cols-1 lg:grid-cols-2 gap-6 mb-6">
      <!-- 任务执行流 -->
      <div class="card bg-base-100 shadow-xl">
        <div class="card-body">
          <div class="flex justify-between items-center mb-4">
            <h3 class="card-title">任务执行流</h3>
            <div class="flex gap-2">
              <button 
                class="btn btn-xs btn-outline"
                @click="pauseTaskStream"
                :disabled="!taskStreamActive"
              >
                {{ taskStreamActive ? '暂停' : '恢复' }}
              </button>
              <button 
                class="btn btn-xs btn-outline"
                @click="clearTaskStream"
              >
                清空
              </button>
            </div>
          </div>
          
          <div class="h-80 overflow-y-auto space-y-2" ref="taskStreamContainer">
            <div 
              v-for="event in taskExecutionEvents" 
              :key="event.id"
              class="p-3 rounded-lg border-l-4 transition-all duration-200"
              :class="getTaskEventClass(event)"
            >
              <div class="flex justify-between items-start mb-1">
                <div class="font-semibold text-sm">{{ event.taskName }}</div>
                <div class="text-xs text-base-content/60">{{ formatTime(event.timestamp) }}</div>
              </div>
              <div class="text-sm text-base-content/80 mb-1">{{ event.description }}</div>
              <div class="flex justify-between items-center">
                <div class="badge badge-sm" :class="getEventTypeBadge(event.type)">{{ event.type }}</div>
                <div v-if="event.duration" class="text-xs">{{ formatDuration(event.duration) }}</div>
              </div>
              
              <!-- 错误详情 -->
              <div v-if="event.error" class="mt-2 p-2 bg-error/10 rounded text-xs">
                <div class="font-semibold text-error">错误:</div>
                <div class="text-error/80">{{ event.error }}</div>
              </div>
              
              <!-- 结果预览 -->
              <div v-if="event.result" class="mt-2">
                <div class="collapse collapse-arrow bg-base-200">
                  <input type="checkbox" /> 
                  <div class="collapse-title text-xs font-medium">查看结果</div>
                  <div class="collapse-content">
                    <pre class="text-xs bg-base-300 p-2 rounded overflow-x-auto">{{ JSON.stringify(event.result, null, 2) }}</pre>
                  </div>
                </div>
              </div>
            </div>
            
            <!-- 空状态 -->
            <div v-if="taskExecutionEvents.length === 0" class="text-center py-8 text-base-content/60">
              <svg xmlns="http://www.w3.org/2000/svg" class="h-12 w-12 mx-auto mb-2 opacity-50" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z" />
              </svg>
              <p>暂无执行事件</p>
            </div>
          </div>
        </div>
      </div>
      
      <!-- 性能监控 -->
      <div class="card bg-base-100 shadow-xl">
        <div class="card-body">
          <h3 class="card-title mb-4">性能监控</h3>
          
          <!-- 实时指标 -->
          <div class="space-y-4">
            <!-- CPU使用率 -->
            <div>
              <div class="flex justify-between items-center mb-1">
                <span class="text-sm font-medium">CPU使用率</span>
                <span class="text-sm">{{ Math.round(performanceMetrics.cpuUsage * 100) }}%</span>
              </div>
              <progress 
                class="progress progress-primary w-full" 
                :value="performanceMetrics.cpuUsage * 100" 
                max="100"
              ></progress>
            </div>
            
            <!-- 内存使用率 -->
            <div>
              <div class="flex justify-between items-center mb-1">
                <span class="text-sm font-medium">内存使用率</span>
                <span class="text-sm">{{ Math.round(performanceMetrics.memoryUsage * 100) }}%</span>
              </div>
              <progress 
                class="progress progress-secondary w-full" 
                :value="performanceMetrics.memoryUsage * 100" 
                max="100"
              ></progress>
            </div>
            
            <!-- 任务队列长度 -->
            <div>
              <div class="flex justify-between items-center mb-1">
                <span class="text-sm font-medium">任务队列</span>
                <span class="text-sm">{{ performanceMetrics.queueLength }} / {{ performanceMetrics.maxQueueSize }}</span>
              </div>
              <progress 
                class="progress progress-accent w-full" 
                :value="(performanceMetrics.queueLength / performanceMetrics.maxQueueSize) * 100" 
                max="100"
              ></progress>
            </div>
            
            <!-- 线程池使用率 -->
            <div>
              <div class="flex justify-between items-center mb-1">
                <span class="text-sm font-medium">线程池</span>
                <span class="text-sm">{{ performanceMetrics.activeThreads }} / {{ performanceMetrics.maxThreads }}</span>
              </div>
              <progress 
                class="progress progress-info w-full" 
                :value="(performanceMetrics.activeThreads / performanceMetrics.maxThreads) * 100" 
                max="100"
              ></progress>
            </div>
          </div>
          
          <!-- 性能图表 -->
          <div class="mt-6">
            <h4 class="font-semibold text-sm mb-2">性能趋势</h4>
            <div class="h-32 bg-base-200 rounded-lg flex items-center justify-center">
              <div class="text-center text-base-content/60">
                <svg xmlns="http://www.w3.org/2000/svg" class="h-8 w-8 mx-auto mb-1 opacity-50" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z" />
                </svg>
                <p class="text-xs">性能图表</p>
                <p class="text-xs">(集成图表库后显示)</p>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
    
    <!-- 任务详细监控 -->
    <div class="card bg-base-100 shadow-xl mb-6">
      <div class="card-body">
        <div class="flex justify-between items-center mb-4">
          <h3 class="card-title">任务详细监控</h3>
          <div class="flex gap-2">
            <!-- 过滤器 -->
            <div class="dropdown dropdown-end">
              <div tabindex="0" role="button" class="btn btn-sm btn-outline">
                <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 4a1 1 0 011-1h16a1 1 0 011 1v2.586a1 1 0 01-.293.707l-6.414 6.414a1 1 0 00-.293.707V17l-4 4v-6.586a1 1 0 00-.293-.707L3.293 7.293A1 1 0 013 6.586V4z" />
                </svg>
                过滤
              </div>
              <ul tabindex="0" class="dropdown-content z-[1] menu p-2 shadow bg-base-100 rounded-box w-40">
                <li><a @click="setTaskFilter('all')">全部任务</a></li>
                <li><a @click="setTaskFilter('running')">执行中</a></li>
                <li><a @click="setTaskFilter('completed')">已完成</a></li>
                <li><a @click="setTaskFilter('failed')">失败</a></li>
              </ul>
            </div>
            
            <!-- 排序 -->
            <div class="dropdown dropdown-end">
              <div tabindex="0" role="button" class="btn btn-sm btn-outline">
                <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 4h13M3 8h9m-9 4h6m4 0l4-4m0 0l4 4m-4-4v12" />
                </svg>
                排序
              </div>
              <ul tabindex="0" class="dropdown-content z-[1] menu p-2 shadow bg-base-100 rounded-box w-40">
                <li><a @click="setSortBy('startTime')">开始时间</a></li>
                <li><a @click="setSortBy('duration')">执行时间</a></li>
                <li><a @click="setSortBy('priority')">优先级</a></li>
                <li><a @click="setSortBy('status')">状态</a></li>
              </ul>
            </div>
          </div>
        </div>
        
        <!-- 任务列表 -->
        <div class="overflow-x-auto">
          <table class="table table-sm">
            <thead>
              <tr>
                <th>任务ID</th>
                <th>任务名称</th>
                <th>类型</th>
                <th>状态</th>
                <th>优先级</th>
                <th>开始时间</th>
                <th>执行时间</th>
                <th>进度</th>
                <th>依赖</th>
                <th>操作</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="task in filteredTasks" :key="task.id" class="hover">
                <td>
                  <code class="text-xs">{{ task.id.substring(0, 8) }}</code>
                </td>
                <td>
                  <div class="font-medium">{{ task.name }}</div>
                  <div class="text-xs text-base-content/60">{{ task.description }}</div>
                </td>
                <td>
                  <div class="badge badge-sm badge-outline">{{ task.type }}</div>
                </td>
                <td>
                  <div class="badge badge-sm" :class="getTaskStatusBadge(task.status)">{{ task.status }}</div>
                </td>
                <td>
                  <div class="badge badge-sm" :class="getPriorityBadge(task.priority)">{{ task.priority }}</div>
                </td>
                <td class="text-xs">{{ formatTime(task.startTime) }}</td>
                <td class="text-xs">{{ formatDuration(task.executionTime) }}</td>
                <td>
                  <div class="flex items-center gap-2">
                    <progress 
                      class="progress progress-primary w-16 h-2" 
                      :value="task.progress" 
                      max="100"
                    ></progress>
                    <span class="text-xs">{{ Math.round(task.progress) }}%</span>
                  </div>
                </td>
                <td>
                  <div class="flex flex-wrap gap-1">
                    <div 
                      v-for="dep in task.dependencies" 
                      :key="dep"
                      class="badge badge-xs badge-ghost"
                    >
                      {{ dep.substring(0, 6) }}
                    </div>
                  </div>
                </td>
                <td>
                  <div class="flex gap-1">
                    <button 
                      class="btn btn-xs btn-outline"
                      @click="viewTaskDetails(task)"
                    >
                      详情
                    </button>
                    <button 
                      v-if="task.status === 'running'"
                      class="btn btn-xs btn-error"
                      @click="cancelTask(task.id)"
                    >
                      取消
                    </button>
                    <button 
                      v-if="task.status === 'failed'"
                      class="btn btn-xs btn-warning"
                      @click="retryTask(task.id)"
                    >
                      重试
                    </button>
                  </div>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
        
        <!-- 分页 -->
        <div class="flex justify-between items-center mt-4">
          <div class="text-sm text-base-content/60">
            显示 {{ (currentPage - 1) * pageSize + 1 }} - {{ Math.min(currentPage * pageSize, filteredTasks.length) }} 条，共 {{ filteredTasks.length }} 条
          </div>
          <div class="join">
            <button 
              class="join-item btn btn-sm"
              @click="currentPage--"
              :disabled="currentPage <= 1"
            >
              «
            </button>
            <button class="join-item btn btn-sm">{{ currentPage }}</button>
            <button 
              class="join-item btn btn-sm"
              @click="currentPage++"
              :disabled="currentPage >= totalPages"
            >
              »
            </button>
          </div>
        </div>
      </div>
    </div>
    
    <!-- 错误和警告 -->
    <div class="card bg-base-100 shadow-xl">
      <div class="card-body">
        <h3 class="card-title mb-4">错误和警告</h3>
        
        <div class="space-y-3">
          <div 
            v-for="alert in systemAlerts" 
            :key="alert.id"
            class="alert" 
            :class="getAlertClass(alert.level)"
          >
            <svg xmlns="http://www.w3.org/2000/svg" class="stroke-current shrink-0 h-6 w-6" fill="none" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L3.732 16.5c-.77.833.192 2.5 1.732 2.5z" />
            </svg>
            <div class="flex-1">
              <div class="font-semibold">{{ alert.title }}</div>
              <div class="text-sm">{{ alert.message }}</div>
              <div class="text-xs text-base-content/60 mt-1">{{ formatTime(alert.timestamp) }}</div>
            </div>
            <button 
              class="btn btn-sm btn-ghost"
              @click="dismissAlert(alert.id)"
            >
              ✕
            </button>
          </div>
        </div>
        
        <!-- 空状态 -->
        <div v-if="systemAlerts.length === 0" class="text-center py-8 text-base-content/60">
          <svg xmlns="http://www.w3.org/2000/svg" class="h-12 w-12 mx-auto mb-2 opacity-50" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
          </svg>
          <p>系统运行正常</p>
          <p class="text-sm">暂无错误或警告</p>
        </div>
      </div>
    </div>
    
    <!-- 任务详情模态框 -->
    <dialog ref="taskDetailsModal" class="modal">
      <div class="modal-box w-11/12 max-w-4xl">
        <form method="dialog">
          <button class="btn btn-sm btn-circle btn-ghost absolute right-2 top-2">✕</button>
        </form>
        
        <h3 class="font-bold text-lg mb-4">任务详情</h3>
        
        <div v-if="selectedTask" class="space-y-4">
          <!-- 基本信息 -->
          <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div>
              <label class="label">任务ID</label>
              <code class="text-sm">{{ selectedTask.id }}</code>
            </div>
            <div>
              <label class="label">任务名称</label>
              <div class="font-medium">{{ selectedTask.name }}</div>
            </div>
            <div>
              <label class="label">任务类型</label>
              <div class="badge badge-outline">{{ selectedTask.type }}</div>
            </div>
            <div>
              <label class="label">状态</label>
              <div class="badge" :class="getTaskStatusBadge(selectedTask.status)">{{ selectedTask.status }}</div>
            </div>
            <div>
              <label class="label">优先级</label>
              <div class="badge" :class="getPriorityBadge(selectedTask.priority)">{{ selectedTask.priority }}</div>
            </div>
            <div>
              <label class="label">进度</label>
              <div class="flex items-center gap-2">
                <progress class="progress progress-primary w-24" :value="selectedTask.progress" max="100"></progress>
                <span class="text-sm">{{ Math.round(selectedTask.progress) }}%</span>
              </div>
            </div>
          </div>
          
          <!-- 时间信息 -->
          <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
            <div>
              <label class="label">开始时间</label>
              <div class="text-sm">{{ formatTime(selectedTask.startTime) }}</div>
            </div>
            <div>
              <label class="label">执行时间</label>
              <div class="text-sm">{{ formatDuration(selectedTask.executionTime) }}</div>
            </div>
            <div>
              <label class="label">预计完成</label>
              <div class="text-sm">{{ selectedTask.estimatedCompletion ? formatTime(selectedTask.estimatedCompletion) : '未知' }}</div>
            </div>
          </div>
          
          <!-- 依赖关系 -->
          <div>
            <label class="label">依赖任务</label>
            <div class="flex flex-wrap gap-2">
              <div 
                v-for="dep in selectedTask.dependencies" 
                :key="dep"
                class="badge badge-ghost"
              >
                {{ dep }}
              </div>
              <div v-if="selectedTask.dependencies.length === 0" class="text-sm text-base-content/60">
                无依赖
              </div>
            </div>
          </div>
          
          <!-- 任务描述 -->
          <div>
            <label class="label">任务描述</label>
            <div class="text-sm bg-base-200 p-3 rounded">{{ selectedTask.description || '无描述' }}</div>
          </div>
          
          <!-- 执行参数 -->
          <div>
            <label class="label">执行参数</label>
            <div class="mockup-code">
              <pre><code>{{ JSON.stringify(selectedTask.parameters || {}, null, 2) }}</code></pre>
            </div>
          </div>
          
          <!-- 执行结果 -->
          <div v-if="selectedTask.result">
            <label class="label">执行结果</label>
            <div class="mockup-code">
              <pre><code>{{ JSON.stringify(selectedTask.result, null, 2) }}</code></pre>
            </div>
          </div>
          
          <!-- 错误信息 -->
          <div v-if="selectedTask.error">
            <label class="label">错误信息</label>
            <div class="alert alert-error">
              <svg xmlns="http://www.w3.org/2000/svg" class="stroke-current shrink-0 h-6 w-6" fill="none" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 14l2-2m0 0l2-2m-2 2l-2-2m2 2l2 2m7-2a9 9 0 11-18 0 9 9 0 0118 0z" />
              </svg>
              <span>{{ selectedTask.error }}</span>
            </div>
          </div>
        </div>
      </div>
    </dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted, onUnmounted, nextTick } from 'vue'

// 类型定义
interface ExecutionStatus {
  active: boolean
  paused: boolean
  sessionId?: string
}

interface ExecutionStats {
  totalTasks: number
  concurrentTasks: number
  completedTasks: number
  failedTasks: number
  efficiency: number
  avgLatency: number
}

interface TaskExecutionEvent {
  id: string
  taskId: string
  taskName: string
  type: 'started' | 'completed' | 'failed' | 'cancelled' | 'dependency_resolved'
  description: string
  timestamp: Date
  duration?: number
  error?: string
  result?: any
}

interface PerformanceMetrics {
  cpuUsage: number
  memoryUsage: number
  queueLength: number
  maxQueueSize: number
  activeThreads: number
  maxThreads: number
}

interface TaskDetails {
  id: string
  name: string
  description: string
  type: string
  status: 'pending' | 'ready' | 'running' | 'completed' | 'failed' | 'cancelled'
  priority: 'low' | 'normal' | 'high' | 'urgent'
  startTime: Date
  executionTime: number
  progress: number
  dependencies: string[]
  parameters?: Record<string, any>
  result?: any
  error?: string
  estimatedCompletion?: Date
}

interface SystemAlert {
  id: string
  level: 'info' | 'warning' | 'error'
  title: string
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
  taskCancelled: [taskId: string]
  taskRetried: [taskId: string]
  alertDismissed: [alertId: string]
}>()

// 响应式数据
const executionStatus = ref<ExecutionStatus>({
  active: false,
  paused: false
})

const executionStats = ref<ExecutionStats>({
  totalTasks: 0,
  concurrentTasks: 0,
  completedTasks: 0,
  failedTasks: 0,
  efficiency: 0,
  avgLatency: 0
})

const taskExecutionEvents = ref<TaskExecutionEvent[]>([])
const performanceMetrics = ref<PerformanceMetrics>({
  cpuUsage: 0,
  memoryUsage: 0,
  queueLength: 0,
  maxQueueSize: 100,
  activeThreads: 0,
  maxThreads: 8
})

const taskDetails = ref<TaskDetails[]>([])
const systemAlerts = ref<SystemAlert[]>([])

// UI状态
const refreshing = ref(false)
const taskStreamActive = ref(true)
const taskStreamContainer = ref<HTMLElement>()
const taskDetailsModal = ref<HTMLDialogElement>()
const selectedTask = ref<TaskDetails | null>(null)

// 过滤和排序
const taskFilter = ref('all')
const sortBy = ref('startTime')
const currentPage = ref(1)
const pageSize = ref(20)

// 计算属性
const filteredTasks = computed(() => {
  let filtered = taskDetails.value
  
  // 应用过滤器
  if (taskFilter.value !== 'all') {
    filtered = filtered.filter(task => task.status === taskFilter.value)
  }
  
  // 应用排序
  filtered.sort((a, b) => {
    switch (sortBy.value) {
      case 'startTime':
        return b.startTime.getTime() - a.startTime.getTime()
      case 'duration':
        return b.executionTime - a.executionTime
      case 'priority':
        const priorityOrder = { urgent: 4, high: 3, normal: 2, low: 1 }
        return priorityOrder[b.priority] - priorityOrder[a.priority]
      case 'status':
        return a.status.localeCompare(b.status)
      default:
        return 0
    }
  })
  
  // 应用分页
  const start = (currentPage.value - 1) * pageSize.value
  const end = start + pageSize.value
  return filtered.slice(start, end)
})

const totalPages = computed(() => {
  return Math.ceil(taskDetails.value.length / pageSize.value)
})

// 方法
const refreshMonitor = async () => {
  refreshing.value = true
  try {
    // 调用后端API刷新监控数据
    await new Promise(resolve => setTimeout(resolve, 1000)) // 模拟API调用
  } finally {
    refreshing.value = false
  }
}

const pauseTaskStream = () => {
  taskStreamActive.value = !taskStreamActive.value
}

const clearTaskStream = () => {
  taskExecutionEvents.value = []
}

const setTaskFilter = (filter: string) => {
  taskFilter.value = filter
  currentPage.value = 1
}

const setSortBy = (sort: string) => {
  sortBy.value = sort
}

const viewTaskDetails = (task: TaskDetails) => {
  selectedTask.value = task
  taskDetailsModal.value?.showModal()
}

const cancelTask = (taskId: string) => {
  const task = taskDetails.value.find(t => t.id === taskId)
  if (task) {
    task.status = 'cancelled'
    emit('taskCancelled', taskId)
  }
}

const retryTask = (taskId: string) => {
  const task = taskDetails.value.find(t => t.id === taskId)
  if (task) {
    task.status = 'pending'
    task.progress = 0
    task.error = undefined
    emit('taskRetried', taskId)
  }
}

const dismissAlert = (alertId: string) => {
  const index = systemAlerts.value.findIndex(alert => alert.id === alertId)
  if (index > -1) {
    systemAlerts.value.splice(index, 1)
    emit('alertDismissed', alertId)
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

// 样式类方法
const getTaskEventClass = (event: TaskExecutionEvent) => {
  const classes = {
    started: 'border-l-blue-400 bg-blue-50',
    completed: 'border-l-green-400 bg-green-50',
    failed: 'border-l-red-400 bg-red-50',
    cancelled: 'border-l-orange-400 bg-orange-50',
    dependency_resolved: 'border-l-purple-400 bg-purple-50'
  }
  return classes[event.type] || 'border-l-gray-400 bg-gray-50'
}

const getEventTypeBadge = (type: string) => {
  const badges = {
    started: 'badge-info',
    completed: 'badge-success',
    failed: 'badge-error',
    cancelled: 'badge-warning',
    dependency_resolved: 'badge-secondary'
  }
  return badges[type as keyof typeof badges] || 'badge-ghost'
}

const getTaskStatusBadge = (status: string) => {
  const badges = {
    pending: 'badge-warning',
    ready: 'badge-info',
    running: 'badge-primary',
    completed: 'badge-success',
    failed: 'badge-error',
    cancelled: 'badge-warning'
  }
  return badges[status as keyof typeof badges] || 'badge-ghost'
}

const getPriorityBadge = (priority: string) => {
  const badges = {
    low: 'badge-ghost',
    normal: 'badge-info',
    high: 'badge-warning',
    urgent: 'badge-error'
  }
  return badges[priority as keyof typeof badges] || 'badge-ghost'
}

const getAlertClass = (level: string) => {
  const classes = {
    info: 'alert-info',
    warning: 'alert-warning',
    error: 'alert-error'
  }
  return classes[level as keyof typeof classes] || 'alert-info'
}

// 模拟数据生成（开发阶段）
const generateMockData = () => {
  // 生成模拟任务
  const mockTasks: TaskDetails[] = [
    {
      id: 'task_001',
      name: '数据预处理',
      description: '清理和预处理输入数据',
      type: 'data_processing',
      status: 'completed',
      priority: 'high',
      startTime: new Date(Date.now() - 300000),
      executionTime: 45000,
      progress: 100,
      dependencies: [],
      parameters: { input_size: 1024, format: 'json' },
      result: { processed_records: 1024, success_rate: 0.98 }
    },
    {
      id: 'task_002',
      name: 'API调用',
      description: '调用外部API获取数据',
      type: 'api_call',
      status: 'running',
      priority: 'normal',
      startTime: new Date(Date.now() - 120000),
      executionTime: 120000,
      progress: 65,
      dependencies: ['task_001'],
      parameters: { endpoint: '/api/v1/data', timeout: 30000 }
    },
    {
      id: 'task_003',
      name: '结果分析',
      description: '分析处理结果并生成报告',
      type: 'analysis',
      status: 'pending',
      priority: 'normal',
      startTime: new Date(),
      executionTime: 0,
      progress: 0,
      dependencies: ['task_002']
    }
  ]
  
  taskDetails.value = mockTasks
  
  // 生成模拟执行事件
  const mockEvents: TaskExecutionEvent[] = [
    {
      id: 'event_001',
      taskId: 'task_001',
      taskName: '数据预处理',
      type: 'completed',
      description: '任务成功完成，处理了1024条记录',
      timestamp: new Date(Date.now() - 180000),
      duration: 45000,
      result: { processed_records: 1024 }
    },
    {
      id: 'event_002',
      taskId: 'task_002',
      taskName: 'API调用',
      type: 'started',
      description: '开始调用外部API',
      timestamp: new Date(Date.now() - 120000)
    }
  ]
  
  taskExecutionEvents.value = mockEvents
  
  // 更新统计数据
  executionStats.value = {
    totalTasks: mockTasks.length,
    concurrentTasks: mockTasks.filter(t => t.status === 'running').length,
    completedTasks: mockTasks.filter(t => t.status === 'completed').length,
    failedTasks: mockTasks.filter(t => t.status === 'failed').length,
    efficiency: 85,
    avgLatency: 2500
  }
  
  // 更新性能指标
  performanceMetrics.value = {
    cpuUsage: 0.45,
    memoryUsage: 0.62,
    queueLength: 3,
    maxQueueSize: 100,
    activeThreads: 4,
    maxThreads: 8
  }
}

// 自动滚动任务流
const scrollTaskStreamToBottom = () => {
  if (taskStreamContainer.value) {
    nextTick(() => {
      taskStreamContainer.value!.scrollTop = taskStreamContainer.value!.scrollHeight
    })
  }
}

// 监听任务事件变化
const watchTaskEvents = () => {
  // 当有新事件时自动滚动到底部
  scrollTaskStreamToBottom()
}

// 生命周期
onMounted(() => {
  // 加载初始数据
  generateMockData()
  
  // 启动定时刷新
  const refreshInterval: ReturnType<typeof setInterval> = setInterval(() => {
    if (taskStreamActive.value) {
      // 模拟新事件
      // 实际应该通过WebSocket或轮询获取
    }
  }, 5000)
  
  // 清理定时器
  onUnmounted(() => {
    clearInterval(refreshInterval)
  })
})
</script>

<style scoped>
.llm-compiler-execution-monitor {
  @apply space-y-6;
}

.task-stream-container {
  @apply max-h-80 overflow-y-auto;
}

.task-event {
  @apply transition-all duration-200;
}

.task-event:hover {
  @apply shadow-sm;
}

.performance-chart {
  @apply h-32 bg-base-200 rounded-lg;
}
</style>