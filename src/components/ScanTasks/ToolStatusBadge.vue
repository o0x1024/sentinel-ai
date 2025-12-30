<template>
  <div 
    class="tool-status-badge" 
    :class="[statusClass, sizeClass]"
    :title="tooltipText"
  >
    <i :class="toolIcon" class="tool-icon"></i>
    <span class="tool-name">{{ tool.tool_name }}</span>
    <span v-if="tool.status === 'running'" class="loading loading-spinner loading-xs ml-1"></span>
    <span v-if="tool.error_count > 0" class="badge badge-error badge-xs ml-1">
      {{ tool.error_count }}
    </span>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue';
import type { ActiveToolInfo } from '@/types/task-tool';

interface Props {
  tool: ActiveToolInfo;
  size?: 'sm' | 'md' | 'lg';
}

const props = withDefaults(defineProps<Props>(), {
  size: 'md'
});

const statusClass = computed(() => {
  switch (props.tool.status) {
    case 'running':
      return 'badge-info animate-pulse';
    case 'completed':
      return 'badge-success';
    case 'error':
      return 'badge-error';
    case 'waiting':
      return 'badge-warning';
    case 'idle':
    default:
      return 'badge-ghost';
  }
});

const sizeClass = computed(() => {
  switch (props.size) {
    case 'sm':
      return 'badge-sm';
    case 'lg':
      return 'badge-lg';
    case 'md':
    default:
      return 'badge-md';
  }
});

const toolIcon = computed(() => {
  switch (props.tool.tool_type) {
    case 'plugin':
      return 'fas fa-plug';
    case 'mcp_server':
      return 'fas fa-server';
    case 'builtin':
      return 'fas fa-tools';
    case 'workflow':
      return 'fas fa-project-diagram';
    default:
      return 'fas fa-wrench';
  }
});

const tooltipText = computed(() => {
  const parts = [
    `${props.tool.tool_name}`,
    `Status: ${props.tool.status}`,
    `Executions: ${props.tool.execution_count}`,
    `Avg Time: ${props.tool.avg_execution_time}ms`
  ];
  
  if (props.tool.error_count > 0) {
    parts.push(`Errors: ${props.tool.error_count}`);
  }
  
  if (props.tool.last_execution_time) {
    const time = new Date(props.tool.last_execution_time).toLocaleString();
    parts.push(`Last Run: ${time}`);
  }
  
  return parts.join('\n');
});
</script>

<style scoped>
.tool-status-badge {
  @apply badge gap-1 cursor-help transition-all;
}

.tool-icon {
  @apply text-xs;
}

.tool-name {
  @apply text-xs font-medium;
}

.badge-sm .tool-name {
  @apply text-[10px];
}

.badge-lg .tool-name {
  @apply text-sm;
}
</style>
