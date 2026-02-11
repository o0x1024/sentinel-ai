# Sentinel AI

> AI + MCP 驱动的桌面安全分析平台（Tauri + Vue）

Sentinel AI 面向安全研究与攻防工程团队，目标不是单点工具，而是把 **流量分析**、**AI 助手**、**工作流自动化**、**知识库（RAG）**、**插件系统** 和 **工具生态（MCP/内置工具）** 连接成一个可持续迭代的分析闭环。

## 为什么是 Sentinel AI

传统安全工具链常见问题：
- 数据分散：流量、漏洞、资产、笔记、脚本分散在多个系统
- 自动化断层：发现问题后，验证、复测、通知、归档仍靠人工串联
- AI 落地弱：AI 只能回答问题，无法稳定调用企业内部能力

Sentinel AI 的设计目标：
- 让流量与扫描结果成为 AI 可直接消费的上下文
- 让插件能力成为 AI 可调用的工具，而不是孤立脚本
- 让高频分析流程可沉淀为工作流并持续调度执行

## 核心能力矩阵

| 模块 | 核心能力 | 对应入口 |
| --- | --- | --- |
| 流量分析 Traffic Analysis | 代理监听、HTTPS 解密、请求/响应/WS 历史、拦截、重放、findings | `src/views/TrafficAnalysis.vue` |
| AI 助手 AI Assistant | 多轮对话、工具调用、子代理追踪、角色策略、会话管理 | `src/views/AIAssistant.vue` |
| 工作流 Workflow Studio | 可视化编排、节点执行、调度运行、运行记录 | `src/views/WorkflowStudio.vue` |
| 知识库 RAG Management | 文档入库、向量检索、集合管理、检索增强问答 | `src/views/RAGManagement.vue` |
| 应用工具 Tools / MCP | 内置工具、MCP Server、插件工具、交互终端 | `src/views/Tools.vue` |
| 插件系统 Plugin Management | 插件开发、测试、审核、启停、商店安装与迭代 | `src/views/PluginManagement.vue` |
| 系统设置 Settings | AI/数据库/网络/代理/RAG/授权等全局管理 | `src/views/Settings.vue` |

## 重点能力：插件系统 × AI 助手

这部分是 Sentinel AI 的核心差异化能力。

### 1. 插件不是“外挂”，而是“可治理的能力单元”

平台内插件主要分两类：
- **Traffic 插件**：挂在流量检测链路中，对 HTTP/WS 数据做规则化分析并产出 findings
- **Agent 工具插件**：注册到统一工具层，供 AI 助手与工作流直接调用

这意味着插件从一开始就具备：
- 生命周期管理（开发、测试、审核、发布、禁用）
- 可观测性（执行记录、结果回流、错误定位）
- 可编排性（可被 AI 和 Workflow 复用）

### 2. AI 助手直接调用插件能力

AI 助手不止用于问答，而是通过工具调用执行真实动作：
- 读取/分析流量与 findings
- 调用插件执行专项检测或数据处理
- 联动 MCP 与内置工具完成上下游动作
- 结合 RAG 知识库生成可复用的分析结论

### 3. 闭环流程（建议作为团队标准流程）

1. **流量侧发现问题**：Traffic 插件输出结构化 findings
2. **AI 侧补全分析**：AI 调用插件工具进行验证、归因、关联查询
3. **流程侧自动执行**：将成熟处置逻辑沉淀为工作流并调度运行
4. **知识侧持续沉淀**：结论写入知识库，供后续相似问题快速复用

这个组合把 “发现 → 研判 → 执行 → 复盘” 从人工链路升级为可持续自动化链路。


## 主要模块说明

### 流量分析（Traffic Analysis）
- 支持代理监听、拦截与重放，覆盖 HTTP/HTTPS/WS 关键分析场景
- 可结合流量插件实时产出检测结果并进入后续处置链路
- 可与 Proxifier、抓包能力组合使用，适配不同网络场景

