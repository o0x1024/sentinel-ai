# Sentinel AI

AI + MCP 的桌面安全分析平台（Tauri + Vue）。核心目标是把 **流量代理/拦截**、**插件化检测**、**AI 助手/子代理**、**RAG 知识库**、**工作流编排与调度** 串起来，形成可自动化的安全分析与漏洞挖掘工作台。

## 技术栈概览

- **桌面端**：Tauri v2（Rust 后端 + WebView 前端）
- **前端**：Vue 3 + Vite + TypeScript + Pinia + Vue Router + Tailwind/daisyUI
- **后端（Rust workspace）**：`src-tauri/` 下多个 crate 组合（数据库/代理/工具/工作流/插件运行时等）
- **数据库**：SQLite（默认路径：`~/Library/Application Support/sentinel-ai/database.db`）

## 目录结构（导航）

- **前端（UI）**：`src/`
- **后端（Tauri/Rust）**：`src-tauri/src/`（应用启动与命令注册），`src-tauri/sentinel-*/`（各子模块 crate）
- **文档站点**：`docs/`（VitePress）
- **脚本**：`scripts/`
- **静态资源**：`public/`
- **打包配置**：`src-tauri/tauri.conf.json`
- **浏览器自动化子项目**：`src-tauri/agent-browser/`（随桌面应用一起打包为资源；提供 daemon + 跨平台原生二进制）

## 功能地图（按前端页面）

入口路由集中在 `src/main.ts`，页面在 `src/views/`。

- **总览 Dashboard**（`src/views/Dashboard.vue`）
  - 聚合展示核心指标、系统状态、快捷入口。
  - 关键组件：`src/components/Dashboard/`

- **安全中心 Security Center**（`src/views/SecurityCenter.vue`）
  - 面向资产、扫描任务、漏洞的统一管理入口。
  - 关键组件：`src/components/SecurityCenter/`、`src/components/ScanTasks/`
  - 后端关联：资产/扫描/漏洞相关命令（见下方“后端命令域”）

- **资产管理 Asset Management**（`src/views/AssetManagement.vue`）
  - 资产创建/导入/关联关系/风险与状态管理。
  - 后端关联：`asset::*`（`src-tauri/src/commands/asset.rs`）

- **字典管理 Dictionary Management**（`src/views/DictionaryManagement.vue`）
  - 管理扫描/枚举用字典、字典集合、子域名字典等。
  - 前端服务：`src/services/dictionary.ts`
  - 后端关联：`dictionary::*`（`src-tauri/src/commands/dictionary.rs`、`subdomain_dictionary`）

- **AI 助手 AI Assistant**（`src/views/AIAssistant.vue`）
  - 多轮对话、工具调用、（子）代理运行与追踪、对话历史管理。
  - 关键组件：`src/components/Agent/`（消息流、工具调用展示、子代理详情、Web Explorer 面板等）
  - 后端关联：`ai::*`、`aisettings::*`、`role::*`

- **知识库管理 RAG Management**（`src/views/RAGManagement.vue`）
  - 文档/文件入库、向量检索、集合管理、RAG 配置与状态查看。
  - 前端服务：`src/services/rag_service.ts`、`src/services/rag_config.ts`、`src/services/search.ts`
  - 后端关联：`rag_commands::*`

- **工作流 Workflow Studio**（`src/views/WorkflowStudio.vue`）
  - 可视化编排节点/连线，关联工具与插件能力，支持运行与调度。
  - 关键组件：`src/components/workflow/FlowchartVisualization.vue`
  - 后端关联：`sentinel-workflow` 相关 commands（启动/停止/定义/调度等）与工具注册（workflow tools）

- **流量分析 Traffic Analysis**（`src/views/TrafficAnalysis.vue`）
  - 本地代理监听、请求/响应/WS 历史、拦截（intercept）、重放（replay/repeater）、插件检测结果（findings）展示。
  - 关键组件：`src/components/traffic/`（ProxyControl/History/Intercept/Repeater、Proxifier、PacketCapture 等）
  - 前端服务：`src/services/proxy_history.ts`
  - 后端关联：`traffic_analysis_commands::*`、`proxifier_commands::*`、`packet_capture_commands::*`

- **插件管理 Plugin Management**（`src/views/PluginManagement.vue`）
  - 插件编写/测试/启用禁用、生成与审核（review）、插件商店（store）拉取与安装。
  - 关键组件：`src/components/PluginManagement/`
  - 后端关联：`plugin_generation_commands::*`、`plugin_review_commands::*`、`traffic_analysis_commands::*`（插件启停/测试/安装）

