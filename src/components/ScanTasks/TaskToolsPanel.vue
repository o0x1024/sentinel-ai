<template>
  <div class="task-tools-panel">
    <div v-if="loading" class="flex justify-center items-center py-8">
      <span class="loading loading-spinner loading-lg"></span>
    </div>

    <div v-else-if="error" class="alert alert-error">
      <i class="fas fa-exclamation-circle"></i>
      <span>{{ error }}</span>
    </div>

    <div v-else-if="activeTools.length === 0" class="text-center py-8 text-base-content/60">
      <i class="fas fa-inbox text-4xl mb-2"></i>
      <p>{{ $t('scanTasks.noToolsActive') }}</p>
    </div>

    <div v-else class="space-y-3">
      <div 
        v-for="tool in activeTools" 
        :key="tool.tool_id"
        class="card bg-base-200 shadow-sm"
      >
        <div class="card-body p-4">
          <div class="flex items-center justify-between">
            <div class="flex items-center gap-3">
              <div class="avatar placeholder">
                <div class="bg-primary text-primary-content rounded-full w-10">
                  <i :class="getToolIcon(tool.tool_type)" class="text-lg"></i>
                </div>
              </div>
              <div>
                <h4 class="font-semibold">{{ tool.tool_name }}</h4>
                <p class="text-xs text-base-content/60">{{ getToolTypeLabel(tool.tool_type) }}</p>
              </div>
            </div>
            <div :class="getStatusBadgeClass(tool.status)" class="badge">
              {{ getStatusLabel(tool.status) }}
            </div>
          </div>

          <div class="grid grid-cols-3 gap-4 mt-3 text-sm">
            <div>
              <div class="text-base-content/60">{{ $t('scanTasks.executions') }}</div>
              <div class="font-semibold">{{ tool.execution_count }}</div>
            </div>
            <div>
              <div class="text-base-content/60">{{ $t('scanTasks.avgTime') }}</div>
              <div class="font-semibold">{{ tool.avg_execution_time }}ms</div>
            </div>
            <div>
              <div class="text-base-content/60">{{ $t('scanTasks.errors') }}</div>
              <div class="font-semibold" :class="{ 'text-error': tool.error_count > 0 }">
                {{ tool.error_count }}
              </div>
            </div>
          </div>

          <div v-if="tool.last_execution_time" class="text-xs text-base-content/60 mt-2">
            {{ $t('scanTasks.lastRun') }}: {{ formatTime(tool.last_execution_time) }}
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { useI18n } from 'vue-i18n';
import type { ActiveToolInfo, ToolStatusChangedEvent } from '@/types/task-tool';

interface Props {
  taskId: string;
}

const props = defineProps<Props>();
const { t } = useI18n();

const activeTools = ref<ActiveToolInfo[]>([]);
const loading = ref(true);
const error = ref('');

let unlistenStatusChanged: UnlistenFn | null = null;

const loadActiveTools = async () => {
  try {
    loading.value = true;
    error.value = '';
    const tools = await invoke<ActiveToolInfo[]>('get_task_active_tools', {
      taskId: props.taskId
    });
    activeTools.value = tools;
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
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

const getToolTypeLabel = (toolType: string): string => {
  const labels: Record<string, string> = {
    'plugin': t('scanTasks.toolTypes.plugin'),
    'mcp_server': t('scanTasks.toolTypes.mcpServer'),
    'builtin': t('scanTasks.toolTypes.builtin'),
    'workflow': t('scanTasks.toolTypes.workflow')
  };
  return labels[toolType] || toolType;
};

const getStatusBadgeClass = (status: string): string => {
  switch (status) {
    case 'running': return 'badge-info';
    case 'completed': return 'badge-success';
    case 'error': return 'badge-error';
    case 'waiting': return 'badge-warning';
    case 'idle':
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

onMounted(async () => {
  await loadActiveTools();
  
  // Listen for tool status changes
  unlistenStatusChanged = await listen<ToolStatusChangedEvent>(
    'task:tool:status_changed',
    (event) => {
      if (event.payload.task_id === props.taskId) {
        activeTools.value = event.payload.active_tools;
      }
    }
  );
});

onUnmounted(() => {
  if (unlistenStatusChanged) {
    unlistenStatusChanged();
  }
});
</script>

<style scoped>
.task-tools-panel {
  @apply min-h-[200px];
}
</style>
