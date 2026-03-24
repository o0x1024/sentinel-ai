import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

export const useMonitorTaskProgress = () => {
  const runningTaskIds = ref<Set<string>>(new Set())
  const taskProgressById = ref<Record<string, any>>({})
  const taskLogsById = ref<Record<string, any[]>>({})

  const isTaskRunning = (taskId: string) => runningTaskIds.value.has(taskId)
  const getTaskProgress = (taskId: string) => taskProgressById.value[taskId] || null
  const getTaskLogs = (taskId: string) => taskLogsById.value[taskId] || []

  const markTaskQueued = (task: any, preparingMessage: string) => {
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
      },
    }
    taskLogsById.value = {
      ...taskLogsById.value,
      [task.id]: [],
    }
  }

  const pruneTaskProgress = (tasks: any[]) => {
    const allowedIds = new Set((tasks || []).map((task: any) => task.id))
    taskProgressById.value = Object.fromEntries(
      Object.entries(taskProgressById.value).filter(([taskId]) => allowedIds.has(taskId))
    )
    taskLogsById.value = Object.fromEntries(
      Object.entries(taskLogsById.value).filter(([taskId]) => allowedIds.has(taskId))
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

      taskProgressById.value = {
        ...taskProgressById.value,
        [taskId]: payload,
      }

      const nextRunning = new Set(runningTaskIds.value)
      if (payload.status === 'running') {
        nextRunning.add(taskId)
      } else {
        nextRunning.delete(taskId)
      }
      runningTaskIds.value = nextRunning

      if (payload.status === 'completed' || payload.status === 'failed' || payload.status === 'stopped') {
        onSettled()
      }
    })

    const unlistenLog = await listen<any>('monitor:task-log', (event) => {
      const payload = event.payload
      const taskId = payload?.task_id
      if (!taskId) return

      const current = taskLogsById.value[taskId] || []
      taskLogsById.value = {
        ...taskLogsById.value,
        [taskId]: [payload, ...current].slice(0, 20),
      }
    })

    return () => {
      unlistenProgress()
      unlistenLog()
    }
  }

  return {
    runningTaskIds,
    taskProgressById,
    taskLogsById,
    isTaskRunning,
    getTaskProgress,
    getTaskLogs,
    markTaskQueued,
    pruneTaskProgress,
    loadRunningTasks,
    setupTaskProgressListener,
  }
}
