<template>
  <div class="page-content-padded safe-top space-y-6">
    <!-- 页面标题和操作 -->
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-3xl font-bold">{{ $t('scanSessions.title') }}</h1>
        <p class="text-base-content/70 mt-1">{{ $t('scanSessions.description') }}</p>
      </div>
      <div class="flex space-x-2">
        <button @click="showCreateModal = true" class="btn btn-primary">
          <svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"></path>
          </svg>
          {{ $t('scanSessions.newSession') }}
        </button>
        <button @click="refreshSessions" class="btn btn-outline">
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
            <option value="">{{ $t('scanSessions.filters.all') }}</option>
            <option value="pending">{{ $t('common.pending') }}</option>
            <option value="running">{{ $t('common.inProgress') }}</option>
            <option value="completed">{{ $t('common.completed') }}</option>
            <option value="failed">{{ $t('common.failed') }}</option>
            <option value="paused">{{ $t('scanSessions.paused') }}</option>
          </select>
        </div>
        <div class="form-control">
          <select v-model="typeFilter" class="select select-bordered select-sm">
            <option value="">{{ $t('scanSessions.filters.all') }}</option>
            <option value="comprehensive">{{ $t('scanSessions.types.comprehensive') }}</option>
            <option value="subdomain">{{ $t('scanSessions.types.subdomain') }}</option>
            <option value="rsubdomain">{{ $t('scanSessions.types.subdomain') }}</option>
            <option value="port">{{ $t('scanSessions.types.port') }}</option>
            <option value="rustscan">{{ $t('scanSessions.types.port') }}</option>
            <option value="vulnerability">{{ $t('scanSessions.types.vulnerability') }}</option>
          </select>
        </div>
      </div>
    </div>

    <!-- 会话列表 -->
    <div class="bg-base-100 rounded-lg shadow-sm border border-base-300">
      <div class="overflow-x-auto">
        <table class="table table-zebra w-full">
          <thead>
            <tr>
              <th>{{ $t('scanSessions.sessionName') }}</th>
              <th>{{ $t('common.target') }}</th>
              <th>{{ $t('common.type') }}</th>
              <th>{{ $t('common.status') }}</th>
              <th>{{ $t('scanSessions.currentStage') }}</th>
              <th>{{ $t('common.progress') }}</th>
              <th>{{ $t('scanSessions.assetsFound') }}</th>
              <th>{{ $t('scanSessions.startTime') }}</th>
              <th>{{ $t('common.actions') }}</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="session in filteredSessions" :key="session.id">
              <td>
                <div class="font-medium">{{ session.name }}</div>
                <div class="text-xs opacity-70">{{ session.id }}</div>
              </td>
              <td>
                <div class="font-medium">{{ session.target }}</div>
                <div class="text-xs opacity-70">{{ session.config.scan_type }}</div>
              </td>
              <td>
                <div class="badge badge-outline">{{ getTypeLabel(session.config.scan_type) }}</div>
              </td>
              <td>
                <div :class="getStatusBadgeClass(session.status)" class="badge">
                  {{ getStatusLabel(session.status) }}
                </div>
              </td>
              <td>
                <div v-if="session.current_stage" class="text-sm">
                  {{ getStageLabel(session.current_stage) }}
                  <div class="text-xs opacity-70">{{ session.stage_progress }}%</div>
                </div>
                <div v-else class="text-xs opacity-70">-</div>
              </td>
              <td>
                <div class="flex items-center space-x-2">
                  <progress 
                    :class="getProgressClass(session.status)" 
                    class="progress w-20" 
                    :value="session.overall_progress" 
                    max="100"
                  ></progress>
                  <span class="text-sm">{{ session.overall_progress }}%</span>
                </div>
                <div v-if="session.status === 'running'" class="text-xs opacity-70 mt-1">
                  {{ $t('scanSessions.estimatedTime') }}: {{ formatDuration(session.estimated_remaining) }}
                </div>
              </td>
              <td>
                <div class="space-y-1">
                  <div v-if="session.assets_found.domains > 0" class="badge badge-info badge-sm">
                    {{ session.assets_found.domains }} {{ $t('scanSessions.domains') }}
                  </div>
                  <div v-if="session.assets_found.ips > 0" class="badge badge-success badge-sm">
                    {{ session.assets_found.ips }} IPs
                  </div>
                  <div v-if="session.assets_found.ports > 0" class="badge badge-warning badge-sm">
                    {{ session.assets_found.ports }} {{ $t('scanSessions.ports') }}
                  </div>
                  <div v-if="session.assets_found.vulnerabilities > 0" class="badge badge-error badge-sm">
                    {{ session.assets_found.vulnerabilities }} {{ $t('vulnerabilities.title') }}
                  </div>
                </div>
              </td>
              <td class="text-sm opacity-70">
                {{ formatTime(session.created_at) }}
                <div v-if="session.completed_at" class="text-xs opacity-50">
                  {{ $t('common.completed') }}: {{ formatTime(session.completed_at) }}
                </div>
              </td>
              <td>
                <div class="flex space-x-1">
                  <button 
                    v-if="session.status === 'pending' || session.status === 'paused'"
                    @click="startSession(session.id)"
                    class="btn btn-xs btn-success"
                    :title="$t('scanSessions.start')"
                  >
                    <svg class="w-3 h-3" fill="currentColor" viewBox="0 0 20 20">
                      <path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zM9.555 7.168A1 1 0 008 8v4a1 1 0 001.555.832l3-2a1 1 0 000-1.664l-3-2z" clip-rule="evenodd"></path>
                    </svg>
                  </button>
                  <button 
                    v-if="session.status === 'running'"
                    @click="pauseSession(session.id)"
                    class="btn btn-xs btn-warning"
                    :title="$t('scanSessions.pause')"
                  >
                    <svg class="w-3 h-3" fill="currentColor" viewBox="0 0 20 20">
                      <path fill-rule="evenodd" d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zM7 8a1 1 0 012 0v4a1 1 0 11-2 0V8zm5-1a1 1 0 00-1 1v4a1 1 0 102 0V8a1 1 0 00-1-1z" clip-rule="evenodd"></path>
                    </svg>
                  </button>
                  <button 
                    v-if="session.status === 'running' || session.status === 'paused'"
                    @click="stopSession(session.id)"
                    class="btn btn-xs btn-error"
                    :title="$t('scanSessions.stop')"
                  >
                    <svg class="w-3 h-3" fill="currentColor" viewBox="0 0 20 20">
                      <path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zM8 7a1 1 0 00-1 1v4a1 1 0 001 1h4a1 1 0 001-1V8a1 1 0 00-1-1H8z" clip-rule="evenodd"></path>
                    </svg>
                  </button>
                  <button 
                    @click="viewSessionDetails(session)"
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
                      <li><a @click="exportSessionReport(session.id)">{{ $t('scanSessions.export') }}</a></li>
                      <li><a @click="cloneSession(session)">{{ $t('common.create') }}</a></li>
                      <li><a @click="deleteSession(session.id)" class="text-error">{{ $t('common.delete') }}</a></li>
                    </ul>
                  </div>
                </div>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>

    <!-- 创建会话模态框 -->
    <div v-if="showCreateModal" class="modal modal-open">
      <div class="modal-box w-11/12 max-w-2xl">
        <h3 class="font-bold text-lg mb-4">{{ $t('scanSessions.newSession') }}</h3>
        
        <form @submit.prevent="createSession" class="space-y-4">
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ $t('scanSessions.sessionName') }}</span>
            </label>
            <input 
              v-model="newSession.name" 
              type="text" 
              :placeholder="$t('scanSessions.form.namePlaceholder')" 
              class="input input-bordered" 
              required
            />
          </div>

          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ $t('scanSessions.targetUrl') }}</span>
            </label>
            <input 
              v-model="newSession.target" 
              type="text" 
              :placeholder="$t('scanSessions.form.targetPlaceholder')" 
              class="input input-bordered" 
              required
            />
          </div>

          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ $t('scanSessions.scanType') }}</span>
            </label>
            <select v-model="newSession.config.scan_type" class="select select-bordered" required>
              <option value="">{{ $t('scanSessions.form.selectType') }}</option>
              <option value="comprehensive">{{ $t('scanSessions.types.comprehensive') }}</option>
              <option value="subdomain">{{ $t('scanSessions.types.subdomain') }}</option>
              <option value="rsubdomain">{{ $t('scanSessions.types.subdomain') }}</option>
              <option value="port">{{ $t('scanSessions.types.port') }}</option>
              <option value="rustscan">{{ $t('scanSessions.types.port') }}</option>
              <option value="vulnerability">{{ $t('scanSessions.types.vulnerability') }}</option>
            </select>
          </div>

          <div class="grid grid-cols-2 gap-4">
            <div class="form-control">
              <label class="label">
                <span class="label-text">{{ $t('scanSessions.scanDepth') }}</span>
              </label>
              <select v-model="newSession.config.depth" class="select select-bordered">
                <option value="1">{{ $t('scanSessions.depth.shallow') }}</option>
                <option value="2">{{ $t('scanSessions.depth.medium') }}</option>
                <option value="3">{{ $t('scanSessions.depth.deep') }}</option>
              </select>
            </div>

            <div class="form-control">
              <label class="label">
                <span class="label-text">{{ $t('scanSessions.maxConcurrency') }}</span>
              </label>
              <input 
                v-model.number="newSession.config.max_concurrency" 
                type="number" 
                min="1" 
                max="20" 
                class="input input-bordered" 
              />
            </div>
          </div>

          <div class="form-control">
            <label class="label cursor-pointer">
              <span class="label-text">{{ $t('scanSessions.enableAI') }}</span>
              <input 
                v-model="newSession.config.enable_ai_analysis" 
                type="checkbox" 
                class="checkbox checkbox-primary" 
              />
            </label>
          </div>

          <div class="form-control">
            <label class="label cursor-pointer">
              <span class="label-text">{{ $t('scanSessions.autoOptimize') }}</span>
              <input 
                v-model="newSession.config.auto_optimize" 
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

    <!-- 会话详情模态框 -->
    <div v-if="showDetailsModal && selectedSession" class="modal modal-open">
      <div class="modal-box w-11/12 max-w-6xl">
        <h3 class="font-bold text-lg mb-4">{{ $t('scanSessions.sessionDetails') }}</h3>
        
        <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
          <!-- 基本信息 -->
          <div class="space-y-4">
            <div>
              <h4 class="font-semibold mb-2">{{ $t('common.details') }}</h4>
              <div class="space-y-2 text-sm">
                <div><span class="opacity-70">{{ $t('common.id') }}:</span> {{ selectedSession.id }}</div>
                <div><span class="opacity-70">{{ $t('scanSessions.sessionName') }}:</span> {{ selectedSession.name }}</div>
                <div><span class="opacity-70">{{ $t('common.target') }}:</span> {{ selectedSession.target }}</div>
                <div><span class="opacity-70">{{ $t('common.status') }}:</span> 
                  <div :class="getStatusBadgeClass(selectedSession.status)" class="badge badge-sm">
                    {{ getStatusLabel(selectedSession.status) }}
                  </div>
                </div>
                <div><span class="opacity-70">{{ $t('common.progress') }}:</span> {{ selectedSession.overall_progress }}%</div>
              </div>
            </div>

            <div>
              <h4 class="font-semibold mb-2">{{ $t('scanSessions.configuration') }}</h4>
              <div class="space-y-2 text-sm">
                <div><span class="opacity-70">{{ $t('scanSessions.scanType') }}:</span> {{ getTypeLabel(selectedSession.config.scan_type) }}</div>
                <div><span class="opacity-70">{{ $t('scanSessions.scanDepth') }}:</span> {{ selectedSession.config.depth }}</div>
                <div><span class="opacity-70">{{ $t('scanSessions.maxConcurrency') }}:</span> {{ selectedSession.config.max_concurrency }}</div>
                <div><span class="opacity-70">{{ $t('scanSessions.enableAI') }}:</span> {{ selectedSession.config.enable_ai_analysis ? $t('common.confirm') : $t('common.cancel') }}</div>
                <div><span class="opacity-70">{{ $t('scanSessions.autoOptimize') }}:</span> {{ selectedSession.config.auto_optimize ? $t('common.confirm') : $t('common.cancel') }}</div>
              </div>
            </div>
          </div>

          <!-- 扫描阶段 -->
          <div class="space-y-4">
            <div>
              <h4 class="font-semibold mb-2">{{ $t('scanSessions.scanStages') }}</h4>
              <div class="space-y-2">
                <div v-for="stage in selectedSession.stages" :key="stage.name" class="flex items-center justify-between p-2 bg-base-200 rounded">
                  <div class="flex items-center space-x-2">
                    <div :class="getStageStatusClass(stage.status)" class="w-3 h-3 rounded-full"></div>
                    <span class="text-sm">{{ getStageLabel(stage.name) }}</span>
                  </div>
                  <div class="text-xs opacity-70">{{ stage.progress }}%</div>
                </div>
              </div>
            </div>

            <div>
              <h4 class="font-semibold mb-2">{{ $t('common.time') }}</h4>
              <div class="space-y-2 text-sm">
                <div><span class="opacity-70">{{ $t('scanSessions.createdAt') }}:</span> {{ formatDateTime(selectedSession.created_at) }}</div>
                <div v-if="selectedSession.started_at"><span class="opacity-70">{{ $t('scanSessions.startTime') }}:</span> {{ formatDateTime(selectedSession.started_at) }}</div>
                <div v-if="selectedSession.completed_at"><span class="opacity-70">{{ $t('scanSessions.endTime') }}:</span> {{ formatDateTime(selectedSession.completed_at) }}</div>
                <div v-if="selectedSession.estimated_remaining"><span class="opacity-70">{{ $t('scanSessions.estimatedTime') }}:</span> {{ formatDuration(selectedSession.estimated_remaining) }}</div>
              </div>
            </div>
          </div>

          <!-- 发现的资产 -->
          <div class="space-y-4">
            <div>
              <h4 class="font-semibold mb-2">{{ $t('scanSessions.assetsFound') }}</h4>
              <div class="grid grid-cols-2 gap-2">
                <div class="stat bg-base-200 rounded p-3">
                  <div class="stat-value text-lg text-info">{{ selectedSession.assets_found.domains }}</div>
                  <div class="stat-title text-xs">{{ $t('scanSessions.domains') }}</div>
                </div>
                <div class="stat bg-base-200 rounded p-3">
                  <div class="stat-value text-lg text-success">{{ selectedSession.assets_found.ips }}</div>
                  <div class="stat-title text-xs">IPs</div>
                </div>
                <div class="stat bg-base-200 rounded p-3">
                  <div class="stat-value text-lg text-warning">{{ selectedSession.assets_found.ports }}</div>
                  <div class="stat-title text-xs">{{ $t('scanSessions.ports') }}</div>
                </div>
                <div class="stat bg-base-200 rounded p-3">
                  <div class="stat-value text-lg text-error">{{ selectedSession.assets_found.vulnerabilities }}</div>
                  <div class="stat-title text-xs">{{ $t('vulnerabilities.title') }}</div>
                </div>
              </div>
            </div>

            <div v-if="selectedSession.error_message">
              <h4 class="font-semibold mb-2 text-error">{{ $t('common.error') }}</h4>
              <div class="text-sm text-error bg-error/10 p-2 rounded">
                {{ selectedSession.error_message }}
              </div>
            </div>
          </div>
        </div>

        <div class="modal-action">
          <button @click="showDetailsModal = false" class="btn">{{ $t('common.close') }}</button>
          <button v-if="selectedSession.status === 'completed'" class="btn btn-primary">{{ $t('scanSessions.viewReport') }}</button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { invoke } from '@tauri-apps/api/core';
