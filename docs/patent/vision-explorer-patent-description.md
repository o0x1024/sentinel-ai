# 基于大语言模型的智能网站探索与API自动发现系统

## 发明名称

**一种基于大语言模型的网站自动化探索与API接口发现方法及系统**

Alternative Titles:
- 基于VLM/LLM的智能Web应用全流量发现引擎
- 一种AI驱动的网站功能探索与接口自动抓取系统

---

## 技术领域

本发明涉及人工智能、Web安全测试、自动化测试领域，特别涉及一种利用大语言模型（LLM）和视觉语言模型（VLM）进行智能网站探索、模拟人类操作行为，并自动发现和收集API接口的方法与系统。

---

## 背景技术

### 现有技术的问题

1. **传统爬虫的局限性**：传统网页爬虫基于静态HTML解析，无法处理现代单页应用（SPA）的动态内容加载、JavaScript渲染、AJAX请求等场景。

2. **人工测试效率低**：安全测试人员需要手动点击页面各个功能，操作繁琐且容易遗漏，尤其对于大型复杂应用，人工全覆盖几乎不可能。

3. **现有自动化工具的不足**：
   - Selenium/Playwright等自动化工具需要预先编写脚本，无法适应未知网站结构
   - 基于规则的爬虫无法理解页面语义，难以做出智能决策
   - 缺乏对登录态、验证码等人机交互场景的处理能力

4. **API发现的挑战**：现代Web应用大量使用RESTful API、GraphQL等接口，传统方法难以全面发现这些动态生成的接口请求。

---

## 发明内容

### 技术问题

本发明要解决的技术问题是：如何实现对现代复杂Web应用的全自动化智能探索，最大化发现所有功能页面和API接口，同时能够处理登录认证、动态内容、SPA路由等复杂场景。

### 技术方案

本发明提供一种基于大语言模型的网站自动化探索与API接口发现方法，包括以下核心技术模块：

#### 1. 双模态探索引擎

**1.1 视觉多模态模式（Vision Mode）**
- 利用Playwright等浏览器自动化工具捕获页面截图
- 将截图输入VLM（如Claude、GPT-4V）进行视觉分析
- VLM识别页面元素、理解布局、判断功能区域
- 基于视觉理解做出下一步操作决策

**1.2 文本结构化模式（Text Mode）**
- 提取页面所有可交互元素，生成结构化元素列表
- 元素信息包括：索引号、类型、标签名、文本内容、href、name、placeholder、ARIA属性等
- 输入LLM进行语义分析和决策
- 适用于纯文本模型或需要节省Token的场景

#### 2. 元素标注与索引系统

**2.1 智能元素标注**
```
实现机制：
- 自动扫描DOM树，识别所有可交互元素（链接、按钮、输入框、下拉框等）
- 为每个元素分配唯一索引号
- 生成元素特征描述（CSV格式）
- 支持多模态模式下的可视化标注叠加
```

**2.2 元素指纹算法**
```rust
// 元素指纹生成：selector + type + text(前20字符) + href/name
fn generate_fingerprint(element, page_url) -> String {
    hash(element.selector + element.type + element.text[0:20] + element.href + element.name)
}
```
- 跨页面元素去重
- 交互状态追踪
- 覆盖率计算基础

#### 3. 被动代理集成的API发现

**3.1 代理流量捕获**
- 集成被动代理（如mitmproxy），拦截所有HTTP/HTTPS请求
- 浏览器通过代理访问目标网站
- 实时捕获每次页面交互触发的API请求

**3.2 API端点提取与分类**
```
捕获信息：
- 请求方法（GET/POST/PUT/DELETE等）
- 完整URL路径
- 请求头（Authorization、Content-Type等）
- 请求参数（Query String、JSON Body）
- 响应状态码
- 触发来源（关联的UI操作）
```

**3.3 智能过滤**
- 基于目标域名过滤外部请求
- 去重合并相同接口
- 识别API模式（RESTful路径参数提取）

#### 4. 多维度覆盖率引擎

**4.1 路由覆盖率（Route Coverage）**
```
RouteTracker:
- discovered_routes: 已发现的所有路由
- visited_routes: 已访问的路由
- pending_routes: 待访问队列
- coverage = visited / discovered * 100%
```

