# AI驱动的被动扫描自动化实现计划

## 📋 需求概述

实现一个AI驱动的自动化安全测试场景：用户向AI助手说："打开被动扫描，然后测试一下 https://zeus.imgo.tv/ 网站是否存在安全风险（越权漏洞、SQL注入、XSS漏洞、信息泄露等等）"

AI助手应该能够：
1. ✅ 自动打开被动扫描代理
2. ✅ 选择合适的空闲端口进行监听
3. 🔄 自动打开浏览器并配置代理
4. 🔄 自动访问目标网站并进行交互测试
5. 🔄 根据网站特征自动生成检测插件
6. 🔄 动态加载插件到被动扫描系统
7. ✅ 实时显示检测过程和安全风险

> ✅ 已实现 | 🔄 需要实现 | ❌ 暂不支持

---

## 🏗️ 当前架构分析

### 已实现的核心组件

#### 1. 被动扫描系统 (Phase 1-6 已完成)
- ✅ **代理核心**: Hudsucker HTTP/HTTPS MITM 代理
- ✅ **插件引擎**: Deno Core，支持 JS/TS 插件动态加载
- ✅ **内置插件**: SQL注入、XSS、敏感信息检测
- ✅ **数据库存储**: SQLite 持久化漏洞发现
- ✅ **实时事件**: Tauri 事件系统推送漏洞和统计信息
- ✅ **MCP工具集成**: passive.list_findings, passive.<plugin_id>

#### 2. AI助手系统
- ✅ **多架构支持**: ReAct, Plan-Execute, ReWoo, LLM-Compiler
- ✅ **工具调用**: 统一工具管理器，支持 MCP 协议
- ✅ **对话管理**: 会话持久化，上下文管理
- ✅ **流式响应**: 实时显示 AI 推理过程

#### 3. 前端 UI
- ✅ **PassiveScanControl**: 代理控制、插件管理
- ✅ **VulnerabilityDashboard**: 漏洞列表、实时发现流
- ✅ **AIChat**: AI对话界面，工具调用结果展示

### 缺失的关键组件

#### 1. 浏览器自动化工具 ❌
- 当前没有集成 Playwright 或 Selenium
- AI助手无法直接控制浏览器

#### 2. 动态插件生成 ❌
- 插件需要手动编写和加载
- AI助手无法根据网站特征生成插件

#### 3. 代理配置自动化 ⚠️
- 浏览器需要手动配置代理
- 没有自动配置代理的工具

---

## 🎯 实现方案

### 方案 A: 完整自动化（推荐）

**实现步骤**：

#### Step 1: 实现浏览器自动化工具 (2-3天)

创建 Playwright 集成，暴露为 MCP 工具：

```rust
// src-tauri/sentinel-tools/src/web_automation/
pub struct BrowserAutomationTool {
    browser: Arc<Mutex<Option<Browser>>>,
}

// MCP 工具接口
Tools:
- browser.launch(proxy_port, headless) -> browser_id
- browser.navigate(browser_id, url) -> page_id
- browser.click(page_id, selector)
- browser.fill(page_id, selector, value)
- browser.wait(page_id, timeout)
- browser.screenshot(page_id) -> path
- browser.close(browser_id)
```

**技术细节**：
- 使用 `playwright` crate (或通过 Node.js 子进程调用)
- 自动配置浏览器代理为被动扫描端口
- 支持 headless 和有头模式
- 捕获网络请求日志（用于生成插件）

#### Step 2: 实现被动扫描控制工具 (1天)

将被动扫描操作暴露为 MCP 工具：

```rust
// src-tauri/src/tools/passive_control.rs
Tools:
- passive_scan.start(config) -> { port, running }
- passive_scan.stop() -> { success }
- passive_scan.status() -> { running, port, stats }
- passive_scan.get_findings(filters) -> { findings[], count }
```

