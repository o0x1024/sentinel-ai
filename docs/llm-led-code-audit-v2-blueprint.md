# LLM 主导代码审计 V2 — 重构蓝图

> **核心思想**：LLM 是"安全研究员的大脑" — 它理解意图、推理逻辑、做出判断。
> 工具是它的"手脚和感官" — 帮它看到代码、理解结构、追踪数据、验证假设。

更新日期：2026-02-24

---

## 一、设计哲学

### 传统 SAST 工具 vs LLM 主导审计

```
传统 SAST:  规则引擎 → 扫描 → 输出误报成堆的报告 → 人工筛选
LLM 主导:   LLM 理解业务 → 提出假设 → 用工具验证 → 研判确认 → 形成结论
```

**LLM 的不可替代优势**：
1. **理解业务语义** — 知道 "admin 不该被普通用户访问" 这类逻辑
2. **跨上下文推理** — 将 A 文件的认证缺失和 B 文件的数据库操作关联起来
3. **适应性强** — 不需要预定义规则就能发现新型漏洞
4. **自然语言表达** — 能把技术发现翻译成可操作的修复建议

**LLM 的天然弱点**（需要工具弥补）： 
1. **无法直接"看到"代码** — 需要文件读取工具
2. **无法做精确的图遍历** — 需要 CPG 查询工具
3. **无法执行代码** — 需要沙箱执行工具
4. **上下文窗口有限** — 需要高密度信息摘要工具
5. **无法进行网络交互** — 需要 HTTP 客户端工具

### 工具设计原则

1. **高信息密度**：每次工具调用返回的信息应最大化有效密度，减少 LLM 的调用次数
2. **结构化输出**：工具永远返回结构化 JSON，不要返回大段纯文本让 LLM 解析
3. **可组合性**：工具应该能自由组合，而不是绑定到某个固定流程
4. **语义友好**：工具名和参数设计应该符合 LLM 的自然推理方式
5. **渐进式深入**：从粗到细，先概览再深入，避免一开始就陷入细节

---

## 二、工具体系架构（5 大工具组）

```
┌─────────────────────────────────────────────────────────────┐
│                    LLM（安全审计大脑）                          │
│  理解业务 → 生成假设 → 选择工具 → 分析结果 → 做出判断             │
└───────┬──────────┬──────────┬──────────┬──────────┬──────────┘
        │          │          │          │          │
   ┌────▼───┐ ┌───▼────┐ ┌──▼───┐ ┌───▼────┐ ┌───▼────┐
   │ 👁 感知 │ │ 🧠 分析 │ │ 🖐 验证│ │ 📋 管理 │ │ 🌐 情报 │
   │  工具组  │ │  工具组  │ │ 工具组 │ │  工具组  │ │  工具组  │
   └────────┘ └────────┘ └──────┘ └────────┘ └────────┘
```

---

## 三、工具组 1：👁 感知层（Perception）— "LLM 的眼睛"

> 让 LLM 能够"看到"和"理解"代码，并以最小的上下文消耗获取最大的信息量。

### 1.1 `read_file` ✅ 已有（保持）

读取文件/目录。V1 已实现，保持不变。

### 1.2 `code_search` ✅ 已有（保持）

ripgrep 搜索。V1 已实现，保持不变。

### 1.3 `smart_file_summary` 🆕 新增

**痛点**：LLM 经常需要理解一个大文件（>500行），但 `read_file` 一次只能看 200 行，来回翻看浪费大量工具调用和上下文。

