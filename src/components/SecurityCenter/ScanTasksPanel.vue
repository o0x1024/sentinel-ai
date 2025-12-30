<template>
  <div class="space-y-6">
    <!-- 操作栏 -->
    <div class="flex items-center justify-between">
      <h2 class="text-xl font-semibold">{{ $t('scanTasks.title') }}</h2>
      <div class="flex space-x-2">
        <button 
          @click="showTestPanel = !showTestPanel" 
          class="btn btn-sm"
          :class="showTestPanel ? 'btn-warning' : 'btn-outline btn-warning'"
        >
          <i class="fas fa-flask mr-2"></i>
          {{ showTestPanel ? '隐藏测试面板' : '🧪 测试追踪功能' }}
        </button>
        <button @click="showCreateModal = true" class="btn btn-primary btn-sm">
          <svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"></path>
          </svg>
          {{ $t('scanTasks.newScan') }}
        </button>
        <button @click="refreshTasks" class="btn btn-outline btn-sm">
          <svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"></path>
          </svg>
          {{ $t('common.refresh') }}
        </button>
      </div>
    </div>

    <!-- 测试面板 -->
    <TrackingTestPanel v-if="showTestPanel" />

    <!-- 筛选和搜索 -->
    <div class="bg-base-100 rounded-lg p-4 shadow-sm border border-base-300">
      <div class="flex flex-wrap gap-4 items-center">
        <div class="form-control">
          <input 
            v-model="searchQuery" 
            type="text" 
            :placeholder="$t('common.search') + '...'" 
            class="input input-bordered input-sm w-64"
          />
        </div>
        <div class="form-control">
          <select v-model="statusFilter" class="select select-bordered select-sm">
            <option value="">{{ $t('scanTasks.filters.all') }}</option>
            <option value="Pending">{{ $t('common.pending') }}</option>
            <option value="Running">{{ $t('common.inProgress') }}</option>
            <option value="Completed">{{ $t('common.completed') }}</option>
            <option value="Failed">{{ $t('common.failed') }}</option>
            <option value="Cancelled">{{ $t('common.cancelled') }}</option>
          </select>
        </div>
        <div class="form-control">
          <select v-model="typeFilter" class="select select-bordered select-sm">
            <option value="">{{ $t('scanTasks.filters.all') }}</option>
            <option value="subdomain">{{ $t('scanTasks.scanTypes.webScan') }}</option>
            <option value="port">{{ $t('scanTasks.scanTypes.portScan') }}</option>
            <option value="vulnerability">{{ $t('scanTasks.scanTypes.vulnerabilityScan') }}</option>
          </select>
        </div>
      </div>
    </div>

    <!-- 任务列表 -->
    <div class="bg-base-100 rounded-lg shadow-sm border border-base-300 overflow-hidden">
      <div class="overflow-x-auto">
        <table class="table table-zebra w-full">
          <thead>
            <tr>
              <th>{{ $t('scanTasks.taskName') }}</th>
              <th>{{ $t('common.target') }}</th>
              <th>{{ $t('common.type') }}</th>
              <th>{{ $t('common.status') }}</th>
              <th>{{ $t('common.progress') }}</th>
              <th>{{ $t('scanTasks.vulnerabilitiesFound') }}</th>
              <th>{{ $t('scanTasks.startTime') }}</th>
              <th>{{ $t('common.actions') }}</th>
            </tr>
          </thead>
          <tbody>
            <tr v-if="filteredTasks.length === 0">
              <td colspan="8" class="text-center py-8 text-base-content/50">
                {{ $t('scanTasks.noTasks') }}
              </td>
            </tr>
            <tr v-for="task in filteredTasks" :key="task.id">
              <td>
                <div class="font-medium">{{ task.name }}</div>
                <div class="text-xs opacity-70">{{ task.id }}</div>
              </td>
              <td>
                <div class="font-medium">{{ task.target }}</div>
              </td>
              <td>
                <div class="badge badge-outline badge-sm">{{ task.config?.tools?.[0] || 'N/A' }}</div>
              </td>
              <td>
                <div :class="getStatusBadgeClass(task.status)" class="badge badge-sm">
                  {{ getStatusLabel(task.status) }}
                </div>
              </td>
              <td>
                <div class="flex items-center space-x-2">
                  <progress 
                    :class="getProgressClass(task.status)" 
                    class="progress w-20" 
                    :value="(task.progress || 0) * 100" 
                    max="100"
                  ></progress>
                  <span class="text-sm">{{ Math.round((task.progress || 0) * 100) }}%</span>
                </div>
              </td>
              <td>
                <div v-if="task.vulnerabilities_found > 0" class="badge badge-error badge-sm">
                  {{ task.vulnerabilities_found }}
                </div>
                <div v-else class="text-xs opacity-70">0</div>
              </td>
              <td class="text-sm opacity-70">
                {{ formatTime(task.created_at) }}
              </td>
              <td>
                <div class="flex space-x-1">
                  <button 
                    v-if="task.status === 'Pending' || task.status === 'Running'"
                    @click="stopTask(task.id)"
                    class="btn btn-xs btn-error"
                    :title="$t('scanTasks.stop')"
                  >
                    <svg class="w-3 h-3" fill="currentColor" viewBox="0 0 20 20">
                      <path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zM8 7a1 1 0 00-1 1v4a1 1 0 001 1h4a1 1 0 001-1V8a1 1 0 00-1-1H8z" clip-rule="evenodd"></path>
                    </svg>
                  </button>
                  <button 
                    @click="viewTaskDetails(task)"
                    class="btn btn-xs btn-info"
                    :title="$t('common.viewDetails')"
                  >
                    <svg class="w-3 h-3" fill="currentColor" viewBox="0 0 20 20">
                      <path d="M10 12a2 2 0 100-4 2 2 0 000 4z"></path>
                      <path fill-rule="evenodd" d="M.458 10C1.732 5.943 5.522 3 10 3s8.268 2.943 9.542 7c-1.274 4.057-5.064 7-9.542 7S1.732 14.057.458 10zM14 10a4 4 0 11-8 0 4 4 0 018 0z" clip-rule="evenodd"></path>
                    </svg>
                  </button>
                  <button 
                    @click="deleteTask(task.id)" 
                    class="btn btn-xs btn-ghost text-error"
                    :title="$t('common.delete')"
                  >
                    <svg class="w-3 h-3" fill="currentColor" viewBox="0 0 20 20">
                      <path fill-rule="evenodd" d="M9 2a1 1 0 00-.894.553L7.382 4H4a1 1 0 000 2v10a2 2 0 002 2h8a2 2 0 002-2V6a1 1 0 100-2h-3.382l-.724-1.447A1 1 0 0011 2H9zM7 8a1 1 0 012 0v6a1 1 0 11-2 0V8zm5-1a1 1 0 00-1 1v6a1 1 0 102 0V8a1 1 0 00-1-1z" clip-rule="evenodd"></path>
                    </svg>
                  </button>
                </div>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>

    <!-- 创建任务模态框 (简化版) -->
    <div v-if="showCreateModal" class="modal modal-open">
      <div class="modal-box">
        <h3 class="font-bold text-lg mb-4">{{ $t('scanTasks.newScan') }}</h3>
        <p class="text-sm text-base-content/70 mb-4">{{ $t('scanTasks.createTaskHint') }}</p>
        <div class="modal-action">
          <button @click="showCreateModal = false" class="btn btn-sm">{{ $t('common.close') }}</button>
        </div>
      </div>
    </div>

    <!-- 任务详情模态框 (简化版) -->
    <div v-if="showDetailsModal && selectedTask" class="modal modal-open">
      <div class="modal-box">
        <h3 class="font-bold text-lg mb-4">{{ $t('scanTasks.taskDetails') }}</h3>
        <div class="space-y-2">
          <p><strong>{{ $t('scanTasks.taskName') }}:</strong> {{ selectedTask.name }}</p>
          <p><strong>{{ $t('common.target') }}:</strong> {{ selectedTask.target }}</p>
          <p><strong>{{ $t('common.status') }}:</strong> {{ selectedTask.status }}</p>
        </div>
        <div class="modal-action">
          <button @click="showDetailsModal = false" class="btn btn-sm">{{ $t('common.close') }}</button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { invoke } from '@tauri-apps/api/core';
