import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

export const useMonitorTaskProgress = () => {
  const runningTaskIds = ref<Set<string>>(new Set())
  const taskProgressById = ref<Record<string, any>>({})

  const isTaskRunning = (taskId: string) => runningTaskIds.value.has(taskId)
  const getTaskProgress = (taskId: string) => taskProgressById.value[taskId] || null

  const clearTaskRuntimeState = (taskId: string) => {
    const nextProgress = { ...taskProgressById.value }
    delete nextProgress[taskId]
    taskProgressById.value = nextProgress
  }

  const markTaskQueued = (task: any, preparingMessage: string) => {
    const now = new Date().toISOString()
    runningTaskIds.value = new Set(runningTaskIds.value).add(task.id)
    taskProgressById.value = {
      ...taskProgressById.value,
      [task.id]: {
        task_id: task.id,
        task_name: task.name,
        execution_mode: 'manual_trigger',
        status: 'running',
        progress: 0,
        completed_steps: 0,
        total_steps: 0,
        current_plugin: null,
        target_count: 0,
        imported_assets: 0,
        message: preparingMessage,
        indeterminate: true,
        started_at: now,
        updated_at: now,
      },
    }
  }

  const pruneTaskProgress = (tasks: any[]) => {
    const allowedIds = new Set((tasks || []).map((task: any) => task.id))
    taskProgressById.value = Object.fromEntries(
      Object.entries(taskProgressById.value).filter(([taskId]) => allowedIds.has(taskId))
    )
  }

  const loadRunningTasks = async (tasks: any[] = []) => {
    try {
      const ids = await invoke('monitor_get_running_tasks') as string[]
      runningTaskIds.value = new Set(ids || [])

      const nextProgress = { ...taskProgressById.value }
      for (const taskId of ids || []) {
        if (!nextProgress[taskId]) {
          const task = tasks.find((item: any) => item.id === taskId)
          const now = new Date().toISOString()
          nextProgress[taskId] = {
            task_id: taskId,
            task_name: task?.name || taskId,
            execution_mode: 'scheduler',
            status: 'running',
            progress: 0,
            completed_steps: 0,
            total_steps: 0,
            current_plugin: null,
            target_count: 0,
            imported_assets: 0,
            message: null,
            indeterminate: true,
            started_at: now,
            updated_at: now,
          }
        }
      }
      taskProgressById.value = nextProgress
    } catch (error) {
      console.error('Failed to load running tasks:', error)
    }
  }

  const setupTaskProgressListener = async (onSettled: () => void) => {
    const unlistenProgress = await listen<any>('monitor:task-progress', (event) => {
      const payload = event.payload
      const taskId = payload?.task_id
      if (!taskId) return

      const previous = taskProgressById.value[taskId] || {}
      const sameRunningPlugin = payload.status === 'running'
        && previous?.status === 'running'
        && payload.current_plugin
        && previous.current_plugin
        && payload.current_plugin === previous.current_plugin

      const hasOngoingScanProgress = sameRunningPlugin
        && Number(previous.scan_total_targets || 0) > 0
        && Number(previous.scan_completed_targets || 0) < Number(previous.scan_total_targets || 0)

      const hasOngoingPluginProgress = sameRunningPlugin
        && Number(previous.plugin_total_units || 0) > 0
        && Number(previous.plugin_completed_units || 0) < Number(previous.plugin_total_units || 0)

      const mergedPayload = {
        ...previous,
        ...payload,
      }

      if (hasOngoingScanProgress) {
        if (payload.scan_completed_targets == null) {
          mergedPayload.scan_completed_targets = previous.scan_completed_targets
        }
        if (payload.scan_total_targets == null) {
          mergedPayload.scan_total_targets = previous.scan_total_targets
        }
        if (payload.scan_completed_units == null) {
          mergedPayload.scan_completed_units = previous.scan_completed_units
        }
        if (payload.scan_total_units == null) {
          mergedPayload.scan_total_units = previous.scan_total_units
        }
        if (payload.current_target == null) {
          mergedPayload.current_target = previous.current_target
        }
        if (payload.indeterminate) {
          mergedPayload.indeterminate = false
        }
        mergedPayload.progress = Math.max(
          Number(previous.progress || 0),
          Number(payload.progress || 0),
        )
      }

      if (hasOngoingPluginProgress) {
        if (payload.plugin_completed_units == null) {
          mergedPayload.plugin_completed_units = previous.plugin_completed_units
        }
        if (payload.plugin_total_units == null) {
          mergedPayload.plugin_total_units = previous.plugin_total_units
        }
        if (payload.plugin_phase == null) {
          mergedPayload.plugin_phase = previous.plugin_phase
        }
        if (payload.plugin_phase_label == null) {
          mergedPayload.plugin_phase_label = previous.plugin_phase_label
        }
        if (payload.current_target == null) {
          mergedPayload.current_target = previous.current_target
        }
        if (payload.indeterminate) {
          mergedPayload.indeterminate = false
        }
        mergedPayload.progress = Math.max(
          Number(previous.progress || 0),
          Number(payload.progress || 0),
        )
      }

      taskProgressById.value = {
        ...taskProgressById.value,
        [taskId]: mergedPayload,
      }

      const nextRunning = new Set(runningTaskIds.value)
      if (payload.status === 'running') {
        nextRunning.add(taskId)
      } else {
        nextRunning.delete(taskId)
      }
      runningTaskIds.value = nextRunning

      if (payload.status === 'completed' || payload.status === 'failed' || payload.status === 'stopped') {
        clearTaskRuntimeState(taskId)
        onSettled()
      }
    })

    return () => {
      unlistenProgress()
    }
  }

  return {
    runningTaskIds,
    taskProgressById,
    isTaskRunning,
    getTaskProgress,
    markTaskQueued,
    pruneTaskProgress,
    loadRunningTasks,
    setupTaskProgressListener,
  }
}
