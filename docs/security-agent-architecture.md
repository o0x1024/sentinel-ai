# Sentinel Security Agent Architecture

## 一、概述

本文档定义 Sentinel AI 安全 Agent 的完整架构设计，参考 Cursor Agent 模式，针对安全任务进行优化。

### 1.1 目标

- **普通对话问答**：支持安全知识问答、概念解释
- **智能工具调用**：自动规划和执行工具链
- **安全任务处理**：
  - 渗透测试（Penetration Testing）
  - 漏洞利用（Exploitation）
  - 代码审计（Code Audit）
  - CTF 比赛（Capture The Flag）
  - 内网渗透（Internal Penetration）
  - 漏洞修复（Vulnerability Remediation）
  - 基线检测与修复（Baseline Detection & Hardening）

### 1.2 设计原则

- **推翻重来**：不考虑向后兼容，全新设计
- **Markdown 优先**：前端渲染使用纯 Markdown，不使用 Icon
- **流式输出**：所有内容支持 SSE 流式传输
- **任务驱动**：Agent 自主规划、执行、迭代直到任务完成

---

## 二、系统架构

### 2.1 整体架构图

```
┌─────────────────────────────────────────────────────────────────┐
│                        前端 (Vue 3)                              │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐  │
│  │ ChatInput   │  │ MessageFlow │  │ MarkdownRenderer        │  │
│  │ (输入区域)   │  │ (消息流)     │  │ (Markdown渲染)          │  │
│  └─────────────┘  └─────────────┘  └─────────────────────────┘  │
├─────────────────────────────────────────────────────────────────┤
│                    Tauri IPC Bridge                              │
├─────────────────────────────────────────────────────────────────┤
│                        后端 (Rust)                               │
├─────────────────────────────────────────────────────────────────┤
│  ┌──────────────────────────────────────────────────────────┐   │
│  │                  Agent Orchestrator                       │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐       │   │
│  │  │ Planner     │  │ Executor    │  │ Reflector   │       │   │
│  │  │ (规划器)     │  │ (执行器)     │  │ (反思器)    │       │   │
│  │  └─────────────┘  └─────────────┘  └─────────────┘       │   │
│  └──────────────────────────────────────────────────────────┘   │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │                  Tool System                              │   │
│  │  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────────┐  │   │
│  │  │ Builtin │  │   MCP   │  │ Plugin  │  │ Security    │  │   │
│  │  │ Tools   │  │ Servers │  │ System  │  │ Tools       │  │   │
│  │  └─────────┘  └─────────┘  └─────────┘  └─────────────┘  │   │
│  └──────────────────────────────────────────────────────────┘   │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │                  LLM Service                              │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐   │   │
│  │  │ OpenAI      │  │ Claude      │  │ Local Models    │   │   │
│  │  └─────────────┘  └─────────────┘  └─────────────────┘   │   │
│  └──────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

### 2.2 核心模块

| 模块 | 职责 | 文件位置 |
|------|------|----------|
| Agent Orchestrator | 任务编排、生命周期管理 | `src-tauri/src/agent/` |
| Planner | 任务分解、步骤规划 | `src-tauri/src/agent/planner.rs` |
| Executor | 步骤执行、工具调用 | `src-tauri/src/agent/executor.rs` |
| Reflector | 结果评估、迭代决策 | `src-tauri/src/agent/reflector.rs` |
| Tool System | 统一工具管理 | `src-tauri/src/tools/` |
| Message Emitter | 流式消息发送 | `src-tauri/src/agent/emitter.rs` |
| **Todo Manager** | **任务进度管理** | `src-tauri/src/agent/todo_manager.rs` |

---

## 三、Todos 任务管理系统

参考 Cursor Agent 的 todos 管理机制，实现可视化的任务进度追踪。

### 3.1 设计理念

Cursor 的 todos 系统核心特点：

1. **任务可视化**：独立面板展示当前任务列表
2. **状态实时更新**：任务状态变化即时反映到 UI
3. **智能触发**：复杂任务（3+ 步骤）自动创建 todos
4. **单一焦点**：同一时间只有一个任务处于 `in_progress`

### 3.2 数据结构

```typescript
// src/types/todo.ts

export interface Todo {
  id: string                    // 唯一标识
  content: string               // 任务描述（限制 70 字符）
  status: TodoStatus            // 任务状态
  created_at: number            // 创建时间
  updated_at: number            // 更新时间
  metadata?: TodoMetadata       // 扩展元数据
}

export type TodoStatus = 
  | 'pending'       // ○ 待办
  | 'in_progress'   // → 进行中
  | 'completed'     // ✓ 已完成
  | 'cancelled'     // ✗ 已取消

export interface TodoMetadata {
  tool_name?: string            // 关联的工具
  step_index?: number           // 步骤序号
  parent_id?: string            // 父任务 ID（支持子任务）
  tags?: string[]               // 标签（如 'pentest', 'audit'）
}

