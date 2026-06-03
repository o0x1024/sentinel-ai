# Sentinel Browser Automation — 设计文档

> **分支**: `feature/browser-automation`  
> **日期**: 2026-05-26  
> **状态**: Draft

## 1. 目标

在 Sentinel 中构建通用的拟人化浏览器自动化基础设施，支持：
- AI Agent 通过 Tool Call 实时驱动浏览器
- 可复用的自动化脚本编排
- 覆盖通用网页操作、安全测试、数据采集等场景
- 高度拟人化，绕过主流反自动化检测

---

## 2. 行业调研分析

### 2.1 Google Antigravity

**方案概述**：Antigravity 是 Google 的 Agent-first IDE/平台，内置专用浏览器子代理（Browser Subagent）。

**关键设计**：
- 主 Agent 与 Browser Subagent 分离：主 Agent 决策"做什么"，浏览器子代理（专用模型）负责"怎么操作页面"
- 浏览器子代理工具集：click、scroll、type、read console logs、DOM capture、screenshot、markdown parsing、recording video
- 页面理解方式：DOM capture + screenshot + markdown parsing 三种模式
- 安全控制：Browser URL Allowlist/Denylist、JavaScript 执行权限审批、Strict Mode

**可借鉴点**：
- ✅ Browser Subagent 架构分离（主 Agent 不需要理解底层 CDP 命令）
- ✅ 多模态页面理解（DOM + screenshot + markdown）
- ✅ 安全策略分层（URL 白名单、JS 执行审批）
- ❌ 无反检测能力（Google 自有生态不需要绕过反爬）
- ❌ 云端托管，不适用于本地化安全工具

### 2.2 Camoufox + Juggler Protocol

**方案概述**：Camoufox 是 Firefox fork，在 C++ 层面进行指纹欺骗，使用 Juggler 协议（非 CDP）进行自动化控制。

**关键发现 — CDP vs Juggler**：

| 维度 | Chrome CDP | Firefox Juggler |
|------|-----------|-----------------|
| 协议暴露 | 页面内可检测 `navigator.webdriver`、`__playwright__binding__` 等 | 完全隔离于页面之外 |
| JS 注入 | 在页面主世界执行，可被 getter 劫持检测 | 在隔离沙箱中执行，页面不可见 |
| 输入事件 | 通过 CDP `Input.dispatchMouseEvent`，某些网站可区分 | 通过 Firefox 原生用户输入处理器，与真人输入无法区分 |
| Headless 检测 | 多种特征可识别 | 已 patch 为与有窗口模式一致 |
| WAF 关联度 | Chrome + 自动化 = 高度关联 | Firefox 较少被关联自动化 |

**2026 反检测基准测试结论**：
> "The matrix driver is automation-protocol fingerprinting, not cipher lists."
> 自动化协议指纹是检测主因，而非 TLS/密码套件。CDP 方案在协议层就已暴露。

**Camoufox 反检测技术栈**：
- C++ 层指纹注入（navigator、screen、WebGL、AudioContext、WebRTC IP）
- Font 指纹欺骗
- Juggler 隔离沙箱（页面无法检测自动化存在）
- 输入走 Firefox 原生处理器
- Headless 模式 patch

**可借鉴点**：
- ✅ **Juggler > CDP 用于反检测场景**（核心发现）
- ✅ C++ 层指纹欺骗比 JS shimming 稳健
- ✅ 输入通过原生处理器（最高拟人度）
- ⚠️ Camoufox 维护周期有 gap，依赖 Firefox 版本更新

### 2.3 Rotunda (MonkeySee-AI)

**方案概述**：Agent-first 浏览器，基于 Camoufox/Firefox，为 AI Agent 专门设计。

**关键设计**：
- 基于 daijro 的 Firefox patching 工作
- 强调"给 Agent 一个和你日常使用一模一样的浏览器"
- CLI 驱动：`uvx rotunda agent ...`
- Browser profiles 持久化（`~/.rotunda`）
- Trace 录制（非视频，包含 network + DOM + console + screenshots）

**可借鉴点**：
- ✅ Profile 持久化设计（登录态/Cookie 保持）
- ✅ Trace 录制用于调试和回放
- ❌ Python 生态为主，不直接适用于 Rust

### 2.4 Fantoma

**方案概述**：通过 Accessibility API（ARIA tree）驱动浏览器，与屏幕阅读器使用相同通道。

