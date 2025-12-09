# Security Agent API Reference

本文档描述 Sentinel AI Security Agent 的核心 API。

---

## 后端模块 (Rust)

### 1. TodoManager

任务进度管理器，追踪 Agent 执行步骤。

```rust
use crate::agents::{Todo, TodoManager, TodoStatus};
```

#### 结构体

##### `Todo`

单个任务项。

```rust
pub struct Todo {
    pub id: String,           // 唯一标识
    pub content: String,      // 任务描述（最长 70 字符）
    pub status: TodoStatus,   // 状态
    pub created_at: u64,      // 创建时间 (ms)
    pub updated_at: u64,      // 更新时间 (ms)
    pub metadata: Option<TodoMetadata>,
}
```

##### `TodoStatus`

```rust
pub enum TodoStatus {
    Pending,      // ○ 待办
    InProgress,   // → 进行中
    Completed,    // ✓ 已完成
    Cancelled,    // ✗ 已取消
}
```

##### `TodoMetadata`

```rust
pub struct TodoMetadata {
    pub tool_name: Option<String>,   // 关联工具
    pub step_index: Option<usize>,   // 步骤序号
    pub parent_id: Option<String>,   // 父任务 ID（支持嵌套）
    pub tags: Option<Vec<String>>,   // 标签
}
```

#### 方法

| 方法 | 描述 | 签名 |
|------|------|------|
| `new` | 创建管理器 | `fn new(app_handle: Option<AppHandle>) -> Self` |
| `write_todos` | 写入/更新 todos | `async fn write_todos(&self, execution_id: &str, todos: Vec<Todo>, merge: bool) -> Result<()>` |
| `update_status` | 更新单个状态 | `async fn update_status(&self, execution_id: &str, todo_id: &str, status: TodoStatus) -> Result<()>` |
| `get_todos` | 获取所有 todos | `async fn get_todos(&self, execution_id: &str) -> Vec<Todo>` |
| `get_in_progress` | 获取进行中任务 | `async fn get_in_progress(&self, execution_id: &str) -> Option<Todo>` |
| `get_stats` | 获取统计信息 | `async fn get_stats(&self, execution_id: &str) -> TodoStats` |
| `clear` | 清除 todos | `async fn clear(&self, execution_id: &str)` |

#### 示例

```rust
let manager = TodoManager::new(Some(app_handle));

// 创建 todos
let todos = vec![
    Todo::new("1", "信息收集").with_status(TodoStatus::InProgress),
    Todo::new("2", "端口扫描").with_tool("port_scan"),
    Todo::new("2-1", "TCP 扫描").with_parent("2"),
];
manager.write_todos("exec-123", todos, false).await?;

// 更新状态
manager.update_status("exec-123", "1", TodoStatus::Completed).await?;
```

---

### 2. TaskPlanner

任务规划器，分析任务复杂度并解析执行计划。

```rust
use crate::agents::{TaskPlanner, PlannerConfig, TaskComplexity, TaskPlan};
```

#### 结构体

##### `TaskPlan`

```rust
pub struct TaskPlan {
    pub task: String,
    pub description: String,
    pub steps: Vec<PlanStep>,
    pub expected_outcome: String,
    pub complexity: TaskComplexity,
}
```

##### `PlanStep`

```rust
pub struct PlanStep {
    pub id: String,
    pub description: String,
    pub tool: Option<String>,
    pub args: Option<serde_json::Value>,
    pub depends_on: Vec<String>,
}
```

##### `TaskComplexity`

```rust
pub enum TaskComplexity {
    Simple,   // 1-2 步
    Medium,   // 3-5 步
    Complex,  // 5+ 步
}
```

#### 方法

| 方法 | 描述 | 签名 |
|------|------|------|
| `new` | 创建规划器 | `fn new(config: PlannerConfig) -> Self` |
| `analyze_complexity` | 分析任务复杂度 | `fn analyze_complexity(&self, task: &str) -> TaskComplexity` |
| `parse_plan_from_response` | 从 LLM 响应解析计划 | `fn parse_plan_from_response(&self, task: &str, response: &str) -> Result<TaskPlan>` |
| `plan_to_todos` | 计划转换为 Todos | `fn plan_to_todos(&self, plan: &TaskPlan) -> Vec<Todo>` |

#### 示例

```rust
let planner = TaskPlanner::new(PlannerConfig::default());

// 分析复杂度
let complexity = planner.analyze_complexity("对 target.com 进行渗透测试");
assert_eq!(complexity, TaskComplexity::Complex);

// 解析 LLM 响应
let plan = planner.parse_plan_from_response("渗透测试", llm_response)?;
let todos = planner.plan_to_todos(&plan);
```

---

### 3. AgentOrchestrator

Agent 编排器，协调规划、执行和进度追踪。

```rust
use crate::agents::{AgentOrchestrator, OrchestratorConfig, TaskPreparation};
```

#### 配置

```rust
pub struct OrchestratorConfig {
    pub auto_create_todos: bool,   // 自动创建 Todos
    pub force_todos: bool,         // 强制创建 Todos
    pub planner_config: PlannerConfig,
}
```

#### 方法

