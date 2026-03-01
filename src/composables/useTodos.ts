/**
 * Todos 状态管理 Composable
 * 监听后端 todos 更新事件，维护响应式状态
 * 使用全局单例模式确保状态共享
 */

import { ref, computed, onMounted, onUnmounted, type Ref, type ComputedRef } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { useTerminal } from '@/composables/useTerminal'
import type { 
  Todo, 
  TodoStatus, 
  TodosUpdatePayload, 
  TodoStats 
} from '@/types/todo'
import { 
  getRootTodos, 
  getChildTodos, 
  calculateProgress,
  getTodoIndicator 
} from '@/types/todo'

export interface UseTodosReturn {
  // 状态
  todos: ComputedRef<Todo[]>
  todosByExecutionId: ComputedRef<Record<string, Todo[]>>
  executionIds: ComputedRef<string[]>
  lastExecutionId: ComputedRef<string | undefined>
  rootTodos: ComputedRef<Todo[]>
  stats: ComputedRef<TodoStats>
  progress: ComputedRef<number>
  hasTodos: ComputedRef<boolean>
  hasHistory: ComputedRef<boolean>
  isTodosPanelActive: Ref<boolean>
  currentTask: ComputedRef<Todo | undefined>
  
  // 方法
  getTodosForExecution: (executionId: string) => Todo[]
  getChildren: (parentId: string) => Todo[]
  getIndicator: (status: TodoStatus) => string
  clearTodos: () => void
  clearTodosForExecution: (executionId: string) => void
  open: () => void
  close: () => void
  toggle: () => void
  
  // 生命周期
  startListening: () => Promise<void>
  stopListening: () => void
}

// Global state for todos panel (singleton pattern like useTerminal)
const globalTodosState = ref<{
  todosByExecutionId: Record<string, Todo[]>
  isTodosPanelActive: boolean
  lastExecutionId: string | undefined
}>({
  todosByExecutionId: {},
  isTodosPanelActive: false,
  lastExecutionId: undefined,
})

// Global unlisten function
let globalUnlisten: UnlistenFn | null = null
let listenerCount = 0

/**
 * Todos 管理 Composable
 * @param executionId 执行 ID，用于过滤事件
 */