- **工具中心 Tools / MCP**（`src/views/Tools.vue`）
  - 统一工具列表、内置工具开关、MCP Server 管理与工具调用、插件工具/工作流工具展示、交互式终端。
  - 关键组件：`src/components/Tools/`
  - 后端关联：`tool_commands::*`、`mcp_commands::*`、`terminal_commands::*`、`shell_commands::*`

- **通知管理 Notification Management**（`src/views/NotificationManagement.vue`）
  - 通知规则的 CRUD、连接测试、发送通知。
  - 后端关联：`notifications::*`（内部实现依赖 `sentinel-notify`）

- **系统设置 Settings**（`src/views/Settings.vue`）
  - 通用/AI/数据库/网络/RAG/安全/代理/第十人等设置面板集合。
  - 关键组件：`src/components/Settings/`
  - 后端关联：`config::*`、`config_commands::*`、`database::*`、`license_commands::*` 等

- **性能监控 Performance**（`src/components/PerformanceMonitor.vue`，路由 `/performance`）
  - 路由与关键操作耗时记录、性能报告与建议。
  - 前端服务：`src/services/performance.ts`、`src/services/cache.ts`
  - 后端关联：`performance::*`、`cache_commands::*`

## 前端模块（按目录）

- **UI 入口与路由**：`src/main.ts`（路由、全局插件、启动设置、性能埋点、外链打开策略）
- **视图层**：`src/views/`（每个页面对应一个功能域）
- **组件层**：`src/components/`
  - `Agent/`：对话/工具调用/子代理/Web Explorer 等
  - `traffic/`：代理控制/历史/拦截/重放/Proxifier/抓包
  - `PluginManagement/`：插件编辑/测试/审核/商店
  - `Tools/`：统一工具/MCP/终端/配置
  - `Settings/`：设置子页
- **服务层（前端调用封装）**：`src/services/`（dictionary/rag/search/proxy_history/performance/cache）
- **国际化**：`src/i18n/`（按功能域拆分的 `locales/*`）

## 后端模块（Tauri/Rust）

### 应用启动与运行时初始化

入口：`src-tauri/src/main.rs` → `sentinel_ai_lib::run()`（`src-tauri/src/lib.rs`）。

启动阶段会完成：
- 初始化日志（写入 `logs/sentinel-ai.log`）
- 初始化数据库与服务（DB、AI 服务、RAG 服务、工具权限、执行追踪等）
- 初始化工作流引擎与调度器（并注册工作流工具）
- 初始化流量分析状态（代理/拦截/插件管线等）
- 延迟自动连接 MCP Server（并注册 MCP 工具）
- 启动 `agent-browser` 守护进程（用于浏览器自动化能力）
- 托盘菜单：显示主界面/开启关闭代理/退出（不同平台做了任务栏表现处理）

### 后端命令域（`src-tauri/src/commands/`）

这些命令通过 Tauri `invoke` 暴露给前端，可按功能域理解：

- **AI 与对话**：`ai.rs`、`aisettings.rs`
  - AI 服务管理、对话/消息、流式取消、子代理运行记录、生成工作流/插件/角色等。
- **角色 Role**：`role.rs`
  - AI 角色 CRUD、当前角色切换（也用于提示词策略/预设）。
- **工具与工具服务器**：`tool_commands/`
  - 统一工具执行（builtin/MCP/workflow/plugin tools）、能力组（Ability Groups）、工具元数据与统计、Shell 权限门禁等。
- **MCP**：`mcp_commands.rs`
  - MCP Server 配置/连接状态/自动连接、工具枚举与调用、导入导出。
- **RAG**：`rag_commands.rs`
  - 入库/查询/集合管理、服务初始化/重载、文件扫描等。
- **工作流**：由 `sentinel-workflow` 提供 commands（在 `lib.rs` 的 `invoke_handler` 中注册）
  - 工作流定义保存/校验、运行与停止、运行记录查询、调度计划管理等。
- **流量分析与代理**：`traffic_analysis_commands.rs`
  - 代理监听、CA 证书管理、请求/响应/WS 历史、拦截/放行/丢弃、重放、插件启停与 findings 管理等。
- **Proxifier（透明代理/全局转发）**：`proxifier_commands.rs`
  - 代理规则/上游代理管理、连接列表、透明代理状态控制等。
