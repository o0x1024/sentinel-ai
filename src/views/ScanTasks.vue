<template>
  <div class="space-y-6">
    <!-- 页面标题和操作 -->
    <div class="flex items-center justify-between">
      <h1 class="text-3xl font-bold">{{ $t('scanTasks.title') }}</h1>
      <div class="flex space-x-2">
        <button @click="showCreateModal = true" class="btn btn-primary">
          <svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"></path>
          </svg>
          {{ $t('scanTasks.newScan') }}
        </button>
        <button @click="refreshTasks" class="btn btn-outline">
          <svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"></path>
          </svg>
          {{ $t('common.refresh') }}
        </button>
      </div>
    </div>

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
            <option value="web">{{ $t('scanTasks.scanTypes.webScan') }}</option>
          </select>
        </div>
      </div>
    </div>

    <!-- 任务列表 -->
    <div class="bg-base-100 rounded-lg shadow-sm border border-base-300">
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
            <tr v-for="task in filteredTasks" :key="task.id">
              <td>
                <div class="font-medium">{{ task.name }}</div>
                <div class="text-xs opacity-70">{{ task.id }}</div>
              </td>
              <td>
                <div class="font-medium">{{ task.target }}</div>
                <div class="text-xs opacity-70">{{ task.config.tools.join(', ') }}</div>
              </td>
              <td>
                <div class="badge badge-outline">{{ getTaskTypeLabel(task.config.tools[0]) }}</div>
              </td>
              <td>
                <div :class="getStatusBadgeClass(task.status)" class="badge">
                  {{ getStatusLabel(task.status) }}
                </div>
              </td>
              <td>
                <div class="flex items-center space-x-2">
                  <progress 
                    :class="getProgressClass(task.status)" 
                    class="progress w-20" 
                    :value="task.progress * 100" 
                    max="100"
                  ></progress>
                  <span class="text-sm">{{ Math.round(task.progress * 100) }}%</span>
                </div>
                <div v-if="task.status === 'Running'" class="text-xs opacity-70 mt-1">
                  {{ $t('scanTasks.duration') }}: {{ formatDuration(task.estimated_remaining) }}
                </div>
              </td>
              <td>
                <div v-if="task.vulnerabilities_found > 0" class="badge badge-error badge-sm">
                  {{ task.vulnerabilities_found }} {{ $t('vulnerabilities.title') }}
                </div>
                <div v-else-if="task.assets_found > 0" class="badge badge-info badge-sm">
                  {{ task.assets_found }} {{ $t('common.target') }}
                </div>
                <div v-else class="text-xs opacity-70">-</div>
              </td>
              <td class="text-sm opacity-70">
                {{ formatTime(task.created_at) }}
                <div v-if="task.completed_at" class="text-xs opacity-50">
                  {{ $t('common.completed') }}: {{ formatTime(task.completed_at) }}
                </div>
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
                    v-if="task.status === 'Failed'"
                    @click="retryTask(task.id)"
                    class="btn btn-xs btn-warning"
                    :title="$t('scanTasks.resume')"
                  >
                    <svg class="w-3 h-3" fill="currentColor" viewBox="0 0 20 20">
                      <path fill-rule="evenodd" d="M4 2a1 1 0 011 1v2.101a7.002 7.002 0 0111.601 2.566 1 1 0 11-1.885.666A5.002 5.002 0 005.999 7H9a1 1 0 010 2H4a1 1 0 01-1-1V3a1 1 0 011-1zm.008 9.057a1 1 0 011.276.61A5.002 5.002 0 0014.001 13H11a1 1 0 110-2h5a1 1 0 011 1v5a1 1 0 11-2 0v-2.101a7.002 7.002 0 01-11.601-2.566 1 1 0 01.61-1.276z" clip-rule="evenodd"></path>
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
                  <div class="dropdown dropdown-end">
                    <button class="btn btn-xs btn-ghost" :title="$t('common.moreInfo')">
                      <svg class="w-3 h-3" fill="currentColor" viewBox="0 0 20 20">
                        <path d="M10 6a2 2 0 110-4 2 2 0 010 4zM10 12a2 2 0 110-4 2 2 0 010 4zM10 18a2 2 0 110-4 2 2 0 010 4z"></path>
                      </svg>
                    </button>
                    <ul class="dropdown-content menu p-2 shadow bg-base-100 rounded-box w-32">
                      <li><a @click="exportTaskReport(task.id)">{{ $t('scanTasks.export') }}</a></li>
                      <li><a @click="cloneTask(task)">{{ $t('common.create') }}</a></li>
                      <li><a @click="deleteTask(task.id)" class="text-error">{{ $t('common.delete') }}</a></li>
                    </ul>
                  </div>
                </div>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>

    <!-- 创建任务模态框 -->
    <div v-if="showCreateModal" class="modal modal-open">
      <div class="modal-box w-11/12 max-w-2xl">
        <h3 class="font-bold text-lg mb-4">{{ $t('scanTasks.newScan') }}</h3>
        
        <form @submit.prevent="createTask" class="space-y-4">
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ $t('scanTasks.taskName') }}</span>
            </label>
            <input 
              v-model="newTask.name" 
              type="text" 
              :placeholder="$t('scanTasks.form.namePlaceholder')" 
              class="input input-bordered" 
              required
            />
          </div>

          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ $t('scanTasks.targetUrl') }}</span>
            </label>
            <input 
              v-model="newTask.target" 
              type="text" 
              :placeholder="$t('scanTasks.form.targetPlaceholder')" 
              class="input input-bordered" 
              required
            />
          </div>

          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ $t('scanTasks.scanOptions') }}</span>
            </label>
            <div class="grid grid-cols-2 gap-2">
              <label v-for="tool in availableTools" :key="tool.id" class="label cursor-pointer">
                <input 
                  v-model="newTask.config.tools" 
                  :value="tool.id" 
                  type="checkbox" 
                  class="checkbox checkbox-primary" 
                />
                <span class="label-text">{{ tool.name }}</span>
              </label>
            </div>
          </div>

          <div class="grid grid-cols-2 gap-4">
            <div class="form-control">
              <label class="label">
                <span class="label-text">{{ $t('scanTasks.scanScope') }}</span>
              </label>
              <select v-model="newTask.config.depth" class="select select-bordered">
                <option value="1">{{ $t('scanTasks.scanTypes.quickScan') }}</option>
                <option value="2">{{ $t('scanTasks.scanTypes.fullScan') }}</option>
                <option value="3">{{ $t('scanTasks.scanTypes.vulnerabilityScan') }}</option>
              </select>
            </div>

            <div class="form-control">
              <label class="label">
                <span class="label-text">{{ $t('settings.scan.defaultTimeout') }}</span>
              </label>
              <input 
                v-model.number="newTask.config.timeout" 
                type="number" 
                min="1" 
                max="1440" 
                class="input input-bordered" 
              />
            </div>
          </div>

          <div class="form-control">
            <label class="label cursor-pointer">
              <span class="label-text">{{ $t('scanTasks.scanScope') }}</span>
              <input 
                v-model="newTask.config.include_subdomains" 
                type="checkbox" 
                class="checkbox checkbox-primary" 
              />
            </label>
          </div>

          <div class="modal-action">
            <button type="button" @click="showCreateModal = false" class="btn">{{ $t('common.cancel') }}</button>
            <button type="submit" class="btn btn-primary" :disabled="isCreating">
              <span v-if="isCreating" class="loading loading-spinner loading-sm"></span>
              {{ isCreating ? $t('common.saving') : $t('common.create') }}
            </button>
          </div>
        </form>
      </div>
    </div>

    <!-- 任务详情模态框 -->
    <div v-if="showDetailsModal && selectedTask" class="modal modal-open">
      <div class="modal-box w-11/12 max-w-4xl">
        <h3 class="font-bold text-lg mb-4">{{ $t('scanTasks.taskDetails') }}</h3>
        
        <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
          <div class="space-y-4">
            <div>
              <h4 class="font-semibold mb-2">{{ $t('common.details') }}</h4>
              <div class="space-y-2 text-sm">
                <div><span class="opacity-70">{{ $t('common.id') }}:</span> {{ selectedTask.id }}</div>
                <div><span class="opacity-70">{{ $t('scanTasks.taskName') }}:</span> {{ selectedTask.name }}</div>
                <div><span class="opacity-70">{{ $t('common.target') }}:</span> {{ selectedTask.target }}</div>
                <div><span class="opacity-70">{{ $t('common.status') }}:</span> 
                  <div :class="getStatusBadgeClass(selectedTask.status)" class="badge badge-sm">
                    {{ getStatusLabel(selectedTask.status) }}
                  </div>
                </div>
                <div><span class="opacity-70">{{ $t('common.progress') }}:</span> {{ Math.round(selectedTask.progress * 100) }}%</div>
              </div>
            </div>

            <div>
              <h4 class="font-semibold mb-2">{{ $t('common.time') }}</h4>
              <div class="space-y-2 text-sm">
                <div><span class="opacity-70">{{ $t('scanTasks.startTime') }}:</span> {{ formatDateTime(selectedTask.created_at) }}</div>
                <div v-if="selectedTask.started_at"><span class="opacity-70">{{ $t('scanTasks.startTime') }}:</span> {{ formatDateTime(selectedTask.started_at) }}</div>
                <div v-if="selectedTask.completed_at"><span class="opacity-70">{{ $t('scanTasks.endTime') }}:</span> {{ formatDateTime(selectedTask.completed_at) }}</div>
              </div>
            </div>
          </div>

          <div class="space-y-4">
            <div>
              <h4 class="font-semibold mb-2">{{ $t('scanTasks.scanParameters') }}</h4>
              <div class="space-y-2 text-sm">
                <div><span class="opacity-70">{{ $t('scanTasks.scanOptions') }}:</span> {{ selectedTask.config.tools.join(', ') }}</div>
                <div><span class="opacity-70">{{ $t('scanTasks.scanScope') }}:</span> {{ selectedTask.config.depth }}</div>
                <div><span class="opacity-70">{{ $t('settings.scan.defaultTimeout') }}:</span> {{ selectedTask.config.timeout }}</div>
                <div><span class="opacity-70">{{ $t('scanTasks.scanScope') }}:</span> {{ selectedTask.config.include_subdomains ? $t('common.confirm') : $t('common.cancel') }}</div>
              </div>
            </div>

            <div>
              <h4 class="font-semibold mb-2">{{ $t('scanTasks.results') }}</h4>
              <div class="space-y-2 text-sm">
                <div><span class="opacity-70">{{ $t('vulnerabilities.title') }}:</span> {{ selectedTask.vulnerabilities_found }}</div>
                <div><span class="opacity-70">{{ $t('scanTasks.results') }}:</span> {{ selectedTask.results_count }}</div>
                <div v-if="selectedTask.error_message" class="text-error">
                  <span class="opacity-70">{{ $t('common.error') }}:</span> {{ selectedTask.error_message }}
                </div>
              </div>
            </div>
          </div>
        </div>

        <div class="modal-action">
          <button @click="showDetailsModal = false" class="btn">{{ $t('common.close') }}</button>
          <button v-if="selectedTask.status === 'Completed'" class="btn btn-primary">{{ $t('scanTasks.viewReport') }}</button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { invoke } from '@tauri-apps/api/core';