**设计**：
```rust
struct SmartFileSummaryArgs {
    /// 文件路径
    path: String,
    /// 摘要焦点: "security" | "api" | "data_flow" | "full"
    /// 不同焦点会强调不同维度的信息
    focus: Option<String>,
}

struct SmartFileSummaryOutput {
    path: String,
    language: String,
    total_lines: usize,
    /// 文件结构骨架（函数签名+行号，带安全相关注解标注）
    skeleton: Vec<SkeletonEntry>,
    /// 该文件中检测到的安全相关模式（如外部输入、数据库调用、认证检查等）
    security_signals: Vec<SecuritySignal>,
    /// 该文件暴露的 HTTP 路由/API 端点
    exposed_endpoints: Vec<EndpointInfo>,
    /// import/require 的关键依赖
    key_imports: Vec<String>,
    /// 最值得深入审查的行范围（按安全风险排序）
    hotspots: Vec<Hotspot>,
}

struct SkeletonEntry {
    kind: String,        // "function" | "class" | "method"
    name: String,
    signature: String,   // 完整签名（含参数、返回类型、注解）
    line_start: usize,
    line_end: usize,
    visibility: String,
    annotations: Vec<String>,  // @auth, @admin, @public 等
}

struct SecuritySignal {
    signal_type: String,    // "external_input" | "db_query" | "file_io" | "auth_check" | "crypto" | "eval"
    line: usize,
    snippet: String,
    confidence: f64,
}

struct Hotspot {
    line_start: usize,
    line_end: usize,
    reason: String,   // 为什么这段代码值得关注
    severity_hint: String, // "critical" | "high" | "medium"
}
```

**实现方式**：利用已有 CPG 数据（如果有的话）+ tree-sitter 快速解析，提取结构化信息。无需 LLM 参与。

**LLM 使用场景**：
```
1. LLM 审计 UserController.java（1200行），先调用 smart_file_summary 获取骨架
2. 从 hotspots 中识别出 3 个高风险区域
3. 只用 read_file 精确读取这 3 段代码，节省 80% 的上下文消耗
```

### 1.4 `get_function_detail` 🆕 新增

**痛点**：当前 `query_cpg(functions)` 只返回函数名和所在文件，LLM 不知道参数是什么、返回什么、有哪些注解。要看参数得调 `read_file`。

**设计**：
```rust
struct GetFunctionDetailArgs {
    path: String,
    /// 函数名（支持模糊匹配: "UserService.*" 或 "save*"）
    function_name: String,
}

struct FunctionDetailOutput {
    functions: Vec<FunctionDetail>,
}

struct FunctionDetail {
    name: String,
    qualified_name: String,     // e.g., "UserService.saveUser"
    file: String,
    line_start: usize,
    line_end: usize,
    signature: String,          // 完整函数签名
    parameters: Vec<ParamInfo>, // 每个参数的名称和类型
    return_type: Option<String>,
    visibility: String,         // public/private/protected
    is_async: bool,
    is_static: bool,
    annotations: Vec<String>,   // @GetMapping, @PreAuthorize 等
    /// 这个函数的关键安全属性
    security_context: SecurityContext,
    /// 函数体代码（可选，默认不返回以节省上下文）
    body: Option<String>,
    /// 调用者列表（简化版）
    callers: Vec<String>,
    /// 被调用函数列表（简化版）
    callees: Vec<String>,
}

struct ParamInfo {
    name: String,
    type_annotation: Option<String>,
    is_from_external_input: bool,   // 标注: 来自 @RequestParam / req.body 等
    annotations: Vec<String>,
}

struct SecurityContext {
    has_auth_check: bool,       // 函数内或其前置中间件有无认证检查
    accepts_external_input: bool, // 是否接收外部输入参数
    has_db_operation: bool,     // 是否包含数据库操作
    has_file_operation: bool,   // 是否包含文件操作
    has_command_exec: bool,     // 是否包含命令执行
    auth_annotations: Vec<String>, // 认证相关注解
}
```

**LLM 使用场景**：
```
LLm: "让我看看 UserService 里所有的公开方法"
→ get_function_detail({ path: "/workspace/project", function_name: "UserService.*" })
→ 一次调用获得所有函数的签名、参数、安全上下文
→ 发现 deleteUser(userId) 没有 @PreAuthorize 注解，且接受外部输入 → 疑似 IDOR
```

### 1.5 `get_attack_surface` 🆕 新增

**痛点**：LLM 需要先理解"哪里是攻击面"才能开始有针对性的审计，但当前没有工具能一键给出全貌。

