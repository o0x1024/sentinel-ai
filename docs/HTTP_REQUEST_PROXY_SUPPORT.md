# HTTP Request工具被动扫描代理支持

## 📋 问题修复总结

### 之前的问题

AI助手在进行安全测试时，使用 `http_request` 工具直接发送HTTP请求，**绕过了被动扫描代理**，导致：
- 被动扫描插件无法检测请求和响应
- 漏洞无法被保存到数据库
- VulnerabilitiesPanel 不显示任何漏洞

### 根本原因

`http_request` 工具没有支持使用代理，所有请求都是直接发送到目标网站。

## ✅ 修复方案

### 1. 添加代理支持参数

在 `http_request` 工具中新增 `use_passive_proxy` 参数：

```rust
ParameterDefinition {
    name: "use_passive_proxy".to_string(),
    param_type: ParameterType::Boolean,
    description: "Route traffic through passive scanning proxy (port 4201) for vulnerability detection".to_string(),
    required: false,
    default_value: Some(json!(false)),
}
```

### 2. 实现代理配置

修改 `execute_request` 方法，配置 reqwest 客户端使用代理：

```rust
let mut client_builder = Client::builder()
    .user_agent("Sentinel-AI/1.0")
    .timeout(Duration::from_secs(config.timeout_seconds))
    .redirect(...)
    .danger_accept_invalid_certs(!config.verify_ssl);

// Configure passive scanning proxy if requested
if config.use_passive_proxy {
    client_builder = client_builder
        .proxy(reqwest::Proxy::http("http://127.0.0.1:4201")?)
        .proxy(reqwest::Proxy::https("http://127.0.0.1:4201")?);
}

let client = client_builder.build()?;
```

### 3. 更新提示词文档

在 `automated_security_testing.md` 中增强说明：

```markdown
## 🔴 CRITICAL REQUIREMENT FOR VULNERABILITY DETECTION

All HTTP traffic MUST go through the passive scanning proxy!

✅ CORRECT: http_request({url: "...", use_passive_proxy: true})
❌ WRONG:  http_request({url: "..."})  // Bypasses proxy!
```

## 🔧 使用方式

### 错误用法（绕过代理）

```javascript
// ❌ 这样发送的请求不会经过被动扫描代理
http_request({
  url: "http://testphp.vulnweb.com/listproducts.php?cat=1' OR '1'='1",
  method: "GET"
})
```

**结果**：
- 请求直接发送到目标网站
- 被动扫描插件看不到流量
- 不会检测到任何漏洞

### 正确用法（使用代理）

```javascript
// ✅ 正确：请求会经过被动扫描代理
http_request({
  url: "http://testphp.vulnweb.com/listproducts.php?cat=1' OR '1'='1",
  method: "GET",
  use_passive_proxy: true  // 🔑 关键参数
})
```

**结果**：
1. 请求发送到 `127.0.0.1:4201` (被动扫描代理)
2. 代理转发请求到目标网站
3. ScanPipeline 调用插件分析请求
4. 目标网站响应返回到代理
5. ScanPipeline 调用插件分析响应
6. 检测到的漏洞保存到数据库
7. VulnerabilitiesPanel 自动显示漏洞

## 📊 数据流对比

### ❌ 不使用代理

```
AI助手
  └─> http_request (use_passive_proxy: false)
       └─> reqwest::Client (直接连接)
            └─> 目标网站
                 └─> 响应返回给AI
                      └─> AI手动分析 (不保存到数据库)
```

### ✅ 使用代理

```
AI助手
  └─> http_request (use_passive_proxy: true)
       └─> reqwest::Client (配置代理)
            └─> Passive Scan Proxy (127.0.0.1:4201)
                 ├─> ScanPipeline.process_request()
                 │    └─> PluginEngine.scan_request()
                 │         └─> 所有启用的插件检测
                 │
                 ├─> 转发请求到目标网站
                 │
                 ├─> 接收响应
                 │
                 ├─> ScanPipeline.process_response()
                 │    └─> PluginEngine.scan_response()
                 │         └─> 所有启用的插件检测
                 │              └─> 发现漏洞 Finding
                 │                   └─> FindingDeduplicator (去重)
                 │                        └─> PassiveDatabaseService.insert_vulnerability()
                 │                             └─> passive_vulnerabilities 表
                 │                                  └─> VulnerabilitiesPanel 显示 ✓
                 │
                 └─> 响应返回给AI
```

## 🎯 实际使用示例

### 完整测试流程

```javascript
// 1. 启动被动扫描
start_passive_scan()

// 2. 测试SQL注入 (使用代理)
http_request({
  url: "http://testphp.vulnweb.com/listproducts.php?cat=1' OR '1'='1",
  method: "GET",
  use_passive_proxy: true
})

// 3. 测试POST表单 (使用代理)
http_request({
  url: "http://testphp.vulnweb.com/login.php",
  method: "POST",
  headers: {"Content-Type": "application/x-www-form-urlencoded"},
  body: "username=admin' OR '1'='1&password=test",
  use_passive_proxy: true
})

// 4. 查看检测到的漏洞
list_findings()

// 5. 停止被动扫描
stop_passive_scan()
```