**当前状态**: 
- ✅ Tauri 命令已实现 (start/stop/get_proxy_status)
- 🔄 需要包装为 MCP 工具并注册到工具系统

#### Step 3: 实现AI驱动的插件生成 (3-4天)

创建插件生成工具，使用 LLM 根据网站特征生成检测代码：

```rust
// src-tauri/src/tools/plugin_generator.rs
Tool: plugin.generate_from_site(url, site_info, vuln_types)

Workflow:
1. 分析网站结构（API端点、参数、技术栈）
2. 识别常见漏洞模式（如: userId 参数可能存在越权）
3. 使用 LLM 生成插件代码（TypeScript）
4. 验证插件语法和逻辑
5. 保存到数据库并加载到插件引擎
```

**Prompt 模板**：
```
你是一个安全测试插件开发专家。根据以下网站信息生成 Sentinel AI 被动扫描插件：

网站: {url}
API端点: {endpoints}
参数: {params}
技术栈: {tech_stack}
目标漏洞: {vuln_types}

请生成符合以下模板的 TypeScript 插件代码:
- 实现 get_metadata() 函数
- 实现 scan_request() 函数
- 实现 scan_response() 函数
- 使用 Sentinel.emitFinding() 报告漏洞

示例模板: {template}
```

#### Step 4: 实现自动化测试工作流工具 (2天)

创建高级工具，组合上述工具实现完整工作流：

```rust
// src-tauri/src/tools/auto_pentest.rs
Tool: security.auto_test_website(url, vuln_types, depth)

Workflow:
1. 启动被动扫描 (passive_scan.start)
2. 启动浏览器 (browser.launch)
3. 抓取网站结构 (crawl API endpoints)
4. 生成针对性插件 (plugin.generate_from_site)
5. 加载插件到扫描器 (passive_scan.load_plugin)
6. 自动化交互测试 (browser.navigate, click, fill)
7. 收集漏洞报告 (passive_scan.get_findings)
8. 生成测试报告
9. 清理资源 (browser.close, passive_scan.stop)
```

#### Step 5: 实现实时进度UI (1-2天)

增强前端实时显示能力：

```vue
<!-- src/components/AutoTestMonitor.vue -->
<template>
  <div class="auto-test-monitor">
    <!-- 测试进度条 -->
    <progress-bar :value="progress" :stages="stages" />
    
    <!-- 实时日志流 -->
    <log-stream :logs="testLogs" />
    
    <!-- 实时漏洞流 -->
    <finding-stream :findings="realtimeFindings" />
    
    <!-- 网络拓扑图 -->
    <network-graph :endpoints="discoveredEndpoints" />
    
    <!-- 插件生成状态 -->
    <plugin-generation-status :plugins="generatedPlugins" />
  </div>
</template>
```

**监听事件**：
- `auto-test:stage-changed` - 阶段变更
- `auto-test:log` - 日志输出
- `auto-test:endpoint-discovered` - 发现新端点
- `auto-test:plugin-generated` - 插件生成完成
- `scan:finding` - 发现新漏洞（已有）

---

### 方案 B: 简化方案（快速实现）

**实现步骤**：

#### Step 1: 手动配置浏览器 (0天)
- 用户手动配置浏览器代理为 `127.0.0.1:4201`
- 用户手动访问目标网站

#### Step 2: AI助手调用现有工具 (1天)
- AI助手通过 `passive_scan.start` 启动代理
- AI助手提示用户配置浏览器代理
- AI助手通过 `passive_scan.get_findings` 查询结果

#### Step 3: 基于模板的插件生成 (1-2天)
- AI助手分析用户描述和已发现的漏洞
- 使用预定义模板生成插件（无需深度代码生成）
- 通过 LLM 填充模板参数（如检测规则、参数名）

**优点**：
- 实现快速，风险低
- 不依赖浏览器自动化库

**缺点**：
- 用户体验不够流畅
- 需要手动操作多个步骤

---

## 🔄 实时动态显示方案

