<template>
  <div class="container mx-auto p-6 space-y-6">
    <!-- 页面标题 -->
    <div class="flex justify-between items-center">
      <div>
        <h1 class="text-3xl font-bold text-base-content">{{ t('agents.title', 'Agent管理') }}</h1>
        <p class="text-base-content/70 mt-2">{{ t('agents.description', '管理和监控智能Agent系统') }}</p>
      </div>
      <div class="flex gap-2">
        <button 
          v-if="!dispatcherInitialized" 
          @click="initializeDispatcher" 
          :disabled="loading"
          class="btn btn-primary btn-sm"
        >
          <span v-if="loading" class="loading loading-spinner loading-xs mr-2"></span>
          <i v-else class="fas fa-power-off mr-2"></i>
          {{ t('agents.initializeDispatcher', '初始化调度系统') }}
        </button>
        <button @click="refreshData" class="btn btn-outline btn-sm">
          <i class="fas fa-sync-alt mr-2"></i>
          {{ t('common.refresh', '刷新') }}
        </button>
        <button @click="showTaskModal = true" class="btn btn-primary btn-sm" :disabled="!dispatcherInitialized">
          <i class="fas fa-play mr-2"></i>
          {{ t('agents.executeTask', '执行任务') }}
        </button>



      </div>
    </div>

    <!-- 系统统计卡片 -->
    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-5 gap-4">
      <StatsCard
        :value="systemStats?.total_agents || 0"
        :label="t('agents.stats.totalAgents', '总Agent数')"
        icon="fas fa-robot"
        theme="primary"
      />
      <StatsCard
        :value="systemStats?.total_tasks || 0"
        :label="t('agents.stats.totalTasks', '总任务数')"
        icon="fas fa-tasks"
        theme="info"
      />
      <StatsCard
        :value="successRate"
        :label="t('agents.stats.successRate', '成功率')"
        suffix="%"
        icon="fas fa-chart-line"
        theme="success"
      />
      <StatsCard
        :value="avgExecutionTime"
        :label="t('agents.stats.avgExecutionTime', '平均执行时间')"
        suffix="s"
        icon="fas fa-clock"
        theme="warning"
      />
      <StatsCard
        :value="dispatcherInitialized ? t('agents.active', '活跃') : t('agents.inactive', '未激活')"
        :label="t('agents.dispatcherSystem', '调度系统')"
        icon="fas fa-cogs"
        :theme="dispatcherInitialized ? 'success' : 'warning'"
      />
    </div>

    <!-- Agent列表 -->
    <div class="card bg-base-100 shadow-xl">
      <div class="card-header">
        <h2 class="card-title text-xl font-semibold p-6 pb-0">
          <i class="fas fa-robot mr-2"></i>
          {{ t('agents.agentList', 'Agent列表') }}
        </h2>
      </div>
      <div class="card-body">
        <div class="overflow-x-auto">
          <table class="table table-zebra w-full">
            <thead>
              <tr>
                <th>{{ t('agents.table.name', '名称') }}</th>
                <th>{{ t('agents.table.type', '类型') }}</th>
                <th>{{ t('agents.table.status', '状态') }}</th>
                <th>{{ t('agents.table.totalTasks', '总任务') }}</th>
                <th>{{ t('agents.table.successRate', '成功率') }}</th>
                <th>{{ t('agents.table.avgTime', '平均时间') }}</th>
                <th>{{ t('agents.table.lastActivity', '最后活动') }}</th>
                <th>{{ t('agents.table.actions', '操作') }}</th>
              </tr>
            </thead>
            <tbody>
              <!-- 可用架构 -->
              <tr v-for="agent in availableArchitectures" :key="agent.name">
                <td>
                  <div>
                    <div class="font-medium">
                      {{ agent.name || agent.agent_type }}
                    </div>
                    <div class="text-sm opacity-70">{{ agent.description || agent.factory_type }}</div>
                  </div>
                </td>
                <td>
                  <span class="badge badge-primary badge-sm">{{ agent.agent_type || agent.factory_type }}</span>
                </td>
                <td>
                  <div class="badge" :class="getStatusBadgeClass(agent.status || 'Ready')">
                    <i :class="getStatusIcon(agent.status || 'Ready')" class="mr-1"></i>
                    {{ agent.status || 'Ready' }}
                  </div>
                </td>
                <td>{{ agent.execution_count || 0 }}</td>
                <td>
                  <span v-if="agent.execution_count > 0">
                    {{ ((agent.success_count / agent.execution_count) * 100).toFixed(1) }}%
                  </span>
                  <span v-else class="text-base-content/50">N/A</span>
                </td>
                <td>{{ agent.average_execution_time || 0 }}s</td>
                <td>
                  <span v-if="agent.last_execution">
                    {{ formatDate(agent.last_execution) }}
                  </span>
                  <span v-else class="text-base-content/50">N/A</span>
                </td>
                <td>
                  <div class="flex gap-1">
                    <button @click="viewAgentDetails(agent)" class="btn btn-ghost btn-xs">
                      <i class="fas fa-eye"></i>
                    </button>
                    <button @click="executeAgentTask(agent)" class="btn btn-primary btn-xs" :disabled="!dispatcherInitialized">
                      <i class="fas fa-play"></i>
                    </button>
                  </div>
                </td>
              </tr>
              
              <!-- 空状态 -->
              <tr v-if="availableArchitectures.length === 0">
                <td colspan="8" class="text-center py-8 text-base-content/50">
                  <i class="fas fa-sitemap text-4xl mb-2"></i>
                  <div>{{ t('agents.noArchitectures', '暂无可用架构') }}</div>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
    </div>

    <!-- 任务历史 -->
    <div v-if="dispatcherInitialized" class="card bg-base-100 shadow-xl">
      <div class="card-header">
        <h2 class="card-title text-xl font-semibold p-6 pb-0">
          <i class="fas fa-history mr-2"></i>
          {{ t('agents.taskHistory', '任务历史') }}
        </h2>
      </div>
      <div class="card-body">
        <div v-if="taskHistory.length === 0" class="text-center py-8 text-base-content/60">
          <i class="fas fa-history text-4xl mb-4"></i>
          <p>{{ t('agents.noTaskHistory', '暂无任务历史') }}</p>
        </div>
        
        <div v-else class="overflow-x-auto">
          <table class="table table-zebra w-full">
            <thead>
              <tr>
                <th>{{ t('agents.taskId', '任务ID') }}</th>
                <th>{{ t('agents.description', '描述') }}</th>
                <th>{{ t('agents.architecture', '架构') }}</th>
                <th>{{ t('agents.status', '状态') }}</th>
                <th>{{ t('agents.createdAt', '创建时间') }}</th>
                <th>{{ t('common.actions', '操作') }}</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="task in taskHistory" :key="task.id">
                <td>
                  <div class="font-mono text-sm">{{ task.id }}</div>
                </td>
                <td>
                  <div class="max-w-xs truncate">{{ task.description }}</div>
                </td>
                <td>
                  <div class="badge badge-outline">{{ task.architecture }}</div>
                </td>
                <td>
                  <div class="badge" :class="getStatusBadgeClass(task.status)">
                    <i :class="getStatusIcon(task.status)" class="mr-1"></i>
                    {{ task.status }}
                  </div>
                </td>
                <td>{{ formatDate(task.created_at) }}</td>
                <td>
                  <div class="flex gap-1">
                    <button @click="viewTaskDetails(task)" class="btn btn-ghost btn-xs">
                      <i class="fas fa-eye"></i>
                    </button>
                    <button v-if="task.status === 'running'" @click="cancelTask(task.id)" class="btn btn-warning btn-xs">
                      <i class="fas fa-stop"></i>
                    </button>
                  </div>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
    </div>

    <!-- 执行任务模态框 -->
    <div v-if="showTaskModal" class="modal modal-open">
      <div class="modal-box w-11/12 max-w-2xl">
        <h3 class="font-bold text-lg mb-4">
          <i class="fas fa-play mr-2"></i>
          {{ t('agents.executeTask', '执行Agent任务') }}
        </h3>
        
        <div class="space-y-4">
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ t('agents.form.taskDescription', '任务描述') }}</span>
            </label>
            <textarea v-model="taskForm.task_description" class="textarea textarea-bordered h-24" placeholder="请详细描述您需要执行的任务..." required></textarea>
          </div>

          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ t('agents.form.target', '目标') }}</span>
            </label>
            <input v-model="taskForm.target" type="text" placeholder="example.com" class="input input-bordered w-full" />
          </div>

          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ t('agents.form.architecture', '架构选择') }}</span>
            </label>
            <select v-model="taskForm.architecture" class="select select-bordered w-full">
              <option value="auto">{{ t('agents.form.autoSelect', '自动选择') }}</option>
              <option v-for="arch in availableArchitectures" :key="arch.name" :value="arch.name">
                {{ arch.name }} - {{ arch.description }}
              </option>
            </select>
          </div>

          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ t('agents.form.userId', '用户ID') }}</span>
            </label>
            <input v-model="taskForm.user_id" type="text" placeholder="admin" class="input input-bordered w-full" />
          </div>

          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ t('agents.form.priority', '优先级') }}</span>
            </label>
            <!-- <select v-model="taskForm.priority" class="select select-bordered w-full">
              <option value="low">{{ t('agents.priority.low', '低') }}</option>
              <option value="normal">{{ t('agents.priority.normal', '普通') }}</option>
              <option value="high">{{ t('agents.priority.high', '高') }}</option>
              <option value="critical">{{ t('agents.priority.critical', '紧急') }}</option>
            </select> -->
          </div>
        </div>

        <div class="modal-action">
          <button @click="showTaskModal = false" class="btn btn-ghost">{{ t('common.cancel', '取消') }}</button>
          <button 
            @click="executeTask" 
            class="btn btn-primary" 
            :disabled="isTaskFormInvalid"
          >
            <i class="fas fa-play mr-2"></i>
            {{ t('agents.executeTask', '执行任务') }}
          </button>
        </div>
      </div>
    </div>





    <!-- 加载状态 -->
    <div v-if="loading" class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
      <div class="bg-base-100 p-6 rounded-lg shadow-xl">
        <div class="flex items-center space-x-3">
          <span class="loading loading-spinner loading-md"></span>
          <span>{{ loadingMessage }}</span>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useI18n } from 'vue-i18n'
