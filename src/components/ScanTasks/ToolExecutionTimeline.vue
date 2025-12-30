<template>
  <div class="tool-execution-timeline">
    <div class="flex justify-between items-center mb-4">
      <h3 class="text-lg font-semibold">{{ $t('scanTasks.executionHistory') }}</h3>
      <div class="flex gap-2">
        <select v-model="selectedToolFilter" class="select select-bordered select-sm">
          <option value="">{{ $t('scanTasks.filters.all') }}</option>
          <option v-for="tool in uniqueTools" :key="tool" :value="tool">
            {{ tool }}
          </option>
        </select>
        <button @click="loadHistory" class="btn btn-sm btn-outline">
          <i class="fas fa-sync-alt"></i>
        </button>
      </div>
    </div>

    <div v-if="loading" class="flex justify-center py-8">
      <span class="loading loading-spinner loading-lg"></span>
    </div>

    <div v-else-if="error" class="alert alert-error">
      <i class="fas fa-exclamation-circle"></i>
      <span>{{ error }}</span>
    </div>

    <div v-else-if="records.length === 0" class="text-center py-8 text-base-content/60">
      <i class="fas fa-clock text-4xl mb-2"></i>
      <p>{{ $t('scanTasks.noExecutionHistory') }}</p>
    </div>

    <div v-else class="timeline-container">
      <div class="timeline">
        <div 
          v-for="(record, index) in records" 
          :key="record.id"
          class="timeline-item"
        >
          <div class="timeline-marker" :class="getMarkerClass(record.status)">
            <i :class="getStatusIcon(record.status)"></i>
          </div>
          
          <div class="timeline-content">
            <div class="card bg-base-200 shadow-sm">
              <div class="card-body p-3">
                <div class="flex items-center justify-between">
                  <div class="flex items-center gap-2">
                    <i :class="getToolIcon(record.tool_type)" class="text-primary"></i>
                    <span class="font-semibold">{{ record.tool_name }}</span>
                  </div>
                  <div :class="getStatusBadgeClass(record.status)" class="badge badge-sm">
                    {{ getStatusLabel(record.status) }}
                  </div>
                </div>

                <div class="grid grid-cols-2 gap-2 mt-2 text-xs">
                  <div>
                    <span class="text-base-content/60">{{ $t('scanTasks.startTime') }}:</span>
                    <span class="ml-1">{{ formatTime(record.started_at) }}</span>
                  </div>
                  <div v-if="record.completed_at">
                    <span class="text-base-content/60">{{ $t('scanTasks.endTime') }}:</span>
                    <span class="ml-1">{{ formatTime(record.completed_at) }}</span>
                  </div>
                </div>

                <div v-if="record.execution_time_ms" class="mt-2 text-xs">
                  <span class="text-base-content/60">{{ $t('scanTasks.duration') }}:</span>
                  <span class="ml-1 font-semibold">{{ record.execution_time_ms }}ms</span>
                </div>

                <div v-if="record.error_message" class="alert alert-error alert-sm mt-2">
                  <i class="fas fa-exclamation-triangle text-xs"></i>
                  <span class="text-xs">{{ record.error_message }}</span>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <div v-if="records.length > 0 && records.length >= limit" class="text-center mt-4">
      <button @click="loadMore" class="btn btn-sm btn-outline">
        {{ $t('common.loadMore') }}
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { useI18n } from 'vue-i18n';
import type { ExecutionRecord } from '@/types/task-tool';

interface Props {
  taskId: string;
}

const props = defineProps<Props>();
const { t } = useI18n();

const records = ref<ExecutionRecord[]>([]);
const loading = ref(true);
const error = ref('');
const selectedToolFilter = ref('');
const limit = ref(20);

const uniqueTools = computed(() => {
  const tools = new Set(records.value.map(r => r.tool_name));
  return Array.from(tools).sort();
});

const loadHistory = async () => {
  try {
    loading.value = true;
    error.value = '';
    
    const toolId = selectedToolFilter.value || null;
    
    const history = await invoke<ExecutionRecord[]>('get_tool_execution_history', {
      taskId: props.taskId,
      toolId,
      limit: limit.value
    });
    
    records.value = history;
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
};

const loadMore = () => {
  limit.value += 20;
  loadHistory();
};

const getToolIcon = (toolType: string): string => {
  const icons: Record<string, string> = {
    'plugin': 'fas fa-plug',
    'mcp_server': 'fas fa-server',
    'builtin': 'fas fa-tools',
    'workflow': 'fas fa-project-diagram'
  };
  return icons[toolType] || 'fas fa-wrench';
};

const getMarkerClass = (status: string): string => {
  switch (status) {
    case 'running': return 'marker-info';
    case 'completed': return 'marker-success';
    case 'error': return 'marker-error';
    default: return 'marker-default';
  }
};

const getStatusIcon = (status: string): string => {
  switch (status) {
    case 'running': return 'fas fa-spinner fa-spin';
    case 'completed': return 'fas fa-check';
    case 'error': return 'fas fa-times';
    default: return 'fas fa-circle';
  }
};

const getStatusBadgeClass = (status: string): string => {
  switch (status) {
    case 'running': return 'badge-info';
    case 'completed': return 'badge-success';
    case 'error': return 'badge-error';
    default: return 'badge-ghost';
  }
};

const getStatusLabel = (status: string): string => {
  const labels: Record<string, string> = {
    'idle': t('scanTasks.toolStatus.idle'),
    'running': t('scanTasks.toolStatus.running'),
    'waiting': t('scanTasks.toolStatus.waiting'),
    'completed': t('scanTasks.toolStatus.completed'),
    'error': t('scanTasks.toolStatus.error')
  };
  return labels[status] || status;
};

const formatTime = (dateString: string): string => {
  return new Date(dateString).toLocaleString();
};

watch(selectedToolFilter, () => {
  loadHistory();
});

onMounted(() => {
  loadHistory();
});
</script>

<style scoped>
.timeline-container {
  @apply relative;
}

.timeline {
  @apply space-y-4;
}

.timeline-item {
  @apply relative flex gap-4;
}

.timeline-marker {
  @apply flex-shrink-0 w-8 h-8 rounded-full flex items-center justify-center;
  @apply border-2 border-base-300 bg-base-100;
}

.marker-info {
  @apply border-info bg-info/10 text-info;
}

.marker-success {
  @apply border-success bg-success/10 text-success;
}

.marker-error {
  @apply border-error bg-error/10 text-error;
}

.marker-default {
  @apply border-base-300 bg-base-100;
}

.timeline-content {
  @apply flex-1;
}

.timeline-item:not(:last-child) .timeline-marker::after {
  content: '';
  @apply absolute left-4 top-8 w-0.5 h-full bg-base-300;
  transform: translateX(-50%);
}
</style>