### 当前已实现的事件系统

```rust
// src-tauri/src/events/passive_scan_events.rs
已有事件:
- proxy:status { running, port, mitm, stats }
- scan:finding { vuln_id, vuln_type, severity, url, ... }
- scan:stats { requests, responses, qps, findings }
- plugin:changed { plugin_id, enabled, name }
```

### 需要新增的事件

```rust
// 自动化测试事件
Events:
- auto-test:started { url, vuln_types, test_id }
- auto-test:stage { stage, progress, message }
- auto-test:endpoint-discovered { endpoint, method, params }
- auto-test:plugin-generated { plugin_id, name, code_snippet }
- auto-test:browser-action { action, target, status }
- auto-test:completed { test_id, duration, findings_count }
- auto-test:error { stage, error_message }
```

### 前端UI实现

#### 选项 1: 在 AIChat 中内嵌进度卡片

```vue
<!-- 在 AI 消息流中插入实时卡片 -->
<div class="ai-message-content">
  <p>正在启动自动化测试...</p>
  
  <AutoTestProgressCard 
    v-if="currentAutoTest"
    :test-id="currentAutoTest.id"
    :stages="currentAutoTest.stages"
    :findings="currentAutoTest.findings"
    @cancel="cancelAutoTest"
  />
</div>
```

#### 选项 2: 独立的测试监控弹窗

```vue
<!-- 全局弹窗，悬浮在页面右下角 -->
<AutoTestMonitorFloater 
  v-if="activeAutoTest"
  :test-id="activeAutoTest.id"
  :minimizable="true"
  @close="closeMonitor"
/>
```

#### 选项 3: 在 PassiveScan 页面新增自动化测试 Tab

```vue
<!-- src/views/PassiveScan.vue -->
<div class="tabs tabs-boxed">
  <a class="tab">代理控制</a>
  <a class="tab">漏洞看板</a>
  <a class="tab">插件管理</a>
  <a class="tab tab-active">自动化测试</a> <!-- 新增 -->
</div>

<AutoTestManager v-if="currentTab === 'auto-test'" />
```

**推荐**: 选项 1 + 选项 3 组合
- AI对话中显示简化进度卡片（便于快速启动）
- PassiveScan 页面中提供完整的测试管理界面

---

## 📊 可行性评估

### 核心功能可行性

| 功能模块 | 可行性 | 工作量 | 依赖 | 风险 |
|---------|-------|-------|------|------|
| 被动扫描控制工具 | ✅ 100% | 0.5天 | 已有代码 | 低 |
| 实时事件推送 | ✅ 100% | 1天 | Tauri Events | 低 |
| 浏览器自动化 | ⚠️ 80% | 3天 | Playwright crate | 中 |
| 插件动态加载 | ✅ 100% | 0.5天 | 已有代码 | 低 |
| AI生成插件代码 | ⚠️ 70% | 4天 | LLM质量 | 高 |
| 网站结构分析 | ⚠️ 60% | 3天 | 爬虫逻辑 | 中 |
| 实时UI展示 | ✅ 95% | 2天 | Vue3 | 低 |

### 技术挑战

#### 1. 浏览器自动化集成 (中等难度)

**挑战**：
- Rust 生态中 Playwright 支持不成熟
- 需要通过子进程调用 Node.js Playwright

**解决方案**：
```rust
// 选项A: 使用 fantoccini (Selenium WebDriver)
use fantoccini::ClientBuilder;

// 选项B: 通过 Node.js 子进程调用 Playwright
tokio::process::Command::new("node")
    .arg("playwright-wrapper.js")
    .spawn()?;

// 选项C: 使用 headless_chrome crate (Chrome DevTools Protocol)
use headless_chrome::{Browser, LaunchOptions};
```

**推荐**: 选项A (fantoccini) - Rust 原生，稳定性好

#### 2. AI生成插件代码质量 (高难度)

