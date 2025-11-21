# Travel 架构消息系统 - 快速参考

## 🎯 核心概念

Travel架构的消息系统将执行的每一步都通过事件消息发送到前端，让用户能够实时看到任务进度。

```
任务开始
  ↓
[Observe] 侦察阶段 → 📚 Memory, 🔍 观察, 🛡️ 护栏
  ↓
[Orient] 分析阶段 → 🧠 知识, 🔍 威胁, ⚠️ 漏洞, 🛡️ 护栏
  ↓
[Decide] 决策阶段 → 📋 计划, 📝 生成, 🛡️ 护栏
  ↓
[Act] 执行阶段 → ⚙️ 分发
  ↓
执行引擎 (Simple/Medium/Complex)
  ↓
[Simple] 直接工具 OR [Medium] 顺序执行 OR [Complex] ReAct推理
  ↓
完成 ✅
```

## 📊 消息类型速查表

| 类型 | 含义 | 场景 |
|------|------|------|
| **Thinking** | 思考/分析过程 | OODA阶段开始/进行，ReAct推理 |
| **Content** | 主要执行内容 | 步骤执行，进度更新 |
| **ToolResult** | 工具执行结果 | 工具完成时 |
| **PlanInfo** | 计划相关信息 | Decide阶段，计划生成 |
| **Error** | 错误信息 | 执行失败，异常情况 |
| **Meta** | 元数据 | 工具调用参数，执行统计 |

## 🚀 前端消息接收模板

```typescript
// 监听消息事件
import { listen } from '@tauri-apps/api/event'

listen('message_chunk', (event) => {
  const {
    execution_id,        // 执行ID
    message_id,          // 消息ID
    conversation_id,     // 会话ID
    sequence,            // 消息序号（严格递增）
    chunk_type,          // 消息类型
    content,             // 消息内容
    stage,               // 执行阶段
    structured_data      // 结构化数据
  } = event.payload

  // 按sequence排序处理消息
  console.log(`[${stage}] (${sequence}) ${content}`)

  // 根据类型处理
  switch(chunk_type) {
    case 'Thinking':
      updateThinkingPanel(content)
      break
    case 'ToolResult':
      updateProgressBar(structured_data)
      break
    case 'Error':
      showErrorNotification(content)
      break
  }
})
```

## 🔧 后端调用示例

```rust
// 1. 创建任务参数（可选提供ID）
let params = HashMap::from([
    ("target".to_string(), json!("example.com")),
    ("execution_id".to_string(), json!("exec-123")), // 可选
    ("message_id".to_string(), json!("msg-456")),    // 可选
    ("conversation_id".to_string(), json!("conv-789")), // 可选
]);

// 2. 创建Task
let task = AgentTask {
    description: "Test example.com".to_string(),
    parameters: params,
    // ... 其他字段
};

// 3. 执行（消息自动发送）
let result = engine.execute(&task, session).await?;
```

## 📈 执行阶段消息清单

### Observe 阶段
```
🔄 Starting Observe phase...
  📚 Found N similar experiences from memory
  🛡️ Guardrail checks: N items checked
  🔍 Target observations collected
✅ Observe phase completed
```

### Orient 阶段
```
🔄 Starting Orient phase...
  🧠 Found N knowledge entities
  🔍 Querying threat intelligence...
  ⚠️ Identified N vulnerabilities
  🛡️ Guardrail checks: N items checked
✅ Orient phase completed
```

### Decide 阶段
```
🔄 Starting Decide phase...
  📋 Found N plan templates
  📝 Generated action plan with N steps
  🛡️ Guardrail checks: N items checked
✅ Decide phase completed
```

### Act 阶段
```
🔄 Starting Act phase...
  ⚙️ Starting execution of action plan: {name}
  🛡️ Final guardrail checks: N items checked
  📊 Dispatching with complexity: {type}
    [Simple/Medium/Complex 任务特定消息]
  ✅ Execution completed
✅ Act phase completed
```

## 💡 常见使用场景

### 场景1: 简单任务进度显示
```
User sees: 
[Observe] 🔍 观察中...
[Orient] 🧠 分析中...
[Decide] 📝 规划中...
[Act] ⚙️ 执行步骤 1/3
[Act] ⚙️ 执行步骤 2/3
[Act] ⚙️ 执行步骤 3/3
[Act] ✅ 完成
```

