/**
 * Todo 任务管理类型定义
 * 参考 Claude Code 的 TodoWrite 工具机制
 */

// Todo 状态（与 Claude Code 保持一致）
export type TodoStatus = 
  | 'pending'       // ○ 待办
  | 'in_progress'   // → 进行中
  | 'completed'     // ✓ 已完成

// Todo 元数据
export interface TodoMetadata {
  tool_name?: string      // 关联的工具
  step_index?: number     // 步骤序号
  parent_id?: string      // 父任务 ID（支持子任务）
  tags?: string[]         // 标签
  error?: string          // 错误信息（如果有）
}

// 单个 Todo 项
// 与 Claude Code 的 TodoWrite 保持一致:
// - content: 祈使句描述任务（如 "Run tests"）
// - active_form: 进行时态描述（如 "Running tests"）
export interface Todo {
  id: string              // 唯一标识
  content: string         // 任务描述（祈使句形式，限制 70 字符）
  active_form?: string    // 进行时描述（用于显示当前执行状态）
  status: TodoStatus      // 任务状态
  created_at: number      // 创建时间戳 (ms)
  updated_at: number      // 更新时间戳 (ms)
  metadata?: TodoMetadata // 扩展元数据
}

// TodoWrite 工具输入（与 Claude Code 一致）
export interface TodoWriteInput {
  todos: TodoItem[]
}

// 单个 Todo 项的输入（与 Claude Code 一致）
export interface TodoItem {
  content: string
  status: TodoStatus
  activeForm: string
}

// Todo 列表
export interface TodoList {
  execution_id: string    // 关联的执行 ID
  todos: Todo[]           // 任务列表
  created_at: number
  updated_at: number
}

// Todos 更新事件负载
export interface TodosUpdatePayload {
  execution_id: string
  todos: Todo[]
  timestamp: number
}

// Todo 统计信息
export interface TodoStats {
  total: number
  pending: number
  in_progress: number
  completed: number
}

// 状态指示符映射
export const TodoIndicators: Record<TodoStatus, string> = {
  pending: '○',
  in_progress: '→',
  completed: '✓',
}

// 状态颜色映射
export const TodoStatusColors: Record<TodoStatus, string> = {
  pending: 'text-base-content/60',
  in_progress: 'text-primary',
  completed: 'text-success',
}

// 获取 Todo 显示文本（如果正在执行，返回 active_form）
export function getTodoDisplayText(todo: Todo): string {
  if (todo.status === 'in_progress' && todo.active_form) {
    return todo.active_form
  }
  return todo.content
}

// 获取状态指示符
export function getTodoIndicator(status: TodoStatus): string {
  return TodoIndicators[status]
}

// 判断 Todo 是否为子任务
export function isSubTodo(todo: Todo): boolean {
  return !!todo.metadata?.parent_id
}

// 获取 Todo 的子任务
export function getChildTodos(todos: Todo[], parentId: string): Todo[] {
  return todos.filter(t => t.metadata?.parent_id === parentId)
}

// 获取顶级 Todo（无 parent_id）
export function getRootTodos(todos: Todo[]): Todo[] {
  return todos.filter(t => !t.metadata?.parent_id)
}

// 计算完成进度
export function calculateProgress(todos: Todo[]): number {
  if (todos.length === 0) return 0
  const completed = todos.filter(t => t.status === 'completed').length
  return Math.round((completed / todos.length) * 100)
}

// 计算子任务完成进度
export function calculateChildrenProgress(todos: Todo[], parentId: string): { completed: number; total: number } {
  const children = getChildTodos(todos, parentId)
  return {
    completed: children.filter(c => c.status === 'completed').length,
    total: children.length,
  }
}