**4.2 元素覆盖率（Element Coverage）**
```
ElementManager:
- all_elements: 所有唯一元素（基于指纹）
- interacted_elements: 已交互元素
- coverage = interacted / total * 100%
```

**4.3 稳定性检测**
```
完成条件判断：
1. 待访问路由队列为空
2. 元素覆盖率 ≥ 95%
3. 连续5轮无新发现（路由/元素/API）
```

#### 5. 智能Takeover机制

**5.1 登录页面检测**
- VLM/LLM分析页面内容，识别登录表单
- 检测username/password类型输入框
- 识别验证码、OTP等二次验证

**5.2 人机交互接管**
```
流程：
1. 检测到登录页 → 暂停自动探索
2. 发送前端通知 → 请求用户输入凭据
3. 用户提交凭据 → 自动填充并登录
4. 登录成功 → 恢复自动探索
5. 可选：跳过登录，仅探索公开页面
```

**5.3 凭据安全管理**
- 凭据仅在内存中暂存，不持久化
- 支持动态字段定义（适应不同登录表单）
- 支持额外验证字段（验证码、OTP等）

#### 6. 上下文摘要管理

**6.1 Token估算**
- 实时统计对话历史的Token消耗
- 图片约1000 tokens，文本约4字符/token

**6.2 自动摘要生成**
```
当 estimated_tokens >= threshold (50000):
1. 保留最近10条消息
2. 将历史消息发送给LLM生成摘要
3. 用摘要替换原始历史
4. 重新计算Token
```

#### 7. 分阶段探索策略

```
Phase 0: Recon（态势识别）
- 分析目标网站类型
- 识别技术栈（SPA/传统MPA）
- 判断是否需要登录

Phase 1: Frontend（前台探索）
- 探索所有公开可访问页面
- 收集公开API接口

Phase 2: Login（登录流程）
- 触发Takeover获取凭据
- 完成登录认证

Phase 3: Backend（后台探索）
- 探索登录后的管理/用户页面
- 收集需认证的API接口
```

#### 8. SPA路由追踪

**8.1 路由变化监听**
- 注入JavaScript监听History API（pushState/replaceState）
- 监听hashchange事件（Hash路由）
- 实时捕获前端路由变化

**8.2 路由规范化**
```
处理各种路由格式：
- 完整URL: https://example.com/page → 规范化
- 相对路径: /page → 补全域名
- Hash路由: #/dashboard → 保留hash部分
- Query参数: /page?id=1 → 移除query
```

**8.3 智能过滤**
```
忽略列表：
- logout/signout（登出操作）
- javascript:/mailto:/tel:（伪协议）
- 外部域名链接
```

---

## 系统架构

```
┌─────────────────────────────────────────────────────────────────┐
│                    Vision Explorer Engine                        │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐  │
│  │  BrowserTools   │  │  StateManager   │  │   LLM Client    │  │
│  │  (Playwright)   │  │  (状态追踪)     │  │  (VLM/LLM)      │  │
│  └────────┬────────┘  └────────┬────────┘  └────────┬────────┘  │
│           │                    │                    │            │
│           └────────────────────┼────────────────────┘            │
│                                │                                 │
│  ┌─────────────────────────────▼──────────────────────────────┐ │
│  │                    Exploration Loop                         │ │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐   │ │
│  │  │ 截图/    │→│ VLM/LLM  │→│ 执行     │→│ 更新     │   │ │
│  │  │ 元素列表 │  │ 分析决策 │  │ 浏览器   │  │ 状态     │   │ │
│  │  └──────────┘  └──────────┘  │ 操作     │  └──────────┘   │ │
│  │                              └──────────┘                   │ │
│  └─────────────────────────────────────────────────────────────┘ │
│                                                                   │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐              │
│  │RouteTracker │  │ElementMgr   │  │CoverageEng  │              │
│  │路由追踪     │  │元素管理     │  │覆盖率引擎   │              │
│  └─────────────┘  └─────────────┘  └─────────────┘              │
│                                                                   │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐              │
│  │PassiveProxy │  │TakeoverMgr  │  │ContextMgr   │              │
│  │API抓包      │  │人机接管     │  │上下文摘要   │              │
│  └─────────────┘  └─────────────┘  └─────────────┘              │
└───────────────────────────────────────────────────────────────────┘
```