**设计**：
```rust
struct GetAttackSurfaceArgs {
    path: String,
}

struct AttackSurfaceOutput {
    /// 所有 HTTP 端点（路由+方法+处理函数+认证状态）
    http_endpoints: Vec<HttpEndpointDetail>,
    /// WebSocket 端点
    websocket_endpoints: Vec<EndpointBrief>,
    /// gRPC 服务
    grpc_services: Vec<EndpointBrief>,
    /// 定时任务/Worker
    scheduled_tasks: Vec<TaskBrief>,
    /// 消息队列消费者
    message_consumers: Vec<ConsumerBrief>,
    /// 文件上传处理点
    file_upload_handlers: Vec<String>,
    /// 管理/调试端点
    admin_endpoints: Vec<HttpEndpointDetail>,
    /// 统计
    total_endpoints: usize,
    unprotected_count: usize,   // 无认证保护的端点数
    admin_without_auth_count: usize, // 管理功能但无权限检查
}

struct HttpEndpointDetail {
    method: String,         // GET, POST, PUT, DELETE
    route: String,          // /api/users/:id
    handler_function: String,
    handler_file: String,
    handler_line: usize,
    /// 认证/授权状态
    auth_status: AuthStatus, // "protected" | "unprotected" | "partially_protected" | "unknown"
    auth_middleware: Vec<String>, // 适用的中间件列表
    required_roles: Vec<String>, // 需要的角色
    /// 接收的输入
    input_params: Vec<InputParam>,
    /// 安全风险提示
    risk_indicators: Vec<String>,
}

struct InputParam {
    name: String,
    source: String,  // "path" | "query" | "body" | "header" | "cookie"
    param_type: Option<String>,
    has_validation: bool,
}
```

**实现方式**：基于 CPG 的 EntryPoint 节点 + 框架特定规则提取。需要扩展 CPG builder 来识别更多框架的路由注册模式（Express/Spring/Django/Laravel/Gin 等）。

**LLM 使用场景**：
```
LLM: "先给我项目的完整攻击面"
→ get_attack_surface({ path: "/workspace/project" })
→ 发现 23 个端点，其中 5 个没有认证保护，2 个管理端点只有 @LoginRequired 没有 @AdminRequired
→ LLM 制定审计计划: 优先审计这 7 个风险端点
```

---

## 四、工具组 2：🧠 分析层（Analysis）— "LLM 的推理辅助"

> 帮助 LLM 做精确的代码级分析，补偿 LLM 无法做图遍历和精确计算的弱点。

### 2.1 `build_cpg` / `query_cpg` ✅ 已有（增强）

**增强点**：
- `query_cpg` 新增查询类型：
  - `function_detail`: 获取函数完整信息（同 1.4，但通过 CPG 查询方式调用）
  - `class_hierarchy`: 获取类继承关系树
  - `data_flow_in_function`: 函数内变量传播分析（见 2.2）
  - `reachable_from`: 从某个函数出发能到达的所有函数（传递闭包）
  - `entry_points_reaching`: 哪些入口点能调用到指定函数

### 2.2 `trace_data_flow` 🆕 新增（**核心工具**）

**痛点**：V1 的污点分析只在调用图级别做 BFS，不追踪变量在函数内部的传播。这是漏报的最大来源。

**设计**：
```rust
struct TraceDataFlowArgs {
    path: String,
    /// 起点: 文件:行号 或 "函数名.参数名" 或 "变量名"
    from: String,
    /// 追踪方向: "forward" (数据从 from 流向哪里) | "backward" (from 的数据从哪里来)
    direction: Option<String>,  // default: "forward"
    /// 最大追踪深度（跨函数跳数）
    max_depth: Option<usize>,  // default: 8
    /// 是否跨文件追踪
    cross_file: Option<bool>,  // default: true
}

struct TraceDataFlowOutput {
    origin: DataPoint,
    /// 数据流路径（按顺序）
    flow_paths: Vec<DataFlowPath>,
    /// 路径经过的安全敏感点
    sensitive_operations: Vec<SensitiveOp>,
    /// 路径上检测到的 sanitizer
    sanitizers_applied: Vec<SanitizerInfo>,
    /// 总结
    summary: String,
}

struct DataFlowPath {
    /// 路径中的每一步
    steps: Vec<DataFlowStep>,
    /// 终点是否是危险 sink
    reaches_dangerous_sink: bool,
    /// 是否经过 sanitizer
    is_sanitized: bool,
    /// 置信度
    confidence: f64,
}

struct DataFlowStep {
    file: String,
    line: usize,
    code_snippet: String,
    /// 步骤类型
    step_type: String,  // "source" | "assignment" | "function_call" | "return" | "parameter_pass" | "sink"
    /// 当前追踪的变量名（可能因赋值而改名）
    tracked_variable: String,
    /// 变量转换描述
    transformation: Option<String>,  // e.g., "assigned to variable x", "passed as arg[0]", "returned from function"
}

struct SensitiveOp {
    kind: String,    // "sql_query" | "command_exec" | "file_write" | "http_request" | "eval"
    file: String,
    line: usize,
    code_snippet: String,
}

struct SanitizerInfo {
    name: String,
    file: String,
    line: usize,
    /// sanitizer 是否有效匹配 sink 类型
    effective_for: Vec<String>,  // ["sql_injection", "xss"] etc.
}
```