const { t } = useI18n();

// 类型定义
interface ScanTask {
  id: string;
  name: string;
  target: string;
  config: {
    tools: string[];
    depth: number;
    timeout: number;
    include_subdomains: boolean;
  };
  status: string;
  progress: number;
  created_at: string;
  started_at?: string;
  completed_at?: string;
  results_count: number;
  vulnerabilities_found: number;
  assets_found: number;
  estimated_remaining?: number;
  error_message?: string;
}

interface Tool {
  id: string;
  name: string;
  description: string;
}

// 响应式数据
const tasks = ref<ScanTask[]>([
  {
    id: 'task-1',
    name: t('scanTasks.scanTypes.webScan') + ' - example.com',
    target: 'example.com',
    config: {
      tools: ['subfinder'],
      depth: 2,
      timeout: 60,
      include_subdomains: true
    },
    status: 'Running',
    progress: 0.65,
    created_at: new Date(Date.now() - 1000 * 60 * 30).toISOString(),
    started_at: new Date(Date.now() - 1000 * 60 * 25).toISOString(),
    results_count: 0,
    vulnerabilities_found: 0,
    assets_found: 12,
    estimated_remaining: 300
  },
  {
    id: 'task-2',
    name: t('scanTasks.scanTypes.vulnerabilityScan') + ' - api.example.com',
    target: 'api.example.com',
    config: {
      tools: ['nuclei'],
      depth: 1,
      timeout: 30,
      include_subdomains: false
    },
    status: 'Completed',
    progress: 1.0,
    created_at: new Date(Date.now() - 1000 * 60 * 60 * 2).toISOString(),
    started_at: new Date(Date.now() - 1000 * 60 * 60 * 2 + 1000 * 60).toISOString(),
    completed_at: new Date(Date.now() - 1000 * 60 * 60).toISOString(),
    results_count: 15,
    vulnerabilities_found: 3,
    assets_found: 8
  }
]);