import { dialog } from '@/composables/useDialog';

const { t } = useI18n();

// 类型定义
interface ScanSession {
  id: string;
  name: string;
  target: string;
  config: {
    scan_type: string;
    depth: number;
    max_concurrency: number;
    enable_ai_analysis: boolean;
    auto_optimize: boolean;
  };
  status: string;
  current_stage?: string;
  stage_progress: number;
  overall_progress: number;
  created_at: string;
  started_at?: string;
  completed_at?: string;
  estimated_remaining?: number;
  assets_found: {
    domains: number;
    ips: number;
    ports: number;
    vulnerabilities: number;
  };
  stages: Array<{
    name: string;
    status: string;
    progress: number;
  }>;
  error_message?: string;
}

// 响应式数据
const sessions = ref<ScanSession[]>([]);
const isLoading = ref(false);

const searchQuery = ref('');
const statusFilter = ref('');
const typeFilter = ref('');
const showCreateModal = ref(false);
const showDetailsModal = ref(false);
const selectedSession = ref<ScanSession | null>(null);
const isCreating = ref(false);

const newSession = ref({
  name: '',
  target: '',
  config: {
    scan_type: '',
    depth: 2,
    max_concurrency: 5,
    enable_ai_analysis: true,
    auto_optimize: true
  }
});

