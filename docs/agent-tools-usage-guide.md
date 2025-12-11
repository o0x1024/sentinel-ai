# Agent 工具集成使用指南

## 功能概述

现在 Agent 已经支持智能工具调用功能，可以根据任务自动选择和执行工具，包括：

- ✅ **内置工具**：port_scan, http_request, local_time, shell
- ✅ **工作流工具**：从数据库加载的工作流定义
- ✅ **MCP 工具**：通过 MCP 协议连接的外部工具
- ✅ **插件工具**：Agent 插件系统中的工具

## 核心特性

### 1. 智能工具选择

**问题**：如果把所有工具都传给 LLM，会导致：
- Token 消耗巨大（可能 10k+ tokens）
- 上下文污染，降低准确性
- 成本高昂

**解决方案**：采用**关键词匹配策略**，根据任务自动选择 3-5 个相关工具。

### 2. 工具选择策略

```rust
pub enum ToolSelectionStrategy {
    /// 全部工具（不推荐，仅用于测试）
    All,
    /// 关键词匹配（快速，免费，默认）
    Keyword,
    /// 用户手动指定
    Manual(Vec<String>),
    /// 不使用工具
    None,
}
```

### 3. 工具配置

```rust
pub struct ToolConfig {
    /// 是否启用工具调用
    pub enabled: bool,
    /// 工具选择策略
    pub selection_strategy: ToolSelectionStrategy,
    /// 最大工具数量
    pub max_tools: usize,
    /// 固定启用的工具
    pub fixed_tools: Vec<String>,
    /// 禁用的工具
    pub disabled_tools: Vec<String>,
}
```

## 使用方法

### 前端界面

在 Agent 对话界面的输入框工具栏中：

```
[📎附件] [🔧工具] [🌐搜索] [🧠知识库] [@引用] [⚡指令] [选择]
```

- **🔧 工具按钮**：点击启用/禁用工具调用
- **🌐 搜索按钮**：点击启用/禁用联网搜索
- **🧠 知识库按钮**：点击启用/禁用 RAG 知识检索

### 前端代码示例

```typescript
// AgentView.vue
const toolsEnabled = ref(false)
const webSearchEnabled = ref(false)
const ragEnabled = ref(false)

// 调用 agent_execute 时传递配置
await invoke('agent_execute', {
  task: userInput,
  config: {
    enable_rag: ragEnabled.value,
    enable_web_search: webSearchEnabled.value,
    tool_config: toolsEnabled.value ? {
      enabled: true,
      selection_strategy: 'Keyword',  // 关键词匹配
      max_tools: 5,                    // 最多选择 5 个工具
      fixed_tools: ['local_time'],     // 始终启用时间工具
      disabled_tools: [],              // 禁用的工具列表
    } : {
      enabled: false,
    },
  }
})
```

### 后端 Rust 代码

工具路由器会自动：
1. 加载所有可用工具（内置、工作流、MCP、插件）
2. 根据任务关键词匹配相关工具
3. 将工具描述添加到 system prompt
4. 执行 Agent 循环，处理工具调用

```rust
// 工具路由器自动加载动态工具
let tool_router = ToolRouter::new_with_dynamic_tools(Some(db_service)).await;

// 根据任务选择工具
let selected_tools = tool_router.select_tools(&task, &tool_config)?;

// Agent 执行循环会自动处理工具调用
execute_agent_loop(app_handle, params, system_prompt, selected_tools, tool_router).await
```

## 工具调用流程

### 1. 工具选择阶段

```
用户任务: "Scan ports on 192.168.1.1"
    ↓
关键词匹配: ["scan", "ports", "192.168.1.1"]
    ↓
匹配到工具: ["port_scan", "local_time"]
    ↓
发送到前端: agent:tools_selected 事件
```

### 2. LLM 工具调用

Agent 会在 system prompt 中看到工具描述：

```markdown
## Available Tools

### port_scan
Scan TCP ports on target IP address to discover open ports and services

Parameters:
```json
{
  "type": "object",
  "properties": {
    "target": {"type": "string", "description": "Target IP address"},
    "ports": {"type": "string", "default": "common"}
  },
  "required": ["target"]
}
```

When you need to use a tool, respond with:
```json
{
  "tool": "port_scan",
  "arguments": {
    "target": "192.168.1.1",
    "ports": "1-1000"
  }
}
```
```

### 3. 工具执行

LLM 响应包含工具调用：

```json
{
  "tool": "port_scan",
  "arguments": {
    "target": "192.168.1.1",
    "ports": "common"
  }
}
```

Agent 自动：
1. 解析工具调用
2. 执行工具（调用 `unified_execute_tool`）
3. 将结果返回给 LLM
4. 继续下一轮对话

### 4. 前端事件

前端会收到以下事件：

```typescript
// 工具选择完成
listen('agent:tools_selected', (event) => {
  console.log('Selected tools:', event.payload.tools)
})

// 工具执行完成
listen('agent:tool_executed', (event) => {
  console.log('Tool:', event.payload.tool)
  console.log('Result:', event.payload.result)
  console.log('Success:', event.payload.success)
})

// Agent 响应流式输出
listen('agent:chunk', (event) => {
  if (event.payload.chunk_type === 'text') {
    // 显示文本内容
  } else if (event.payload.chunk_type === 'reasoning') {
    // 显示推理过程（如 Claude 的思考）
  }
})
```

## 工具类型

### 内置工具