const searchQuery = ref('');
const statusFilter = ref('');
const typeFilter = ref('');
const showCreateModal = ref(false);
const showDetailsModal = ref(false);
const selectedTask = ref<ScanTask | null>(null);
const isCreating = ref(false);

const newTask = ref({
  name: '',
  target: '',
  config: {
    tools: [] as string[],
    depth: 1,
    timeout: 60,
    include_subdomains: false
  }
});

const availableTools = ref<Tool[]>([
  { id: 'subfinder', name: 'Subfinder', description: t('scanTasks.scanTypes.webScan') },
  { id: 'nuclei', name: 'Nuclei', description: t('scanTasks.scanTypes.vulnerabilityScan') },
  { id: 'httpx', name: 'Httpx', description: t('scanTasks.scanTypes.webScan') },
  { id: 'nmap', name: 'Nmap', description: t('scanTasks.scanTypes.portScan') }
]);

// 计算属性
const filteredTasks = computed(() => {
  return tasks.value.filter(task => {
    const matchesSearch = task.name.toLowerCase().includes(searchQuery.value.toLowerCase()) ||
                         task.target.toLowerCase().includes(searchQuery.value.toLowerCase());
    const matchesStatus = !statusFilter.value || task.status === statusFilter.value;
    const matchesType = !typeFilter.value || task.config.tools.includes(typeFilter.value);
    
    return matchesSearch && matchesStatus && matchesType;
  });
});

