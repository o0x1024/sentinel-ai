<template>
  <div class="container mx-auto p-6">
    <div class="mb-6">
      <h1 class="text-3xl font-bold text-gray-800 mb-2">工作流监控</h1>
      <p class="text-gray-600">查看和管理工作流执行状态</p>
    </div>

    <!-- 统计信息卡片 -->
    <div class="grid grid-cols-1 md:grid-cols-4 gap-4 mb-6">
      <div class="stat bg-base-100 shadow rounded-lg">
        <div class="stat-title">总工作流</div>
        <div class="stat-value text-primary">{{ statistics.total_workflows || 0 }}</div>
      </div>
      <div class="stat bg-base-100 shadow rounded-lg">
        <div class="stat-title">总执行次数</div>
        <div class="stat-value text-secondary">{{ statistics.total_executions || 0 }}</div>
      </div>
      <div class="stat bg-base-100 shadow rounded-lg">
        <div class="stat-title">成功执行</div>
        <div class="stat-value text-success">{{ statistics.successful_executions || 0 }}</div>
      </div>
      <div class="stat bg-base-100 shadow rounded-lg">
        <div class="stat-title">正在运行</div>
        <div class="stat-value text-warning">{{ statistics.running_executions || 0 }}</div>
      </div>
    </div>

    <!-- 操作按钮 -->
    <div class="flex gap-4 mb-6">
      <button class="btn btn-primary" @click="refreshExecutions">
        <svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
        </svg>
        刷新
      </button>
      <button class="btn btn-outline" @click="refreshStatistics">
        <svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z" />
        </svg>
        更新统计
      </button>
    </div>

    <!-- 执行列表 -->
    <div class="card bg-base-100 shadow-xl">
      <div class="card-header">
        <h2 class="card-title text-xl font-semibold">工作流执行列表</h2>
      </div>
      <div class="card-body">
        <div v-if="loading" class="flex justify-center py-8">
          <span class="loading loading-spinner loading-lg"></span>
        </div>
        
        <div v-else-if="executions.length === 0" class="text-center py-8 text-gray-500">
          <svg class="w-16 h-16 mx-auto mb-4 text-gray-300" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5H7a2 2 0 00-2 2v10a2 2 0 002 2h8a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2" />
          </svg>
          <p>暂无工作流执行记录</p>
        </div>

        <div v-else class="overflow-x-auto">
          <table class="table table-zebra w-full">
            <thead>
              <tr>
                <th>执行ID</th>
                <th>工作流ID</th>
                <th>状态</th>
                <th>进度</th>
                <th>当前步骤</th>
                <th>开始时间</th>
                <th>操作</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="execution in executions" :key="execution.execution_id">
                <td>
                  <code class="text-sm bg-gray-100 px-2 py-1 rounded">{{ execution.execution_id.substring(0, 8) }}...</code>
                </td>
                <td>{{ execution.workflow_id }}</td>
                <td>
                  <div class="badge" :class="getStatusBadgeClass(execution.status)">{{ getStatusText(execution.status) }}</div>
                </td>
                <td>
                  <div class="flex items-center gap-2">
                    <progress class="progress w-20" :class="getProgressClass(execution.status)" :value="execution.progress" max="100"></progress>
                    <span class="text-sm">{{ Math.round(execution.progress) }}%</span>
                  </div>
                </td>
                <td>
                  <span v-if="execution.current_step" class="text-sm">{{ execution.current_step }}</span>
                  <span v-else class="text-gray-400 text-sm">-</span>
                </td>
                <td>{{ formatTime(execution.started_at) }}</td>
                <td>
                  <div class="flex gap-2">
                    <button class="btn btn-sm btn-outline" @click="viewExecution(execution.execution_id)">
                      查看
                    </button>
                    <button 
                      v-if="execution.status === 'Running'" 
                      class="btn btn-sm btn-error" 
                      @click="cancelExecution(execution.execution_id)"
                    >
                      取消
                    </button>
                  </div>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
    </div>

    <!-- 执行详情模态框 -->
    <dialog ref="executionModal" class="modal">
      <div class="modal-box w-11/12 max-w-4xl">
        <h3 class="font-bold text-lg mb-4">执行详情</h3>
        <div v-if="selectedExecution" class="space-y-4">
          <div class="grid grid-cols-2 gap-4">
            <div>
              <label class="label">执行ID</label>
              <code class="block bg-gray-100 p-2 rounded text-sm">{{ selectedExecution.execution_id }}</code>
            </div>
            <div>
              <label class="label">工作流ID</label>
              <div class="bg-gray-100 p-2 rounded text-sm">{{ selectedExecution.workflow_id }}</div>
            </div>
            <div>
              <label class="label">状态</label>
              <div class="badge" :class="getStatusBadgeClass(selectedExecution.status)">{{ getStatusText(selectedExecution.status) }}</div>
            </div>
            <div>
              <label class="label">进度</label>
              <div class="flex items-center gap-2">
                <progress class="progress flex-1" :class="getProgressClass(selectedExecution.status)" :value="selectedExecution.progress" max="100"></progress>
                <span class="text-sm">{{ Math.round(selectedExecution.progress) }}%</span>
              </div>
            </div>
            <div>
              <label class="label">开始时间</label>
              <div class="bg-gray-100 p-2 rounded text-sm">{{ formatTime(selectedExecution.started_at) }}</div>
            </div>
            <div>
              <label class="label">完成时间</label>
              <div class="bg-gray-100 p-2 rounded text-sm">{{ selectedExecution.completed_at ? formatTime(selectedExecution.completed_at) : '-' }}</div>
            </div>
          </div>
          
          <div v-if="selectedExecution.error" class="alert alert-error">
            <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
            </svg>
            <div>
              <h4 class="font-bold">执行错误</h4>
              <p>{{ selectedExecution.error }}</p>
            </div>
          </div>
        </div>
        
        <div class="modal-action">
          <button class="btn" @click="closeModal">关闭</button>
        </div>
      </div>
    </dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'