export function useTodos(executionId?: Ref<string> | string): UseTodosReturn {
  // 获取当前 executionId
  const getExecutionId = (): string | undefined => {
    if (!executionId) return undefined
    return typeof executionId === 'string' ? executionId : executionId.value
  }

  const todos = computed<Todo[]>(() => {
    const targetId = getExecutionId()
    if (targetId) return globalTodosState.value.todosByExecutionId[targetId] || []
    const latestId = globalTodosState.value.lastExecutionId
    if (latestId) return globalTodosState.value.todosByExecutionId[latestId] || []
    return []
  })

  const todosByExecutionId = computed(() => globalTodosState.value.todosByExecutionId)
  const executionIds = computed(() => Object.keys(globalTodosState.value.todosByExecutionId))
  const lastExecutionId = computed(() => globalTodosState.value.lastExecutionId)

  // 顶级任务（无 parent_id）
  const rootTodos = computed(() => getRootTodos(todos.value))

  // 统计信息
  const stats = computed<TodoStats>(() => ({
    total: todos.value.length,
    pending: todos.value.filter(t => t.status === 'pending').length,
    in_progress: todos.value.filter(t => t.status === 'in_progress').length,
    completed: todos.value.filter(t => t.status === 'completed').length,
  }))

  // 完成进度
  const progress = computed(() => calculateProgress(todos.value))

  // 是否有 todos（实时数据）
  const hasTodos = computed(() => todos.value.length > 0)

  // 是否有历史记录（用于判断是否可以重新打开面板）
  const hasHistory = computed(() => {
    return Object.values(globalTodosState.value.todosByExecutionId).some((items) => items.length > 0)
  })

  // 当前进行中的任务
  const currentTask = computed(() => 
    todos.value.find(t => t.status === 'in_progress')
  )

  const getTodosForExecution = (id: string): Todo[] => {
    if (!id) return []
    return globalTodosState.value.todosByExecutionId[id] || []
  }

  // 获取子任务
  const getChildren = (parentId: string): Todo[] => {
    return getChildTodos(todos.value, parentId)
  }

  // 获取状态指示符
  const getIndicator = (status: TodoStatus): string => {
    return getTodoIndicator(status)
  }

  // 清空 todos
  const clearTodos = (): void => {
    const targetId = getExecutionId()
    if (targetId) {
      delete globalTodosState.value.todosByExecutionId[targetId]
      if (globalTodosState.value.lastExecutionId === targetId) {
        globalTodosState.value.lastExecutionId = undefined
      }
      return
    }
    globalTodosState.value.todosByExecutionId = {}
    globalTodosState.value.lastExecutionId = undefined
  }

  const clearTodosForExecution = (id: string): void => {
    if (!id) return
    if (!(id in globalTodosState.value.todosByExecutionId)) return
    delete globalTodosState.value.todosByExecutionId[id]
    if (globalTodosState.value.lastExecutionId === id) {
      globalTodosState.value.lastExecutionId = undefined
    }
  }

  // 打开面板
  const open = (): void => {
    globalTodosState.value.isTodosPanelActive = true
  }

  // 关闭面板
  const close = (): void => {
    globalTodosState.value.isTodosPanelActive = false
  }

  // 切换面板
  const toggle = (): void => {
    globalTodosState.value.isTodosPanelActive = !globalTodosState.value.isTodosPanelActive
  }

  // 开始监听事件（全局单例）
  const startListening = async (): Promise<void> => {
    listenerCount++
    if (globalUnlisten) return // 已在监听

    const unlistenTodos = await listen<TodosUpdatePayload>('agent-todos-update', (event) => {
      const targetId = getExecutionId()
      
      // 如果指定了 executionId，则过滤
      if (targetId && event.payload.execution_id !== targetId) {
        return
      }

      globalTodosState.value.todosByExecutionId = {
        ...globalTodosState.value.todosByExecutionId,
        [event.payload.execution_id]: event.payload.todos,
      }
      globalTodosState.value.lastExecutionId = event.payload.execution_id

      // 当有新 todos 时自动打开面板，同时关闭终端面板
      if (event.payload.todos.length > 0) {
        globalTodosState.value.isTodosPanelActive = true
        // Close terminal panel to ensure only one panel is active
        const terminal = useTerminal()
        terminal.closeTerminal()
      }
    })

    // 监听 agent 完成事件，可选择性关闭面板（但保留历史）
    const unlistenComplete = await listen<{ execution_id: string; success: boolean }>('agent:complete', (event) => {
      const targetId = getExecutionId()
      if (targetId && event.payload.execution_id !== targetId) {
        return
      }
      console.log('[useTodos] Agent execution completed:', event.payload.execution_id)
      // 不清空 todos，保留历史记录供用户查看
    })

    // 监听 agent 错误事件
    const unlistenError = await listen<{ execution_id: string; error: string }>('agent:error', (event) => {
      const targetId = getExecutionId()
      if (targetId && event.payload.execution_id !== targetId) {
        return
      }
      console.log('[useTodos] Agent execution failed:', event.payload.execution_id)
      // 不清空 todos，保留历史记录供用户查看
    })

    // 将所有 unlisten 函数组合
    globalUnlisten = () => {
      unlistenTodos()
      unlistenComplete()
      unlistenError()
    }
  }

  // 停止监听
  const stopListening = (): void => {
    listenerCount--
    // Only actually stop when no more listeners
    if (listenerCount <= 0 && globalUnlisten) {
      globalUnlisten()
      globalUnlisten = null
      listenerCount = 0
    }
  }

  // 自动开始/停止监听
  onMounted(() => {
    startListening()
  })

  onUnmounted(() => {
    stopListening()
  })

  return {
    todos,
    todosByExecutionId,
    executionIds,
    lastExecutionId,
    rootTodos,
    stats,
    progress,
    hasTodos,
    hasHistory,
    isTodosPanelActive: computed(() => globalTodosState.value.isTodosPanelActive),
    currentTask,
    getTodosForExecution,
    getChildren,
    getIndicator,
    clearTodos,
    clearTodosForExecution,
    open,
    close,
    toggle,
    startListening,
    stopListening,
  }
}

/**
 * 全局 Todos 管理（不过滤 executionId）
 */
export function useGlobalTodos(): UseTodosReturn {
  return useTodos()
}