import StatsCard from '@/components/Dashboard/StatsCard.vue'

import { dialog } from '@/composables/useDialog'

const { t } = useI18n()
// Toast功能通过dialog.toast提供

// 接口定义

interface TaskHistoryItem {
  id: string
  description: string
  architecture: string
  status: string
  created_at: string
  completed_at?: string
  result?: any
}

// 响应式数据
const taskHistory = ref<TaskHistoryItem[]>([])
const systemStats = ref<any>(null)
const loading = ref(false)
const loadingMessage = ref('')
const showTaskModal = ref(false)



// 多Agent调度系统相关数据
const dispatcherInitialized = ref(false)
const dispatcherStats = ref({
  totalDispatches: 0,
  successfulDispatches: 0,
  failedDispatches: 0,
  averageDuration: 0
})
const availableArchitectures = ref<any[]>([])

// 表单数据
const taskForm = ref({
  task_description: '',
  user_id: 'admin',
  target: '',
  architecture: 'auto'
})



// 计算属性
const successRate = computed(() => {
  if (!systemStats.value?.overall_success_rate) return 0
  return (systemStats.value.overall_success_rate * 100).toFixed(1)
})

const avgExecutionTime = computed(() => {
  return systemStats.value?.average_execution_time || 0
})

const isTaskFormInvalid = computed(() => {
  return !dispatcherInitialized.value || 
         !taskForm.value.task_description.trim() || 
         !taskForm.value.user_id.trim()
})

