# Travel 工具过滤修复文档

## 问题描述

在 Travel 架构的 ReAct 执行阶段，发现所有 60 个可用工具都被附加到了 LLM prompt 中，而没有根据 Agent 设置中的工具白名单/黑名单进行过滤。这导致：

1. **Prompt 过长**：60 个工具的描述使得 prompt 长度达到 27,690+ 字符
2. **工具权限失控**：即使在 Agent 设置中配置了工具白名单，ReAct 执行器仍然可以看到所有工具
3. **LLM 混淆**：过多的工具选项可能导致 LLM 选择不合适的工具

### 日志证据

```log
INFO sentinel_ai_lib::engines::travel::react_executor: 283: Travel ReAct: Global engine adapter provided 60 tools
INFO sentinel_ai_lib::engines::travel::react_executor: 322: Travel ReAct: Building tool information for 60 tools
```

```log
[2025-11-20 07:10:03.949 UTC] [SYSTEM REQUEST] [Session: 83f48f12-cbef-4fe0-af0f-24d32efe01db]
Available tools:
- playwright_upload_file(selector: string, filePath: string) - Upload a file...
- http_request(url: string, method?: string, ...) - 通用HTTP客户端工具...
[... 58 more tools ...]
```

## 根本原因

`TravelReactExecutor` 在构建工具信息时，直接从 `FrameworkToolAdapter` 或全局 `EngineToolAdapter` 获取所有可用工具，没有应用工具白名单/黑名单过滤。

虽然 `EngineDispatcher::execute_tool` 方法有工具权限检查，但这个检查只在工具执行时生效，而不影响 LLM 可见的工具列表。

## 解决方案

### 1. 在 `TravelReactExecutor` 中添加工具权限字段

**文件**: `src-tauri/src/engines/travel/react_executor.rs`

```rust
/// Travel专用ReAct执行器
pub struct TravelReactExecutor {
    ai_service: Arc<AiService>,
    prompt_repo: Option<Arc<PromptRepository>>,
    framework_adapter: Option<Arc<dyn FrameworkToolAdapter>>,
    max_iterations: u32,
    conversation_id: Option<String>,
    message_id: Option<String>,
    app_handle: Option<tauri::AppHandle>,
    cancellation_token: CancellationToken,
    /// 允许的工具白名单
    allowed_tools: Option<Vec<String>>,
    /// 禁止的工具黑名单
    denied_tools: Option<Vec<String>>,
}
```

### 2. 添加 Builder 方法

```rust
/// 设置允许的工具白名单
pub fn with_allowed_tools(mut self, tools: Vec<String>) -> Self {
    self.allowed_tools = Some(tools);
    self
}

/// 设置禁止的工具黑名单
pub fn with_denied_tools(mut self, tools: Vec<String>) -> Self {
    self.denied_tools = Some(tools);
    self
}
```

### 3. 在 `build_tools_information` 中应用过滤

```rust
// 去重工具（按名称）
let mut unique_tools: HashMap<String, ToolInfo> = HashMap::new();
for tool in all_tools {
    unique_tools.entry(tool.name.clone()).or_insert(tool);
}

// 应用工具白名单/黑名单过滤
let mut filtered_tools: Vec<ToolInfo> = unique_tools.into_values().collect();

// 如果有白名单，只保留白名单中的工具
if let Some(allowed) = &self.allowed_tools {
    if !allowed.is_empty() {
        filtered_tools.retain(|tool| allowed.contains(&tool.name));
        log::info!(
            "Travel ReAct: Applied allow list, {} tools remain",
            filtered_tools.len()
        );
    }
}

// 如果有黑名单，移除黑名单中的工具
if let Some(denied) = &self.denied_tools {
    if !denied.is_empty() {
        filtered_tools.retain(|tool| !denied.contains(&tool.name));
        log::info!(
            "Travel ReAct: Applied deny list, {} tools remain",
            filtered_tools.len()
        );
    }
}
```

### 4. 在 `EngineDispatcher` 中传递工具权限

**文件**: `src-tauri/src/engines/travel/engine_dispatcher.rs`

```rust
// 提取工具权限
let (allowed_tools, denied_tools, _) = self.extract_tool_permissions(context);

// 创建ReAct执行器
let mut react_executor = TravelReactExecutor::new(
    ai_service.clone(),
    self.prompt_repo.clone(),
    framework_adapter,
    self.max_react_iterations,
    self.conversation_id.clone(),
    self.message_id.clone(),
    self.app_handle.clone(),
    self.cancellation_token.clone(),
);

// 设置工具权限
if !allowed_tools.is_empty() {
    react_executor = react_executor.with_allowed_tools(allowed_tools);
}
if !denied_tools.is_empty() {
    react_executor = react_executor.with_denied_tools(denied_tools);
}
```

