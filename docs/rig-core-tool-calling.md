# Rig-Core 原生工具调用集成

本文档描述了 Sentinel AI 如何使用 rig-core 的原生工具调用机制。

## 架构概览

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                              Frontend (Vue)                                       │
│   ┌─────────────────────────────────────────────────────────────────────────┐   │
│   │  监听事件:                                                               │   │
│   │  - agent:tool_call_start    → 工具调用开始                              │   │
│   │  - agent:tool_call_delta    → 工具参数增量                              │   │
│   │  - agent:tool_call_complete → 工具调用完成                              │   │
│   │  - agent:tool_result        → 工具执行结果                              │   │
│   └─────────────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────────────┘
                                        │
                                        ▼ Tauri Events
┌─────────────────────────────────────────────────────────────────────────────────┐
│                          Backend (Rust)                                          │
│                                                                                  │
│   ┌──────────────────────────────────────────────────────────────────────────┐  │
│   │  sentinel-llm/streaming.rs                                                │  │
│   │                                                                           │  │
│   │  StreamContent 枚举:                                                      │  │
│   │  ├── Text(String)           - 文本内容                                   │  │
│   │  ├── Reasoning(String)      - 推理过程                                   │  │
│   │  ├── ToolCallStart          - 工具调用开始 { id, name }                  │  │
│   │  ├── ToolCallDelta          - 工具参数增量 { id, delta }                 │  │
│   │  ├── ToolCallComplete       - 工具调用完成 { id, name, arguments }       │  │
│   │  ├── ToolResult             - 工具执行结果 { id, result }                │  │
│   │  └── Done                   - 流完成                                     │  │
│   └──────────────────────────────────────────────────────────────────────────┘  │
│                                        │                                         │
│                                        ▼                                         │
│   ┌──────────────────────────────────────────────────────────────────────────┐  │
│   │  rig-core Agent                                                           │  │
│   │                                                                           │  │
│   │  agent.tool(PortScanTool)       ← 所有工具实现 rig::tool::Tool trait     │  │
│   │        .tool(HttpRequestTool)                                             │  │
│   │        .tool(LocalTimeTool)                                               │  │
│   │        .tool(ShellTool)                                                   │  │
│   │        .build()                                                           │  │
│   │                                                                           │  │
│   │  multi_turn(100) → 自动处理工具调用循环（最多 100 轮）                    │  │
│   └──────────────────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────────────┘
```

## 核心组件

### 1. StreamContent 枚举 (`sentinel-llm/src/streaming.rs`)

```rust
pub enum StreamContent {
    /// 文本内容
    Text(String),
    /// 推理内容（思考过程）
    Reasoning(String),
    /// 工具调用开始
    ToolCallStart { id: String, name: String },
    /// 工具调用参数增量
    ToolCallDelta { id: String, delta: String },
    /// 工具调用完成
    ToolCallComplete { id: String, name: String, arguments: String },
    /// 工具执行结果
    ToolResult { id: String, result: String },
    /// 流完成
    Done,
}
```

### 2. execute_stream 函数

核心流处理函数，处理 rig-core 的 `MultiTurnStreamItem` 事件：

- `StreamedAssistantContent::Text` → `StreamContent::Text`
- `StreamedAssistantContent::Reasoning` → `StreamContent::Reasoning`
- `StreamedAssistantContent::ToolCall` → `StreamContent::ToolCallComplete`
- `StreamedAssistantContent::ToolCallDelta` → `StreamContent::ToolCallDelta`
- `StreamedUserContent::ToolResult` → `StreamContent::ToolResult`
- `FinalResponse` → `StreamContent::Done`

### 3. 工具实现 (`sentinel-tools/src/buildin_tools/`)

所有内置工具实现 `rig::tool::Tool` trait：

```rust
impl Tool for LocalTimeTool {
    const NAME: &'static str = "local_time";
    type Args = LocalTimeArgs;      // 使用 schemars::JsonSchema 派生
    type Output = LocalTimeOutput;
    type Error = LocalTimeError;