### 场景2: 复杂任务ReAct推理显示
```
User sees:
[Act-ReAct] 🤖 ReAct executor started
[Act-ReAct] 🔄 Iteration 1/10 starting
[Act-ReAct] 💭 Thinking: "I should check..."
[Act-ReAct] 🔧 Executing tool: analyze_target
[Act-ReAct] 🔄 Iteration 2/10 starting
...
[Act-ReAct] ✅ ReAct completed in 3 iterations
```

### 场景3: 错误恢复
```
User sees:
[Orient] 🔍 Querying threat intelligence...
[Orient] ❌ Threat query failed: timeout
[Orient] ⚠️ Guardrail check failed
[Orient] ✅ Orient phase completed (with fallback)
[Decide] 📝 Generated simplified action plan
```

## 🔍 调试技巧

### 1. 检查execution_id是否正确传递
```rust
// 在engine_adapter.rs中添加日志
log::info!("Execution ID: {}, Message ID: {}", execution_id, message_id);
```

### 2. 验证消息发送
```rust
// 在emit_message方法中检查
log::debug!("Emitting message: {} (seq: {})", content, sequence);
```

### 3. 追踪消息顺序
```javascript
// 前端验证序号
let lastSeq = 0
listen('message_chunk', (event) => {
  if (event.payload.sequence <= lastSeq) {
    console.warn('Sequence error!', event.payload)
  }
  lastSeq = event.payload.sequence
})
```

## 📋 集成检查清单

- [ ] 后端: 导入了正确的消息模块
- [ ] 后端: TravelEngine 设置了 app_handle
- [ ] 后端: 任务参数包含或自动生成了 ID
- [ ] 前端: 监听了 'message_chunk' 事件
- [ ] 前端: 按 sequence 排序消息
- [ ] 前端: 根据 chunk_type 显示不同的 UI
- [ ] 前端: 正确处理了错误消息
- [ ] 测试: 完整执行流程并验证消息

## 🎓 消息字段详解

```rust
pub struct OrderedMessageChunk {
    pub execution_id: String,                    // 唯一执行ID
    pub message_id: String,                      // 消息ID（同一execution的消息共用）
    pub conversation_id: Option<String>,         // 可选：会话ID
    pub sequence: u64,                           // 严格递增序号（前端排序用）
    pub chunk_type: ChunkType,                   // 消息类型（Thinking/Content/Error等）
    pub content: String,                         // 消息文本
    pub timestamp: SystemTime,                   // 发送时间戳
    pub is_final: bool,                          // 是否为最后一个块
    pub stage: Option<String>,                   // 执行阶段（Observe/Orient/Decide/Act）
    pub tool_name: Option<String>,               // 工具名称（如果是工具相关）
    pub architecture: Option<ArchitectureType>,  // 架构标识（Travel）
    pub structured_data: Option<serde_json::Value>, // 结构化元数据
}
```

## 🔗 相关文件位置

| 功能 | 文件 |
|------|------|
| OODA消息 | `src-tauri/src/engines/travel/ooda_executor.rs` |
| 引擎调度消息 | `src-tauri/src/engines/travel/engine_dispatcher.rs` |
| ReAct消息 | `src-tauri/src/engines/travel/react_executor.rs` |
| 消息发送函数 | `src-tauri/src/utils/ordered_message.rs` |
| 主引擎集成 | `src-tauri/src/engines/travel/engine_adapter.rs` |
| 完整文档 | `docs/travel_messaging_implementation.md` |

## 📞 常见问题

**Q: 前端没有收到消息？**  
A: 检查 app_handle 是否正确传递，execution_id 是否生成/传递

**Q: 消息顺序错乱？**  
A: 前端按 sequence 字段排序，不要按接收顺序处理

**Q: 同时执行多个任务？**  
A: 每个任务有独立的 execution_id 和 message_id，前端分别处理即可

**Q: 如何添加自定义消息？**  
A: 调用 `emit_message()` 或 `emit_message_chunk_arc()` 函数

---

**最后更新**: 2025-11-21  
**版本**: 1.0 正式版