export interface TodoList {
  execution_id: string          // 关联的执行 ID
  todos: Todo[]                 // 任务列表
  created_at: number
  updated_at: number
}
```

### 3.3 后端实现

```rust
// src-tauri/src/agent/todo_manager.rs

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Todo {
    pub id: String,
    pub content: String,
    pub status: TodoStatus,
    pub created_at: u64,
    pub updated_at: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<TodoMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TodoStatus {
    Pending,
    InProgress,
    Completed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoMetadata {
    pub tool_name: Option<String>,
    pub step_index: Option<usize>,
    pub parent_id: Option<String>,
    pub tags: Option<Vec<String>>,
}

/// Todo 管理器
pub struct TodoManager {
    todos: Arc<RwLock<HashMap<String, Vec<Todo>>>>,
    emitter: MessageEmitter,
}

impl TodoManager {
    pub fn new(emitter: MessageEmitter) -> Self {
        Self {
            todos: Arc::new(RwLock::new(HashMap::new())),
            emitter,
        }
    }

    /// 写入/更新 todos
    pub async fn write_todos(
        &self,
        execution_id: &str,
        todos: Vec<Todo>,
        merge: bool,
    ) -> Result<(), anyhow::Error> {
        let mut store = self.todos.write().await;
        
        if merge {
            // 合并模式：根据 id 更新现有 todos
            let existing = store.entry(execution_id.to_string())
                .or_insert_with(Vec::new);
            
            for new_todo in todos {
                if let Some(pos) = existing.iter().position(|t| t.id == new_todo.id) {
                    existing[pos] = new_todo;
                } else {
                    existing.push(new_todo);
                }
            }
        } else {
            // 替换模式：直接替换所有 todos
            store.insert(execution_id.to_string(), todos);
        }
        
        // 发送更新事件到前端
        let current = store.get(execution_id).cloned().unwrap_or_default();
        self.emitter.emit_todos_update(execution_id, &current);
        
        Ok(())
    }

    /// 更新单个 todo 状态
    pub async fn update_status(
        &self,
        execution_id: &str,
        todo_id: &str,
        status: TodoStatus,
    ) -> Result<(), anyhow::Error> {
        let mut store = self.todos.write().await;
        
        if let Some(todos) = store.get_mut(execution_id) {
            if let Some(todo) = todos.iter_mut().find(|t| t.id == todo_id) {
                todo.status = status;
                todo.updated_at = chrono::Utc::now().timestamp_millis() as u64;
            }
        }
        
        let current = store.get(execution_id).cloned().unwrap_or_default();
        self.emitter.emit_todos_update(execution_id, &current);
        
        Ok(())
    }

    /// 获取当前 todos
    pub async fn get_todos(&self, execution_id: &str) -> Vec<Todo> {
        let store = self.todos.read().await;
        store.get(execution_id).cloned().unwrap_or_default()
    }

    /// 清除 todos
    pub async fn clear(&self, execution_id: &str) {
        let mut store = self.todos.write().await;
        store.remove(execution_id);
    }
}
```

### 3.4 消息发送器扩展

```rust
// 在 MessageEmitter 中添加 todos 相关方法

impl MessageEmitter {
    /// 发送 todos 更新事件
    pub fn emit_todos_update(&self, execution_id: &str, todos: &[Todo]) {
        if let Some(app) = &self.app_handle {
            let _ = app.emit_all("agent-todos-update", serde_json::json!({
                "execution_id": execution_id,
                "todos": todos,
                "timestamp": chrono::Utc::now().timestamp_millis(),
            }));
        }
    }
}
```

### 3.5 前端组件

```vue
<!-- src/components/Agent/TodoPanel.vue -->
<template>
  <div class="todo-panel" v-if="todos.length > 0">
    <div class="todo-header">
      **To-dos** {{ rootTodos.length }}
    </div>
    
    <div class="todo-list">
      <!-- 递归渲染支持嵌套 -->
      <TodoItem 
        v-for="todo in rootTodos" 
        :key="todo.id"
        :todo="todo"
        :children="getChildren(todo.id)"
        :get-children="getChildren"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import type { Todo } from '@/types/todo'
import TodoItem from './TodoItem.vue'

const props = defineProps<{
  todos: Todo[]
}>()

// 顶级任务（无 parent_id）
const rootTodos = computed(() => 
  props.todos.filter(t => !t.metadata?.parent_id)
)

// 获取某个任务的子任务
const getChildren = (parentId: string): Todo[] => {
  return props.todos.filter(t => t.metadata?.parent_id === parentId)
}
</script>

<style scoped>
.todo-panel {
  border: 1px solid var(--color-border);
  border-radius: 0.5rem;
  padding: 0.75rem;
  margin: 1rem 0;
  background: var(--color-bg-secondary);
}

.todo-header {
  font-size: 0.875rem;
  font-weight: 600;
  color: var(--color-text-secondary);
  margin-bottom: 0.5rem;
}

.todo-list {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}
</style>
```

#### TodoItem 子组件（支持递归嵌套）

```vue
<!-- src/components/Agent/TodoItem.vue -->
<template>
  <div class="todo-item-wrapper">
    <!-- 当前任务 -->
    <div :class="['todo-item', `status-${todo.status}`]">
      <span class="todo-indicator">{{ getIndicator(todo.status) }}</span>
      <span class="todo-content">{{ todo.content }}</span>
      <span v-if="children.length > 0" class="todo-children-count">
        ({{ completedChildren }}/{{ children.length }})
      </span>
    </div>
    
    <!-- 子任务（递归） -->
    <div v-if="children.length > 0" class="todo-children">
      <TodoItem 
        v-for="child in children" 
        :key="child.id"
        :todo="child"
        :children="getChildren(child.id)"
        :get-children="getChildren"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import type { Todo, TodoStatus } from '@/types/todo'

const props = defineProps<{
  todo: Todo
  children: Todo[]
  getChildren: (parentId: string) => Todo[]
}>()

const getIndicator = (status: TodoStatus): string => {
  switch (status) {
    case 'pending': return '○'
    case 'in_progress': return '→'
    case 'completed': return '✓'
    case 'cancelled': return '✗'
  }
}

// 已完成的子任务数
const completedChildren = computed(() => 
  props.children.filter(c => c.status === 'completed').length
)
</script>

<style scoped>
.todo-item-wrapper {
  display: flex;
  flex-direction: column;
}

.todo-item {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  font-size: 0.875rem;
  padding: 0.25rem 0;
}

.todo-indicator {
  width: 1rem;
  text-align: center;
  font-weight: bold;
  flex-shrink: 0;
}

.todo-content {
  flex: 1;
}

.todo-children-count {
  font-size: 0.75rem;
  color: var(--color-text-secondary);
}

/* 子任务缩进 */
.todo-children {
  margin-left: 1.25rem;
  padding-left: 0.5rem;
  border-left: 1px solid var(--color-border);
}

/* 状态样式 */
.status-pending .todo-indicator { color: var(--color-text-secondary); }
.status-in_progress .todo-indicator { color: var(--color-primary); }
.status-completed .todo-indicator { color: var(--color-success); }
.status-completed .todo-content { 
  text-decoration: line-through; 
  color: var(--color-text-secondary); 
}
.status-cancelled .todo-indicator { color: var(--color-error); }
.status-cancelled .todo-content { 
  text-decoration: line-through; 
  color: var(--color-text-secondary); 
}
</style>
```

### 3.6 事件监听 Composable

```typescript
// src/composables/useTodos.ts

import { ref, onMounted, onUnmounted } from 'vue'
import { listen } from '@tauri-apps/api/event'
import type { Todo } from '@/types/todo'

export function useTodos(executionId: string) {
  const todos = ref<Todo[]>([])
  let unlisten: (() => void) | null = null

  onMounted(async () => {
    unlisten = await listen('agent-todos-update', (event: any) => {
      if (event.payload.execution_id === executionId) {
        todos.value = event.payload.todos
      }
    })
  })

  onUnmounted(() => {
    unlisten?.()
  })

  return {
    todos,
  }
}
```

### 3.7 与 Agent 执行流程集成

```rust
// 在 Orchestrator 中集成 TodoManager

impl AgentOrchestrator {
    pub async fn run(&self, task: &str) -> Result<AgentResult> {
        let execution_id = uuid::Uuid::new_v4().to_string();
        
        // 1. 规划阶段 - 创建 todos
        let plan = self.planner.plan(&context).await?;
        
        // 将计划步骤转换为 todos
        let todos: Vec<Todo> = plan.steps.iter().enumerate().map(|(i, step)| {
            Todo {
                id: step.id.clone(),
                content: truncate(&step.description, 70),
                status: if i == 0 { TodoStatus::InProgress } else { TodoStatus::Pending },
                created_at: now(),
                updated_at: now(),
                metadata: Some(TodoMetadata {
                    step_index: Some(i),
                    tool_name: step.tool.as_ref().map(|t| t.name.clone()),
                    ..Default::default()
                }),
            }
        }).collect();
        
        // 写入 todos（替换模式）
        self.todo_manager.write_todos(&execution_id, todos, false).await?;
        
        // 2. 执行阶段 - 更新 todos 状态
        for (i, step) in plan.steps.iter().enumerate() {
            // 标记当前步骤为进行中
            self.todo_manager.update_status(&execution_id, &step.id, TodoStatus::InProgress).await?;
            
            // 执行步骤
            let result = self.executor.execute(&step, &context).await?;
            
            // 标记完成或失败
            let status = if result.success {
                TodoStatus::Completed
            } else {
                TodoStatus::Cancelled
            };
            self.todo_manager.update_status(&execution_id, &step.id, status).await?;
        }
        
        // ...
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        s.chars().take(max_len - 3).collect::<String>() + "..."
    }
}
```

### 3.8 使用规则（参考 Cursor）

**何时创建 Todos：**

1. 复杂任务（3+ 个不同步骤）
2. 需要仔细规划的非简单任务
3. 用户明确要求创建任务列表
4. 用户提供多个任务（编号/逗号分隔）
5. 安全测试流程（多阶段执行）

**何时不创建 Todos：**

1. 单一、直接的任务
2. 少于 3 步的简单任务
3. 纯对话/信息查询请求
4. 单个工具调用

**状态管理规则：**

1. 同一时间只有一个任务处于 `in_progress`
2. 完成任务后立即标记为 `completed`
3. 开始新任务前确保当前任务已完成
4. 实时更新状态，不要批量更新

---

## 四、消息协议

### 3.1 消息块类型

所有 Agent 输出统一为 Markdown 格式的消息块：

```typescript
interface AgentMessage {
  id: string
  type: MessageType
  content: string           // Markdown 格式内容
  timestamp: number
  metadata?: MessageMetadata
}

type MessageType = 
  | 'thinking'      // 思考过程
  | 'planning'      // 任务规划
  | 'tool_call'     // 工具调用
  | 'tool_result'   // 工具结果
  | 'progress'      // 进度更新
  | 'final'         // 最终答案
  | 'error'         // 错误信息

interface MessageMetadata {
  tool_name?: string
  tool_args?: Record<string, any>
  duration_ms?: number
  step_index?: number
  total_steps?: number
}
```

### 3.2 Markdown 渲染规范

#### 3.2.1 思考块

```markdown
> **Thinking**
> 
> 分析用户请求，这是一个针对 example.com 的渗透测试任务...
> 
> 需要执行以下步骤：
> 1. 信息收集
> 2. 端口扫描
> 3. 漏洞探测
```

#### 3.2.2 工具调用块

```markdown
**Tool Call** `nmap_scan`

```json
{
  "target": "example.com",
  "ports": "1-1000",
  "type": "syn"
}
```

---
```

#### 3.2.3 工具结果块

```markdown
**Result** `nmap_scan` ✓ 2.3s

```
PORT     STATE SERVICE
22/tcp   open  ssh
80/tcp   open  http
443/tcp  open  https
```

---
```

#### 3.2.4 进度块

```markdown
**Progress** Step 2/5: Port Scanning

- [x] Information Gathering
- [→] Port Scanning
- [ ] Vulnerability Detection
- [ ] Exploitation
- [ ] Reporting
```

#### 3.2.5 最终答案块

```markdown
## Summary

### Findings

| Severity | Vulnerability | Target |
|----------|--------------|--------|
| High     | SQL Injection | /api/users |
| Medium   | XSS          | /search |

### Recommendations

1. 对所有用户输入进行参数化查询
2. 实施 CSP 策略防止 XSS
```

---

## 四、Agent 执行引擎

### 4.1 执行循环

```rust
// src-tauri/src/agent/orchestrator.rs

pub struct AgentOrchestrator {
    planner: Planner,
    executor: Executor,
    reflector: Reflector,
    emitter: MessageEmitter,
    tool_manager: ToolManager,
    config: AgentConfig,
}

impl AgentOrchestrator {
    pub async fn run(&self, task: &str) -> Result<AgentResult> {
        let mut context = ExecutionContext::new(task);
        
        // 发送开始信号
        self.emitter.emit_start(&context);
        
        loop {
            // 检查取消
            if context.is_cancelled() {
                return Ok(AgentResult::cancelled());
            }
            
            // 检查迭代上限
            if context.iteration >= self.config.max_iterations {
                return Ok(AgentResult::max_iterations_reached(context));
            }
            
            // 1. 规划阶段
            let plan = self.planner.plan(&context).await?;
            self.emitter.emit_plan(&plan);
            
            // 2. 执行阶段
            for step in plan.steps {
                let result = self.executor.execute(&step, &context).await?;
                self.emitter.emit_step_result(&step, &result);
                context.add_result(step.id, result);
            }
            
            // 3. 反思阶段
            let reflection = self.reflector.reflect(&context).await?;
            
            match reflection.decision {
                Decision::Complete(answer) => {
                    self.emitter.emit_final(&answer);
                    return Ok(AgentResult::completed(answer));
                }
                Decision::Continue => {
                    context.iteration += 1;
                    continue;
                }
                Decision::Replan(reason) => {
                    self.emitter.emit_thinking(&format!("Replanning: {}", reason));
                    context.iteration += 1;
                    continue;
                }
            }
        }
    }
}
```

### 4.2 规划器

```rust
// src-tauri/src/agent/planner.rs

pub struct Planner {
    llm: LlmClient,
    prompt_builder: PromptBuilder,
}

impl Planner {
    pub async fn plan(&self, context: &ExecutionContext) -> Result<Plan> {
        let prompt = self.prompt_builder.build_planning_prompt(context);
        let response = self.llm.complete(&prompt).await?;
        
        // 解析 LLM 输出为结构化计划
        let plan = self.parse_plan(&response)?;
        Ok(plan)
    }
}

pub struct Plan {
    pub description: String,
    pub steps: Vec<PlanStep>,
    pub expected_outcome: String,
}

pub struct PlanStep {
    pub id: String,
    pub description: String,
    pub tool: Option<ToolCall>,
    pub depends_on: Vec<String>,
    pub fallback: Option<String>,
}
```

### 4.3 执行器

```rust
// src-tauri/src/agent/executor.rs

pub struct Executor {
    tool_manager: ToolManager,
    emitter: MessageEmitter,
}

impl Executor {
    pub async fn execute(
        &self,
        step: &PlanStep,
        context: &ExecutionContext,
    ) -> Result<StepResult> {
        // 发送工具调用消息
        if let Some(tool_call) = &step.tool {
            self.emitter.emit_tool_call(tool_call);
            
            let start = Instant::now();
            let result = self.tool_manager.execute(tool_call).await;
            let duration = start.elapsed();
            
            match result {
                Ok(output) => {
                    self.emitter.emit_tool_result(tool_call, &output, duration);
                    Ok(StepResult::success(output))
                }
                Err(e) => {
                    self.emitter.emit_tool_error(tool_call, &e);
                    Ok(StepResult::failed(e.to_string()))
                }
            }
        } else {
            // 无工具步骤（如分析、总结）
            Ok(StepResult::no_tool())
        }
    }
}
```

### 4.4 反思器

```rust
// src-tauri/src/agent/reflector.rs

pub struct Reflector {
    llm: LlmClient,
    prompt_builder: PromptBuilder,
}

impl Reflector {
    pub async fn reflect(&self, context: &ExecutionContext) -> Result<Reflection> {
        let prompt = self.prompt_builder.build_reflection_prompt(context);
        let response = self.llm.complete(&prompt).await?;
        
        // 解析反思结果
        let reflection = self.parse_reflection(&response)?;
        Ok(reflection)
    }
}

pub struct Reflection {
    pub decision: Decision,
    pub reasoning: String,
    pub improvements: Vec<String>,
}

pub enum Decision {
    Complete(String),      // 任务完成，返回最终答案
    Continue,              // 继续执行当前计划
    Replan(String),        // 需要重新规划，附带原因
}
```

---

## 五、工具系统

### 5.1 工具定义

```rust
// src-tauri/src/tools/definition.rs

pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub category: ToolCategory,
    pub parameters: Vec<ToolParameter>,
    pub returns: ToolReturn,
    pub examples: Vec<ToolExample>,
    pub timeout: Duration,
    pub requires_confirmation: bool,
}

pub enum ToolCategory {
    // 信息收集
    Reconnaissance,
    // 扫描探测
    Scanning,
    // 漏洞利用
    Exploitation,
    // 后渗透
    PostExploitation,
    // 代码分析
    CodeAnalysis,
    // 基线检查
    BaselineCheck,
    // 修复建议
    Remediation,
    // 通用工具
    Utility,
}
```

### 5.2 安全工具清单

#### 5.2.1 信息收集工具

| 工具名称 | 描述 | 参数 |
|---------|------|------|
| `whois_lookup` | WHOIS 信息查询 | `domain: string` |
| `dns_enum` | DNS 枚举 | `domain: string, types: string[]` |
| `subdomain_scan` | 子域名扫描 | `domain: string, wordlist?: string` |
| `ip_lookup` | IP 地理位置查询 | `ip: string` |
| `certificate_info` | SSL 证书信息 | `domain: string` |
| `web_tech_detect` | Web 技术栈检测 | `url: string` |

#### 5.2.2 扫描探测工具

| 工具名称 | 描述 | 参数 |
|---------|------|------|
| `port_scan` | 端口扫描 | `target: string, ports: string, type: string` |
| `service_detect` | 服务识别 | `target: string, port: number` |
| `vuln_scan` | 漏洞扫描 | `target: string, scan_type: string` |
| `web_crawl` | Web 爬虫 | `url: string, depth: number` |
| `dir_brute` | 目录爆破 | `url: string, wordlist: string` |

#### 5.2.3 漏洞利用工具

| 工具名称 | 描述 | 参数 |
|---------|------|------|
| `sql_inject_test` | SQL 注入测试 | `url: string, param: string` |
| `xss_test` | XSS 测试 | `url: string, payload_type: string` |
| `ssrf_test` | SSRF 测试 | `url: string, target_url: string` |
| `rce_test` | 远程代码执行测试 | `target: string, payload: string` |
| `exploit_run` | 运行 Exploit | `exploit_id: string, options: object` |

#### 5.2.4 代码审计工具

| 工具名称 | 描述 | 参数 |
|---------|------|------|
| `code_scan` | 代码安全扫描 | `path: string, language: string` |
| `sast_analyze` | 静态分析 | `code: string, rules: string[]` |
| `secret_scan` | 敏感信息扫描 | `path: string` |
| `dependency_check` | 依赖漏洞检查 | `path: string` |

#### 5.2.5 基线检测工具

| 工具名称 | 描述 | 参数 |
|---------|------|------|
| `baseline_check` | 基线合规检查 | `target: string, baseline: string` |
| `config_audit` | 配置审计 | `service: string, config_path: string` |
| `permission_check` | 权限检查 | `path: string` |
| `network_policy_check` | 网络策略检查 | `host: string` |

### 5.3 工具注册

```rust
// src-tauri/src/tools/registry.rs

pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
    categories: HashMap<ToolCategory, Vec<String>>,
}