| 工具 ID | 名称 | 描述 | 分类 |
|---------|------|------|------|
| `port_scan` | 端口扫描 | 扫描目标 IP 的开放端口 | Network |
| `http_request` | HTTP 请求 | 发送 HTTP/HTTPS 请求 | Network |
| `local_time` | 本地时间 | 获取当前时间 | System |
| `shell` | Shell 命令 | 执行 Shell 命令 | System |

### 工作流工具

格式：`workflow::{workflow_id}`

从数据库自动加载，包含：
- 工作流名称
- 工作流描述
- 自动提取的标签

### MCP 工具

格式：`mcp::{server_name}::{tool_name}`

从已连接的 MCP 服务器加载。

### 插件工具

格式：`plugin::{plugin_id}`

从 Agent 插件系统加载。

## 关键词匹配规则

工具路由器使用以下规则匹配工具：

1. **工具名称匹配**：+20 分
2. **标签匹配**：+10 分/标签
3. **描述关键词**：+3 分/词
4. **始终可用**：+5 分
5. **特殊规则**：
   - "scan" + Network 类别：+15 分
   - "http"/"api" + http_request：+15 分
   - "time"/"date" + local_time：+15 分
   - "workflow"/"工作流" + Workflow 类别：+20 分

示例：

```
任务: "Scan ports on 192.168.1.1 and check HTTP service"
匹配结果:
  - port_scan: 35 分 (名称+标签+特殊规则)
  - http_request: 25 分 (标签+特殊规则)
  - local_time: 5 分 (始终可用)
选择: [port_scan, http_request, local_time]
```

## Token 成本对比

假设有 50 个工具，每个工具定义平均 200 tokens：

| 方案 | 工具数 | Token 成本 | 节省 |
|------|--------|-----------|------|
| 全部工具 | 50 | ~10,000 tokens | 0% |
| 关键词匹配 | 5 | ~1,000 tokens | **90%** |
| 手动选择 | 3 | ~600 tokens | **94%** |

## 配置建议

### 默认配置（推荐）

适合大多数场景：

```rust
ToolConfig {
    enabled: true,
    selection_strategy: ToolSelectionStrategy::Keyword,
    max_tools: 5,
    fixed_tools: vec!["local_time".to_string()],
    disabled_tools: vec![],
}
```

### 安全测试场景

只使用网络工具：

```rust
ToolConfig {
    enabled: true,
    selection_strategy: ToolSelectionStrategy::Manual(vec![
        "port_scan".to_string(),
        "http_request".to_string(),
    ]),
    max_tools: 3,
    fixed_tools: vec![],
    disabled_tools: vec!["shell".to_string()],  // 禁用 shell
}
```

### 数据分析场景

启用知识检索和搜索：

```rust
ToolConfig {
    enabled: true,
    selection_strategy: ToolSelectionStrategy::Keyword,
    max_tools: 8,
    fixed_tools: vec![
        "local_time".to_string(),
    ],
    disabled_tools: vec![],
}
```

## 注意事项

1. **工具安全性**：
   - `shell` 工具默认可用但有安全风险，建议在生产环境中禁用或限制
   - 工作流工具执行前应验证权限

2. **Token 优化**：
   - 默认最多选择 5 个工具
   - 可以通过 `max_tools` 调整
   - 建议不超过 10 个

3. **工具执行超时**：
   - 每个工具有独立的超时设置
   - Agent 总超时由 `timeout_secs` 控制

4. **错误处理**：
   - 工具执行失败不会中断 Agent
   - LLM 会收到错误信息并尝试其他方法

## 扩展开发

### 添加新的内置工具

1. 在 `sentinel-tools/src/buildin_tools/` 添加工具实现
2. 在 `tool_router.rs` 的 `build_default_tools()` 添加元数据
3. 在 `executor.rs` 的 `get_tool_schema()` 添加 schema
4. 在 `tool_commands.rs` 的 `unified_execute_tool()` 添加执行逻辑

### 添加自定义工具选择策略

实现新的选择策略：

```rust
impl ToolRouter {
    pub async fn select_by_llm(&self, task: &str, max_tools: usize) -> Result<Vec<String>> {
        // 使用轻量级 LLM 分析任务
        // 返回最相关的工具 ID
    }
}
```

## 调试

### 查看工具选择日志

```
[INFO] Selected 3 tools for execution_id xxx: ["port_scan", "http_request", "local_time"]
```

### 查看工具执行日志

```
[INFO] Executing tool: port_scan with args: {"target": "192.168.1.1", "ports": "common"}
[INFO] Tool execution successful: ...
```

### 前端调试

打开浏览器控制台：

```javascript
// 监听所有 Agent 事件
window.__TAURI__.event.listen('agent:tools_selected', console.log)
window.__TAURI__.event.listen('agent:tool_executed', console.log)
window.__TAURI__.event.listen('agent:chunk', console.log)
```

## 未来计划

- [ ] 支持 LLM 智能工具选择（使用轻量级模型）
- [ ] 支持工具使用统计和分析
- [ ] 支持工具执行可视化
- [ ] 支持工具链（一个工具的输出作为另一个的输入）
- [ ] 支持并行工具执行
- [ ] 支持工具执行缓存

## 总结

通过智能工具集成，Agent 现在可以：
- ✅ 自动选择相关工具（节省 90% token）
- ✅ 执行多轮工具调用
- ✅ 支持内置、工作流、MCP、插件工具
- ✅ 前端实时显示工具执行状态
- ✅ 灵活配置工具策略

这使得 Agent 能够完成更复杂的任务，同时保持较低的成本和较高的准确性。
