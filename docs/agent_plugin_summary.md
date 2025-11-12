# Agent插件工具集成 - 总结报告

## 问题分析

### 原始需求
用户询问：**agent插件工具应该怎么结合 tools 进行使用，当前是否满足？**

### 发现的问题

经过代码分析，发现**当前实现不满足需求**，存在以下问题：

1. **缺少Agent专用的插件工具Provider**
   - 现有的 `PassiveToolProvider` 只服务于被动扫描的MCP接口
   - 插件工具名称格式不匹配（前端用 `plugin::xxx`，但Provider返回 `xxx`）
   - Agent无法通过统一工具系统调用插件

2. **工具调用链路断裂**
   ```
   AgentManager.vue → agent.tools.allow: ["plugin::builtin.sqli"]
                                    ↓
   ReAct引擎 → 过滤白名单 → 查找 "plugin::builtin.sqli"
                                    ↓
   FrameworkAdapter → UnifiedToolManager.call_tool()
                                    ↓
   ❌ 没有Provider能处理 "plugin::" 前缀的工具！
   ```

3. **类别区分问题**
   - `PassiveToolProvider` 注册所有已启用插件，没有过滤 `category`
   - Agent需要的是 `category === 'agentTools'` 的插件
   - 两种场景应该有独立的Provider

## 解决方案

### 创建 AgentPluginProvider

**文件**: `src-tauri/src/tools/agent_plugin_provider.rs`

**核心功能**:
1. 实现 `ToolProvider` trait
2. 自动发现 `category === 'agentTools'` 且已启用的插件
3. 每个插件注册为一个工具，名称格式：`plugin::{plugin_id}`
4. 提供灵活的参数接口（context, target, data）

**关键代码**:
```rust
impl UnifiedTool for AgentPluginTool {
    fn name(&self) -> &str {
        &self.full_tool_name  // "plugin::builtin.sqli"
    }
    
    async fn execute(&self, params: ToolExecutionParams) -> Result<ToolExecutionResult> {
        // 构建RequestContext并调用插件
        let request_ctx = RequestContext {
            method: "AGENT_CALL",
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
        
        plugin_manager.scan_request(&self.plugin_id, &request_ctx).await
    }
}
```

### 注册到全局工具系统

**文件**: `src-tauri/src/lib.rs`

在系统初始化时注册：
```rust
// 1. 注册被动扫描工具（用于MCP接口）
register_passive_tools(passive_state).await;

// 2. 注册Agent插件工具（用于Agent）
let agent_plugin_provider = AgentPluginProvider::new(passive_state);
tool_system.register_provider(agent_plugin_provider).await;
```

### 完整调用链路

```
用户在AgentManager.vue中选择插件
    ↓
保存到agent.tools.allow: ["plugin::builtin.sqli", ...]
    ↓
Agent运行时读取白名单
    ↓
ReAct引擎查询可用工具
    ↓
FrameworkAdapter.list_available_tools()
    ↓
UnifiedToolManager遍历所有Provider
    ↓
AgentPluginProvider返回 ["plugin::builtin.sqli", ...]
    ↓
白名单过滤后注入System Prompt
    ↓
LLM决定调用 plugin::builtin.sqli
    ↓
FrameworkAdapter.execute_tool("plugin::builtin.sqli", params)
    ↓
UnifiedToolManager.call_tool()
    ↓
AgentPluginProvider匹配 "plugin::" 前缀
    ↓
AgentPluginTool.execute()
    ↓
PluginManager.scan_request()
    ↓
插件代码执行
    ↓
返回findings给Agent
```

## 技术设计

### 1. 工具名称格式统一

| 工具类型 | 名称格式 | 示例 |
|---------|---------|------|
| 内置工具 | `tool_name` | `http_request` |
| MCP工具 | `tool_name` | `fetch`, `filesystem_read` |
| 插件工具 | `plugin::{plugin_id}` | `plugin::builtin.sqli` |

### 2. Provider职责分离

| Provider | 服务对象 | 工具来源 | 名称格式 |
|----------|---------|---------|---------|
| BuiltinToolProvider | 所有场景 | 内置工具 | `http_request` |
| McpToolProvider | 所有场景 | MCP连接 | `fetch` |
| PassiveToolProvider | MCP接口 | 被动扫描插件 | `builtin.sqli` |
| **AgentPluginProvider** | **Agent** | **agentTools插件** | **`plugin::builtin.sqli`** |

### 3. 插件分类体系

```
Plugin
├── category = "agentTools"     → AgentPluginProvider
├── category = "passiveScan"    → PassiveToolProvider (MCP)
├── category = "builtinTools"   → 未使用
├── category = "mcpTools"       → 未使用
└── category = "custom"         → 根据用途决定
```

### 4. 参数传递设计

Agent调用插件时的参数结构：
```json
{
  "context": {
    "conversation_id": "...",
    "task_description": "检测SQL注入",
    "previous_findings": [...]
  },
  "target": "https://example.com/api/users",
  "data": {
    "method": "POST",
    "params": {"id": "1' OR '1'='1"}
  }
}
```