impl ToolRegistry {
    pub fn register_builtin_tools(&mut self) {
        // 信息收集
        self.register(WhoisLookup::new());
        self.register(DnsEnum::new());
        self.register(SubdomainScan::new());
        
        // 扫描探测
        self.register(PortScan::new());
        self.register(VulnScan::new());
        
        // 漏洞利用
        self.register(SqlInjectTest::new());
        self.register(XssTest::new());
        
        // 代码审计
        self.register(CodeScan::new());
        self.register(SecretScan::new());
        
        // 基线检测
        self.register(BaselineCheck::new());
    }
    
    pub fn register_mcp_tools(&mut self, mcp_client: &McpClient) {
        // 从 MCP Server 动态注册工具
        for tool in mcp_client.list_tools() {
            self.register(McpToolWrapper::new(tool));
        }
    }
}
```

---

## 六、前端组件

### 6.1 组件结构

```
src/components/Agent/
├── index.ts                    # 导出
├── AgentView.vue              # 主视图容器
├── MessageFlow.vue            # 消息流
├── MessageBlock.vue           # 消息块（通用）
├── MarkdownRenderer.vue       # Markdown 渲染器
├── ChatInput.vue              # 输入框
├── ToolCallBlock.vue          # 工具调用展示
├── ProgressBlock.vue          # 进度展示
└── types.ts                   # 类型定义
```

### 6.2 消息流组件

```vue
<!-- src/components/Agent/MessageFlow.vue -->
<template>
  <div class="message-flow">
    <div v-for="msg in messages" :key="msg.id" class="message-block">
      <MessageBlock :message="msg" />
    </div>
    
    <!-- 流式输出指示器 -->
    <div v-if="isStreaming" class="streaming-indicator">
      <span class="cursor">▊</span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import type { AgentMessage } from './types'
