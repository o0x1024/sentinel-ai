# Travel 架构消息系统完善指南

## 概述

Travel架构已完善了从后端到前端的消息流。执行任务时，系统会在OODA四个阶段以及引擎调度阶段发送详细的消息到前端，让用户能够实时看到任务执行进度。

## 实现细节

### 1. OODA 各阶段的消息发送 (ooda_executor.rs)

#### Observe 阶段（侦察）
- **开始消息**: "🔄 Starting Observe phase..."
- **进度消息**:
  - "📚 Found {N} similar experiences from memory" - Memory查询结果
  - "🛡️ Guardrail checks: {N} items checked" - 护栏检查结果
  - "🔍 Target observations collected" - 目标信息收集
- **完成消息**: "✅ Observe phase completed"

#### Orient 阶段（分析定位）
- **开始消息**: "🔄 Starting Orient phase..."
- **进度消息**:
  - "🧠 Found {N} knowledge entities" - 知识图谱查询
  - "🔍 Querying threat intelligence..." - 威胁情报查询
  - "⚠️ Identified {N} vulnerabilities" - 漏洞识别
  - "🛡️ Guardrail checks: {N} items checked" - 护栏检查
- **完成消息**: "✅ Orient phase completed"

#### Decide 阶段（决策）
- **开始消息**: "🔄 Starting Decide phase..."
- **进度消息**:
  - "📋 Found {N} plan templates" - 计划模板获取
  - "📝 Generated action plan with {N} steps" - 行动计划生成
  - "🛡️ Guardrail checks: {N} items checked" - 护栏检查
- **完成消息**: "✅ Decide phase completed"

#### Act 阶段（执行）
- **开始消息**: "🔄 Starting Act phase..."
- **进度消息**:
  - "⚙️ Starting execution of action plan: {plan_name}" - 计划执行开始
  - "🛡️ Final guardrail checks: {N} items checked" - 最终护栏检查
  - "🚀 Dispatching execution to appropriate engine..." - 引擎分发
  - "✅ Execution completed" - 执行完成
- **完成消息**: "✅ Act phase completed"

### 2. 引擎调度器的消息发送 (engine_dispatcher.rs)

#### 简单任务 (Simple Task)
```
📊 Dispatching with complexity: Simple
🔧 Simple task: Direct tool execution
📍 Executing step {N}/{M}: {step_name}
✅ Step {N} completed: {step_name}
📊 Simple task completed with {N} steps
```

#### 中等任务 (Medium Task)
```
📊 Dispatching with complexity: Medium
🔧 Medium task: Sequential tool execution
📍 Executing step {N}/{M}: {step_name}
✅ Step {N} completed: {step_name}
❌ Step {N} failed: {error}
📊 Medium task completed: {successful}/{total} steps successful
```

#### 复杂任务 (Complex Task)
```
📊 Dispatching with complexity: Complex
🤖 Complex task: Using ReAct executor for intelligent reasoning
🧠 Initializing ReAct executor...
🚀 Starting ReAct reasoning loop...
✅ ReAct execution completed successfully
```

### 3. ReAct 执行器的消息发送 (react_executor.rs)

#### 循环开始
```
🤖 ReAct executor started
🔄 Iteration {N}/{max} starting
💭 Thinking phase...
```

#### 思考阶段 (Thought)
```
💭 {thought_content}
```

#### 决策与最终答案
```
{final_answer}
✅ ReAct completed in {N} iterations
```

#### 工具调用 (Action)
```
🔧 Executing tool: {tool_name}
🔧 Tool {tool_name} completed (duration: {ms}ms)
```

## 消息结构

所有消息都通过统一的 `OrderedMessageChunk` 结构发送，包含以下信息：

