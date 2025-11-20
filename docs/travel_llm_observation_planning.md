# Travel 动态 Observe 流程规划

## 问题背景

原有的 `collect_observations` 方法硬编码了 Web 渗透测试的流程，无法适应其他任务类型：

```rust
// ❌ 硬编码的流程
async fn collect_observations(...) {
    // 1. analyze_website
    // 2. http_request  
    // 3. port_scan
}
```

**问题**:
- ❌ 只适用于 Web 渗透测试
- ❌ 无法扩展到代码审计、CTF、移动安全等任务
- ❌ 不同任务需要不同的工具组合
- ❌ 缺乏灵活性

## 解决方案

使用 LLM 动态规划 Observe 流程，根据任务类型和目标智能选择合适的工具。

### 架构设计

```
用户查询 + 任务类型 + 目标
         ↓
    LLM 规划器
         ↓
  观察步骤计划 (JSON)
         ↓
    执行工具调用
         ↓
    观察结果汇总
```

## 实现细节

### 1. 主流程：LLM 动态规划

```rust
async fn collect_observations(
    &self,
    context: &HashMap<String, serde_json::Value>,
) -> Result<HashMap<String, serde_json::Value>> {
    // 提取任务信息
    let target = context.get("target")...;
    let task_type = context.get("task_type")...;
    let query = context.get("query")...;

    // 使用 LLM 规划观察流程
    if let Some(ai_service) = &self.engine_dispatcher.ai_service {
        match self.plan_observation_with_llm(...).await {
            Ok(planned_observations) => {
                // 执行 LLM 规划的步骤
                observations.extend(planned_observations);
            }
            Err(e) => {
                // 降级到默认流程
                self.collect_observations_fallback(...).await;
            }
        }
    } else {
        // 无 AI 服务，使用降级流程
        self.collect_observations_fallback(...).await;
    }

    Ok(observations)
}
```

### 2. LLM 规划器

```rust
async fn plan_observation_with_llm(
    &self,
    ai_service: &Arc<AiService>,
    task_type: &str,
    target: &str,
    query: &str,
    context: &HashMap<String, serde_json::Value>,
) -> Result<HashMap<String, serde_json::Value>> {
    // 构建可用工具列表
    let available_tools = self.get_available_tools_for_observation().await;
    
    // 构建 LLM Prompt
    let prompt = format!(
        r#"你是一个安全测试专家，负责规划 Observe (侦察) 阶段的信息收集流程。

**任务类型**: {}
**目标**: {}
**用户查询**: {}

**可用工具**:
{}

**任务类型说明**:
- web_pentest: Web 渗透测试 → 使用 analyze_website, http_request, port_scan
- api_pentest: API 安全测试 → 使用 http_request, analyze_website
- code_audit: 代码审计 → 不需要网络工具，直接分析代码
- ctf: CTF 夺旗 → 根据题目类型选择工具
- mobile_security: 移动应用安全 → 分析 APK/IPA 文件
- cloud_security: 云安全评估 → 使用云服务 API
- network_security: 网络安全 → 使用 port_scan, rsubdomain

**请规划需要执行的观察步骤**，以 JSON 格式返回：

```json
{{
  "steps": [
    {{
      "tool": "工具名称",
      "args": {{"参数名": "参数值"}},
      "description": "步骤描述"
    }}
  ],
  "reasoning": "规划理由"
}}
```

**注意事项**:
1. 根据任务类型选择合适的工具
2. 代码审计、CTF 等任务可能不需要网络扫描
3. 工具参数必须正确（如 analyze_website 需要 domain，不是 url）
4. 端口扫描需要 IP 地址，不是域名
5. 只规划侦察阶段，不要包含攻击步骤

只返回 JSON，不要其他文字。"#,
        task_type, target, query, available_tools
    );

    // 调用 LLM
    let response = ai_service
        .send_message_stream(Some(&prompt), None, None, None, false, true, None)
        .await?;

    // 解析 LLM 响应
    let plan: serde_json::Value = self.parse_llm_observation_plan(&response)?;

    // 执行规划的步骤
    let mut observations = HashMap::new();
    
    if let Some(steps) = plan.get("steps").and_then(|s| s.as_array()) {
        for (idx, step) in steps.iter().enumerate() {
            let tool_name = step.get("tool")...;
            let args = step.get("args")...;
            
            // 执行工具
            match self.engine_dispatcher.execute_tool(tool_name, &args, context).await {
                Ok(result) => {
                    observations.insert(format!("{}_result", tool_name), result);
                }
                Err(e) => {
                    log::warn!("Observation step {} ({}) failed: {}", idx + 1, tool_name, e);
                }
            }
        }
    }

    Ok(observations)
}
```

### 3. 降级流程：按任务类型分类