import MessageBlock from './MessageBlock.vue'

const props = defineProps<{
  messages: AgentMessage[]
  isStreaming: boolean
}>()

// 自动滚动到底部
const scrollToBottom = () => {
  // ...
}
</script>

<style scoped>
.message-flow {
  display: flex;
  flex-direction: column;
  gap: 1rem;
  padding: 1.5rem;
}

.streaming-indicator {
  display: inline;
}

.cursor {
  animation: blink 1s infinite;
  color: var(--color-primary);
}

@keyframes blink {
  0%, 50% { opacity: 1; }
  51%, 100% { opacity: 0; }
}
</style>
```

### 6.3 消息块组件

```vue
<!-- src/components/Agent/MessageBlock.vue -->
<template>
  <div :class="['message-block', `type-${message.type}`]">
    <MarkdownRenderer :content="formattedContent" />
    
    <!-- 工具调用特殊处理：可折叠 -->
    <div v-if="message.type === 'tool_call'" class="tool-details">
      <button @click="toggleDetails" class="toggle-btn">
        {{ isExpanded ? '收起详情' : '展开详情' }}
      </button>
      <pre v-if="isExpanded" class="tool-args">{{ formatArgs }}</pre>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import type { AgentMessage } from './types'
import MarkdownRenderer from './MarkdownRenderer.vue'

