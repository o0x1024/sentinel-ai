/**
 * Todos 状态管理 Composable
 * 监听后端 todos 更新事件，维护响应式状态
 */

import { ref, computed, onMounted, onUnmounted, type Ref, type ComputedRef } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
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
  todos: Ref<Todo[]>
  rootTodos: ComputedRef<Todo[]>
  stats: ComputedRef<TodoStats>
  progress: ComputedRef<number>
  hasTodos: ComputedRef<boolean>
  currentTask: ComputedRef<Todo | undefined>
  
  // 方法
  getChildren: (parentId: string) => Todo[]
  getIndicator: (status: TodoStatus) => string
  
  // 生命周期
  startListening: () => Promise<void>
  stopListening: () => void
}

/**
 * Todos 管理 Composable
 * @param executionId 执行 ID，用于过滤事件
 */
export function useTodos(executionId?: Ref<string> | string): UseTodosReturn {
  const todos = ref<Todo[]>([])
  let unlisten: UnlistenFn | null = null

  // 获取当前 executionId
  const getExecutionId = (): string | undefined => {
    if (!executionId) return undefined
    return typeof executionId === 'string' ? executionId : executionId.value
  }

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

  // 是否有 todos
  const hasTodos = computed(() => todos.value.length > 0)

  // 当前进行中的任务
  const currentTask = computed(() => 
    todos.value.find(t => t.status === 'in_progress')
  )

  // 获取子任务
  const getChildren = (parentId: string): Todo[] => {
    return getChildTodos(todos.value, parentId)
  }

  // 获取状态指示符
  const getIndicator = (status: TodoStatus): string => {
    return getTodoIndicator(status)
  }

  // 开始监听事件
  const startListening = async (): Promise<void> => {
    if (unlisten) return // 已在监听

    unlisten = await listen<TodosUpdatePayload>('agent-todos-update', (event) => {
      const targetId = getExecutionId()
      
      // 如果指定了 executionId，则过滤
      if (targetId && event.payload.execution_id !== targetId) {
        return
      }

      todos.value = event.payload.todos
    })
  }

  // 停止监听
  const stopListening = (): void => {
    if (unlisten) {
      unlisten()
      unlisten = null
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
    rootTodos,
    stats,
    progress,
    hasTodos,
    currentTask,
    getChildren,
    getIndicator,
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

