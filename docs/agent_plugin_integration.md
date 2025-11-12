# Agent插件工具集成方案

## 概述

本文档说明Agent如何使用插件工具（Plugin Tools）进行安全分析。

## 架构设计

### 1. 前端实现（✅ 已完成）

**AgentManager.vue**
- 显示所有 `category === 'agentTools'` 的已启用插件
- 用户选择插件工具时，以 `"plugin::pluginId"` 格式存储到 `agent.tools.allow` 数组
- 工具显示逻辑正确处理 `plugin::` 前缀，显示为 "插件名称 (插件)"

**Tools.vue**
- "插件工具" tab 显示所有 `agentTools` 类别的已启用插件
- 用户可以在此处查看和管理可用于Agent的插件

### 2. 后端实现（✅ 已完成）

#### 2.1 AgentPluginProvider

**位置**: `src-tauri/src/tools/agent_plugin_provider.rs`

**功能**:
- 实现 `ToolProvider` trait
- 自动发现并注册所有 `category === 'agentTools'` 且 `status === Enabled` 的插件
- 每个插件对应一个工具，工具名称为 `plugin::{plugin_id}`

**工具接口**:
```rust
pub struct AgentPluginTool {
    // 参数:
    // - context: 分析上下文（灵活的JSON对象）
    // - target: 目标URL/域名/标识符
    // - data: 输入数据（字符串/对象/数组）
}
```

**执行流程**:
```
Agent调用 -> UnifiedToolManager -> AgentPluginProvider -> PluginManager -> 插件代码执行
```

#### 2.2 注册到统一工具系统

**位置**: `src-tauri/src/lib.rs`

在系统初始化时，`AgentPluginProvider` 被注册到全局工具系统：

```rust
// 1. 初始化全局工具系统
initialize_global_tool_system(db_service).await

// 2. 注册被动扫描工具
register_passive_tools(passive_state).await

// 3. 注册Agent插件工具
let agent_plugin_provider = AgentPluginProvider::new(passive_state);
tool_system.register_provider(agent_plugin_provider).await
```

### 3. Agent执行流程

#### 3.1 工具过滤

**ReAct引擎** (`src-tauri/src/engines/react/executor.rs`):
```rust
// 从 task_parameters 读取 tools_allow
let allow = params.get("tools_allow") // ["plugin::builtin.sqli", "http_request", ...]

// 从框架适配器获取所有可用工具
let available_tools = framework_adapter.list_available_tools().await;

// 过滤：只保留在白名单中的工具
for tool_name in available_tools {
    if !allow.is_empty() && !allow.contains(&tool_name) {
        continue; // 跳过不在白名单的工具
    }
    all_tools.push(tool_info);
}
```

#### 3.2 工具调用

**框架适配器** (`src-tauri/src/tools/framework_adapters.rs`):
```rust
// Agent请求调用 "plugin::builtin.sqli"
async fn execute_tool(&self, call: UnifiedToolCall) -> Result<UnifiedToolResult> {
    let tool_manager = self.tool_manager.read().await;
    
    // UnifiedToolManager查找工具提供者
    // AgentPluginProvider匹配 "plugin::" 前缀
    // 返回对应的 AgentPluginTool
    tool_manager.call_tool(&call.tool_name, execution_params).await
}
```

#### 3.3 插件执行

**AgentPluginTool** (`agent_plugin_provider.rs`):
```rust
async fn execute(&self, params: ToolExecutionParams) -> Result<ToolExecutionResult> {
    // 1. 构建RequestContext（包含Agent传入的参数）
    let request_ctx = RequestContext {
        method: "AGENT_CALL",
        url: params.inputs.get("target"),
        body: json!({
            "context": params.inputs.get("context"),
            "data": params.inputs.get("data"),
            "inputs": params.inputs,
        }),
        headers: {
            "X-Agent-Plugin": "true",
            "X-Plugin-Id": self.plugin_id,
        },
        ...
    };
    
    // 2. 调用插件管理器执行插件
    let findings = plugin_manager.scan_request(&self.plugin_id, &request_ctx).await?;
    
    // 3. 返回结果
    Ok(ToolExecutionResult {
        tool_name: format!("plugin::{}", self.plugin_id),
        output: json!({ "findings": findings, ... }),
        ...
    })
}
```

## 使用流程

### 1. 创建Agent工具插件

在 **PluginManagement.vue** 中：
1. 创建新插件
2. 设置 `category = "agentTools"`
3. 编写插件代码（接收 RequestContext，返回 Finding[]）
4. 启用插件

### 2. 配置Agent使用插件

在 **AgentManager.vue** 中：
1. 新建或编辑Agent
2. 在"可用工具"部分展开"插件工具"
3. 勾选需要的插件（如 "SQL注入检测插件"）
4. 保存Agent配置

此时 `agent.tools.allow` 包含：
```json
[
  "http_request",
  "plugin::builtin.sqli",
  "plugin::custom.xss_detector"
]
```

### 3. Agent运行时调用插件

Agent执行查询时：
1. **工具发现**: 系统列出所有可用工具（包括 `plugin::*`）
2. **工具过滤**: 根据 `tools_allow` 白名单过滤工具
3. **System Prompt**: 将允许的工具描述注入到Agent prompt中
4. **工具调用**: Agent决定调用 `plugin::builtin.sqli`
5. **插件执行**: AgentPluginProvider 转发调用到PluginManager
6. **结果返回**: 插件返回漏洞发现（findings）给Agent