## 工具权限来源

工具权限从 `context` 中提取，具体来自：

1. **前端 Agent 设置**：用户在 `AgentManager.vue` 中配置的工具白名单/黑名单
2. **传递路径**：
   - `AgentManager.vue` → `dispatch_agent_task` 命令
   - `ai_commands.rs::dispatch_with_travel` → 构建 `task.parameters`
   - `TravelEngine::execute` → 传递给 `OodaExecutor`
   - `OodaExecutor::execute_act_phase` → 传递给 `EngineDispatcher`
   - `EngineDispatcher::dispatch_complex_task` → 传递给 `TravelReactExecutor`

## 预期效果

修复后，Travel 的 ReAct 执行器将：

1. **只显示允许的工具**：LLM prompt 中只包含白名单中的工具
2. **减少 Prompt 长度**：如果白名单只有 10 个工具，prompt 长度将显著减少
3. **提高工具选择准确性**：更少的工具选项使 LLM 更容易选择正确的工具
4. **增强安全性**：确保 LLM 无法"看到"或尝试调用被禁止的工具

### 日志示例（修复后）

```log
INFO sentinel_ai_lib::engines::travel::react_executor: 283: Travel ReAct: Global engine adapter provided 60 tools
INFO sentinel_ai_lib::engines::travel::react_executor: Applied allow list, 10 tools remain
INFO sentinel_ai_lib::engines::travel::react_executor: Building tool information for 10 tools (after filtering)
```

## 测试建议

1. **基本功能测试**：
   - 在 Agent 设置中配置工具白名单（例如：`["http_request", "analyze_website", "port_scan"]`）
   - 执行 Travel 任务
   - 检查日志，确认只有白名单中的工具被附加到 prompt

2. **黑名单测试**：
   - 配置工具黑名单（例如：`["playwright_close", "playwright_delete"]`）
   - 确认这些工具不出现在 prompt 中

3. **Prompt 长度验证**：
   - 对比修复前后的 LLM Request 日志
   - 确认 Input length 显著减少

4. **工具执行验证**：
   - 确认 LLM 只尝试调用白名单中的工具
   - 如果 LLM 尝试调用被禁止的工具，`execute_tool` 应该返回错误

## 相关文件

- `src-tauri/src/engines/travel/react_executor.rs` - ReAct 执行器主文件
- `src-tauri/src/engines/travel/engine_dispatcher.rs` - 任务调度器
- `src-tauri/src/commands/ai_commands.rs` - AI 命令入口
- `src/views/AgentManager.vue` - 前端 Agent 管理界面

## 注意事项

1. **初始化顺序**：必须在 `TravelReactExecutor::new()` 之后、`execute()` 之前调用 `with_allowed_tools()` 和 `with_denied_tools()`
2. **空白名单处理**：如果白名单为空，将保留所有工具（不过滤）
3. **黑名单优先级**：黑名单在白名单之后应用，即使工具在白名单中，如果也在黑名单中，仍会被移除
4. **工具执行权限**：即使工具在 prompt 中可见，执行时仍会进行二次权限检查（在 `EngineDispatcher::execute_tool` 中）

## 后续修复：analyze_website 参数错误

### 问题描述

在修复工具过滤后，发现 `generate_action_plan` 方法生成的 `ActionStep` 中，`analyze_website` 工具的参数使用了错误的 `url` 而不是正确的 `domain`。

**日志证据**：
```log
# LLM 规划的步骤（正确）
INFO: Executing observation step 1: analyze_website with args: {"domain": String("testphp.vulnweb.com")}
INFO: Tool analyze_website executed successfully

# Act 阶段生成的步骤（错误）
INFO: Executing tool: analyze_website with args: {"url": String("http://testphp.vulnweb.com")}
ERROR: Parameter validation failed for tool 'analyze_website': Missing required parameter: domain
```

### 根本原因

在 `ooda_executor.rs` 的 `generate_action_plan` 方法中，为 `Simple` 和 `Medium` 任务复杂度生成的 `analyze_website` 步骤使用了错误的参数名 `url`，而 `analyze_website` 工具要求的参数名是 `domain`。

同样的问题也存在于 `engine_dispatcher.rs` 的 `try_execute_react_step_fallback` 方法中。

### 修复内容

**文件**: `src-tauri/src/engines/travel/ooda_executor.rs`