import { dialog } from '@/composables/useDialog';
import TrackingTestPanel from '@/components/ScanTasks/TrackingTestPanel.vue';

const { t } = useI18n();
const emit = defineEmits<{
  'stats-updated': [stats: { total: number; running: number }]
}>();

interface ScanTask {
  id: string;
  name: string;
  target: string;
  config?: {
    tools: string[];
  };
  status: string;
  progress?: number;
  created_at: string;
  vulnerabilities_found: number;
}

const tasks = ref<ScanTask[]>([]);
const searchQuery = ref('');
const statusFilter = ref('');
const typeFilter = ref('');
const showCreateModal = ref(false);
const showTestPanel = ref(false);
const showDetailsModal = ref(false);
const selectedTask = ref<ScanTask | null>(null);

const filteredTasks = computed(() => {
  return tasks.value.filter(task => {
    const matchesSearch = task.name.toLowerCase().includes(searchQuery.value.toLowerCase()) ||
                         task.target.toLowerCase().includes(searchQuery.value.toLowerCase());
    const matchesStatus = !statusFilter.value || task.status === statusFilter.value;
    const matchesType = !typeFilter.value || task.config?.tools?.includes(typeFilter.value);
    
    return matchesSearch && matchesStatus && matchesType;
  });
});

const refreshTasks = async () => {
  try {
    const response = await invoke('get_scan_tasks');
    tasks.value = response as ScanTask[];
    
    // 更新统计信息
    const stats = {
      total: tasks.value.length,
      running: tasks.value.filter(t => t.status === 'Running').length
    };
    emit('stats-updated', stats);
  } catch (error) {
    console.error('Failed to load scan tasks:', error);
    // 使用模拟数据
    tasks.value = [];
  }
};