const props = defineProps<{
  message: AgentMessage
}>()

const isExpanded = ref(false)

const toggleDetails = () => {
  isExpanded.value = !isExpanded.value
}

const formattedContent = computed(() => {
  // 根据消息类型格式化为 Markdown
  switch (props.message.type) {
    case 'thinking':
      return `> **Thinking**\n>\n> ${props.message.content.replace(/\n/g, '\n> ')}`
    
    case 'tool_call':
      const meta = props.message.metadata
      return `**Tool Call** \`${meta?.tool_name}\``
    
    case 'tool_result':
      const m = props.message.metadata
      const status = m?.success ? '✓' : '✗'
      const duration = m?.duration_ms ? `${(m.duration_ms / 1000).toFixed(1)}s` : ''
      return `**Result** \`${m?.tool_name}\` ${status} ${duration}\n\n\`\`\`\n${props.message.content}\n\`\`\``
    
    case 'progress':
      return props.message.content
    
    case 'final':
      return props.message.content
    
    case 'error':
      return `> **Error**\n>\n> ${props.message.content}`
    
    default:
      return props.message.content
  }
})
</script>

<style scoped>
.message-block {
  border-radius: 0.5rem;
  padding: 1rem;
  background: var(--color-bg-secondary);
}

.type-thinking {
  border-left: 3px solid var(--color-info);
  background: var(--color-info-bg);
}