1. **Simple 任务复杂度**（第 971-986 行）：
```rust
// 从 URL 中提取域名
let domain = target
    .trim_start_matches("http://")
    .trim_start_matches("https://")
    .split('/')
    .next()
    .unwrap_or(target)
    .split(':')
    .next()
    .unwrap_or(target);

// 修改参数
args.insert("domain".to_string(), serde_json::json!(domain));  // ✅ 正确
```

2. **Medium 任务复杂度**（第 998-1013 行）：同样的修复

**文件**: `src-tauri/src/engines/travel/engine_dispatcher.rs`

3. **ReactEngine 降级方案**（第 246-257 行）：同样的修复

### 修复效果

修复后，`generate_action_plan` 生成的所有 `analyze_website` 步骤都将使用正确的 `domain` 参数，与 LLM 动态规划的步骤保持一致。

## 后续修复：Observe 阶段工具列表动态化

### 问题描述

在 Observe 阶段的 LLM 规划中，`get_available_tools_for_observation` 方法硬编码了 4 个工具（`analyze_website`, `http_request`, `port_scan`, `rsubdomain`），但这些工具可能不在 Agent 设置的工具白名单中，导致 LLM 规划使用了不允许的工具。

**日志证据**：
```log
# LLM 规划使用了 port_scan 和 rsubdomain
INFO: Executing observation step 2: rsubdomain with args: {"domain": String("testphp.vulnweb.com")}
WARN: Observation step 2 (rsubdomain) failed: Tool 'rsubdomain' not in allow list
INFO: Executing observation step 4: port_scan with args: {"target": String("testphp.vulnweb.com"), "ports": String("80,443,8080")}
WARN: Observation step 4 (port_scan) failed: Tool 'port_scan' not in allow list
```

**LLM Prompt 中的硬编码工具**：
```
## 可用工具
- analyze_website(domain: string) - 分析网站结构和技术栈
- http_request(url: string, method: string) - 发送 HTTP 请求
- port_scan(target: string, ports: string) - 扫描端口（需要 IP 地址）
- rsubdomain(domain: string) - 枚举子域名
```

但实际上 Agent 设置中只允许了部分工具（没有 `port_scan` 和 `rsubdomain`）。

### 根本原因

`get_available_tools_for_observation` 方法硬编码了工具列表，没有从 `context` 中读取 `tools_allow` 白名单，导致 LLM 看到的工具列表与实际允许的工具列表不一致。

### 修复内容

**文件**: `src-tauri/src/engines/travel/ooda_executor.rs`

1. **修改方法签名**（第 710 行）：
```rust
// 修改前
async fn get_available_tools_for_observation(&self) -> String

// 修改后
async fn get_available_tools_for_observation(&self, context: &HashMap<String, serde_json::Value>) -> String
```

2. **从 context 中提取工具白名单**：
```rust
let allowed_tools: Vec<String> = context
    .get("tools_allow")
    .and_then(|v| v.as_array())
    .map(|arr| {
        arr.iter()
            .filter_map(|x| x.as_str().map(|s| s.to_string()))
            .collect()
    })
    .unwrap_or_else(Vec::new);
```

3. **动态获取工具详细信息**：
- 优先使用 `FrameworkToolAdapter` 获取工具信息
- 降级使用全局 `EngineToolAdapter`
- 构建包含参数签名的工具描述

4. **更新调用点**（第 532 行）：
```rust
// 修改前
let available_tools = self.get_available_tools_for_observation().await;

// 修改后
let available_tools = self.get_available_tools_for_observation(context).await;
```

### 修复效果

修复后，Observe 阶段的 LLM 规划将：
1. **只看到允许的工具**：LLM prompt 中只包含 Agent 设置中白名单的工具
2. **包含完整的工具签名**：每个工具都有详细的参数列表和类型信息
3. **避免执行失败**：LLM 不会尝试使用不在白名单中的工具

### 预期日志

```log
INFO: Building observation tool list from 28 allowed tools
INFO: Generated 28 tool descriptions for observation
INFO: LLM planned 3 observation steps
INFO: Executing observation step 1: analyze_website with args: {"domain": String("testphp.vulnweb.com")}
INFO: Tool analyze_website executed successfully
INFO: Executing observation step 2: http_request with args: {"url": String("http://testphp.vulnweb.com"), "method": String("GET")}
INFO: Tool http_request executed successfully
# 不再有 "not in allow list" 的警告
```

## 修复日期

2025-11-20

## 修复人员

AI Assistant (Claude Sonnet 4.5)

