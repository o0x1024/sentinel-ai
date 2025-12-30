/**
 * Composable for task-tool integration across pages
 * Provides utilities for cross-page navigation and tool status tracking
 */

import { ref, onMounted, onUnmounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { ActiveToolInfo, ToolStatusChangedEvent } from '@/types/task-tool';

/**
 * Track which tools are currently being used by tasks
 */
export function useToolUsageTracking() {
  const toolUsageMap = ref<Map<string, string[]>>(new Map()); // tool_id -> [task_id1, task_id2, ...]
  let unlistenStatusChanged: UnlistenFn | null = null;

  const loadAllToolUsage = async () => {
    try {
      // Get all active tasks
      const tasks = await invoke<any[]>('get_scan_tasks');
      const newMap = new Map<string, string[]>();

      // Load active tools for each task
      for (const task of tasks) {
        if (task.status === 'Running' || task.status === 'Pending') {
          try {
            const activeTools = await invoke<ActiveToolInfo[]>('get_task_active_tools', {
              taskId: task.id
            });

            for (const tool of activeTools) {
              if (tool.status === 'running' || tool.status === 'waiting') {
                const existing = newMap.get(tool.tool_id) || [];
                existing.push(task.id);
                newMap.set(tool.tool_id, existing);
              }
            }
          } catch (e) {
            console.error(`Failed to load tools for task ${task.id}:`, e);
          }
        }
      }

      toolUsageMap.value = newMap;
    } catch (e) {
      console.error('Failed to load tool usage:', e);
    }
  };

  const isToolInUse = (toolId: string): boolean => {
    return toolUsageMap.value.has(toolId) && (toolUsageMap.value.get(toolId)?.length || 0) > 0;
  };

  const getToolUsageTasks = (toolId: string): string[] => {
    return toolUsageMap.value.get(toolId) || [];
  };

  onMounted(async () => {
    await loadAllToolUsage();

    // Listen for tool status changes
    unlistenStatusChanged = await listen<ToolStatusChangedEvent>(
      'task:tool:status_changed',
      async (event) => {
        // Reload usage when tools change
        await loadAllToolUsage();
      }
    );
  });

  onUnmounted(() => {
    if (unlistenStatusChanged) {
      unlistenStatusChanged();
    }
  });

  return {
    toolUsageMap,
    isToolInUse,
    getToolUsageTasks,
    loadAllToolUsage
  };
}

/**
 * Navigate to scan task details from other pages
 */
export function useTaskNavigation() {
  const navigateToTask = (taskId: string) => {
    // Emit event to navigate to task
    window.location.hash = `#/scan-tasks?task=${taskId}`;
  };

  const navigateToTaskTools = (taskId: string) => {
    // Navigate to task details with tools tab open
    window.location.hash = `#/scan-tasks?task=${taskId}&tab=tools`;
  };

  return {
    navigateToTask,
    navigateToTaskTools
  };
}

/**
 * Get tool status badge info for display
 */
export function useToolStatusBadge(toolId: string) {
  const isInUse = ref(false);
  const taskCount = ref(0);
  const taskIds = ref<string[]>([]);

  const { isToolInUse, getToolUsageTasks, loadAllToolUsage } = useToolUsageTracking();

  const updateStatus = () => {
    isInUse.value = isToolInUse(toolId);
    taskIds.value = getToolUsageTasks(toolId);
    taskCount.value = taskIds.value.length;
  };

  onMounted(async () => {
    await loadAllToolUsage();
    updateStatus();
  });

  return {
    isInUse,
    taskCount,
    taskIds,
    updateStatus
  };
}