### AI 助手（AI Assistant）
- 提供多轮上下文会话，支持角色策略和工具调用轨迹
- 支持子代理执行与结果回传，便于复杂任务拆解
- 可直接消费流量、资产、知识库、工具执行结果

### 工作流（Workflow Studio）
- 将高频分析流程抽象为节点化流程
- 支持保存、运行、停止、调度和运行记录查询
- 可复用插件工具与 MCP 工具，实现跨系统联动

### 知识库管理（RAG Management）
- 管理文档导入、分块、向量化和检索
- 为 AI 助手提供可追溯的知识增强上下文
- 支持规则、案例、手册等资产沉淀

### 应用工具（Tools / MCP）
- 统一管理内置工具、插件工具、工作流工具、MCP 工具
- 支持 MCP Server 接入、工具枚举和调用
- 内置终端能力，支持在同一工作台完成执行与验证

### 插件系统（Plugin Management）
- 支持从开发到审核再到启停的完整流程
- 面向两类执行面：流量检测面与 Agent 工具面
- 插件能力可被 AI 与工作流重复利用，避免能力孤岛

### 系统设置（Settings）
- 统一管理 AI、数据库、代理、RAG、安全、授权等配置
- 便于团队部署、环境迁移和策略治理

## 技术栈

- **Desktop**: Tauri v2
- **Frontend**: Vue 3 + Vite + TypeScript + Pinia + Vue Router + Tailwind/daisyUI
- **Backend**: Rust workspace (`src-tauri/sentinel-*`)
- **Storage**: SQLite

关键 Rust 模块：
- `sentinel-traffic`: 流量代理、拦截、历史、findings 管线
- `sentinel-plugins`: 插件运行时与插件能力承载
- `sentinel-tools`: 统一工具系统（内置/MCP/插件/工作流）
- `sentinel-workflow`: 工作流引擎与调度
- `sentinel-rag`: 检索增强与向量检索
- `sentinel-llm`: LLM 客户端与工具调用桥接
- `sentinel-db`: 持久化与数据访问

## 快速开始

### 环境要求

- Node.js 18+
- Rust stable toolchain
- Tauri v2 构建依赖（按你的操作系统安装）

### 安装依赖

```bash
npm install
```

### 前端开发

```bash
npm run dev
```

### 桌面应用开发

```bash
npm run tauri dev
```

### 构建

```bash
npm run build
npm run build:release
```

## 开发与测试

```bash
# 类型检查
npm run type-check

# 单元/集成测试
npm run test
npm run test:unit
npm run test:integration

# E2E
npm run test:e2e

# Rust 检查
cd src-tauri && cargo check
```

## 项目结构

```text
.
├─ src/                      # Vue 前端
│  ├─ views/                 # 业务页面
│  ├─ components/            # 组件
│  └─ services/              # 前端服务层
├─ src-tauri/                # Tauri + Rust workspace
│  ├─ src/commands/          # Tauri 命令域
│  ├─ sentinel-traffic/      # 流量能力
│  ├─ sentinel-plugins/      # 插件运行时
│  ├─ sentinel-tools/        # 工具系统
│  ├─ sentinel-workflow/     # 工作流引擎
│  ├─ sentinel-rag/          # 知识库与检索
│  └─ agent-browser/         # 浏览器自动化守护进程
├─ plugins/                  # 插件目录
├─ docs/                     # 项目文档
└─ scripts/                  # 开发脚本
```

## 适用场景

- 安全团队的日常流量研判与漏洞验证
- 红蓝对抗中的自动化资产与风险分析
- 漏洞运营中的复测、通知、归档自动化
- 将团队经验沉淀为插件与知识库，持续复用

## License

本项目采用最严格许可策略：`All Rights Reserved`（保留所有权利）。

- 未经书面授权，不得复制、修改、分发、再许可或用于商业用途。
- 详细条款见根目录 `/Users/a1024/code/ai/sentinel-ai/LICENSE`。