## 插件开发指南

### 插件接口约定

Agent插件接收的 `RequestContext`:
```typescript
interface RequestContext {
  method: "AGENT_CALL",
  url: string,  // 从 params.target 获取
  headers: {
    "X-Agent-Plugin": "true",
    "X-Plugin-Id": string,
  },
  body: {
    context: object,  // Agent提供的分析上下文
    data: any,        // Agent提供的输入数据
    inputs: object,   // 完整的工具调用参数
  },
  ...
}
```

### 示例插件代码

```typescript
export default {
  id: 'custom.url_analyzer',
  name: 'URL分析器',
  category: 'agentTools',
  version: '1.0.0',
  
  async onRequest(ctx: RequestContext): Promise<Finding[]> {
    // 解析Agent传入的参数
    const body = JSON.parse(new TextDecoder().decode(ctx.body));
    const target = body.inputs?.target || ctx.url;
    const context = body.context || {};
    
    // 执行分析逻辑
    const findings: Finding[] = [];
    
    if (target.includes('admin')) {
      findings.push({
        vuln_type: 'sensitive_path',
        severity: 'medium',
        title: '敏感路径检测',
        description: `发现管理路径: ${target}`,
        evidence: { url: target },
      });
    }
    
    return findings;
  }
}
```

## 技术细节

### 工具名称格式

- **内置工具**: `http_request`, `port_scan`, `subdomain_scan`
- **MCP工具**: `fetch`, `filesystem_read` (无前缀，由provider识别)
- **插件工具**: `plugin::builtin.sqli`, `plugin::custom.xss` (带 `plugin::` 前缀)

### 工具提供者层次

```
UnifiedToolManager
├── BuiltinToolProvider       (内置工具)
├── McpToolProvider            (MCP连接的工具)
├── PassiveToolProvider        (被动扫描工具 - 用于MCP接口)
└── AgentPluginProvider        (Agent插件工具 - 用于Agent)
    └── 动态加载 category=agentTools 的插件
```

### 工具调用链路

```
Agent Query
    ↓
AI Commands (ai_commands.rs)
    ↓ tools_allow: ["plugin::xxx", ...]
Agent Engine (react/plan_execute/rewoo)
    ↓
Framework Adapter
    ↓ list_available_tools() + 白名单过滤
System Prompt with filtered tools
    ↓
LLM generates tool call
    ↓
Framework Adapter.execute_tool("plugin::xxx", params)
    ↓
UnifiedToolManager.call_tool()
    ↓
AgentPluginProvider.get_tool("plugin::xxx")
    ↓
AgentPluginTool.execute()
    ↓
PluginManager.scan_request(plugin_id, request_ctx)
    ↓
Plugin Code Execution
    ↓
Return findings to Agent
```

## 与被动扫描的区别

| 特性 | 被动扫描插件 | Agent插件工具 |
|------|-------------|--------------|
| 触发方式 | HTTP流量触发 | Agent主动调用 |
| 输入 | RequestContext/ResponseContext (真实HTTP流量) | RequestContext (Agent构造) |
| 类别标识 | category = "passiveScan" | category = "agentTools" |
| 工具提供者 | PassiveToolProvider (for MCP) | AgentPluginProvider (for Agent) |
| 使用场景 | 实时流量分析 | AI驱动的主动安全测试 |
| 工具名称 | `builtin.sqli` (插件ID) | `plugin::builtin.sqli` (带前缀) |

## 当前状态

### ✅ 已完成
- 前端UI：插件工具选择界面
- 前端逻辑：工具显示和存储（`plugin::` 前缀）
- 后端Provider：AgentPluginProvider实现
- 系统注册：全局工具系统集成
- 工具过滤：Agent引擎白名单支持

### ⚠️ 待测试
- 端到端流程：创建插件 → 配置Agent → Agent调用插件
- 插件参数传递：确认 RequestContext 格式是否满足插件需求
- 错误处理：插件执行失败时的Agent反馈

### 🔄 可选优化
- **专用插件接口**: 为Agent插件设计专门的接口（不复用 RequestContext）
  - 优点：更清晰的语义，更灵活的参数结构
  - 当前方案：复用被动扫描的 RequestContext，通过 body 传递参数
  
- **插件类型系统**: 支持插件声明输入/输出schema
  - 优点：Agent可以更好地理解插件能力
  - 当前方案：通用的 ToolParameters 定义

- **插件测试工具**: 在UI中提供插件测试界面
  - 优点：插件开发者可以快速验证功能
  - 当前方案：需要通过Agent调用来测试

## AI助手集成建议

根据项目要求（所有模块应与AI助手联动），插件工具已经完全集成到Agent系统中：

1. **AI驱动发现**: Agent可以根据对话上下文决定调用哪些插件
2. **智能组合**: Agent可以组合多个插件工具进行复杂分析
3. **结果解释**: 插件返回的findings由Agent解释并融入对话
4. **动态调整**: 用户可以随时修改Agent的可用插件列表

这符合"vibe hacking"理念，让AI助手能够灵活运用各种安全插件工具。