```rust
async fn collect_observations_fallback(
    &self,
    target: &str,
    task_type: &str,
    context: &HashMap<String, serde_json::Value>,
    observations: &mut HashMap<String, serde_json::Value>,
) {
    match task_type {
        "web_pentest" | "api_pentest" => {
            // Web/API 渗透测试：网站分析 + HTTP 请求 + 端口扫描
            if let Some(result) = self.try_analyze_website(target, context).await {
                observations.insert("website_analysis".to_string(), result);
            }
            
            if let Some(result) = self.try_http_request(target, context).await {
                observations.insert("http_response".to_string(), result);
            }
            
            if let Some(result) = self.try_port_scan(target, context).await {
                observations.insert("port_scan".to_string(), result);
            }
        }
        "code_audit" => {
            // 代码审计：不需要网络扫描
            observations.insert("code_target".to_string(), serde_json::json!(target));
            observations.insert("audit_type".to_string(), serde_json::json!("static_analysis"));
        }
        "ctf" => {
            // CTF：根据目标类型决定
            if target.starts_with("http://") || target.starts_with("https://") {
                // Web CTF
                if let Some(result) = self.try_http_request(target, context).await {
                    observations.insert("http_response".to_string(), result);
                }
            } else {
                // 其他类型 CTF
                observations.insert("ctf_target".to_string(), serde_json::json!(target));
            }
        }
        _ => {
            // 未知任务类型：尝试基本的 HTTP 请求
            if target.starts_with("http://") || target.starts_with("https://") {
                if let Some(result) = self.try_http_request(target, context).await {
                    observations.insert("http_response".to_string(), result);
                }
            }
        }
    }
}
```

## 支持的任务类型

### 1. Web 渗透测试 (web_pentest)

**LLM 规划示例**:
```json
{
  "steps": [
    {
      "tool": "analyze_website",
      "args": {"domain": "example.com"},
      "description": "分析网站结构和技术栈"
    },
    {
      "tool": "http_request",
      "args": {"url": "http://example.com", "method": "GET"},
      "description": "获取首页内容"
    },
    {
      "tool": "port_scan",
      "args": {"target": "192.168.1.1", "ports": "80,443,8080"},
      "description": "扫描常见 Web 端口"
    }
  ],
  "reasoning": "Web 渗透测试需要全面了解目标网站的结构、技术栈和开放端口"
}
```

### 2. 代码审计 (code_audit)

**LLM 规划示例**:
```json
{
  "steps": [],
  "reasoning": "代码审计不需要网络扫描，直接分析源代码即可"
}
```

**降级流程**:
- 不执行任何网络工具
- 只记录目标代码路径
- 标记为静态分析任务

### 3. CTF 夺旗 (ctf)

**LLM 规划示例** (Web CTF):
```json
{
  "steps": [
    {
      "tool": "http_request",
      "args": {"url": "http://ctf.example.com/challenge", "method": "GET"},
      "description": "获取 CTF 题目页面"
    }
  ],
  "reasoning": "Web CTF 需要先获取题目内容，分析可能的漏洞点"
}
```

**LLM 规划示例** (Crypto CTF):
```json
{
  "steps": [],
  "reasoning": "密码学 CTF 不需要网络扫描，直接分析加密算法"
}
```

### 4. API 安全测试 (api_pentest)

**LLM 规划示例**:
```json
{
  "steps": [
    {
      "tool": "http_request",
      "args": {"url": "https://api.example.com/v1/users", "method": "GET"},
      "description": "测试 API 端点可访问性"
    },
    {
      "tool": "analyze_website",
      "args": {"domain": "api.example.com"},
      "description": "分析 API 服务器技术栈"
    }
  ],
  "reasoning": "API 测试重点关注端点和认证机制"
}
```

### 5. 移动应用安全 (mobile_security)

**LLM 规划示例**:
```json
{
  "steps": [],
  "reasoning": "移动应用安全测试需要分析 APK/IPA 文件，不需要网络扫描"
}
```

### 6. 云安全评估 (cloud_security)

**LLM 规划示例**:
```json
{
  "steps": [
    {
      "tool": "http_request",
      "args": {"url": "https://api.aws.amazon.com/...", "method": "GET"},
      "description": "查询云服务配置"
    }
  ],
  "reasoning": "云安全评估需要通过 API 查询配置信息"
}
```

## 技术亮点

### 1. 智能规划

- ✅ LLM 根据任务类型自动选择工具
- ✅ 理解任务上下文，生成合理的步骤顺序
- ✅ 避免不必要的工具调用（如代码审计不需要端口扫描）

### 2. 参数正确性

LLM Prompt 中明确说明了工具参数要求：
- ✅ `analyze_website` 需要 `domain`，不是 `url`
- ✅ `port_scan` 需要 IP 地址，不是域名
- ✅ 避免常见的参数错误

### 3. 降级机制