.type-tool_call {
  border-left: 3px solid var(--color-warning);
}

.type-tool_result {
  border-left: 3px solid var(--color-success);
}

.type-error {
  border-left: 3px solid var(--color-error);
  background: var(--color-error-bg);
}

.tool-details {
  margin-top: 0.5rem;
}

.toggle-btn {
  font-size: 0.75rem;
  color: var(--color-text-secondary);
  background: none;
  border: none;
  cursor: pointer;
  text-decoration: underline;
}

.tool-args {
  margin-top: 0.5rem;
  padding: 0.5rem;
  background: var(--color-bg-tertiary);
  border-radius: 0.25rem;
  font-size: 0.8rem;
  overflow-x: auto;
}
</style>
```

### 6.4 Markdown 渲染器

```vue
<!-- src/components/Agent/MarkdownRenderer.vue -->
<template>
  <div class="markdown-body" v-html="renderedHtml"></div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { marked } from 'marked'
import hljs from 'highlight.js'

const props = defineProps<{
  content: string
}>()

// 配置 marked
marked.setOptions({
  highlight: (code, lang) => {
    if (lang && hljs.getLanguage(lang)) {
      return hljs.highlight(code, { language: lang }).value
    }
    return hljs.highlightAuto(code).value
  },
  gfm: true,
  breaks: true,
})

const renderedHtml = computed(() => {
  return marked(props.content)
})
</script>

<style scoped>
.markdown-body {
  line-height: 1.6;
  font-size: 0.9375rem;
}

.markdown-body :deep(h1),
.markdown-body :deep(h2),
.markdown-body :deep(h3) {
  margin-top: 1rem;
  margin-bottom: 0.5rem;
  font-weight: 600;
}

.markdown-body :deep(code) {
  padding: 0.125rem 0.25rem;
  background: var(--color-code-bg);
  border-radius: 0.25rem;
  font-family: 'JetBrains Mono', monospace;
  font-size: 0.875em;
}

.markdown-body :deep(pre) {
  padding: 1rem;
  background: var(--color-code-bg);
  border-radius: 0.5rem;
  overflow-x: auto;
}

.markdown-body :deep(pre code) {
  padding: 0;
  background: none;
}

.markdown-body :deep(blockquote) {
  margin: 0;
  padding: 0.5rem 1rem;
  border-left: 3px solid var(--color-border);
  color: var(--color-text-secondary);
}

.markdown-body :deep(table) {
  width: 100%;
  border-collapse: collapse;
  margin: 1rem 0;
}

.markdown-body :deep(th),
.markdown-body :deep(td) {
  padding: 0.5rem;
  border: 1px solid var(--color-border);
  text-align: left;
}

.markdown-body :deep(th) {
  background: var(--color-bg-tertiary);
  font-weight: 600;
}

.markdown-body :deep(ul),
.markdown-body :deep(ol) {
  padding-left: 1.5rem;
  margin: 0.5rem 0;
}

.markdown-body :deep(li) {
  margin: 0.25rem 0;
}

/* 任务列表样式 */
.markdown-body :deep(li.task-list-item) {
  list-style: none;
  margin-left: -1.5rem;
}

.markdown-body :deep(input[type="checkbox"]) {
  margin-right: 0.5rem;
}
</style>
```

---

## 七、提示词工程

提示词通过 **PromptManagement** 界面在数据库中动态管理，而非硬编码在代码中。

### 7.1 提示词管理架构

```
┌─────────────────────────────────────────────────────────────┐
│                 PromptManagement.vue                         │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │ 分类选择     │  │ 模板列表    │  │ 编辑器 + 预览       │  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
└────────────────────────┬────────────────────────────────────┘
                         │ Tauri IPC
                         ▼
┌─────────────────────────────────────────────────────────────┐
│                   数据库 (SQLite)                            │
│  ┌─────────────────┐  ┌─────────────────┐                   │
│  │ prompt_templates│  │ prompt_groups   │                   │
│  │ - id            │  │ - id            │                   │
│  │ - name          │  │ - architecture  │                   │
│  │ - category      │  │ - name          │                   │
│  │ - template_type │  │ - is_default    │                   │
│  │ - content       │  └─────────────────┘                   │
│  │ - variables     │  ┌─────────────────┐                   │
│  │ - is_active     │  │ prompt_group_   │                   │
│  │ - priority      │  │ items           │                   │
│  └─────────────────┘  └─────────────────┘                   │
└─────────────────────────────────────────────────────────────┘
```

### 7.2 提示词分类

| 分类 | 用途 | 示例 |
|------|------|------|
| **System** | 跨架构通用的系统提示 | 意图分类器 |
| **LlmArchitecture** | 特定架构的提示模板 | ReAct Planning/Execution |
| **Application** | 应用特定的提示模板 | 插件生成、VisionExplorer |
| **UserDefined** | 用户自定义模板 | 自定义安全任务 |

### 7.3 模板类型 (template_type)

```typescript
type TemplateType = 
  | 'SystemPrompt'           // 系统提示
  | 'IntentClassifier'       // 意图分类器
  | 'Planner'                // 规划器
  | 'Executor'               // 执行器
  | 'Replanner'              // 重规划器
  | 'Evaluator'              // 评估器
  | 'PluginGeneration'       // 插件生成(被动扫描)
  | 'AgentPluginGeneration'  // 插件生成(Agent工具)
  | 'PluginFix'              // 插件修复
  | 'VisionExplorerSystem'   // VisionExplorer系统提示
  | 'Custom'                 // 自定义
