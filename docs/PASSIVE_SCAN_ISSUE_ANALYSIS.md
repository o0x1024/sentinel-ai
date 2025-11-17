# 被动扫描漏洞未保存问题分析

## 📋 问题描述

用户报告：AI助手测试完安全漏洞后，漏洞没有出现在 `VulnerabilitiesPanel.vue` 的漏洞列表中。

## 🔍 根本原因

通过分析日志 `sentinel-ai.log.2025-11-14`，发现：

### 1. AI助手使用了错误的测试方法

**日志证据**（行 405-443）：
```log
INFO sentinel_tools::builtin::http_request: Executing HTTP GET request to: http://testphp.vulnweb.com/listproducts.php?cat=1' OR '1'='1
INFO sentinel_tools::builtin::http_request: Executing HTTP POST request to: http://testphp.vulnweb.com/login.php
INFO sentinel_tools::builtin::http_request: Executing HTTP POST request to: http://testphp.vulnweb.com/guestbook.php
INFO sentinel_tools::builtin::http_request: Executing HTTP POST request to: http://testphp.vulnweb.com/userinfo.php
```

AI助手直接使用 `http_request` 工具发送请求，**这些请求没有经过被动扫描代理**。

### 2. 被动扫描代理未被使用

- **日志中没有** `start_passive_scan` 的调用记录
- **日志中没有** 代理服务器启动的日志
- **日志中没有** 插件检测漏洞的日志（如 "Plugin XXX found N issues"）

### 3. AI的"发现"只是推测

AI在最终答案中报告了发现的漏洞（SQL注入、XSS等），但这些**不是真正通过被动扫描插件检测到的**，而是：
- AI根据HTTP响应内容做的人工分析
- 通过payload测试得到的推测性结论
- 没有保存到数据库的 `passive_vulnerabilities` 表

### 4. 实际已有9条漏洞记录

日志显示（行 400-402）：
```log
INFO sentinel_ai_lib::commands::passive_scan_commands: Loaded 9 findings with evidence from database
INFO sentinel_ai_lib::commands::passive_scan_commands: Total findings count: 9
```

说明数据库中已经有9条漏洞记录（可能是之前测试留下的），但本次AI测试并没有产生新的漏洞记录。

## 🛠️ 正确的工作流程

根据 `automated_security_testing.md` 提示词，AI应该遵循以下流程：

### Step 1: 启动被动扫描代理

```javascript
// 先检查状态
get_passive_scan_status()

// 如果未运行，则启动
start_passive_scan()
```

### Step 2: 使用代理化浏览器

```javascript
// 使用 playwright_navigate 而不是 http_request
playwright_navigate({
  url: "http://testphp.vulnweb.com",
  browserType: "chromium",
  headless: false
})
```

**关键**：Playwright浏览器的流量会自动经过被动扫描代理，被动扫描插件才能检测。

### Step 3: 浏览器交互测试

```javascript
// 填写表单
playwright_fill({ selector: "input[name='username']", value: "test' OR '1'='1" })
playwright_click({ selector: "button[type='submit']" })
```

这样所有的请求和响应都会经过：
1. 被动扫描代理（ProxyService）
2. 扫描管道（ScanPipeline）
3. 插件引擎（PluginEngine）
4. 去重服务（FindingDeduplicator）
5. 数据库存储（PassiveDatabaseService）

## 📊 数据流对比

### ❌ 当前（错误）流程

```
AI助手
  └─> http_request 工具
       └─> 直接发送HTTP请求到目标网站
            └─> 返回响应给AI
                 └─> AI手动分析 (不保存到数据库)
```

### ✅ 正确流程

```
AI助手
  └─> start_passive_scan (启动代理)
       └─> playwright_navigate (打开浏览器)
            └─> 浏览器流量 → 被动扫描代理 (port 4201)
                 └─> ScanPipeline → PluginEngine
                      └─> 插件检测漏洞
                           └─> Finding → FindingDeduplicator
                                └─> PassiveDatabaseService.insert_vulnerability()
                                     └─> passive_vulnerabilities 表
                                          └─> VulnerabilitiesPanel 可见 ✓
```