**关键设计**：
- **无鼠标移动、无截图、无像素坐标** — 纯通过可访问性树交互
- 元素通过 ARIA role/name/state 定位
- Tree diffing：标记新元素用 `*` 前缀（受 WebVoyager 启发）
- 支持 Camoufox + Patchright 双后端

**核心洞察**：
> Accessibility Tree 是 AI Agent 理解页面的最佳接口 — Token 消耗比原始 HTML 小 ~90%

**可借鉴点**：
- ✅ **Accessibility Tree 作为页面感知层**（关键借鉴）
- ✅ Token-efficient 的页面表示
- ✅ Tree diffing 减少重复信息
- ❌ 纯 ARIA 模式某些交互场景不足（需要 fallback 到坐标操作）

### 2.5 agentic-stealth-browser

**方案概述**：生产级隐身浏览器，存活于 Cloudflare/LinkedIn/Amazon 反 bot 系统。

**关键设计**：
- TLS 指纹：JA3/JA4 区域 profile
- 人类行为模拟：Mouse wobble、typing mistakes、fatigue、distraction
- 自动恢复：检测封锁 → 代理/会话轮换 → 重试
- 账号预热：14 天渐进式使用
- Per-domain 行为 profile + FeedbackStore 遥测
- Plugin 系统 + Operator Dashboard

**可借鉴点**：
- ✅ Per-domain adaptive behavior profiles
- ✅ 自动恢复与重试策略
- ✅ "疲劳"和"分心"等高级拟人特征
- ✅ Operator Dashboard 用于人工干预

---

## 3. 修订后的架构决策

基于调研，对方案 B（Rust 直连 CDP）进行重要修订：

### 3.1 双后端支持（CDP + Juggler）

```
┌────────────────────────────────────────────────────────┐
│                 Unified Browser API                      │
│    (相同的 Tool 接口，上层不感知协议差异)                  │
├────────────────────────────────────────────────────────┤
│         Protocol Adapter Layer                          │
│  ┌──────────────┐       ┌──────────────────┐           │
│  │ CDP Backend  │       │ Juggler Backend  │           │
│  │ (Chrome)     │       │ (Camoufox/FF)    │           │
│  └──────────────┘       └──────────────────┘           │
└────────────────────────────────────────────────────────┘
```

**策略**：
- **默认后端**：Chrome CDP — 最成熟、Rust 生态最好（chromiumoxide crate）、开发调试方便
- **高隐身后端**：Camoufox/Juggler — 反检测场景切换使用
- **Protocol Adapter** 抽象层确保上层 Tools 代码不感知底层协议

### 3.2 页面感知：Accessibility Tree 优先

```
页面理解层级（由上到下递进）：
1. Accessibility Tree（主要）— token 高效、结构化、AI 友好
2. Screenshot + OCR（辅助）— 处理 Canvas/图片内容
3. DOM Snapshot（fallback）— 复杂交互需要完整 DOM 时
```

### 3.3 拟人化层级

```
Level 0: Raw（无拟人化）— 调试/开发模式
Level 1: Basic（基础延迟 + 随机化）— 一般自动化
Level 2: Human（Bezier轨迹 + 键盘节奏 + 滚动物理）— 通用反检测
Level 3: Stealth（Level 2 + 指纹管理 + 行为 Profile + 自动恢复）— 高对抗场景
```

---

## 4. 最终架构设计

```
┌─────────────────────────────────────────────────────────────────┐
│                        Sentinel AI Agent                         │
│  LLM Tool Call Loop / Workflow Script / Plugin Runtime           │
├─────────────────────────────────────────────────────────────────┤
│                     Browser Tool Layer                            │
│  browser_navigate | browser_click | browser_type | browser_eval  │
│  browser_screenshot | browser_wait | browser_scroll              │
│  browser_page_state | browser_network | browser_tabs             │
├─────────────────────────────────────────────────────────────────┤
│                  Page Understanding Layer                         │
│  Accessibility Tree Parser | Tree Diffing | Element Resolver     │
│  Screenshot + OCR | DOM Snapshot | Markdown Extractor            │
├─────────────────────────────────────────────────────────────────┤
│                   Humanization Engine                             │
│  Mouse: Bezier + Fitts' Law + Overshoot                          │
│  Keyboard: Bigram timing + Typos + Think pauses                  │
│  Scroll: Physics sim + Reading pauses                            │
│  Timing: Log-normal delays + Fatigue + Distraction               │
│  Profile: Casual / Expert / Cautious / Custom                    │
├─────────────────────────────────────────────────────────────────┤
│                Protocol Adapter Layer                             │
│  ┌───────────────────┐     ┌────────────────────────┐           │
│  │   CDP Adapter     │     │   Juggler Adapter      │           │
│  │   (Chrome/Edge)   │     │   (Camoufox/Firefox)   │           │
│  │   Port: 9222      │     │   Pipe: fd 3/4         │           │
│  └───────────────────┘     └────────────────────────┘           │
├─────────────────────────────────────────────────────────────────┤
│               Browser Lifecycle Manager                           │
│  Launch | Discover | Connect | Profile Mgmt | Process Watchdog   │
├─────────────────────────────────────────────────────────────────┤
│          Chrome Extension (Optional Enhancement)                  │
│  Real-time A11y Tree | Op Status Overlay | Manual Takeover       │
└─────────────────────────────────────────────────────────────────┘
```