插件接收的 RequestContext：
```rust
RequestContext {
    method: "AGENT_CALL",
    url: "https://example.com/api/users",
    headers: {
        "X-Agent-Plugin": "true",
        "X-Plugin-Id": "builtin.sqli",
    },
    body: json!({
        "context": {...},
        "data": {...},
        "inputs": {...}  // 完整的原始参数
    }),
    ...
}
```

## 实现状态

### ✅ 已完成

1. **前端功能** (之前完成)
   - Tools.vue: "插件工具" tab显示agentTools类别插件
   - AgentManager.vue: 插件工具选择UI
   - 工具存储格式: `plugin::{plugin_id}`

2. **后端Provider** (本次完成)
   - `AgentPluginProvider` 实现
   - 自动发现agentTools类别插件
   - 工具名称格式匹配前端
   - 灵活的参数接口

3. **系统集成** (本次完成)
   - 注册到全局工具系统
   - 导出到模块接口
   - 初始化流程正确

4. **文档** (本次完成)
   - `docs/agent_plugin_integration.md`: 完整的集成说明
   - 架构设计、使用流程、开发指南

### ⚠️ 待验证

1. **端到端测试**
   - 创建agentTools类别插件
   - 在Agent中选择插件
   - Agent成功调用插件并获取结果

2. **参数兼容性**
   - 插件能否正确解析 RequestContext.body 中的参数
   - 插件返回的findings格式是否符合Agent预期

3. **错误处理**
   - 插件执行失败时的错误传递
   - Agent如何处理插件错误

### 🔄 未来优化方向

1. **专用插件接口**
   - 设计 `AgentContext` 替代 `RequestContext`
   - 更清晰的语义，更符合Agent场景

2. **插件能力声明**
   - 插件声明输入/输出schema
   - Agent可以更智能地选择和组合插件

3. **插件测试工具**
   - UI中提供插件测试界面
   - 不依赖Agent即可测试插件功能

## 与被动扫描对比

| 维度 | 被动扫描插件 | Agent插件工具 |
|------|-------------|--------------|
| **触发方式** | HTTP流量自动触发 | Agent主动调用 |
| **输入数据** | 真实HTTP请求/响应 | Agent构造的分析上下文 |
| **类别标识** | `category = "passiveScan"` | `category = "agentTools"` |
| **Provider** | PassiveToolProvider | **AgentPluginProvider** |
| **工具名称** | `builtin.sqli` | `plugin::builtin.sqli` |
| **使用场景** | 实时流量监控 | AI驱动的主动测试 |
| **调用链路** | ProxyServer → Scanner → Plugin | Agent → ToolSystem → Plugin |

## AI助手集成（vibe hacking）

根据项目指导原则，所有模块应与AI助手联动。本次实现完全符合：

1. **AI驱动工具选择**: Agent根据对话决定调用哪些插件
2. **智能参数构造**: Agent将用户意图转换为插件参数
3. **结果智能解释**: 插件返回的技术发现由Agent翻译为用户友好的解释
4. **动态工具组合**: Agent可以组合多个插件工具形成分析流程
5. **上下文感知**: 插件接收完整的对话上下文，提供更精准的分析

示例对话流程：
```
用户: "帮我检查这个API是否有SQL注入漏洞: https://api.example.com/users?id=1"
    ↓
Agent: "好的，我将使用SQL注入检测插件进行分析"
    ↓
调用: plugin::builtin.sqli({
    target: "https://api.example.com/users?id=1",
    context: { task: "SQL injection detection" }
})
    ↓
插件返回: [{ vuln_type: "sqli", severity: "high", ... }]
    ↓
Agent: "⚠️ 发现高危SQL注入漏洞！该API的id参数未进行充分过滤..."
```

## 文件清单

### 新增文件
- `src-tauri/src/tools/agent_plugin_provider.rs` (301行) - Agent插件工具Provider实现
- `docs/agent_plugin_integration.md` (400+行) - 完整集成文档

### 修改文件
- `src-tauri/src/tools/mod.rs` - 导出 `AgentPluginProvider`
- `src-tauri/src/lib.rs` - 注册 `AgentPluginProvider` 到全局工具系统

### 相关文件（之前完成）
- `src/views/Tools.vue` - "插件工具" tab
- `src/views/AgentManager.vue` - 插件工具选择UI

## 总结

**问题**: 当前系统不满足Agent使用插件工具的需求，缺少关键的Provider层。

**解决**: 实现了 `AgentPluginProvider`，完整打通了从前端选择到后端执行的全链路。

**状态**: 
- ✅ 架构完整：前端UI + 后端Provider + 系统集成
- ✅ 代码无错：通过TypeScript和Rust编译检查
- ⚠️ 待测试：需要端到端测试验证功能

**下一步建议**:
1. 创建一个 `category = "agentTools"` 的测试插件
2. 在Agent中选择该插件
3. 通过对话触发Agent调用插件
4. 验证参数传递和结果返回

---

*本次实现完全符合"vibe hacking"理念，让AI Agent能够灵活运用各种安全插件工具进行智能化的安全分析。*