// 方法
const initializeSystem = async () => {
  try {
    loading.value = true
    loadingMessage.value = t('agents.initializing', '初始化Agent系统...')
    await invoke('initialize_agent_system')
    await loadData()
    // dialog.toast.success(t('agents.initSuccess', 'Agent系统初始化成功'))
  } catch (error) {
    console.error('Failed to initialize agent system:', error)
    dialog.toast.error(t('agents.initFailed', 'Agent系统初始化失败'))
  } finally {
    loading.value = false
  }
}

// 多Agent调度系统方法
const initializeDispatcher = async () => {
  try {
    loading.value = true
    loadingMessage.value = t('agents.initializingDispatcher', '初始化多Agent调度系统...')
    // 调度系统在后端启动时已初始化，这里只需要加载数据
    dispatcherInitialized.value = true
    await loadDispatcherData()
    dialog.toast.success(t('agents.dispatcherInitSuccess', '多Agent调度系统初始化成功'))
  } catch (error) {
    console.error('Failed to initialize dispatcher:', error)
    dialog.toast.error(t('agents.dispatcherInitFailed', '多Agent调度系统初始化失败'))
  } finally {
    loading.value = false
  }
}

const loadDispatcherData = async () => {
  try {
    const [architectures, stats] = await Promise.all([
      invoke<string[]>('list_agent_architectures'),
      invoke<any>('get_dispatch_statistics')
    ])
    
    // 转换架构列表为对象格式
    availableArchitectures.value = architectures.map(arch => ({
      name: arch,
      description: getArchitectureDescription(arch)
    }))
    
    if (stats) {
      dispatcherStats.value = {
        totalDispatches: stats.total_dispatches || 0,
        successfulDispatches: stats.successful_dispatches || 0,
        failedDispatches: stats.failed_dispatches || 0,
        averageDuration: stats.average_duration || 0
      }
    }
  } catch (error) {
    console.error('Failed to load dispatcher data:', error)
  }
}

const checkDispatcherStatus = async () => {
  try {
    // 调度系统在后端启动时已初始化，这里直接设置为true
    dispatcherInitialized.value = true
    await loadDispatcherData()
  } catch (error) {
    console.error('Failed to check dispatcher status:', error)
    dispatcherInitialized.value = false
  }
}