```rust
pub struct OrderedMessageChunk {
    pub execution_id: String,           // 执行唯一ID
    pub message_id: String,             // 消息ID
    pub conversation_id: Option<String>, // 会话ID
    pub sequence: u64,                  // 严格递增序号
    pub chunk_type: ChunkType,          // 消息类型
    pub content: String,                // 消息内容
    pub timestamp: SystemTime,          // 时间戳
    pub is_final: bool,                 // 是否最后一个块
    pub stage: Option<String>,          // 阶段标识
    pub tool_name: Option<String>,      // 工具名称
    pub architecture: Option<ArchitectureType>, // 架构类型 (Travel)
    pub structured_data: Option<serde_json::Value>, // 结构化数据
}
```

## 消息类型 (ChunkType)

- **Thinking**: 思考过程消息
- **Content**: 主要内容消息
- **ToolResult**: 工具执行结果
- **PlanInfo**: 计划信息
- **Error**: 错误消息
- **Meta**: 元数据信息
- **StreamComplete**: 流完成信号

## 消息流向

```
TravelEngine.execute()
    ↓
OodaExecutor.execute_cycle()
    ├─ execute_observe_phase() → emit_message()
    ├─ execute_orient_phase() → emit_message()
    ├─ execute_decide_phase() → emit_message()
    └─ execute_act_phase() → emit_message()
        ↓
    EngineDispatcher.dispatch()
        ├─ dispatch_simple_task() → emit_message()
        ├─ dispatch_medium_task() → emit_message()
        └─ dispatch_complex_task() → emit_message()
            ↓
        TravelReactExecutor.execute()
            ├─ iteration start → emit_message()
            ├─ thought phase → emit_message()
            ├─ action phase → emit_message()
            └─ final answer → emit_message()
                ↓
            前端消息接收处理
```

## 前端接收配置

前端需要监听 `message_chunk` 事件来接收所有消息：

```typescript
// Vue/React 组件中
import { listen } from '@tauri-apps/api/event'

listen('message_chunk', (event) => {
  const chunk = event.payload
  console.log(`[${chunk.stage}] ${chunk.content}`)
  
  // 根据chunk_type处理不同类型的消息
  switch(chunk.chunk_type) {
    case 'Thinking':
      // 显示思考过程
      break
    case 'ToolResult':
      // 显示工具执行结果
      break
    case 'Error':
      // 显示错误信息
      break
    // ... 其他消息类型
  }
})
```

## 使用示例

### 任务参数

在调用Travel引擎时，需要传递以下参数：

```rust
let task_params = HashMap::from([
    ("target".to_string(), json!("example.com")),
    ("authorized".to_string(), json!(true)),
    ("execution_id".to_string(), json!("exec-123")), // 可选，自动生成
    ("message_id".to_string(), json!("msg-456")),    // 可选，自动生成
    ("conversation_id".to_string(), json!("conv-789")), // 可选
]);
```

### 消息ID自动生成

如果任务参数中没有提供 `execution_id` 或 `message_id`，系统会自动生成 UUID。

## 故障排除

### 前端没有收到消息

1. **检查execution_id是否传递**
   - 确保在AgentTask中包含execution_id或让系统自动生成

2. **检查app_handle是否正确传递**
   - TravelEngine需要通过`with_app_handle()`设置AppHandle

3. **检查消息事件监听**
   - 确保前端正确监听了 `message_chunk` 事件

### 消息顺序不正确

1. **消息序号机制**
   - 系统使用严格递增的sequence号保证消息顺序
   - 前端应按sequence号排序处理消息

2. **多个execution的交错**
   - 不同execution的消息应分别追踪

## 性能考虑

1. **消息频率**: 每个阶段开始、中间、完成各发送一次消息，不会过度频繁
2. **消息大小**: 结构化数据用于传递元信息，避免过大的payload
3. **异步发送**: 消息发送不会阻塞执行流程

## 未来改进

- [ ] 添加消息优先级机制
- [ ] 支持消息分类过滤
- [ ] 添加消息聚合选项（减少消息数量）
- [ ] 支持自定义消息格式
- [ ] 添加消息持久化选项

---

**最后更新**: 2025-11-21  
**维护者**: AI Assistant  
**状态**: 实现完成 ✅