**实现方式**：
1. 从 CPG 获取函数内的参数节点和调用节点
2. 在 tree-sitter AST 上实现**函数内的 def-use 链追踪**
3. 遇到函数调用时，通过 CPG 的 Calls 边跳到被调用函数
4. 记录变量在每一步的名称变化（重命名、赋值、返回等）

**LLM 使用场景**：
```
LLM: "req.body.username 这个输入会流到哪里？"
→ trace_data_flow({
    path: "/workspace/project",
    from: "UserController.createUser:req.body.username",
    direction: "forward"
  })
→ 返回:
  步骤1: UserController.java:45, req.body.username 赋值给 name
  步骤2: UserController.java:47, name 传入 userService.save(name)
  步骤3: UserService.java:23, 参数 name 传入 db.query("INSERT ... " + name)
  ⚠️ reaches_dangerous_sink: true, is_sanitized: false
→ LLM 确认: SQL 注入漏洞
```

### 2.3 `cpg_taint_analysis` / `cpg_security_scan` ✅ 已有（保持）

基线扫描仍然有用，作为 LLM 侦察阶段的快速全局扫描。

### 2.4 `check_auth_chain` 🆕 新增

**痛点**：业务逻辑漏洞（IDOR/越权/BOLA）是 SAST 工具几乎无法检测的，而 LLM 恰恰擅长。但 LLM 需要工具帮它快速分析一个端点的完整认证链。

**设计**：
```rust
struct CheckAuthChainArgs {
    path: String,
    /// 要检查的目标函数或端点
    target: String,  // "POST /api/users/:id/delete" 或 "UserService.deleteUser"
}

struct CheckAuthChainOutput {
    target: String,
    /// 请求到达目标函数前经过的所有中间件/拦截器
    middleware_chain: Vec<MiddlewareInfo>,
    /// 认证检查
    authentication: AuthCheckResult,
    /// 授权检查
    authorization: AuthzCheckResult,
    /// 输入验证
    input_validation: ValidationResult,
    /// IDOR 风险分析
    idor_risk: IdorRiskResult,
    /// 建议 LLM 深入检查的代码区域
    review_suggestions: Vec<String>,
}

struct AuthCheckResult {
    has_auth: bool,
    auth_type: String,  // "jwt" | "session" | "basic" | "oauth" | "none" | "unknown"
    auth_source: String, // 认证代码位置
    bypasses: Vec<String>, // 可能的绕过路径
}

struct AuthzCheckResult {
    has_role_check: bool,
    required_roles: Vec<String>,
    /// 是否检查资源归属（防 IDOR）
    has_ownership_check: bool,
    /// 归属检查的代码位置
    ownership_check_location: Option<String>,
}

struct IdorRiskResult {
    /// 用户可控的 ID 参数
    user_controlled_ids: Vec<String>,
    /// 是否用这些 ID 直接查询数据库（无归属校验）
    direct_db_lookup: bool,
    /// 风险等级
    risk_level: String,
}
```

**LLM 使用场景**：
```
LLM: "检查 DELETE /api/users/:id 的认证授权链"
→ check_auth_chain({
    path: "/workspace/project",
    target: "DELETE /api/users/:id"
  })
→ 返回:
  authentication: { has_auth: true, auth_type: "jwt" }
  authorization: { has_role_check: false, has_ownership_check: false }
  idor_risk: {
    user_controlled_ids: ["id"],
    direct_db_lookup: true,
    risk_level: "critical"
  }
→ LLM 确认: IDOR 漏洞 — 任何登录用户可以删除其他用户
```

### 2.5 `diff_analysis` 🆕 新增（增量审计场景）

**痛点**：企业场景下很多审计是针对 PR/MR 的增量审计。`git_diff_scope` 只给出变更范围，LLM 需要更多上下文来理解变更的安全影响。