// 计算属性
const filteredSessions = computed(() => {
  return sessions.value.filter(session => {
    const matchesSearch = session.name.toLowerCase().includes(searchQuery.value.toLowerCase()) ||
                         session.target.toLowerCase().includes(searchQuery.value.toLowerCase());
    const matchesStatus = !statusFilter.value || session.status === statusFilter.value;
    const matchesType = !typeFilter.value || session.config.scan_type === typeFilter.value;
    
    return matchesSearch && matchesStatus && matchesType;
  });
});

// 方法
const refreshSessions = async () => {
  isLoading.value = true;
  try {
    const request = {
      limit: null,
      offset: null,
      status_filter: null
    };
    const response:any = await invoke('list_scan_sessions', { request });
    sessions.value = response.data as ScanSession[];
  } catch (error) {
    console.error('Failed to refresh sessions:', error);
    await dialog.toast.error(t('scanSessions.notifications.loadFailed'));
  } finally {
    isLoading.value = false;
  }
};

const createSession = async () => {
  if (!newSession.value.name || !newSession.value.target || !newSession.value.config.scan_type) {
    await dialog.toast.error(t('scanSessions.form.selectType'));
    return;
  }

  isCreating.value = true;
  try {
    const response = await invoke('create_scan_session', {
      name: newSession.value.name,
      target: newSession.value.target,
      scan_type: newSession.value.config.scan_type,
      config: newSession.value.config
    });
    
    await dialog.toast.success(t('scanSessions.notifications.sessionCreated'));
    showCreateModal.value = false;
    
    // 重置表单
    newSession.value = {
      name: '',
      target: '',
      config: {
        scan_type: '',
        depth: 2,
        max_concurrency: 5,
        enable_ai_analysis: true,
        auto_optimize: true
      }
    };
    
    await refreshSessions();
  } catch (error) {
    console.error('Failed to create session:', error);
    await dialog.toast.error(t('scanSessions.notifications.createFailed'));
  } finally {
    isCreating.value = false;
  }
};