const stopTask = async (taskId: string) => {
  try {
    await invoke('stop_scan_task', { taskId });
    await dialog.toast.success(t('scanTasks.notifications.scanStopped'));
    await refreshTasks();
  } catch (error) {
    console.error('Failed to stop task:', error);
  }
};

const deleteTask = async (taskId: string) => {
  const confirmed = await dialog.confirm({
    message: t('scanTasks.notifications.confirmDelete'),
    title: t('common.confirm')
  });
  
  if (confirmed) {
    try {
      await invoke('delete_scan_task', { taskId });
      await dialog.toast.success(t('scanTasks.notifications.scanDeleted'));
      await refreshTasks();
    } catch (error) {
      console.error('Failed to delete task:', error);
    }
  }
};

const viewTaskDetails = (task: ScanTask) => {
  selectedTask.value = task;
  showDetailsModal.value = true;
};

const getStatusBadgeClass = (status: string) => {
  switch (status) {
    case 'Running': return 'badge-info';
    case 'Completed': return 'badge-success';
    case 'Failed': return 'badge-error';
    case 'Pending': return 'badge-warning';
    case 'Cancelled': return 'badge-neutral';
    default: return 'badge-ghost';
  }
};

const getProgressClass = (status: string) => {
  switch (status) {
    case 'Running': return 'progress-info';
    case 'Completed': return 'progress-success';
    case 'Failed': return 'progress-error';
    default: return 'progress';
  }
};

const getStatusLabel = (status: string) => {
  const labels: Record<string, string> = {
    'Pending': t('common.pending'),
    'Running': t('common.inProgress'),
    'Completed': t('common.completed'),
    'Failed': t('common.failed'),
    'Cancelled': t('common.cancelled')
  };
  return labels[status] || status;
};

const formatTime = (dateString: string) => {
  if (!dateString) return '-';
  const date = new Date(dateString);
  const now = new Date();
  const diff = now.getTime() - date.getTime();
  const minutes = Math.floor(diff / (1000 * 60));
  const hours = Math.floor(diff / (1000 * 60 * 60));
  const days = Math.floor(diff / (1000 * 60 * 60 * 24));

  if (days > 0) return `${days}${t('common.daysAgo')}`;
  if (hours > 0) return `${hours}${t('common.hoursAgo')}`;
  if (minutes > 0) return `${minutes}${t('common.minutesAgo')}`;
  return t('common.justNow');
};

// 监听全局刷新事件
const handleRefresh = () => {
  refreshTasks();
};

onMounted(() => {
  refreshTasks();
  window.addEventListener('security-center-refresh', handleRefresh);
});

onUnmounted(() => {
  window.removeEventListener('security-center-refresh', handleRefresh);
});
</script>