**设计**：
```rust
struct DiffAnalysisArgs {
    path: String,
    /// git range (e.g., "main..HEAD", "abc123..def456")
    range: Option<String>,
    /// 只分析指定文件
    files: Option<Vec<String>>,
}

struct DiffAnalysisOutput {
    /// 变更概览
    summary: DiffSummary,
    /// 按安全影响分组的变更
    security_relevant_changes: Vec<SecurityRelevantChange>,
    /// 新增的攻击面
    new_attack_surface: Vec<String>,
    /// 被修改的安全控制
    modified_security_controls: Vec<String>,
    /// 依赖变更
    dependency_changes: Vec<DepChange>,
    /// LLM 应重点关注的代码段
    priority_review_areas: Vec<ReviewArea>,
}

struct SecurityRelevantChange {
    file: String,
    change_type: String,  // "new_endpoint" | "auth_change" | "input_handling" | "crypto_change" | "config_change"
    description: String,
    diff_snippet: String,
    risk_level: String,
}
```

---

## 五、工具组 3：🖐 验证层（Verification）— "LLM 的手"

> 让 LLM 不仅能"看"和"分析"，还能主动验证假设。把 LLM 的结论从 "可能存在漏洞" 提升到 "确认存在漏洞"。

### 3.1 `sandbox_exec` 🆕 新增

**痛点**：LLM 推理出一个潜在漏洞后，无法验证。只能输出 "疑似"，confidence 很低。

**设计**：
```rust
struct SandboxExecArgs {
    /// 要执行的代码（仅限审计辅助脚本，在 Docker sandbox 中运行）
    code: String,
    /// 语言: "python" | "javascript" | "bash"
    language: String,
    /// 超时（秒）
    timeout: Option<u64>,  // default: 30, max: 120
    /// 是否允许网络访问（仅限访问被审计项目的本地服务）
    allow_network: Option<bool>,  // default: false
}

struct SandboxExecOutput {
    stdout: String,
    stderr: String,
    exit_code: i32,
    timed_out: bool,
}
```

**安全约束**：
- 只在 Docker 容器内执行
- 默认无网络访问
- 文件系统只读挂载或只能访问被审计项目目录
- 严格超时限制
- 禁止持久化进程

**LLM 使用场景**：
```
LLM: "让我验证这个正则表达式是否真的能被 ReDoS 攻击"
→ sandbox_exec({
    language: "python",
    code: """
import re, time
pattern = r'^(a+)+$'
payload = 'a' * 25 + 'b'
start = time.time()
re.match(pattern, payload)
print(f'Time: {time.time() - start:.2f}s')
""",
    timeout: 10
  })
→ stdout: "Time: 8.73s"
→ LLM 确认: ReDoS 漏洞确认，25个字符就需要 8.73 秒
```

### 3.2 `http_probe` 🆕 新增

**痛点**：如果被审计的项目在 sandbox 中运行，LLM 想验证一个端点是否真的存在漏洞（如未授权访问），目前无法做到。

**设计**：
```rust
struct HttpProbeArgs {
    /// URL (只允许 localhost 或 sandbox 网络)
    url: String,
    method: Option<String>,  // default: "GET"
    headers: Option<HashMap<String, String>>,
    body: Option<String>,
    /// 是否跟随重定向
    follow_redirects: Option<bool>,
    /// 超时
    timeout_secs: Option<u64>,
}

struct HttpProbeOutput {
    status_code: u16,
    headers: HashMap<String, String>,
    /// body 的前 N 字符（避免大响应耗尽上下文）
    body_preview: String,
    body_size: usize,
    response_time_ms: u64,
    /// 安全相关 header 分析
    security_headers: SecurityHeaderAnalysis,
}

struct SecurityHeaderAnalysis {
    has_csp: bool,
    has_hsts: bool,
    has_x_frame_options: bool,
    cors_policy: Option<String>,
    cookies_secure: bool,
    cookies_httponly: bool,
    /// 不安全的 header
    issues: Vec<String>,
}
```

**安全约束**：
- **只允许访问 localhost / 127.0.0.1 / Docker 内网**
- 禁止访问外部网络
- body_preview 限制长度，避免大响应耗尽上下文

