import { ref, onMounted, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type { ActiveToolInfo } from '@/types/task-tool'

/**
 * Composable for tracking active tools across all tasks
 */
export function useActiveTools() {
  const activeTools = ref<Map<string, ActiveToolInfo[]>>(new Map())
  const loading = ref(false)
  let unlisten: UnlistenFn | null = null

  /**
   * Get active tools for a specific tool ID
   */
  const getActiveToolsByToolId = (toolId: string): ActiveToolInfo[] => {
    return activeTools.value.get(toolId) || []
  }

  /**
   * Check if a tool is currently active
   */
  const isToolActive = (toolId: string): boolean => {
    const tools = activeTools.value.get(toolId)
    return tools !== undefined && tools.length > 0
  }

  /**
   * Get count of active instances for a tool
   */
  const getActiveCount = (toolId: string): number => {
    return getActiveToolsByToolId(toolId).length
  }

  /**
   * Fetch all active tools from backend
   */
  const fetchActiveTools = async () => {
    loading.value = true
    try {
      const response = await invoke<{ success: boolean; data?: ActiveToolInfo[] }>('get_all_active_tools')
      if (response.success && response.data) {
        // Group by tool_id
        const grouped = new Map<string, ActiveToolInfo[]>()
        response.data.forEach(tool => {
          const existing = grouped.get(tool.tool_id) || []
          existing.push(tool)
          grouped.set(tool.tool_id, existing)
        })
        activeTools.value = grouped
      }
    } catch (error) {
      console.error('Failed to fetch active tools:', error)
    } finally {
      loading.value = false
    }
  }

  /**
   * Handle tool execution start event
   */
  const handleToolStart = (toolInfo: ActiveToolInfo) => {
    const existing = activeTools.value.get(toolInfo.tool_id) || []
    existing.push(toolInfo)
    activeTools.value.set(toolInfo.tool_id, existing)
  }

  /**
   * Handle tool execution complete event
   */
  const handleToolComplete = (payload: { tool_id: string; log_id: string }) => {
    const existing = activeTools.value.get(payload.tool_id)
    if (existing) {
      const filtered = existing.filter(t => t.log_id !== payload.log_id)
      if (filtered.length > 0) {
        activeTools.value.set(payload.tool_id, filtered)
      } else {
        activeTools.value.delete(payload.tool_id)
      }
    }
  }

  /**
   * Setup event listeners
   */
  const setupListeners = async () => {
    // Listen for tool execution start
    const unlistenStart = await listen<ActiveToolInfo>('tool-execution-start', (event) => {
      handleToolStart(event.payload)
    })

    // Listen for tool execution complete
    const unlistenComplete = await listen<{ tool_id: string; log_id: string }>('tool-execution-complete', (event) => {
      handleToolComplete(event.payload)
    })

    // Listen for tool execution error (also removes from active)
    const unlistenError = await listen<{ tool_id: string; log_id: string }>('tool-execution-error', (event) => {
      handleToolComplete(event.payload)
    })

    // Combined unlisten function
    unlisten = () => {
      unlistenStart()
      unlistenComplete()
      unlistenError()
    }
  }

  onMounted(async () => {
    await fetchActiveTools()
    await setupListeners()
  })

  onUnmounted(() => {
    if (unlisten) {
      unlisten()
    }
  })

  return {
    activeTools,
    loading,
    getActiveToolsByToolId,
    isToolActive,
    getActiveCount,
    fetchActiveTools,
  }
}