---

## 技术效果

1. **全自动化探索**：无需编写任何测试脚本，AI自主决策探索路径
2. **高覆盖率**：多维度覆盖率追踪确保不遗漏功能页面和API
3. **智能适应**：能够处理SPA、登录认证、动态内容等复杂场景
4. **实时API发现**：通过代理集成实时捕获所有接口请求
5. **人机协作**：Takeover机制支持人工介入处理验证码等场景
6. **资源优化**：上下文摘要管理避免Token溢出，降低成本
7. **双模态支持**：视觉模式和文本模式灵活切换，适应不同需求

---

## 权利要求（建议）

1. 一种基于大语言模型的网站自动化探索方法，其特征在于包括以下步骤：
   - 启动浏览器访问目标网站
   - 捕获页面截图或提取结构化元素列表
   - 将页面信息输入大语言模型进行分析和决策
   - 执行模型决定的浏览器操作
   - 通过被动代理捕获API请求
   - 循环执行直至覆盖率达标

2. 如权利要求1所述的方法，其特征在于所述元素标注系统包括：
   - 自动扫描DOM树识别可交互元素
   - 为元素分配唯一索引号
   - 生成元素指纹用于去重和追踪

3. 如权利要求1所述的方法，其特征在于所述覆盖率引擎包括：
   - 路由覆盖率计算
   - 元素覆盖率计算
   - 稳定性检测算法

4. 一种实现权利要求1-3任一项方法的系统，其特征在于包括：
   - 浏览器自动化模块
   - 大语言模型推理模块
   - 被动代理模块
   - 状态管理模块
   - 人机交互接管模块

---

## 关键创新点总结

| 创新点 | 描述 |
|--------|------|
| 双模态探索 | 同时支持VLM视觉分析和LLM文本分析两种模式 |
| 元素指纹算法 | 跨页面元素去重和交互追踪 |
| 被动代理集成 | 实时捕获所有HTTP请求发现API |
| 多维覆盖率引擎 | 路由+元素+组件三维度覆盖率追踪 |
| 智能Takeover | 登录检测与人机交互无缝切换 |
| 上下文摘要 | 自动生成摘要避免Token溢出 |
| 分阶段策略 | Recon→Frontend→Login→Backend渐进探索 |
| SPA路由追踪 | Hash路由/History API变化实时监听 |

---

## 附录：核心数据结构

### 探索配置
```rust
struct VisionExplorerConfig {
    target_url: String,           // 目标URL
    max_iterations: u32,          // 最大迭代次数
    enable_multimodal: bool,      // 是否启用多模态（截图）
    enable_passive_proxy: bool,   // 是否启用被动代理
    enable_takeover: bool,        // 是否启用人机接管
    enable_context_summary: bool, // 是否启用上下文摘要
    context_summary_threshold: u32, // 摘要触发阈值
    vlm_provider: String,         // 模型提供商
    vlm_model: String,            // 模型名称
}
```

### API端点信息
```rust
struct ApiEndpoint {
    method: String,               // GET/POST/PUT/DELETE
    path: String,                 // URL路径
    full_url: String,             // 完整URL
    headers: HashMap<String, String>,
    parameters: HashMap<String, String>,
    body: Option<String>,
    status_code: Option<u16>,
    discovered_at: DateTime<Utc>,
    source_action_id: Option<String>, // 触发来源
}
```

### 覆盖率报告
```rust
struct CoverageReport {
    route_coverage: f32,          // 路由覆盖率
    element_coverage: f32,        // 元素覆盖率
    overall_coverage: f32,        // 综合覆盖率
    api_count: usize,             // API发现数量
    routes_discovered: usize,     // 发现路由数
    routes_visited: usize,        // 访问路由数
    elements_total: usize,        // 总元素数
    elements_interacted: usize,   // 已交互元素数
    consecutive_no_discovery: u32, // 连续无新发现轮次
    is_stable_complete: bool,     // 是否稳定完成
}
```

---

*文档生成日期：2025年12月16日*
*版本：1.0*