**LLM 使用场景**：
```
LLM: "验证 /api/admin/users 是否需要认证才能访问"
→ http_probe({
    url: "http://localhost:3000/api/admin/users",
    method: "GET"
  })
→ status_code: 200, body_preview: "[{id: 1, name: 'admin', email: ...}]"
→ LLM 确认: 管理端点未授权访问，严重安全漏洞
```

### 3.3 `regex_analyzer` 🆕 新增

**痛点**：很多安全漏洞与正则表达式有关（ReDoS、绕过验证等），LLM 推理正则表达式的行为时准确度不高。

**设计**：
```rust
struct RegexAnalyzerArgs {
    /// 正则表达式
    pattern: String,
    /// 分析类型: "redos_check" | "bypass_check" | "coverage_check"
    analysis_type: String,
    /// bypass_check 时的上下文: 这个正则用于什么场景
    context: Option<String>,  // "email_validation" | "sql_filter" | "xss_filter" | "path_filter"
    /// 可选: 测试用的 payload 列表
    test_payloads: Option<Vec<String>>,
}

struct RegexAnalyzerOutput {
    pattern: String,
    is_valid: bool,
    /// ReDoS 风险评估
    redos_risk: RedosRisk,
    /// payload 测试结果
    test_results: Vec<PayloadTestResult>,
    /// 绕过建议（如果用于安全过滤）
    potential_bypasses: Vec<String>,
    analysis_notes: String,
}
```

---

## 六、工具组 4：📋 治理层（Governance）— "LLM 的笔记本"

> 管理审计进度、发现落库、质量控制。

### 4.1 `audit_finding_upsert` ✅ 已有（保持）
### 4.2 `transition_lifecycle` ✅ 已有（保持）
### 4.3 `audit_report` ✅ 已有（增强）

**增强点**：
- 新增 `attack_chains` 字段：支持将多个 findings 关联为完整攻击链
- 新增 OWASP Top 10 分类自动映射
- 新增修复优先级评分（可利用性 × 影响面 × 修复成本）

### 4.4 `audit_plan` 🆕 新增（替代 audit_coverage）

**痛点**：`audit_coverage` 只能追踪 "哪些文件看过了"，无法表达 "审计策略" 和 "优先级"。

**设计**：
```rust
struct AuditPlanArgs {
    session_id: String,
    /// 操作
    action: String,  // "auto_generate" | "add_task" | "complete_task" | "list" | "suggest_next"
    /// auto_generate 时使用：基于 get_attack_surface 结果自动生成审计计划
    attack_surface_path: Option<String>,
    /// add_task 时使用
    task: Option<AuditTask>,
    /// complete_task 时使用
    task_id: Option<String>,
    completion_note: Option<String>,
}

struct AuditTask {
    id: String,
    /// 审计目标描述
    target: String,       // "POST /api/users/:id/delete"
    /// 审计维度
    check_types: Vec<String>,  // ["auth", "idor", "injection", "input_validation"]
    /// 优先级 1-5
    priority: u8,
    /// 状态
    status: String,       // "pending" | "in_progress" | "completed" | "skipped"
    /// 关联的发现 ID
    linked_findings: Vec<String>,
}

struct AuditPlanOutput {
    session_id: String,
    tasks: Vec<AuditTask>,
    total: usize,
    completed: usize,
    /// 推荐的下一个审计任务（基于优先级和依赖关系）
    suggested_next: Option<AuditTask>,
    coverage_percent: f64,
}
```

**`auto_generate` 的核心逻辑**：
1. 调用 `get_attack_surface` 获取所有端点
2. 对每个端点生成审计任务，附带检查维度
3. 按风险排序：无认证端点 > 管理端点 > 有输入的端点 > 其他
4. 返回排序后的审计计划

**LLM 使用场景**：
```
LLM: "帮我生成审计计划"
→ audit_plan({ action: "auto_generate", attack_surface_path: "/workspace/project" })
→ 返回 15 个审计任务，按优先级排序
→ LLM 按顺序执行，每完成一个标记为 completed
→ suggest_next 自动推荐下一个目标
```

### 4.5 `register_custom_rule` 🆕 新增

**痛点**：LLM 在审计过程中发现项目特有的安全模式，但无法让扫描工具识别。