const startSession = async (sessionId: string) => {
  try {
    await invoke('start_scan_session', { sessionId });
    await dialog.toast.success(t('scanSessions.notifications.sessionStarted'));
    await refreshSessions();
  } catch (error) {
    console.error('Failed to start session:', error);
    await dialog.toast.error(t('scanSessions.notifications.startFailed'));
  }
};

const pauseSession = async (sessionId: string) => {
  try {
    await invoke('pause_scan_session', { sessionId });
    await dialog.toast.success(t('scanSessions.notifications.sessionPaused'));
    await refreshSessions();
  } catch (error) {
    console.error('Failed to pause session:', error);
    await dialog.toast.error(t('scanSessions.notifications.pauseFailed'));
  }
};

const stopSession = async (sessionId: string) => {
  try {
    await invoke('stop_scan_session', { sessionId });
    await dialog.toast.success(t('scanSessions.notifications.sessionStopped'));
    await refreshSessions();
  } catch (error) {
    console.error('Failed to stop session:', error);
    await dialog.toast.error(t('scanSessions.notifications.stopFailed'));
  }
};

const deleteSession = async (sessionId: string) => {
  const confirmed = await dialog.confirm(
    t('scanSessions.notifications.confirmDelete')
  );
  if (confirmed) {
    try {
      await invoke('delete_scan_session', { sessionId });
      await dialog.toast.success(t('scanSessions.notifications.sessionDeleted'));
      await refreshSessions();
    } catch (error) {
      console.error('Failed to delete session:', error);
      await dialog.toast.error(t('scanSessions.notifications.deleteFailed'));
    }
  }
};