**挑战**：
- LLM 生成的代码可能有语法错误
- 生成的检测逻辑可能不准确
- 需要验证生成的插件安全性

**解决方案**：
1. **使用结构化 Prompt**
   ```
   输出格式必须是JSON:
   {
     "plugin_id": "...",
     "code": "...",
     "test_cases": [...]
   }
   ```

2. **代码验证流程**
   - 使用 Deno 语法检查器验证代码
   - 运行沙箱测试（模拟请求/响应）
   - 检查是否调用了 Sentinel API

3. **人工审核机制**
   - 生成的插件默认为"待审核"状态
   - UI 显示生成的代码供用户审查
   - 用户确认后才启用

#### 3. 网站结构自动分析 (中等难度)

**挑战**：
- 需要识别 API 端点、参数、认证方式
- SPA 应用需要执行 JavaScript 后才能获取完整结构
- 需要区分静态资源和业务接口

**解决方案**：
```rust
// 网站分析器
struct WebsiteAnalyzer {
    browser: Browser,
    proxy_log: ProxyLogReader,
}

impl WebsiteAnalyzer {
    async fn analyze(&self, url: &str) -> WebsiteInfo {
        // 1. 被动分析：从代理日志读取已访问的请求
        let passive_endpoints = self.proxy_log.get_requests();
        
        // 2. 主动爬取：模拟用户交互
        self.browser.navigate(url).await?;
        self.browser.click_all_links().await?;
        self.browser.fill_forms().await?;
        
        // 3. 分析参数模式
        let params = self.extract_params(&passive_endpoints);
        
        // 4. 识别技术栈
        let tech_stack = self.detect_tech_stack(&passive_endpoints);
        
        WebsiteInfo { endpoints, params, tech_stack }
    }
}
```

---

## 🚀 实施计划

### 阶段 1: MVP（最小可行产品） - 1周

**目标**: 实现基本的自动化工作流

- [ ] **Day 1-2**: 实现被动扫描控制MCP工具
  - 包装现有 Tauri 命令
  - 注册到工具管理器
  - AI助手可调用启动/停止代理

- [ ] **Day 3-4**: 实现简化版浏览器指引
  - AI助手生成代理配置指南
  - 提示用户手动配置浏览器
  - 监控代理流量判断浏览器是否已连接

- [ ] **Day 5-6**: 实现基于模板的插件生成
  - 创建插件代码模板库
  - LLM 根据用户描述选择模板
  - 填充检测规则参数
  - 保存到数据库并加载

- [ ] **Day 7**: 实现实时进度UI
  - 在 AIChat 中显示测试进度卡片
  - 监听事件更新状态
  - 集成测试

**验收标准**:
- ✅ 用户对AI说"测试 xxx 网站"
- ✅ AI自动启动被动扫描
- ✅ AI提示用户配置代理
- ✅ AI根据流量生成插件
- ✅ 实时显示发现的漏洞

### 阶段 2: 浏览器自动化 - 1周

**目标**: 实现完全自动化的浏览器控制

- [ ] **Day 1-2**: 集成 Fantoccini (Selenium WebDriver)
  - 添加依赖和初始化逻辑
  - 实现基础浏览器操作

- [ ] **Day 3-4**: 实现浏览器自动化MCP工具
  - browser.launch / navigate / click / fill
  - 自动配置代理
  - 错误处理和重试

- [ ] **Day 5-6**: 实现智能交互测试
  - 自动识别可点击元素
  - 自动填充表单
  - 模拟登录流程

- [ ] **Day 7**: 集成和测试
  - 与被动扫描联调
  - 端到端测试

### 阶段 3: 高级插件生成 - 1周

**目标**: 实现AI驱动的高质量插件生成

- [ ] **Day 1-2**: 网站结构分析器
  - 从代理日志提取API端点
  - 参数模式识别
  - 技术栈检测

- [ ] **Day 3-4**: 插件代码生成器
  - 设计生成 Prompt
  - 实现代码验证逻辑
  - 插件测试框架