- ✅ LLM 规划失败 → 降级到基于任务类型的默认流程
- ✅ 无 AI 服务 → 直接使用降级流程
- ✅ 确保系统始终可用

### 4. 可扩展性

- ✅ 添加新任务类型：只需在 Prompt 中添加说明
- ✅ 添加新工具：更新 `get_available_tools_for_observation`
- ✅ 无需修改核心逻辑

## 使用示例

### 示例 1: Web 渗透测试

**输入**:
```rust
context = {
    "task_type": "web_pentest",
    "target": "http://testphp.vulnweb.com",
    "query": "对网站进行全面的安全测试"
}
```

**LLM 规划**:
```json
{
  "steps": [
    {"tool": "analyze_website", "args": {"domain": "testphp.vulnweb.com"}},
    {"tool": "http_request", "args": {"url": "http://testphp.vulnweb.com", "method": "GET"}},
    {"tool": "port_scan", "args": {"target": "44.228.249.3", "ports": "80,443,8080,8443"}}
  ],
  "reasoning": "全面测试需要分析网站结构、获取内容、扫描端口"
}
```

**执行结果**:
```rust
observations = {
    "target": "http://testphp.vulnweb.com",
    "task_type": "web_pentest",
    "analyze_website_result": {...},
    "http_request_result": {...},
    "port_scan_result": {...},
    "observation_reasoning": "全面测试需要分析网站结构、获取内容、扫描端口"
}
```

### 示例 2: 代码审计

**输入**:
```rust
context = {
    "task_type": "code_audit",
    "target": "/path/to/source/code",
    "query": "审计 Java 项目的安全问题"
}
```

**LLM 规划**:
```json
{
  "steps": [],
  "reasoning": "代码审计是静态分析任务，不需要网络扫描工具"
}
```

**执行结果**:
```rust
observations = {
    "target": "/path/to/source/code",
    "task_type": "code_audit",
    "observation_reasoning": "代码审计是静态分析任务，不需要网络扫描工具"
}
```

### 示例 3: CTF Web 题目

**输入**:
```rust
context = {
    "task_type": "ctf",
    "target": "http://ctf.example.com/challenge",
    "query": "解决 Web CTF 题目"
}
```

**LLM 规划**:
```json
{
  "steps": [
    {"tool": "http_request", "args": {"url": "http://ctf.example.com/challenge", "method": "GET"}}
  ],
  "reasoning": "Web CTF 需要先获取题目页面，分析可能的漏洞"
}
```

## 修改的文件

### 1. `src-tauri/src/engines/travel/ooda_executor.rs`

**主要修改**:
- ✅ `collect_observations`: 改为调用 LLM 规划器
- ✅ `plan_observation_with_llm`: 新增 LLM 规划方法
- ✅ `parse_llm_observation_plan`: 解析 LLM 响应
- ✅ `get_available_tools_for_observation`: 获取可用工具列表
- ✅ `collect_observations_fallback`: 降级流程（按任务类型）

### 2. `src-tauri/src/engines/travel/engine_dispatcher.rs`

**主要修改**:
- ✅ 将字段改为 `pub(crate)`，允许 `ooda_executor` 访问 `ai_service`

## 编译状态

```bash
✅ Finished `dev` profile [unoptimized] target(s) in 14.89s
⚠️ 122 warnings (unused imports - 无关紧要)
❌ 0 errors
```

## 优势对比

### 修改前 ❌

```rust
// 硬编码流程
async fn collect_observations(...) {
    // 1. analyze_website (总是执行)
    // 2. http_request (总是执行)
    // 3. port_scan (总是执行)
}
```

**问题**:
- ❌ 代码审计也会执行网络扫描（不合理）
- ❌ CTF 密码学题目也会扫描端口（浪费资源）
- ❌ 无法适应新的任务类型

### 修改后 ✅

```rust
// LLM 动态规划
async fn collect_observations(...) {
    // LLM 根据任务类型智能规划
    match task_type {
        "web_pentest" => [analyze_website, http_request, port_scan],
        "code_audit" => [],
        "ctf" => [根据题目类型动态决定],
        ...
    }
}
```

**优势**:
- ✅ 智能选择工具，避免不必要的调用
- ✅ 适应所有任务类型
- ✅ 易于扩展新任务类型
- ✅ LLM 理解上下文，生成合理的步骤

## 后续优化建议

1. **缓存 LLM 规划结果**: 相同任务类型的规划可以缓存
2. **工具依赖关系**: LLM 可以理解工具之间的依赖（如先 DNS 解析再端口扫描）
3. **动态工具列表**: 根据任务类型动态过滤可用工具
4. **多轮规划**: 根据第一轮观察结果，LLM 可以规划第二轮观察

---

**实现日期**: 2025-11-20  
**实现人员**: AI Assistant  
**状态**: ✅ 已完成并编译通过