const viewSessionDetails = (session: ScanSession) => {
  selectedSession.value = session;
  showDetailsModal.value = true;
};

const getStatusBadgeClass = (status: string) => {
  switch (status) {
    case 'running': return 'badge-info';
    case 'completed': return 'badge-success';
    case 'failed': return 'badge-error';
    case 'pending': return 'badge-warning';
    case 'paused': return 'badge-neutral';
    default: return 'badge-ghost';
  }
};

const getProgressClass = (status: string) => {
  switch (status) {
    case 'running': return 'progress-info';
    case 'completed': return 'progress-success';
    case 'failed': return 'progress-error';
    default: return 'progress';
  }
};

const getStatusLabel = (status: string) => {
  const labels: Record<string, string> = {
    'pending': t('common.pending'),
    'running': t('common.inProgress'),
    'completed': t('common.completed'),
    'failed': t('common.failed'),
    'paused': t('scanSessions.paused')
  };
  return labels[status] || status;
};

const getTypeLabel = (type: string) => {
  const labels: Record<string, string> = {
    'comprehensive': t('scanSessions.types.comprehensive'),
    'subdomain': t('scanSessions.types.subdomain'),
    'rsubdomain': t('scanSessions.types.subdomain'),
    'port': t('scanSessions.types.port'),
    'rustscan': t('scanSessions.types.port'),
    'vulnerability': t('scanSessions.types.vulnerability')
  };
  return labels[type] || type;
};