---

## 5. 模块详细设计

### 5.1 目录结构

```
sentinel-browser/
├── Cargo.toml
├── src/
│   ├── lib.rs                  # Crate 入口，pub API
│   ├── cdp/
│   │   ├── mod.rs
│   │   ├── connection.rs       # CDP WebSocket 连接
│   │   ├── session.rs          # Target/Tab 会话
│   │   ├── commands.rs         # CDP 命令类型封装
│   │   └── events.rs           # CDP 事件流处理
│   ├── juggler/
│   │   ├── mod.rs
│   │   ├── pipe.rs             # Juggler pipe (fd3/fd4) 通信
│   │   ├── session.rs          # Juggler 会话管理
│   │   └── commands.rs         # Juggler 命令封装
│   ├── adapter/
│   │   ├── mod.rs
│   │   ├── traits.rs           # BrowserBackend trait 定义
│   │   ├── cdp_adapter.rs      # CDP 实现
│   │   └── juggler_adapter.rs  # Juggler 实现
│   ├── humanize/
│   │   ├── mod.rs
│   │   ├── mouse.rs            # Bezier 轨迹 + Fitts' Law
│   │   ├── keyboard.rs         # Bigram timing + Typos
│   │   ├── scroll.rs           # 物理滚动模拟
│   │   ├── timing.rs           # 全局节奏控制
│   │   └── profile.rs          # 行为风格配置
│   ├── page/
│   │   ├── mod.rs
│   │   ├── accessibility.rs    # A11y Tree 解析与 diffing
│   │   ├── element.rs          # 元素定位与引用
│   │   ├── snapshot.rs         # 页面快照（多模态）
│   │   └── wait.rs             # 智能等待策略
│   ├── lifecycle/
│   │   ├── mod.rs
│   │   ├── launcher.rs         # 浏览器进程启动
│   │   ├── discovery.rs        # 已运行实例发现
│   │   └── profile_mgr.rs      # 用户 Profile 管理
│   └── automation/
│       ├── mod.rs
│       ├── actions.rs          # 高级动作封装
│       └── script.rs           # 脚本执行器
```

### 5.2 Protocol Adapter Trait

```rust
/// 统一的浏览器后端 trait — 屏蔽 CDP/Juggler 差异
#[async_trait]
pub trait BrowserBackend: Send + Sync {
    // --- Lifecycle ---
    async fn connect(&mut self, endpoint: &str) -> Result<()>;
    async fn disconnect(&mut self) -> Result<()>;

    // --- Navigation ---
    async fn navigate(&self, tab: &TabId, url: &str) -> Result<NavigationResult>;
    async fn reload(&self, tab: &TabId) -> Result<()>;
    async fn go_back(&self, tab: &TabId) -> Result<()>;
    async fn go_forward(&self, tab: &TabId) -> Result<()>;

    // --- Input (底层，拟人化引擎调用) ---
    async fn dispatch_mouse_event(&self, tab: &TabId, event: MouseEvent) -> Result<()>;
    async fn dispatch_key_event(&self, tab: &TabId, event: KeyEvent) -> Result<()>;
    async fn dispatch_scroll_event(&self, tab: &TabId, event: ScrollEvent) -> Result<()>;

    // --- Page State ---
    async fn get_accessibility_tree(&self, tab: &TabId) -> Result<AccessibilityTree>;
    async fn get_dom_snapshot(&self, tab: &TabId) -> Result<DomSnapshot>;
    async fn capture_screenshot(&self, tab: &TabId, opts: ScreenshotOpts) -> Result<Vec<u8>>;
    async fn evaluate_js(&self, tab: &TabId, expression: &str) -> Result<JsValue>;

    // --- Tabs ---
    async fn create_tab(&self, url: Option<&str>) -> Result<TabId>;
    async fn close_tab(&self, tab: &TabId) -> Result<()>;
    async fn list_tabs(&self) -> Result<Vec<TabInfo>>;
    async fn activate_tab(&self, tab: &TabId) -> Result<()>;

    // --- Network ---
    async fn intercept_requests(&self, tab: &TabId, patterns: &[String]) -> Result<()>;
    async fn continue_request(&self, req_id: &str, modifications: Option<RequestMod>) -> Result<()>;
    async fn get_cookies(&self, tab: &TabId) -> Result<Vec<Cookie>>;
    async fn set_cookies(&self, cookies: &[Cookie]) -> Result<()>;
}
```