## 🔧 解决方案

### 方案1: 改进AI提示词（推荐）

修改 `src-tauri/src/prompts/automated_security_testing.md`，强调：

```markdown
⚠️ CRITICAL: You MUST use passive scanning for all vulnerability testing

DO NOT use http_request tool for security testing.
ALWAYS use this workflow:
1. start_passive_scan()
2. playwright_navigate({url: [target]})
3. Use playwright_* tools for all interactions
4. list_findings() to get detected vulnerabilities
5. stop_passive_scan()

Using http_request bypasses the passive scanning proxy and plugins will NOT detect vulnerabilities!
```

### 方案2: 增强 http_request 工具支持代理

修改 `src-tauri/sentinel-tools/src/builtin/http_request.rs`，添加：

```rust
pub struct HttpRequestParams {
    pub url: String,
    pub method: Option<String>,
    pub headers: Option<HashMap<String, String>>,
    pub body: Option<String>,
    pub use_passive_proxy: Option<bool>,  // 新增：使用被动扫描代理
    pub timeout: Option<u64>,
}

// 在execute中：
if params.use_passive_proxy.unwrap_or(false) {
    // 配置代理
    client = client.proxy("http://127.0.0.1:4201")?;
}
```

### 方案3: 自动化集成检查

在AI执行开始前，自动检查：

```rust
// 在 ReAct executor 中
if query.contains("安全测试") || query.contains("漏洞扫描") {
    // 自动启动被动扫描
    if !passive_scan_running {
        warn!("Security testing detected but passive scan not running!");
        // 建议或自动启动
    }
}
```

## 📝 立即操作建议

告诉用户：

1. **当前漏洞列表查询**：
   ```bash
   刷新 VulnerabilitiesPanel 页面
   应该能看到之前的9条漏洞记录
   ```

2. **正确的测试命令**：
   ```
   请对 http://testphp.vulnweb.com 进行被动扫描测试：
   1. 先启动被动扫描代理
   2. 使用 playwright_navigate 打开浏览器
   3. 在浏览器中手动或自动测试各个功能
   4. 查看漏洞列表获取检测结果
   5. 完成后停止被动扫描
   ```

3. **验证流程**：
   ```bash
   # 查看日志中是否有：
   - "Proxy server started on port 4201"
   - "Plugin XXX found N issues"
   - "New finding inserted to DB"
   - "Inserting vulnerability: title='...'"
   ```

## 🎯 预期改进效果

修复后：
- ✅ AI测试产生的HTTP流量会经过被动扫描代理
- ✅ 被动扫描插件能够检测流量中的漏洞
- ✅ 检测到的漏洞会自动保存到数据库
- ✅ VulnerabilitiesPanel 会实时显示新发现的漏洞
- ✅ 漏洞记录包含完整的请求/响应证据

## 📚 相关文件

- 提示词：`src-tauri/src/prompts/automated_security_testing.md`
- 被动扫描命令：`src-tauri/src/commands/passive_scan_commands.rs`
- 扫描管道：`src-tauri/sentinel-passive/src/scanner.rs`
- 数据库操作：`src-tauri/sentinel-passive/src/database.rs`
- 前端UI：`src/components/SecurityCenter/VulnerabilitiesPanel.vue`

## 🔗 数据库查询

如果需要手动验证数据库：

```sql
-- 查看所有漏洞
SELECT id, title, severity, vuln_type, plugin_id, first_seen_at, hit_count 
FROM passive_vulnerabilities 
ORDER BY first_seen_at DESC;

-- 查看证据
SELECT v.title, e.url, e.method, e.evidence_snippet 
FROM passive_vulnerabilities v 
LEFT JOIN passive_evidence e ON v.id = e.vuln_id 
ORDER BY v.first_seen_at DESC;
```

数据库路径：`~/Library/Application Support/sentinel-ai/database.db`

---

**结论**：问题不在数据库或UI，而是AI助手的测试方法不正确，绕过了被动扫描系统。需要修正AI的工作流程以正确使用被动扫描功能。