- [ ] **Day 5-6**: 插件质量优化
  - 添加人工审核界面
  - 实现插件评分机制
  - 收集反馈改进生成效果

- [ ] **Day 7**: 完整工作流集成
  - 组合所有工具
  - 优化用户体验

---

## 🎨 用户体验设计

### 对话示例

**用户**: 打开被动扫描，然后测试一下 https://zeus.imgo.tv/ 网站是否存在安全风险（越权漏洞、SQL注入、XSS漏洞、信息泄露等等）

**AI助手**: 
```
好的，我将为您执行自动化安全测试。测试流程如下：

1. 启动被动扫描代理 ⏳
2. 打开浏览器并访问目标网站 ⏳
3. 分析网站结构和API端点 ⏳
4. 生成针对性检测插件 ⏳
5. 自动化交互测试 ⏳
6. 收集和分析漏洞 ⏳

[正在启动被动扫描...]

✅ 被动扫描已启动，监听端口: 4201

[正在启动浏览器...]

✅ Chrome 浏览器已启动（代理已配置）

[正在访问 https://zeus.imgo.tv/...]

✅ 页面加载完成
📊 发现 15 个API端点
📊 识别到 32 个参数

[正在生成检测插件...]

✅ 已生成插件: zeus_sqli_detector.ts
✅ 已生成插件: zeus_xss_detector.ts
✅ 已生成插件: zeus_auth_bypass_detector.ts
✅ 插件已加载到扫描引擎

[正在执行自动化测试...]

🔍 测试场景 1/8: 用户登录流程
🔍 测试场景 2/8: 用户资料查看
⚠️ 发现漏洞: 用户ID参数存在越权风险 (HIGH)
🔍 测试场景 3/8: 搜索功能
⚠️ 发现漏洞: 搜索参数可能存在SQL注入 (CRITICAL)
...

[测试完成]

📋 测试报告摘要:
- 测试时长: 3分42秒
- 请求总数: 127
- 发现漏洞: 8个
  - Critical: 2
  - High: 3
  - Medium: 2
  - Low: 1

🔴 高危漏洞:
1. SQL注入 (搜索参数) - CWE-89
2. 垂直越权 (用户ID参数) - CWE-284
3. 敏感信息泄露 (API响应) - CWE-200

详细报告已生成: /path/to/report.html

需要我对某个漏洞进行深入分析吗？
```

### UI交互流程图

```
┌─────────────────────────────────────────────────────────┐
│  AIChat - 用户输入测试命令                                │
└────────────────┬────────────────────────────────────────┘
                 │
                 ▼
┌─────────────────────────────────────────────────────────┐
│  AI 解析命令 → 生成执行计划                               │
│  - 工具: security.auto_test_website                      │
│  - 参数: url, vuln_types                                 │
└────────────────┬────────────────────────────────────────┘
                 │
                 ▼
┌─────────────────────────────────────────────────────────┐
│  弹出实时测试监控卡片                                      │
│  ┌───────────────────────────────────────────────────┐  │
│  │ 🎯 自动化测试进行中...                  [最小化][X] │  │
│  │                                                     │  │
│  │ 进度: ████████████░░░░░ 65%                        │  │
│  │                                                     │  │
│  │ 当前阶段: 自动化交互测试 (3/5)                      │  │
│  │                                                     │  │
│  │ 📊 统计:                                            │  │
│  │  • 请求: 87    • 发现端点: 15                      │  │
│  │  • 漏洞: 5     • 插件: 3                           │  │
│  │                                                     │  │
│  │ ⚠️ 最新发现:                                        │  │
│  │  • SQL注入 (搜索参数) - CRITICAL                    │  │
│  │  • 越权漏洞 (用户ID) - HIGH                        │  │
│  │                                                     │  │
│  │ [查看详情] [暂停测试]                               │  │
│  └───────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
                 │
                 ▼
┌─────────────────────────────────────────────────────────┐
│  测试完成 → AI 生成报告                                   │
│  - 自动跳转到漏洞看板                                     │
│  - 高亮显示新发现的漏洞                                   │
└─────────────────────────────────────────────────────────┘
```