```

### 7.4 变量替换机制

提示词支持 `{variable_name}` 格式的变量占位符，运行时动态替换：

```typescript
// 模板内容
const template = "目标: {target}\n范围: {scope}\n阶段: {phase}"

// 运行时上下文
const context = {
  target: "example.com",
  scope: "Web应用",
  phase: "信息收集"
}

// 渲染结果
const rendered = "目标: example.com\n范围: Web应用\n阶段: 信息收集"
```

### 7.5 Agent 中加载提示词

```rust
// src-tauri/src/agent/prompt_loader.rs

use crate::services::prompt_db::PromptRepository;
use sentinel_core::models::prompt::{ArchitectureType, StageType, TemplateType};

pub struct PromptLoader {
    repo: Arc<PromptRepository>,
}

impl PromptLoader {
    /// 加载指定架构和阶段的激活提示词
    pub async fn load_active_prompt(
        &self,
        architecture: ArchitectureType,
        stage: StageType,
    ) -> Result<String> {
        // 从数据库获取激活的模板
        let template = self.repo
            .get_template_by_arch_stage(architecture, stage)
            .await?
            .ok_or_else(|| anyhow!("No active template found"))?;
        
        Ok(template.content)
    }

    /// 加载特定类型的应用级提示词
    pub async fn load_by_type(
        &self,
        template_type: TemplateType,
    ) -> Result<String> {
        let template = self.repo
            .get_active_by_type(template_type)
            .await?
            .ok_or_else(|| anyhow!("No active template found for type"))?;
        
        Ok(template.content)
    }

    /// 渲染提示词（替换变量）
    pub fn render(&self, template: &str, context: &HashMap<String, String>) -> String {
        let mut result = template.to_string();
        for (key, value) in context {
            result = result.replace(&format!("{{{}}}", key), value);
        }
        result
    }
}
```

### 7.6 预置提示词示例

以下是建议在 PromptManagement 中创建的核心提示词：

#### 系统提示词（System）

```markdown
# Sentinel Security Agent

你是 Sentinel AI 安全助手，一个专业的网络安全 Agent。你可以自主规划和执行安全任务。

## 身份与能力

- **身份**: 专业安全工程师 + AI Agent
- **专长**: 渗透测试、漏洞分析、代码审计、安全加固
- **特点**: 自主规划、迭代执行、持续反思

## 可用工具

{tools}

## 响应格式

你的响应必须遵循以下 JSON 格式：

### 规划响应

```json
{
  "type": "plan",
  "thinking": "分析任务需求...",
  "plan": {
    "description": "计划描述",
    "steps": [
      {
        "id": "1",
        "description": "步骤描述",
        "tool": {
          "name": "tool_name",
          "args": {}
        }
      }
    ],
    "expected_outcome": "预期结果"
  }
}
```

### 工具调用响应

```json
{
  "type": "tool_call",
  "tool": "tool_name",
  "args": {
    "param1": "value1"
  }
}
```

### 最终答案响应

```json
{
  "type": "final_answer",
  "answer": "Markdown 格式的完整答案"
}
```

## 工作流程

1. **理解任务**: 分析用户需求，明确目标
2. **制定计划**: 分解任务为可执行步骤
3. **执行步骤**: 调用工具，获取结果
4. **分析结果**: 评估工具输出
5. **迭代/完成**: 决定继续执行或返回答案

## 安全原则

- 仅在授权范围内进行测试
- 优先使用非侵入性技术
- 详细记录所有操作
- 发现漏洞时提供修复建议

## 输出风格

- 使用 Markdown 格式
- 关键信息加粗或高亮
- 使用表格展示结构化数据
- 提供清晰的步骤说明
```

#### 任务类型提示词示例

以下提示词可通过 PromptManagement 界面创建为 **UserDefined** 分类的模板：

**渗透测试模板** (变量: `target`, `scope`, `phase`)
```
目标: {target} | 范围: {scope} | 阶段: {phase}
执行: 信息收集 → 扫描探测 → 漏洞验证 → 报告生成
```

**代码审计模板** (变量: `target`, `language`, `focus_areas`)
```
目标: {target} | 语言: {language} | 重点: {focus_areas}
维度: 输入验证、认证授权、敏感数据、依赖安全
```

**CTF 挑战模板** (变量: `challenge_type`, `difficulty`, `hints`)
```
类型: {challenge_type} | 难度: {difficulty}
提示: {hints}
```

### 7.7 提示词管理最佳实践

1. **分组管理**：为不同场景创建提示词分组（如：渗透测试组、代码审计组）
2. **版本控制**：通过 `version` 字段追踪提示词迭代
3. **变量复用**：定义通用变量（如 `{target}`, `{tools}`）便于复用
4. **优先级设置**：高优先级模板优先加载
5. **激活互斥**：同类型模板只能激活一个

---

## 八、数据流

### 8.1 请求流程

```
用户输入
    │
    ▼
┌─────────────────┐
│   ChatInput     │
└────────┬────────┘
         │ emit('send', message)
         ▼
┌─────────────────┐
│   AgentView     │ 
└────────┬────────┘
         │ invoke('agent_execute')
         ▼
┌─────────────────┐
│  Tauri Command  │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Orchestrator   │──────────────┐
└────────┬────────┘              │
         │                       │
         ▼                       │