const loadData = async () => {
  try {
    const [statsData] = await Promise.all([
      invoke('get_agent_system_stats')
    ])
    systemStats.value = statsData
    
    // 检查并加载多Agent调度系统数据
    await checkDispatcherStatus()
    await loadTaskHistory()
  } catch (error) {
    console.error('Failed to load data:', error)
    dialog.toast.error(t('agents.loadFailed', '加载数据失败'))
  }
}

const refreshData = async () => {
  loading.value = true
  loadingMessage.value = t('agents.refreshing', '刷新数据...')
  await loadData()
  loading.value = false
  dialog.toast.success(t('agents.refreshSuccess', '数据刷新成功'))
}

const executeTask = async () => {
  try {
    // 构建多Agent调度请求
    const requestData = {
      user_input: taskForm.value.task_description,
      target: taskForm.value.target || null,
      context: {
        priority: 'normal',
        user_id: taskForm.value.user_id,
        architecture: taskForm.value.architecture === 'auto' ? null : taskForm.value.architecture
      },
      conversation_id: null,
      provider: null,
      model: null,
      system_prompt: null
    }

    // 执行多Agent调度任务
    const response = await invoke('dispatch_multi_agent_task', { request: requestData })
    
    dialog.toast.success(t('agents.taskExecuteSuccess', '任务调度成功'))
    showTaskModal.value = false
    await loadData()
  } catch (error) {
    console.error('Failed to execute task:', error)
    dialog.toast.error(t('agents.taskExecuteFailed', '任务调度失败'))
  }
}

const loadTaskHistory = async () => {
  try {
    // 模拟任务历史数据，实际应该从后端获取
    taskHistory.value = []
  } catch (error) {
    console.error('Failed to load task history:', error)
  }
}

const viewTaskDetails = (task: TaskHistoryItem) => {
  // TODO: 实现任务详情查看
  console.log('View task details:', task)
  dialog.toast.info(t('agents.taskDetails', '任务详情功能开发中'))
}

const cancelTask = async (taskId: string) => {
  try {
    // TODO: 实现任务取消功能
    dialog.toast.success(t('agents.taskCancelled', '任务已取消'))
    await loadTaskHistory()
  } catch (error) {
    console.error('Failed to cancel task:', error)
    dialog.toast.error(t('agents.taskCancelFailed', '任务取消失败'))
  }
}



const getStatusBadgeClass = (status: string) => {
  const statusClasses = {
    Ready: 'badge-success',
    Running: 'badge-info',
    Error: 'badge-error',
    Stopped: 'badge-neutral',
    Paused: 'badge-warning',
    Completed: 'badge-success',
    Failed: 'badge-error'
  }
  return statusClasses[status as keyof typeof statusClasses] || 'badge-neutral'
}

const getStatusIcon = (status: string) => {
  const statusIcons = {
    Ready: 'fas fa-check-circle',
    Running: 'fas fa-spinner fa-spin',
    Error: 'fas fa-exclamation-circle',
    Stopped: 'fas fa-stop-circle',
    Paused: 'fas fa-pause-circle',
    Completed: 'fas fa-check-circle',
    Failed: 'fas fa-times-circle'
  }
  return statusIcons[status as keyof typeof statusIcons] || 'fas fa-question-circle'
}

const formatDate = (dateString: string) => {
  return new Date(dateString).toLocaleString()
}

const viewAgentDetails = (agent: any) => {
  // TODO: 实现Agent详情查看
  console.log('View agent details:', agent)
  dialog.toast.info(t('agents.agentDetails', 'Agent详情功能开发中'))
}

const executeAgentTask = (architecture: any) => {
  // 重置表单
  taskForm.value = {
    task_description: '',
    target: '',
    architecture: 'auto',
    user_id: 'admin'
  }
  
  // 根据架构信息预填表单
  if (architecture) {
    taskForm.value.task_description = `使用${architecture.name}架构执行任务`
    taskForm.value.architecture = architecture.name
  }
  
  showTaskModal.value = true
}







// 工具函数
const getArchitectureDescription = (architecture: string): string => {
  const descriptions = {
    'PlanAndExecute': '计划执行架构 - 适合复杂多步骤任务',
    'LlmCompiler': 'LLM编译器架构 - 适合并行处理任务',
    'ReWoo': 'ReWOO架构 - 适合推理密集型任务'
  }
  return descriptions[architecture as keyof typeof descriptions] || '未知架构'
}

// 生命周期
onMounted(() => {
  initializeSystem()
})
</script>

<style scoped>
.modal {
  backdrop-filter: blur(4px);
}

.table th {
  background-color: hsl(var(--b2));
  font-weight: 600;
}

.progress {
  height: 0.5rem;
}

.card-header {
  border-bottom: 1px solid hsl(var(--b3));
}
</style>