const getStageLabel = (stage: string) => {
  const labels: Record<string, string> = {
    'subdomain_discovery': t('scanSessions.stages.subdomainDiscovery'),
    'port_scanning': t('scanSessions.stages.portScanning'),
    'service_detection': t('scanSessions.stages.serviceDetection'),
    'vulnerability_scanning': t('scanSessions.stages.vulnerabilityScanning'),
    'ai_analysis': t('scanSessions.stages.aiAnalysis')
  };
  return labels[stage] || stage;
};

const getStageStatusClass = (status: string) => {
  switch (status) {
    case 'running': return 'bg-info';
    case 'completed': return 'bg-success';
    case 'failed': return 'bg-error';
    case 'pending': return 'bg-base-300';
    default: return 'bg-base-300';
  }
};

const formatTime = (dateString: string) => {
  const date = new Date(dateString);
  const now = new Date();
  const diff = now.getTime() - date.getTime();
  const minutes = Math.floor(diff / (1000 * 60));
  const hours = Math.floor(diff / (1000 * 60 * 60));
  const days = Math.floor(diff / (1000 * 60 * 60 * 24));

  if (days > 0) return `${days}天前`;
  if (hours > 0) return `${hours}小时前`;
  if (minutes > 0) return `${minutes}分钟前`;
  return '刚刚';
};

const formatDateTime = (dateString: string) => {
  return new Date(dateString).toLocaleString();
};

const formatDuration = (seconds: number | undefined) => {
  if (!seconds) return '-';
  const mins = Math.floor(seconds / 60);
  const secs = seconds % 60;
  return `${mins}分${secs}秒`;
};

const exportSessionReport = async (sessionId: string) => {
  try {
    await invoke('export_session_report', { sessionId });
  } catch (error) {
    console.error('Failed to export report:', error);
  }
};

const cloneSession = (session: ScanSession) => {
  newSession.value = {
    name: `${session.name} (副本)`,
    target: session.target,
    config: { ...session.config }
  };
  showCreateModal.value = true;
};

// 生命周期
onMounted(() => {
  refreshSessions();
});
</script>

<style scoped>
.stat {
  @apply text-center;
}

.stat-value {
  @apply font-bold;
}

.stat-title {
  @apply opacity-70;
}
</style>