┌─────────────────┐              │
│    Planner      │◄─────────────┤
└────────┬────────┘              │
         │                       │
         ▼                       │
┌─────────────────┐              │
│    Executor     │◄─────────────┤
└────────┬────────┘              │
         │                       │
         ▼                       │
┌─────────────────┐              │
│   Reflector     │──────────────┘
└─────────────────┘
```

### 8.2 流式输出

```
Orchestrator
    │
    │ emit_event('agent-message')
    ▼
┌─────────────────┐
│  Tauri Events   │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ useAgentEvents  │ (composable)
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  MessageFlow    │
└─────────────────┘
```

---

## 九、实现计划

### Phase 1: 核心框架 (Week 1-2)

- [ ] Agent Orchestrator 基础结构
- [ ] Planner 实现
- [ ] Executor 实现  
- [ ] Reflector 实现
- [ ] MessageEmitter 流式输出

### Phase 2: 工具系统 (Week 3-4)

- [ ] 工具注册机制
- [ ] 内置安全工具实现
- [ ] MCP 工具集成
- [ ] 工具参数验证

### Phase 3: 前端重构 (Week 5-6)

- [ ] MessageFlow 组件
- [ ] MarkdownRenderer 组件
- [ ] ChatInput 组件
- [ ] 流式事件监听

### Phase 4: 提示词优化 (Week 7)

- [ ] 系统提示词模板
- [ ] 任务类型提示词
- [ ] Few-shot 示例

### Phase 5: 测试与优化 (Week 8)

- [ ] 单元测试
- [ ] 集成测试
- [ ] 性能优化
- [ ] 文档完善

---

## 十、文件清单

### 10.1 需要新建的文件

```
src-tauri/src/agent/
├── mod.rs
├── orchestrator.rs      # Agent 编排器
├── planner.rs           # 任务规划器
├── executor.rs          # 步骤执行器
├── reflector.rs         # 结果反思器
├── emitter.rs           # 消息发送器
├── todo_manager.rs      # Todos 任务管理器
├── types.rs             # 类型定义
├── config.rs            # 配置
└── prompt_loader.rs     # 从数据库加载提示词

src/components/Agent/
├── index.ts
├── AgentView.vue
├── MessageFlow.vue
├── MessageBlock.vue
├── MarkdownRenderer.vue
├── ChatInput.vue
├── ToolCallBlock.vue
├── ProgressBlock.vue
├── TodoPanel.vue        # Todos 面板组件
├── TodoItem.vue         # Todo 项组件（支持递归嵌套）
└── types.ts

src/types/
└── todo.ts              # Todos 类型定义

src/composables/
├── useAgentEvents.ts    # Agent 事件监听
└── useTodos.ts          # Todos 状态管理

src/types/
└── agent.ts             # Agent 类型定义
```

### 10.2 需要修改的文件

```
src-tauri/src/lib.rs          # 注册 agent 模块
src-tauri/src/commands/mod.rs # 添加 agent commands
src/views/ChatView.vue        # 集成新 Agent 组件
src/router/index.ts           # 路由配置
```

### 10.3 需要删除的文件

```
src/components/Agent/AgentExecutionFlow.vue  # 旧实现
src/components/Agent/AgentTaskBlock.vue      # 旧实现
src/components/Agent/AgentThinkingBlock.vue  # 旧实现
src/components/Agent/AgentToolBlock.vue      # 旧实现
src/components/Agent/AgentResultBlock.vue    # 旧实现
```

---

## 附录 A: Tauri 命令接口

```rust
// src-tauri/src/commands/agent_commands.rs

#[tauri::command]
pub async fn agent_execute(
    app: AppHandle,
    task: String,
    config: Option<AgentConfig>,
) -> Result<String, String> {
    let orchestrator = get_agent_orchestrator(&app).await?;
    let result = orchestrator.run(&task).await
        .map_err(|e| e.to_string())?;
    Ok(serde_json::to_string(&result).unwrap())
}

#[tauri::command]
pub async fn agent_cancel(
    app: AppHandle,
    execution_id: String,
) -> Result<(), String> {
    let orchestrator = get_agent_orchestrator(&app).await?;
    orchestrator.cancel(&execution_id).await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn agent_list_tools(
    app: AppHandle,
) -> Result<Vec<ToolInfo>, String> {
    let tool_manager = get_tool_manager(&app).await?;
    Ok(tool_manager.list_all())
}
```

---

## 附录 B: 事件类型定义

```typescript
// src/types/agent-events.ts

export interface AgentStartEvent {
  execution_id: string
  task: string
  timestamp: number
}

export interface AgentMessageEvent {
  execution_id: string
  message: AgentMessage
}

export interface AgentCompleteEvent {
  execution_id: string
  result: AgentResult
  duration_ms: number
}

export interface AgentErrorEvent {
  execution_id: string
  error: string
  recoverable: boolean
}
```

---

## 附录 C: 样式变量

```css
/* src/styles/agent-variables.css */

:root {
  /* 颜色 */
  --color-primary: #6366f1;
  --color-success: #22c55e;
  --color-warning: #f59e0b;
  --color-error: #ef4444;
  --color-info: #3b82f6;
  
  /* 背景 */
  --color-bg-primary: #0f0f0f;
  --color-bg-secondary: #1a1a1a;
  --color-bg-tertiary: #262626;
  --color-code-bg: #1e1e1e;
  
  /* 文字 */
  --color-text-primary: #fafafa;
  --color-text-secondary: #a1a1aa;
  
  /* 边框 */
  --color-border: #27272a;
  
  /* 状态背景 */
  --color-info-bg: rgba(59, 130, 246, 0.1);
  --color-error-bg: rgba(239, 68, 68, 0.1);
  
  /* 字体 */
  --font-mono: 'JetBrains Mono', 'Fira Code', monospace;
}
```

