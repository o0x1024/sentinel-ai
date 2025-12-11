# Rig-Core 重构计划

## 概述

将项目的 Agent 系统从自研架构迁移到 rig-core 框架，包括：
- 使用 rig-core Tool trait 重写所有工具
- 使用 rig Agent 替代自研的 React 引擎
- 使用 rmcp 替代自研的 MCP 客户端

## 模块分析

### 需要删除的模块

| 模块 | 原因 |
|------|------|
| `src/engines/react/` | 自研 React 引擎，由 rig Agent 替代 |
| `src/engines/intelligent_dispatcher/` | 任务调度器，由 rig Agent 替代 |
| `src/agents/planner.rs` | 自研计划器，由 rig Agent 替代 |
| `src/agents/reflector.rs` | 自研反思器，由 rig Agent 替代 |
| `src/agents/orchestrator.rs` | 自研编排器，由 rig Agent 替代 |
| `src/agents/executor.rs` | 自研执行器，由 rig Tool 替代 |
| `src/tools/framework_adapters.rs` | 框架适配器，不再需要 |
| `src/tools/registry.rs` | 工具注册表，由 rig ToolSet 替代 |
| `src/tools/unified_manager.rs` | 统一管理器，由 rig 替代 |
| `src/tools/protocol.rs` | 协议定义，由 rig 类型替代 |
| `src/tools/adapter_factory.rs` | 适配器工厂，不再需要 |
| `sentinel-tools/src/unified_types.rs` | 统一类型，由 rig 类型替代 |
| `sentinel-tools/src/manager.rs` | 工具管理器，由 rig 替代 |
| `sentinel-tools/src/mapping.rs` | 映射，不再需要 |

### 需要保留并改造的模块

| 模块 | 改造内容 |
|------|---------|
| `sentinel-tools/src/builtin/` | 改为实现 rig-core Tool trait |
| `src/tools/mcp_provider.rs` | 改为使用 rmcp |
| `src/agents/todo_manager.rs` | 保留，与 rig agent 集成 |
| `src/agents/emitter.rs` | 保留，用于前端消息通知 |
| `src/agents/manager.rs` | 重构为 rig Agent 管理 |
| `src/engines/memory/` | 保留，可与 rig 集成 |

### 需要新增的模块

| 模块 | 功能 |
|------|------|
| `src/rig_agent/mod.rs` | rig Agent 配置和管理 |
| `src/rig_agent/tools.rs` | rig Tool 集成 |
| `src/rig_agent/mcp.rs` | rmcp MCP 客户端 |
| `src/rig_agent/providers.rs` | LLM Provider 配置 |

## 新架构设计

### 1. Tool 实现 (rig-core Tool trait)

```rust
use rig::tool::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct PortScanArgs {
    /// Target IP address
    pub target: String,
    /// Port range (e.g., "1-1000" or "80,443")
    #[serde(default = "default_ports")]
    pub ports: String,
    /// Concurrent threads
    #[serde(default = "default_threads")]
    pub threads: usize,
}

fn default_ports() -> String { "common".to_string() }
fn default_threads() -> usize { 100 }

#[derive(Debug, Serialize)]
pub struct PortScanOutput {
    pub target: String,
    pub open_ports: Vec<PortInfo>,
    pub scan_duration_ms: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum PortScanError {
    #[error("Invalid target: {0}")]
    InvalidTarget(String),
    #[error("Scan failed: {0}")]
    ScanFailed(String),
}

pub struct PortScanTool;

impl Tool for PortScanTool {
    const NAME: &'static str = "port_scan";
    type Args = PortScanArgs;
    type Output = PortScanOutput;
    type Error = PortScanError;

    async fn definition(&self, _prompt: String) -> rig::tool::ToolDefinition {
        rig::tool::ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Scan TCP ports on target host".to_string(),
            parameters: schemars::schema_for!(PortScanArgs).into(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // 实现端口扫描逻辑
    }
}
```

### 2. Agent 创建

```rust
use rig::{providers::openai, completion::Prompt};

let client = openai::Client::from_env();
let agent = client
    .agent("gpt-4o")
    .preamble("You are a security expert...")
    .tool(PortScanTool)
    .tool(SubdomainScanTool)
    .tool(HttpRequestTool)
    .rmcp_tools(mcp_tools, mcp_client) // MCP 工具
    .build();

let response = agent.prompt("Scan ports on 192.168.1.1").await?;
```

### 3. MCP 集成 (rmcp)

```rust
use rmcp::{ServiceExt, transport::StreamableHttpClientTransport};

// 连接 MCP 服务器
let transport = StreamableHttpClientTransport::from_uri("http://localhost:8080");
let client = client_info.serve(transport).await?;

// 获取 MCP 工具列表
let mcp_tools = client.list_tools(Default::default()).await?.tools;

// 集成到 rig agent
let agent = openai_client
    .agent("gpt-4o")
    .rmcp_tools(mcp_tools, client.peer().to_owned())
    .build();
```

## 实施步骤

### Phase 1: sentinel-tools 改造

1. 创建新的 `sentinel-tools/src/rig_tools/` 目录
2. 实现 rig Tool trait 版本的内置工具:
   - `port_scan.rs`
   - `subdomain_scan.rs`
   - `http_request.rs`
   - `local_time.rs`
3. 删除旧的 `unified_types.rs`, `manager.rs`, `mapping.rs`
4. 更新 `lib.rs`

### Phase 2: tools 模块清理

1. 删除不再需要的文件:
   - `framework_adapters.rs`
   - `registry.rs`
   - `unified_manager.rs`
   - `protocol.rs`
   - `adapter_factory.rs`
2. 保留并重构:
   - `mcp_provider.rs` → 使用 rmcp
   - `builtin/` → 移至 sentinel-tools

### Phase 3: engines 模块清理

1. 删除 `react/` 目录
2. 删除 `intelligent_dispatcher/` 目录
3. 保留 `memory/` 目录（可选集成）
4. 保留 `vision_explorer/`（如需要）

### Phase 4: agents 模块重构

1. 创建新的 `src/rig_agent/` 模块
2. 删除旧的 agent 文件:
   - `planner.rs`
   - `reflector.rs`
   - `orchestrator.rs`
   - `executor.rs`
3. 保留:
   - `todo_manager.rs`
   - `emitter.rs`
   - `manager.rs` (重构)

### Phase 5: 前端接口适配

更新 Tauri 命令以适配新的 rig Agent 系统

## 依赖更新

```toml
[dependencies]
rig-core = { version = "0.25.0", features = ["derive"] }
rmcp = { version = "0.9.1", features = [...] }
schemars = "0.8"  # 用于 JsonSchema derive
```