### 5.3 拟人化引擎

#### 鼠标轨迹生成（Bezier + Fitts' Law）

```rust
pub struct MouseHumanizer {
    profile: HumanProfile,
    rng: StdRng,
}

impl MouseHumanizer {
    /// 生成从起点到终点的拟人鼠标路径
    pub fn generate_path(&mut self, from: Point, to: Point, target_size: f64) -> Vec<MouseStep> {
        let distance = from.distance_to(&to);

        // Fitts' Law: 移动时间与 log2(2*distance/target_width) 成正比
        let movement_time_ms = self.fitts_time(distance, target_size);

        // 2-3 个随机 Bezier 控制点
        let control_points = self.random_control_points(&from, &to, distance);

        // 沿曲线采样，步数根据时间和速度决定
        let steps = self.sample_bezier_curve(&from, &to, &control_points, movement_time_ms);

        // 添加微抖动（高斯噪声 σ=1-2px）
        let steps = self.add_jitter(steps);

        // 10% 概率过冲 + 修正
        let steps = self.maybe_overshoot(steps, &to, target_size);

        steps
    }
}
```

#### 键盘输入模拟

```rust
pub struct KeyboardHumanizer {
    profile: HumanProfile,
    /// 常见字母对的输入间隔分布（从真实打字数据采集）
    bigram_model: BigramTimingModel,
    rng: StdRng,
}

impl KeyboardHumanizer {
    pub fn generate_keystrokes(&mut self, text: &str) -> Vec<KeyStroke> {
        let chars: Vec<char> = text.chars().collect();
        let mut keystrokes = Vec::new();

        for i in 0..chars.len() {
            let base_delay = if i > 0 {
                self.bigram_model.sample_delay(chars[i-1], chars[i])
            } else {
                self.profile.initial_delay()
            };

            // 随机思考停顿（句首/标点后）
            let think_pause = self.maybe_think_pause(i, &chars);

            // 可配置的打错字 + 退格修正
            let typo = self.maybe_typo(chars[i]);

            keystrokes.push(KeyStroke {
                key: self.char_to_key(chars[i]),
                delay_before_ms: base_delay + think_pause,
                hold_duration_ms: self.random_hold_time(),
                corrections: typo,
            });
        }
        keystrokes
    }
}
```

### 5.4 页面感知层（Accessibility Tree）

```rust
/// 精简的可访问性树节点 — 对 AI 友好的页面表示
#[derive(Debug, Serialize)]
pub struct A11yNode {
    pub ref_id: String,         // 稳定元素引用 "e1", "e2", ...
    pub role: String,           // button, link, textbox, heading, ...
    pub name: String,           // 可访问名称
    pub value: Option<String>,  // 当前值（输入框等）
    pub state: Vec<String>,     // focused, disabled, checked, expanded, ...
    pub bounds: Rect,           // 视口内坐标（用于鼠标操作）
    pub children: Vec<A11yNode>,
    pub is_new: bool,           // Tree diffing: 上次快照后新增
}

/// 生成 AI 可读的页面描述
pub fn tree_to_text(tree: &A11yNode, depth: usize) -> String {
    // 输出格式示例:
    // [e1] heading "Welcome to Dashboard"
    // [e2] button "Login" (focused)
    // [e3] textbox "Username" value=""
    // [e4] textbox "Password" value="" (password)
    // [e5] link "Forgot password?"
    //   *[e6] alert "Invalid credentials" (NEW)
}
```

### 5.5 Browser Tools（暴露给 AI Agent）