---

## 🔒 安全考虑

### 1. 生成插件的安全性

**风险**: AI生成的插件可能包含恶意代码

**控制措施**:
- ✅ Deno沙箱环境（已有）
- ✅ 代码静态分析（检测危险API调用）
- ✅ 人工审核机制（生成后需用户确认）
- ✅ 插件权限控制（未来功能）

### 2. 浏览器自动化的安全性

**风险**: 浏览器可能访问恶意网站

**控制措施**:
- ⚠️ 仅在用户明确指定的URL范围内操作
- ⚠️ 使用隔离的浏览器配置文件
- ⚠️ 限制网络访问范围
- ⚠️ 显式用户确认机制

### 3. 数据隐私

**风险**: 自动化测试可能捕获敏感数据

**控制措施**:
- ✅ 数据库加密存储（建议用户启用磁盘加密）
- ✅ 明确的用户提示和授权
- ✅ 提供数据清理功能

---

## 📈 后续优化方向

### 短期（1-2个月）

- [ ] 增加更多内置插件模板
- [ ] 优化插件生成质量（Few-shot learning）
- [ ] 支持认证流程自动化（登录表单识别）
- [ ] 支持 CAPTCHA 绕过提示

### 中期（3-6个月）

- [ ] 集成主动扫描工具（Nuclei/XRAY）
- [ ] 支持分布式测试（多浏览器并发）
- [ ] AI驱动的漏洞验证和利用
- [ ] 自动生成PoC代码

### 长期（6-12个月）

- [ ] 支持移动应用测试（Android/iOS）
- [ ] 集成漏洞数据库（CVE/NVD）
- [ ] AI驱动的安全建议和修复方案
- [ ] 社区插件市场

---

## 🎯 结论

### 核心结论

✅ **当前架构完全支持实现该功能**
- 被动扫描系统完善（已完成 Phase 1-6）
- AI助手系统成熟（多架构、工具调用）
- 实时事件系统完备（Tauri Events）

⚠️ **缺失关键组件需要开发**
1. 浏览器自动化工具（3天）
2. 被动扫描MCP工具包装（0.5天）
3. AI驱动的插件生成（4天）
4. 实时进度UI（2天）

### 推荐实施路径

**阶段 1 MVP（1周）**: 手动浏览器 + 模板插件生成
- 快速验证核心流程
- 最小风险

**阶段 2（1周）**: 完整浏览器自动化
- 提升用户体验
- 需要集成 Selenium/Playwright

**阶段 3（1周）**: 高级AI插件生成
- 提升插件质量
- 优化生成逻辑

### 估计工作量

- **MVP版本**: 1周（40小时）
- **完整版本**: 3周（120小时）
- **高级优化**: 持续迭代

### 技术风险评估

| 风险项 | 等级 | 缓解措施 |
|-------|------|---------|
| 浏览器自动化稳定性 | 中 | 使用成熟的 Selenium 库 |
| AI生成代码质量 | 高 | 人工审核 + 沙箱测试 |
| 性能影响 | 低 | 异步执行 + 资源限制 |
| 安全风险 | 中 | 沙箱隔离 + 权限控制 |

---

## 📚 参考资料

- [Playwright文档](https://playwright.dev/)
- [Fantoccini (Rust Selenium)](https://docs.rs/fantoccini/)
- [Deno Core安全模型](https://deno.land/manual/runtime/permission_apis)
- [OWASP Testing Guide](https://owasp.org/www-project-web-security-testing-guide/)

---

**文档版本**: 1.0  
**创建日期**: 2025-11-12  
**更新日期**: 2025-11-12  
**状态**: 待审核