| 方法 | 描述 | 签名 |
|------|------|------|
| `new` | 创建编排器 | `fn new(config: OrchestratorConfig, app_handle: Option<AppHandle>) -> Self` |
| `prepare_task` | 准备任务执行 | `async fn prepare_task(&self, execution_id: &str, task: &str) -> Result<TaskPreparation>` |
| `update_plan` | 更新计划并刷新 Todos | `async fn update_plan(&self, execution_id: &str, task: &str, llm_response: &str) -> Result<Option<TaskPlan>>` |
| `mark_step_started` | 标记步骤开始 | `async fn mark_step_started(&self, execution_id: &str, step_id: &str) -> Result<()>` |
| `mark_step_completed` | 标记步骤完成 | `async fn mark_step_completed(&self, execution_id: &str, step_id: &str) -> Result<()>` |
| `mark_step_failed` | 标记步骤失败 | `async fn mark_step_failed(&self, execution_id: &str, step_id: &str) -> Result<()>` |
| `advance_todos` | 自动推进 Todos | `async fn advance_todos(&self, execution_id: &str) -> Result<Option<String>>` |
| `cleanup` | 清理执行数据 | `async fn cleanup(&self, execution_id: &str)` |

#### 示例

```rust
let orchestrator = AgentOrchestrator::new(OrchestratorConfig::default(), app_handle);

// 准备任务
let prep = orchestrator.prepare_task("exec-123", "渗透测试任务").await?;
if prep.todos_created {
    println!("Todos created for complex task");
}

// 更新计划
orchestrator.update_plan("exec-123", "渗透测试", llm_plan_response).await?;

// 推进执行
orchestrator.mark_step_completed("exec-123", "step-1").await?;
orchestrator.advance_todos("exec-123").await?;
```

---

### 4. StreamingOptimizer

流式输出优化工具。

```rust
use crate::utils::streaming_optimizer::{StreamBuffer, StreamBufferConfig, Debouncer, BatchCollector};
```

#### StreamBuffer

内容缓冲器，减少 IPC 开销。

```rust
let buffer = StreamBuffer::new(StreamBufferConfig {
    min_buffer_size: 10,
    max_buffer_size: 100,
    flush_interval_ms: 50,
    flush_on_chars: vec!['\n', '。'],
});

// 添加内容（自动判断是否刷新）
if let Some(content) = buffer.push("Hello").await {
    send_to_frontend(&content);
}

// 强制刷新
if let Some(remaining) = buffer.flush().await {
    send_to_frontend(&remaining);
}
```

#### BatchCollector

批量收集器。

```rust
let collector: BatchCollector<TodoUpdate> = BatchCollector::new(5, DebounceConfig::default());

// 添加项目
if let Some(batch) = collector.add(update).await {
    process_batch(batch);
}

// 定时检查刷新
if let Some(batch) = collector.try_flush().await {
    process_batch(batch);
}
```

---

## 前端 API (TypeScript)

### 1. Types

```typescript
// src/types/todo.ts

type TodoStatus = 'pending' | 'in_progress' | 'completed' | 'cancelled'

interface Todo {
  id: string
  content: string
  status: TodoStatus
  created_at: number
  updated_at: number
  metadata?: TodoMetadata
}

interface TodoMetadata {
  tool_name?: string
  step_index?: number
  parent_id?: string
  tags?: string[]
}
```

### 2. useTodos Composable

```typescript
import { useTodos } from '@/composables/useTodos'

// 基本用法
const { 
  todos,           // Ref<Todo[]>
  rootTodos,       // ComputedRef<Todo[]> - 顶级任务
  stats,           // ComputedRef<TodoStats>
  progress,        // ComputedRef<number> - 完成百分比
  hasTodos,        // ComputedRef<boolean>
  currentTask,     // ComputedRef<Todo | undefined>
  getChildren,     // (parentId: string) => Todo[]
  getIndicator,    // (status: TodoStatus) => string
} = useTodos(executionId)

// 指定 executionId 过滤
const { todos } = useTodos(ref('exec-123'))

// 全局监听（不过滤）
const { todos } = useTodos()
```

### 3. TodoPanel 组件

```vue
<template>
  <TodoPanel :todos="todos" />
</template>

<script setup>
import { TodoPanel } from '@/components/Agent'
import { useTodos } from '@/composables/useTodos'

const { todos } = useTodos()
</script>
```

### 4. 辅助函数

```typescript
import { 
  getTodoIndicator,
  isSubTodo,
  getChildTodos,
  getRootTodos,
  calculateProgress,
  calculateChildrenProgress,
} from '@/types/todo'

// 获取状态指示符
getTodoIndicator('completed')  // '✓'

// 判断是否为子任务
isSubTodo(todo)  // boolean

// 计算进度
calculateProgress(todos)  // 0-100
```

---

## Tauri Events

### agent-todos-update

Todos 更新事件。

```typescript
import { listen } from '@tauri-apps/api/event'

interface TodosUpdatePayload {
  execution_id: string
  todos: Todo[]
  timestamp: number
}

await listen<TodosUpdatePayload>('agent-todos-update', (event) => {
  console.log('Todos updated:', event.payload.todos)
})
```

---

## 使用流程

### 完整执行流程

```
1. 用户输入任务
       ↓
2. Orchestrator.prepare_task()
   - 分析复杂度
   - 决定是否创建 Todos
       ↓
3. ReAct 引擎执行
   - LLM 生成计划
   - Orchestrator.update_plan() → 更新 Todos
       ↓
4. 执行工具调用
   - Orchestrator.mark_step_started()
   - 执行工具
   - Orchestrator.mark_step_completed() / mark_step_failed()
       ↓
5. 前端通过 agent-todos-update 事件实时更新 UI
       ↓
6. 执行完成
   - Orchestrator.cleanup()
```

---

## 版本信息

- 文档版本: 1.0.0
- 创建日期: 2024-12-08
- 适用版本: Sentinel AI v0.1.x

