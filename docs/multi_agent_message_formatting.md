# 多Agent架构消息格式化完善总结

## 概述

本次任务完善了ReWOO、LLMCompiler和Plan-and-Execute三大架构的消息格式化，使其与React架构保持一致，实现统一的前端消息推送机制。

## 完成的工作

### 1. ReWOO架构消息格式化

**文件**: `src-tauri/src/engines/rewoo/engine_adapter.rs`

#### 新增功能
- **消息ID提取**: 从任务参数中提取`conversation_id`、`message_id`和`execution_id`
- **Planning阶段消息**:
  - 发送规划开始消息（Thinking类型）
  - 发送计划信息（PlanInfo类型）
  - 发送规划错误消息（Error类型）
  
- **Tool Execution阶段消息**:
  - 发送执行阶段开始消息
  - 发送每个工具执行开始消息
  - 发送工具执行结果（ToolResult类型）
  - 发送工具执行错误消息（Error类型）
  
- **Solving阶段消息**:
  - 发送求解开始消息
  - 发送执行完成元数据（Meta类型）

#### 新增方法
```rust
pub fn set_app_handle(&mut self, app_handle: tauri::AppHandle)
```

### 2. LLMCompiler架构消息格式化

**文件**: `src-tauri/src/engines/llm_compiler/engine_adapter.rs`

#### 优化功能
- **Planning阶段消息**:
  - 发送规划开始消息（Thinking类型）
  - 发送DAG计划信息（PlanInfo类型）
  - 发送规划错误消息（Error类型）
  
- **Joiner决策阶段消息**:
  - 发送决策分析开始消息（Thinking类型）
  - 发送决策失败错误消息（Error类型）
  - 发送决策结果（Meta类型）
    - Complete决策: "✓ 决策: 完成执行"
    - Continue决策: "→ 决策: 继续执行"

#### 说明
- Planner内部已经通过`ChunkType::Thinking`发送LLM思考过程
- Executor的工具执行结果已经通过`emit_message_chunk`发送
- Joiner内部的AI分析也已经通过AI服务流式发送

### 3. Plan-and-Execute架构消息格式化

**文件**: `src-tauri/src/engines/plan_and_execute/executor.rs`

#### 新增功能
- **执行上下文改进**:
  - `ExecutionContext`添加`Clone` trait
  - `initialize_context`返回`ExecutionContext`实例
  - 提取消息ID的辅助方法已存在

- **执行开始消息**:
  - 发送执行计划开始消息（Thinking类型）
  - 包含计划名称和执行ID信息

#### 说明
- Executor内部的步骤执行已经通过`emit_message_chunk_arc`发送
- Planner的规划过程已经通过AI服务流式发送
- Replanner的重规划过程也已经通过AI服务流式发送

## 统一的消息类型

所有架构现在使用统一的`ChunkType`枚举：

```rust
pub enum ChunkType {
    Content,      // 主要内容
    Thinking,     // AI思考过程
    ToolResult,   // 工具执行结果
    PlanInfo,     // 计划信息
    Error,        // 错误信息
    Meta,         // 元数据信息
}
```

## 消息推送函数

使用`ordered_message`模块的便捷函数：

```rust
// 发送思考块
emit_thinking_chunk(app, execution_id, message_id, conversation_id, content, stage)

// 发送计划信息块
emit_plan_info_chunk(app, execution_id, message_id, conversation_id, content, stage, tool_name)

// 发送工具结果块
emit_tool_result_chunk(app, execution_id, message_id, conversation_id, content, stage, tool_name)

// 发送错误块
emit_error_chunk(app, execution_id, message_id, conversation_id, content, stage, tool_name)

// 发送元数据块
emit_meta_chunk(app, execution_id, message_id, conversation_id, content, tool_name)
```

## 消息序列保证

- 所有消息使用`message_id`作为序号计数的键
- 确保同一条前端消息的所有来源共享严格递增的序列
- 消除跨`execution_id`的消息交错问题

## 架构特点对比

| 架构 | Planning | Execution | Decision/Solving | 特点 |
|------|----------|-----------|------------------|------|
| ReWOO | ✓ | ✓ (每个工具) | ✓ (Solver) | 三阶段清晰分离 |
| LLMCompiler | ✓ (DAG) | ✓ (并行) | ✓ (Joiner) | 并行执行+智能决策 |
| Plan-and-Execute | ✓ | ✓ (顺序) | ✓ (Replanner) | 动态重规划 |
| React | ✓ (每轮) | ✓ (每轮) | ✓ (每轮) | 迭代思考-行动 |

## 测试结果

- ✅ 编译通过（`cargo check`）
- ✅ 无linter错误
- ✅ 所有架构消息格式统一
- ✅ 前端可以接收到各阶段的结构化消息

## 前端集成建议

前端可以根据`ChunkType`和`stage`字段区分不同类型的消息：

```typescript
switch (chunk.chunk_type) {
  case 'Thinking':
    // 显示AI思考过程（灰色文本）
    break;
  case 'PlanInfo':
    // 显示计划信息（蓝色卡片）
    break;
  case 'ToolResult':
    // 显示工具执行结果（绿色卡片）
    break;
  case 'Error':
    // 显示错误信息（红色警告）
    break;
  case 'Meta':
    // 显示元数据（灰色小字）
    break;
  case 'Content':
    // 显示主要内容（正常文本）
    break;
}
```

根据`stage`字段可以进一步区分：
- `rewoo_planning`, `rewoo_execution`, `rewoo_solving`
- `llm_compiler_planning`, `llm_compiler_joiner`
- `plan_execute_start`
- `react` (React架构)

## 总结

本次完善实现了：
1. ✅ 统一的消息格式化机制
2. ✅ 清晰的阶段标识
3. ✅ 完整的错误处理
4. ✅ 结构化的消息类型
5. ✅ 与React架构保持一致的用户体验

所有四大架构（React、ReWOO、LLMCompiler、Plan-and-Execute）现在都使用相同的消息推送机制，为前端提供统一、结构化的实时反馈。