**设计**：
```rust
struct RegisterCustomRuleArgs {
    /// 规则名
    name: String,
    /// CWE 编号
    cwe: Option<String>,
    severity: String,
    /// Source 模式（正则或函数名）
    sources: Vec<String>,
    /// Sink 模式
    sinks: Vec<String>,
    /// Sanitizer 模式
    sanitizers: Option<Vec<String>>,
    /// 适用语言
    languages: Option<Vec<String>>,
    /// 规则描述
    description: String,
}
```

**LLM 使用场景**：
```
LLM: "这个项目用了自定义的 PermissionService.check()，我需要把它加入 sanitizer"
→ register_custom_rule({
    name: "custom_auth_bypass",
    sources: ["@Controller.*"],
    sinks: [".*Repository.*"],
    sanitizers: ["PermissionService.check", "SecurityFilter.verify"],
    description: "项目特有的权限检查模式"
  })
→ 后续的 cpg_taint_analysis 会自动使用这条规则
```

---

## 七、工具组 5：🌐 情报层（Intelligence）— "LLM 的知识库"

> 提供外部安全知识支持，让 LLM 不仅基于代码分析，还能结合已知漏洞情报做出判断。

### 5.1 `dependency_audit` ✅ 已有（保持）

### 5.2 `search_cve` 🆕 新增

**设计**：
```rust
struct SearchCveArgs {
    /// 搜索查询（包名、CVE ID、关键词）
    query: String,
    /// 可选：指定包名
    package: Option<String>,
    /// 可选：指定版本
    version: Option<String>,
    /// 最大结果数
    limit: Option<usize>,
}

struct SearchCveOutput {
    results: Vec<CveInfo>,
    total: usize,
}

struct CveInfo {
    cve_id: String,
    title: String,
    description: String,
    severity: String,
    cvss_score: Option<f64>,
    affected_versions: Vec<String>,
    fixed_versions: Vec<String>,
    references: Vec<String>,
    exploit_available: bool,
    /// PoC 概要（如果有）
    poc_summary: Option<String>,
}
```

**实现方式**：查询本地 CVE 数据库（如 OSV DB 离线副本）或通过 API 查询 NVD/OSV/ExploitDB。

### 5.3 `search_security_patterns` 🆕 新增

**设计**：一个内置的安全知识库查询工具，包含常见的漏洞模式、安全编码最佳实践、框架安全配置等。

```rust
struct SearchSecurityPatternsArgs {
    /// 查询关键词
    query: String,
    /// 框架过滤
    framework: Option<String>,  // "spring" | "django" | "express" | "laravel"
    /// 漏洞类型过滤
    vulnerability_type: Option<String>, // "sqli" | "xss" | "idor" | "auth_bypass"
}
```

**LLM 使用场景**：
```
LLM: "这个 Django 项目用了 raw SQL，哪些情况下是安全的？"
→ search_security_patterns({
    query: "raw SQL safe usage",
    framework: "django"
  })
→ 返回 Django raw SQL 的安全使用模式和常见错误
→ LLM 对比项目代码，判断是否遵循了安全实践
```

---

## 八、完整的审计流程（V2 重构后）

```
Phase 1: 快速侦察（1-2 轮工具调用）
  ├─ build_cpg()                    → 构建代码图
  ├─ get_attack_surface()           → 获取完整攻击面
  └─ audit_plan(auto_generate)      → 自动生成审计计划

Phase 2: 逐目标深度审计（核心循环）
  ├─ audit_plan(suggest_next)       → 获取下一个审计目标
  ├─ smart_file_summary()           → 快速理解目标文件
  ├─ get_function_detail()          → 查看关键函数签名和安全上下文
  ├─ check_auth_chain()             → 分析认证授权链（如果是端点）
  ├─ trace_data_flow()              → 追踪用户输入的数据流
  ├─ read_file()                    → 精确读取可疑代码段
  ├─ [LLM 推理研判]
  │   ├─ sandbox_exec()             → 验证漏洞（如 ReDoS/注入）
  │   ├─ http_probe()               → 动态验证端点（如果有运行环境）
  │   └─ search_cve()               → 查询已知漏洞情报
  ├─ audit_finding_upsert()         → 落库发现
  └─ audit_plan(complete_task)      → 标记任务完成

Phase 3: 交叉分析
  ├─ cpg_security_scan()            → 全局扫描补漏
  ├─ dependency_audit()             → 依赖漏洞
  └─ 攻击链关联分析                   → 多漏洞组合

Phase 4: 复核与报告
  ├─ Judge 子代理复核
  ├─ transition_lifecycle()         → 状态流转
  └─ audit_report()                 → 生成报告
```