interface WorkflowExecution {
  execution_id: string
  workflow_id: string
  status: string
  started_at: string
  completed_at?: string
  current_step?: string
  total_steps: number
  completed_steps: number
  progress: number
  error?: string
  result?: any
}

interface WorkflowStatistics {
  total_workflows: number
  total_executions: number
  successful_executions: number
  failed_executions: number
  running_executions: number
}

const executions = ref<WorkflowExecution[]>([])
const statistics = ref<WorkflowStatistics>({
  total_workflows: 0,
  total_executions: 0,
  successful_executions: 0,
  failed_executions: 0,
  running_executions: 0
})
const loading = ref(false)
const selectedExecution = ref<WorkflowExecution | null>(null)
const executionModal = ref<HTMLDialogElement>()

// 获取执行列表
const refreshExecutions = async () => {
  loading.value = true
  try {
    const result = await invoke<WorkflowExecution[]>('list_workflow_executions')
    executions.value = result
  } catch (error) {
    console.error('获取执行列表失败:', error)
  } finally {
    loading.value = false
  }
}

// 获取统计信息
const refreshStatistics = async () => {
  try {
    const result = await invoke<WorkflowStatistics>('get_workflow_statistics')
    statistics.value = result
  } catch (error) {
    console.error('获取统计信息失败:', error)
  }
}

// 查看执行详情
const viewExecution = async (executionId: string) => {
  try {
    const result = await invoke<WorkflowExecution>('get_workflow_execution', { executionId })
    if (result) {
      selectedExecution.value = result
      executionModal.value?.showModal()
    }
  } catch (error) {
    console.error('获取执行详情失败:', error)
  }
}

// 取消执行
const cancelExecution = async (executionId: string) => {
  try {
    await invoke('cancel_workflow_execution', { executionId })
    await refreshExecutions()
  } catch (error) {
    console.error('取消执行失败:', error)
  }
}

// 关闭模态框
const closeModal = () => {
  executionModal.value?.close()
  selectedExecution.value = null
}

// 获取状态徽章样式
const getStatusBadgeClass = (status: string) => {
  const statusMap: Record<string, string> = {
    'Pending': 'badge-ghost',
    'Running': 'badge-primary',
    'Completed': 'badge-success',
    'Failed': 'badge-error',
    'Cancelled': 'badge-warning',
    'Paused': 'badge-info'
  }
  return statusMap[status] || 'badge-ghost'
}

// 获取状态文本
const getStatusText = (status: string) => {
  const statusMap: Record<string, string> = {
    'Pending': '等待中',
    'Running': '运行中',
    'Completed': '已完成',
    'Failed': '失败',
    'Cancelled': '已取消',
    'Paused': '已暂停'
  }
  return statusMap[status] || status
}

// 获取进度条样式
const getProgressClass = (status: string) => {
  const progressMap: Record<string, string> = {
    'Pending': 'progress-ghost',
    'Running': 'progress-primary',
    'Completed': 'progress-success',
    'Failed': 'progress-error',
    'Cancelled': 'progress-warning',
    'Paused': 'progress-info'
  }
  return progressMap[status] || 'progress-ghost'
}

// 格式化时间
const formatTime = (timeStr: string) => {
  return new Date(timeStr).toLocaleString('zh-CN')
}

// 组件挂载时获取数据
onMounted(() => {
  refreshExecutions()
  refreshStatistics()
})
</script>

<style scoped>
.card-header {
  @apply p-6 pb-0;
}
</style>