### 预期结果

执行上述流程后：

1. **日志显示**（`sentinel-ai.log`）：
   ```
   INFO Executing HTTP GET request to: ... (via passive proxy)
   INFO Plugin sqli_detector found 1 issues in request ...
   INFO New finding inserted to DB: SQL Injection - high
   ```

2. **数据库记录**：
   ```sql
   SELECT title, severity, vuln_type FROM passive_vulnerabilities;
   -- SQL Injection Detected | high | sqli
   ```

3. **前端显示**：
   - VulnerabilitiesPanel 显示 1 条漏洞
   - 严重程度：高危
   - 类型：SQL注入
   - 包含完整的请求/响应证据

## 🔍 验证方法

### 检查代理是否生效

1. **查看日志**：
   ```bash
   tail -f ~/Library/Application\ Support/sentinel-ai/logs/sentinel-ai.log
   ```

2. **确认关键日志**：
   - ✅ `Executing HTTP GET request to: ... (via passive proxy)` - 使用了代理
   - ❌ `Executing HTTP GET request to: ...` - 没有使用代理

3. **确认插件检测**：
   - ✅ `Plugin xxx found N issues` - 插件检测成功
   - ✅ `New finding inserted to DB` - 漏洞保存成功

### 数据库验证

```bash
cd ~/Library/Application\ Support/sentinel-ai
sqlite3 database.db

-- 查看所有漏洞
SELECT 
    id, 
    title, 
    severity, 
    vuln_type, 
    plugin_id, 
    hit_count,
    first_seen_at 
FROM passive_vulnerabilities 
ORDER BY first_seen_at DESC 
LIMIT 10;

-- 查看证据
SELECT 
    v.title,
    e.url,
    e.method,
    substr(e.evidence_snippet, 1, 100) as evidence_preview
FROM passive_vulnerabilities v
JOIN passive_evidence e ON v.id = e.vuln_id
ORDER BY v.first_seen_at DESC
LIMIT 5;
```

## 🚨 常见问题

### Q1: 为什么 `use_passive_proxy` 默认是 false？

**A**: 为了向后兼容和避免意外流量被代理。只有在明确进行安全测试时才应该使用代理。

### Q2: 可以强制所有 `http_request` 都使用代理吗？

**A**: 可以，但不推荐。更好的方式是在 AI 提示词中强调必须使用这个参数。

### Q3: Playwright 浏览器需要设置这个参数吗？

**A**: 不需要。Playwright MCP 已经自动配置了代理，所有浏览器流量都会经过被动扫描代理。

### Q4: 如果代理没启动，设置了 `use_passive_proxy: true` 会怎样？

**A**: 请求会失败，返回连接错误。这实际上是好事，能够明确告知AI需要先启动代理。

### Q5: 为什么有些漏洞需要多次请求才能检测？

**A**: 
- 某些漏洞需要对比多个请求的响应（如时间盲注）
- 去重机制会忽略重复的漏洞
- 插件可能需要收集足够的证据才会报告

## 📝 修改的文件

1. **src-tauri/sentinel-tools/src/builtin/http_request.rs**
   - 添加 `use_passive_proxy` 参数定义
   - 在 `HttpRequestConfig` 中添加字段
   - 在 `execute_request` 中配置代理

2. **src-tauri/src/prompts/automated_security_testing.md**
   - 添加关键警告说明
   - 增加 Option B: HTTP Request-Based Testing
   - 强调必须设置 `use_passive_proxy: true`

3. **docs/PASSIVE_SCAN_ISSUE_ANALYSIS.md**
   - 问题根因分析文档

4. **docs/HTTP_REQUEST_PROXY_SUPPORT.md** (本文档)
   - 功能说明和使用指南

## 🎓 最佳实践

1. **测试前必做**：
   - 先执行 `start_passive_scan()`
   - 等待代理启动成功

2. **使用 http_request 测试**：
   - 总是添加 `use_passive_proxy: true`
   - 检查日志确认使用了代理

3. **使用浏览器测试**：
   - 优先使用 `playwright_navigate`
   - 浏览器自动使用代理

4. **测试后检查**：
   - 执行 `list_findings()` 查看漏洞
   - 查看 VulnerabilitiesPanel
   - 必要时查询数据库确认

5. **清理工作**：
   - 测试完成后执行 `stop_passive_scan()`
   - 避免代理长期运行

## 🔗 相关文档

- 被动扫描架构：`src-tauri/sentinel-passive/README.md`
- 插件开发指南：`src-tauri/sentinel-plugins/README.md`
- AI测试工作流：`src-tauri/src/prompts/automated_security_testing.md`
- 问题分析报告：`docs/PASSIVE_SCAN_ISSUE_ANALYSIS.md`

---

**日期**: 2025-11-14  
**版本**: 1.0.0  
**作者**: AI Assistant