| Tool Name | 描述 | 参数 |
|-----------|------|------|
| `browser_launch` | 启动/连接浏览器实例 | `backend`, `headless`, `profile`, `proxy` |
| `browser_navigate` | 导航到 URL | `url`, `tab_id?`, `wait_until?` |
| `browser_page_state` | 获取当前页面状态 | `mode: a11y_tree \| screenshot \| dom \| markdown` |
| `browser_click` | 点击元素 | `ref_id \| selector \| coordinates` |
| `browser_type` | 输入文本 | `ref_id \| selector`, `text`, `clear_first?` |
| `browser_scroll` | 滚动页面 | `direction`, `amount`, `ref_id?` |
| `browser_wait` | 等待条件 | `condition: element \| navigation \| network_idle \| time` |
| `browser_eval` | 执行 JavaScript | `expression`, `return_value?` |
| `browser_screenshot` | 截图 | `full_page?`, `selector?`, `format?` |
| `browser_network` | 网络拦截/修改 | `action: intercept \| mock \| log`, `patterns` |
| `browser_tabs` | Tab 管理 | `action: list \| create \| close \| switch` |
| `browser_cookies` | Cookie 操作 | `action: get \| set \| clear`, `cookies?` |

---

## 6. 实施路线

### Phase 1: 基础能力（MVP）
- [ ] `sentinel-browser` crate 骨架 + Cargo.toml
- [ ] CDP 连接层（chromiumoxide 或自实现轻量 CDP client）
- [ ] 基础 Browser Tools: navigate, click, type, page_state, screenshot
- [ ] Level 1 拟人化（基础随机延迟）
- [ ] 注册到 ToolServer

### Phase 2: 拟人化引擎
- [ ] Level 2: Bezier 鼠标 + 键盘节奏 + 物理滚动
- [ ] Accessibility Tree 解析 + Tree Diffing
- [ ] HumanProfile 配置系统

### Phase 3: 高级能力
- [ ] Juggler Adapter（Camoufox 支持）
- [ ] Level 3: 指纹管理 + 自动恢复
- [ ] 网络拦截与修改
- [ ] Script 编排引擎
- [ ] Chrome Extension 增强（A11y overlay）

### Phase 4: 生态整合
- [ ] Plugin Runtime 暴露 browser API
- [ ] Workflow 节点支持
- [ ] 操作录制与回放
- [ ] 多实例并行管理

---

## 7. 技术选型

| 组件 | 选择 | 理由 |
|------|------|------|
| CDP 客户端 | `chromiumoxide` crate | Rust 原生、async、维护活跃 |
| Juggler 通信 | 自实现（pipe I/O） | 无现成 Rust Juggler client |
| 随机数 | `rand` + `rand_distr` | 正态/对数正态分布 |
| Bezier 计算 | `kurbo` crate | 2D 几何/曲线计算 |
| 图片处理 | `image` crate | 截图格式转换 |
| JSON 协议 | `serde_json` | CDP/Juggler 都是 JSON |
| 异步运行时 | `tokio` | 与 Sentinel 一致 |

---

## 8. 与现有系统集成

### 8.1 替换 vs 扩展 `browser_shell`

现有 `browser_shell` 工具定位是"WebSocket 终端会话控制"，新 `browser_*` 工具是"页面自动化操作"。两者互补：
- `browser_shell` → 保留，用于 WebSocket Shell 控制
- `browser_*` → 新增，用于页面交互自动化

### 8.2 ToolServer 注册

```rust
// 在 tool_server.rs 的 init_builtin_tools() 中添加
DynamicToolBuilder::new("browser_launch", browser_launch_schema())
    .category(ToolCategory::Browser)
    .source(ToolSource::Builtin)
    .policy(ToolExecutionPolicy { mutating: true, requires_permission: true, .. })
    .executor(|args| Box::pin(execute_browser_launch(args)))
    .build()
```

### 8.3 Chrome Extension 增强

在现有 `chrome-behavior-capture` 扩展中新增：
- `content-a11y.js`: 提取当前页面 Accessibility Tree 并上报
- 操作状态 overlay（显示 Agent 当前在"看"什么、"做"什么）
- Manual takeover: 用户随时可接管操作

---

## 9. 安全考量

- **权限审批**：所有 browser_* 工具标记为 `requires_permission: true`
- **URL 白名单/黑名单**：可配置允许/禁止访问的域名
- **操作日志**：所有浏览器操作记录完整审计日志
- **沙箱隔离**：Browser Profile 与用户日常浏览隔离
- **超时保护**：单次操作/脚本整体超时限制