---

## 九、V1 → V2 工具对照表

| 状态 | 工具名 | 工具组 | V1 存在？ | V2 变化 |
|---|---|---|---|---|
| ✅ | `read_file` | 感知 | ✅ | 保持 |
| ✅ | `code_search` | 感知 | ✅ | 保持 |
| 🆕 | `smart_file_summary` | 感知 | ❌ | **新增** — 高密度文件摘要 |
| 🆕 | `get_function_detail` | 感知 | ❌ | **新增** — 函数签名/参数/安全上下文 |
| 🆕 | `get_attack_surface` | 感知 | ❌ | **新增** — 一键攻击面分析 |
| ✅ | `build_cpg` | 分析 | ✅ | 保持 |
| ✅ | `query_cpg` | 分析 | ✅ | **增强** — 新增查询类型 |
| 🆕 | `trace_data_flow` | 分析 | ❌ | **新增** — 变量级数据流追踪 |
| ✅ | `cpg_taint_analysis` | 分析 | ✅ | 保持 |
| ✅ | `cpg_security_scan` | 分析 | ✅ | 保持 |
| 🆕 | `check_auth_chain` | 分析 | ❌ | **新增** — 认证授权链分析 |
| 🆕 | `diff_analysis` | 分析 | ❌ | **新增** — 增量安全分析 |
| 🆕 | `sandbox_exec` | 验证 | ❌ | **新增** — 沙箱代码执行 |
| 🆕 | `http_probe` | 验证 | ❌ | **新增** — HTTP 探测验证 |
| 🆕 | `regex_analyzer` | 验证 | ❌ | **新增** — 正则安全分析 |
| ✅ | `audit_finding_upsert` | 治理 | ✅ | 保持 |
| ✅ | `transition_lifecycle` | 治理 | ✅ | 保持 |
| ✅ | `audit_report` | 治理 | ✅ | **增强** — 攻击链+优先级 |
| 🆕 | `audit_plan` | 治理 | ❌ | **新增** — 替代 audit_coverage |
| 🆕 | `register_custom_rule` | 治理 | ❌ | **新增** — 动态规则注册 |
| ✅ | `dependency_audit` | 情报 | ✅ | 保持 |
| 🆕 | `search_cve` | 情报 | ❌ | **新增** — CVE 情报查询 |
| 🆕 | `search_security_patterns` | 情报 | ❌ | **新增** — 安全知识库 |
| ❌ | `cross_file_taint` | 分析 | ✅ | **移除** — 被 trace_data_flow 替代 |
| ❌ | `audit_coverage` | 治理 | ✅ | **移除** — 被 audit_plan 替代 |
| ❌ | `call_graph_lite` | 分析 | ✅ | **移除** — 被 query_cpg 替代 |
| ❌ | `project_overview` | 感知 | ✅ | **移除** — 被 get_attack_surface + query_cpg(summary) 替代 |

---

## 十、实现优先级

### P0 — 必须（直接影响审计效果） ✅ 已实现
1. **`trace_data_flow`** ✅ — 基于 CPG 调用图的数据流追踪（forward/backward）
2. **`get_function_detail`** ✅ — 富函数详情（签名/参数/安全上下文/调用者/被调者）
3. **`get_attack_surface`** ✅ — 一键攻击面分析（端点/认证状态/风险指标）
4. **`smart_file_summary`** ✅ — 高密度文件摘要（骨架/安全信号/热点）

### P1 — 重要（显著提升审计深度）
5. **`check_auth_chain`** — 认证授权链分析
6. **`audit_plan`** — 审计计划管理（替代 audit_coverage）
7. **`sandbox_exec`** — 沙箱代码执行验证
8. **`register_custom_rule`** — 动态规则注册

### P2 — 增强（提升完整度）
9. **`diff_analysis`** — 增量安全分析
10. **`regex_analyzer`** — 正则安全分析
11. **`search_cve`** — CVE 情报查询
12. **`search_security_patterns`** — 安全知识库