- **抓包 Packet Capture**：`packet_capture_commands.rs`
  - 网卡列表、抓包启停、PCAP 读取/保存、文件提取与关联包分析等。
- **终端/沙箱**：`terminal_commands.rs`、`shell_commands.rs`
  - 交互式终端会话、WebSocket 终端服务、Docker Sandbox 初始化与容器清理等。
- **资产与漏洞**：`asset.rs`（以及服务层中的漏洞服务）
  - 资产 CRUD/导入/关系/查询/统计；漏洞相关通常与扫描/流量 findings 关联。
- **扫描任务/会话**：`scan_task_commands.rs`、`scan_session_commands.rs`
  - 扫描任务生命周期控制、进度与阶段查询、会话管理。
- **字典**：`dictionary.rs`
  - 字典/字词/集合/导入导出、子域名字典等。
- **通知**：`notifications.rs`
  - 通知规则 CRUD、连接测试、发送通知。
- **性能与缓存**：`performance.rs`、`cache_commands.rs`
  - 性能指标/报告/建议、缓存读写与清理。
- **插件生成/审核**：`plugin_generation_commands.rs`、`plugin_review_commands.rs`
  - 生成插件代码、验证、审核/批量审批、导出、收藏等。
- **License**：`license_commands.rs`
  - 授权信息、激活/反激活、机器码等。
- **窗口与资源**：`window.rs`、`asset.rs`
  - 多窗口管理、资源相关操作等。

## Rust Workspace 子模块（`src-tauri/sentinel-*/`）

后端按 crate 拆分，便于复用与边界清晰：

- `sentinel-core`：通用基础能力（错误类型、通用模型、全局代理配置等）。
- `sentinel-db`：SQLite 数据访问与持久化（同时承载 RAG/插件等相关表与存取）。
- `sentinel-llm`：统一 LLM 客户端封装（多轮、流式、图像支持、工具调用桥接）。
- `sentinel-rag`：RAG 与向量检索（LanceDB、embedding 调用、chunk/检索/重排等）。
- `sentinel-memory`：记忆/知识片段的结构与持久化（与 RAG/DB 结合，供代理使用）。
- `sentinel-tools`：统一工具体系（内置工具、MCP 工具、工作流工具、插件工具适配；含交互终端与 agent-browser 守护进程管理）。
- `sentinel-plugins`：插件运行时与内置安全插件体系（基于 Deno Core 执行 TS/JS 插件）。
- `sentinel-traffic`：HTTP(S)/WebSocket 代理、拦截与历史、findings 管线、CA 证书、抓包能力等。
- `sentinel-workflow`：工作流引擎与调度（执行器、工具集成、运行记录与调度计划）。
- `sentinel-notify`：通知通道与签名/发送（如 webhook、邮件等能力的实现载体）。
- `sentinel-license`：授权校验与机器码（含 `license_generator` 工具）。
- `sentinel-services`：服务层聚合（对上提供更高层的业务服务组合，供 commands 调用）。
- `sentinel-commands`：命令域的抽象/复用（供主应用注册与调用）。

## 插件体系（Traffic 插件 + Agent 工具插件）

插件在平台中主要有两类用途：

- **Traffic 扫描插件**：对 HTTP 请求/响应/WS 消息进行检测与产出 findings（由 `sentinel-traffic` 驱动管线，`sentinel-plugins` 提供运行时）。
- **Agent 工具插件**：作为“工具”注册到统一工具服务器，供 AI/工作流调用（启动时会从 DB 拉取 `main_category='agent'` 且启用/审核通过的插件并注册）。

前端入口：`src/views/PluginManagement.vue`（编辑/测试/审核/商店/启用禁用等）。

## 平台相关能力

- **macOS 网络扩展（可选）**：`src-tauri/macos-extension/`
  - 用于实现应用级透明代理（类似 Proxifier），基于 `NETransparentProxyProvider`。
- **浏览器自动化（agent-browser）**：`src-tauri/agent-browser/`
  - 以 daemon 方式提供 headless 浏览器能力（Playwright），供 Web Explorer / Agent 工具链做页面操作与信息采集。
- **托盘与窗口行为**：`src-tauri/src/lib.rs`（关闭时隐藏、托盘开关代理、跨平台任务栏表现差异处理）。

## 常用开发命令（简要）

- 前端开发：`yarn dev`
- 桌面开发：`yarn tauri dev`
- 类型检查：`yarn type-check`
- Rust 检查：在 `src-tauri/` 下执行 `cargo check`