// 方法
const refreshTasks = async () => {
  try {
    // 这里将来会调用后端API
    console.log(t('common.refresh'));
  } catch (error) {
    console.error(t('common.error'), error);
  }
};

const createTask = async () => {
  if (newTask.value.config.tools.length === 0) {
    alert(t('scanTasks.form.selectScanType'));
    return;
  }

  isCreating.value = true;
  try {
    // 调用后端API创建任务
    const response = await invoke('create_scan_task', {
      target: newTask.value.target,
      config: newTask.value.config
    });
    
    console.log(t('scanTasks.notifications.scanStarted'), response);
    showCreateModal.value = false;
    
    // 重置表单
    newTask.value = {
      name: '',
      target: '',
      config: {
        tools: [],
        depth: 1,
        timeout: 60,
        include_subdomains: false
      }
    };
    
    await refreshTasks();
  } catch (error) {
    console.error(t('scanTasks.notifications.scanFailed'), error);
    alert(t('scanTasks.notifications.scanFailed'));
  } finally {
    isCreating.value = false;
  }
};

const stopTask = async (taskId: string) => {
  try {
    await invoke('stop_scan_task', { taskId });
    await refreshTasks();
  } catch (error) {
    console.error(t('scanTasks.notifications.scanStopped'), error);
  }
};

const deleteTask = async (taskId: string) => {
  if (confirm(t('common.confirm'))) {
    try {
      await invoke('delete_scan_task', { taskId });
      tasks.value = tasks.value.filter(task => task.id !== taskId);
    } catch (error) {
      console.error(t('scanTasks.notifications.scanDeleted'), error);
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

const getTaskTypeLabel = (tool: string) => {
  const labels: Record<string, string> = {
    'subfinder': t('scanTasks.scanTypes.webScan'),
    'nuclei': t('scanTasks.scanTypes.vulnerabilityScan'),
    'httpx': t('scanTasks.scanTypes.webScan'),
    'nmap': t('scanTasks.scanTypes.portScan')
  };
  return labels[tool] || tool;
};

const formatTime = (dateString: string) => {
  const date = new Date(dateString);
  const now = new Date();
  const diff = now.getTime() - date.getTime();
  const minutes = Math.floor(diff / (1000 * 60));
  const hours = Math.floor(diff / (1000 * 60 * 60));
  const days = Math.floor(diff / (1000 * 60 * 60 * 24));

  if (days > 0) return `${days}${t('common.date')}`;
  if (hours > 0) return `${hours}${t('common.time')}`;
  if (minutes > 0) return `${minutes}${t('common.time')}`;
  return t('common.time');
};

const formatDateTime = (dateString: string) => {
  return new Date(dateString).toLocaleString();
};

const formatDuration = (seconds: number | undefined) => {
  if (!seconds) return '-';
  const mins = Math.floor(seconds / 60);
  const secs = seconds % 60;
  return `${mins}${t('common.time')}${secs}${t('common.time')}`;
};

const retryTask = async (taskId: string) => {
  try {
    await invoke('retry_scan_task', { taskId });
    await refreshTasks();
  } catch (error) {
    console.error(t('scanTasks.notifications.scanFailed'), error);
  }
};

const exportTaskReport = async (taskId: string) => {
  try {
    await invoke('export_task_report', { taskId });
    // 可以添加下载提示
  } catch (error) {
    console.error(t('common.error'), error);
  }
};

const cloneTask = (task: ScanTask) => {
  newTask.value = {
    name: `${task.name} (${t('common.create')})`,
    target: task.target,
    config: { ...task.config }
  };
  showCreateModal.value = true;
};

// 生命周期
onMounted(() => {
  refreshTasks();
});
</script> 