    async fn definition(&self, _prompt: String) -> ToolDefinition { ... }
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> { ... }
}
```

## 工具调用流程

1. **用户发送请求** → `agent_execute` 命令
2. **加载工具配置** → `ToolRouter` 选择相关工具
3. **创建 rig Agent** → 通过 `.tool()` 方法注册工具
4. **流式调用** → `agent.stream_prompt().multi_turn(100)`
5. **rig-core 自动处理**:
   - 将工具定义发送给 LLM
   - 解析 LLM 的工具调用响应
   - 自动执行工具
   - 将结果反馈给 LLM
   - 循环直到 LLM 给出最终回答
6. **事件流转发** → 所有事件通过 Tauri 发送到前端

## 与之前实现的区别

### 之前：手动 JSON 解析

```rust
// 旧方式：手动解析 LLM 输出中的 JSON 工具调用
fn extract_tool_call(response: &str) -> Option<ToolCall> {
    let json_pattern = regex::Regex::new(r"```json\s*(\{[^`]+\})\s*```").ok()?;
    // ...
}
```

### 现在：rig-core 原生支持

```rust
// 新方式：rig-core 自动处理工具调用
match item {
    Ok(MultiTurnStreamItem::StreamAssistantItem(
        StreamedAssistantContent::ToolCall(tool_call),
    )) => {
        // rig 已经自动执行了工具
        on_content(StreamContent::ToolCallComplete {
            id: tool_call.id.clone(),
            name: tool_call.function.name.clone(),
            arguments: tool_call.function.arguments.to_string(),
        });
    }
    Ok(MultiTurnStreamItem::StreamUserItem(
        StreamedUserContent::ToolResult(tool_result),
    )) => {
        // rig 返回工具执行结果
        on_content(StreamContent::ToolResult {
            id: tool_result.id,
            result: serde_json::to_string(&tool_result.content).unwrap_or_default(),
        });
    }
    // ...
}
```

## 优势

1. **类型安全**: 工具参数通过 schemars 自动生成 JSON Schema
2. **自动多轮对话**: `multi_turn(100)` 自动处理工具调用循环
3. **Provider 兼容性**: rig-core 自动处理不同 LLM Provider 的工具调用格式差异
4. **流式支持**: 完整支持工具调用过程的流式输出
5. **动态工具支持**: MCP、Plugin、Workflow 工具都可以使用 rig-core 原生调用

## 动态工具支持

### DynamicTool

所有动态工具（MCP、Plugin、Workflow）通过 `DynamicTool` 结构体适配到 rig-core 的 `Tool` trait：

```rust
// DynamicTool 实现了 rig::tool::Tool trait
impl Tool for DynamicTool {
    const NAME: &'static str = "dynamic_tool";
    type Args = Value;
    type Output = Value;
    type Error = DynamicToolError;

    // 覆盖 name() 返回实际工具名
    fn name(&self) -> String {
        self.def.name.clone()
    }

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: self.def.name.clone(),
            description: self.def.description.clone(),
            parameters: self.def.input_schema.clone(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let executor = self.def.executor.clone();
        executor(args).await.map_err(|e| DynamicToolError::ExecutionFailed(e))
    }
}
```

### 获取动态工具实例

```rust
// 从 ToolServer 获取 DynamicTool 实例
let dynamic_tools = tool_server.get_dynamic_tools(&selected_tool_ids).await;
```

### 调用带动态工具的流式方法

```rust
let result = client
    .stream_chat_with_dynamic_tools(
        Some(&system_prompt),
        &user_task,
        &[], // 历史记录
        None, // 图片
        dynamic_tools,
        true, // 是否包含内置工具
        |content| {
            // 处理流式内容
        },
    )
    .await;
```

## 前端集成

前端需要监听以下 Tauri 事件：

```typescript
// 工具调用开始
listen('agent:tool_call_start', (event) => {
  const { execution_id, tool_call_id, tool_name } = event.payload;
  // 显示工具调用开始状态
});

// 工具调用完成
listen('agent:tool_call_complete', (event) => {
  const { execution_id, tool_call_id, tool_name, arguments } = event.payload;
  // 显示工具调用详情
});

// 工具执行结果
listen('agent:tool_result', (event) => {
  const { execution_id, tool_call_id, result } = event.payload;
  // 显示工具执行结果
});
```

## 调试

启用 rig-core 详细日志：

```bash
RUST_LOG=rig=debug cargo run
```

这将显示发送给 LLM 的完整请求（包括工具定义）和响应内容。
