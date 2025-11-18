# Orchestrator Engine Implementation Plan

## 一、概述

### 目标
构建一个**独立的安全测试 Orchestrator 引擎**，通过智能调度 ReWOO / Plan-and-Execute / LLM-Compiler 三大子 Agent，完成复杂的安全任务（Web/API 渗透测试、取证、CTF、逆向等）。

### 核心设计思路
- **Orchestrator 作为独立引擎**：与 ReAct 引擎并列，专注于安全测试场景的任务理解、规划、执行和状态管理。
- **三大子 Agent 协同工作**：
  - **ReWOO**：多分支规划、全局路线设计。
  - **Plan-and-Execute**：线性任务链执行（如登录→抓包→测试）。
  - **LLM-Compiler**：生成/修正脚本、payload、规则。
- **统一安全任务状态模型**：贯穿整个流程，支持前后步骤依赖。
- **完全独立的架构**：不依赖 ReAct 引擎，使用自己的 Prompt 体系和调度逻辑。

### 设计约束
1. **优先范围**：Web/API+认证渗透测试，兼容取证、CTF、逆向等安全场景。
2. **自动化程度**：默认自动跑完整个流程。
3. **展示方式**：只展示摘要和关键决策，详细日志可折叠。
4. **入口方式**：不新增模式入口，在现有 AI 助手页面中使用。
5. **显示组件**：新增 `OrchestratorStepDisplay` 组件，不修改现有步骤展示组件。
6. **架构独立性**：与 ReAct 引擎完全解耦，只共享底层基础设施（AI 服务、工具系统、数据库）。

---

## 二、架构设计

### 1. 总体架构图（概念层）

```
用户 → AI 助手页面（AIChat/AIAssistant）
         ↓
    Orchestrator Engine (独立引擎)
         ↓
    ┌─────────────────────────────────┐
    │  Orchestrator Planning Phase    │
    │  (使用 ReWOO 生成安全测试计划)   │
    └─────────────────────────────────┘
         ↓
    ┌─────────────────────────────────┐
    │  Orchestrator Execution Phase   │
    │  (按计划调度子 Agent 执行)       │
    └─────────────────────────────────┘
         ↓
    统一子 Agent 调用接口
    ├─→ ReWOO (规划层)
    ├─→ Plan-and-Execute (执行层)
    └─→ LLM-Compiler (脚本层)
         ↓
    安全任务状态管理
    ├─ TestSession (会话)
    ├─ TestStep (步骤)
    ├─ Finding (发现)
    └─ AuthContext (认证上下文)
         ↓
    前端展示 (OrchestratorStepDisplay)
```

### 2. 核心组件

#### 2.1 Orchestrator Engine
- 独立的执行引擎，实现 `ExecutionEngine` trait。
- 使用专用的 Orchestrator Prompt 体系（Planning + Execution）。
- 维护 `TestSession` 状态。
- 通过 Rust 调度逻辑协调子 Agent 执行。

#### 2.2 统一子 Agent 调用接口
- `SubAgentKind` 枚举：`ReWOO | PlanAndExecute | LLMCompiler`
- `run_sub_agent(kind, context) -> output`：统一调用入口

#### 2.3 安全任务状态模型
- **TestSession**：
  - `task_kind`: `WebPentest | APIPentest | Forensics | CTF | ReverseEngineering | OtherSecurity`
  - `primary_target`: URL/文件/PCAP/二进制等
  - `stage`: 当前阶段
  - `auth_context`: 认证信息（Cookie/Token/Headers）
  
- **TestStep**：
  - `step_type`: 步骤类型（Recon/Login/APIMapping/VulnScan/Exploit/LogCollection/等）
  - `sub_agent_kind`: 使用的子 Agent
  - `short_summary`: 关键决策摘要
  - `risk_impact`: 风险等级
  - `status`: pending/running/completed/failed
  
- **Finding**：
  - 接口/路径、HTTP 方法、请求示例、响应片段、风险等级、影响说明

#### 2.4 前端展示组件
- **OrchestratorStepDisplay**：新增组件，展示 Orchestrator 层的步骤和关键决策
- 保留现有 `ReActStepDisplay` / `ReWOOStepDisplay` 用于子层细节展示

---

## 三、实现任务清单

### 任务 1：后端 Orchestrator Agent 基础搭建
**状态**: completed

#### 1.1 定义统一"子 Agent 调用接口"
- [x] 新增 `SubAgentKind` 枚举（ReWOO / PlanAndExecute / LLMCompiler）
- [x] 定义统一请求/响应结构 `SubAgentRequest` / `SubAgentResponse`
- [x] 实现 `run_sub_agent(kind, context)` 函数，路由到已有引擎 adapter
- [x] 文件位置：`src-tauri/src/agents/orchestrator/sub_agent_interface.rs`

#### 1.2 新增"SecurityTest Orchestrator Agent"
- [x] 创建 Orchestrator engine 模块：`src-tauri/src/engines/orchestrator/`
- [x] 实现 Orchestrator engine adapter（负责调度子 Agent）
- [x] 注册到现有 Agent 管理器
- [x] 提供统一接口由上层 Agent（如 ReAct 或直接调度）调用

---

### 任务 2：安全任务领域模型与状态管理
**状态**: completed

#### 2.1 定义状态结构体
- [x] 创建 `src-tauri/src/models/security_testing.rs`
- [x] 定义 `SecurityTaskKind` 枚举
- [x] 定义 `TestSession` 结构体（包含 task_kind, primary_target, auth_context 等）
- [x] 定义 `TestStep` 结构体（包含 step_type, sub_agent_kind, short_summary, risk_impact 等）
- [x] 定义 `Finding` 结构体
- [x] 定义 `AuthContext` 结构体（Cookie/Token/Headers）
- [x] 定义步骤类型枚举（支持渗透/取证/CTF/逆向等场景）

#### 2.2 会话生命周期管理
- [x] 实现会话创建/更新/结束接口
- [x] 集成到现有任务/会话存储机制
- [x] 如有需要增加 DB 表支持持久化
- [x] 实现状态查询接口（供前端和 Orchestrator 使用）

---

### 任务 3：Orchestrator Prompt 体系设计
**状态**: completed

#### 3.1 Orchestrator Planning Prompt
- [x] 创建 `src-tauri/prompts/orchestrator_planning.md`
- [x] 编写规划阶段 Prompt：
  - 明确角色：安全测试规划专家
  - 输出 ReWOO 标准 JSON 计划格式
  - 支持多种安全任务类型（Web/API/取证/CTF/逆向）
  - 强制包含资源清理和 AI 插件生成步骤
- [x] 集成到 ReWOO Planner 作为 custom_system_prompt

#### 3.2 Orchestrator Execution Prompt
- [x] 创建 `src-tauri/prompts/orchestrator_execution.md`
- [x] 编写执行阶段 Prompt：
  - 明确角色：安全测试执行协调者
  - 说明如何调度子 Agent（ReWOO/Plan-and-Execute/LLM-Compiler）
  - 定义状态管理工具（update_session_state/record_finding/update_auth_context）
  - 约束自动推进流程，避免无意义循环
- [x] 集成到 Plan-and-Execute 作为执行指导

---

### 任务 4：与 ReWOO / Plan-and-Execute / LLM-Compiler 的集成
**状态**: completed

#### 4.1 ReWOO 集成（规划层）
- [x] 为 ReWOO 增加安全测试规划专用 Prompt 模式
- [x] 调整 ReWOO 输出格式，支持结构化计划节点
- [x] 实现 Orchestrator 到 ReWOO 的调用适配
- [x] 实现 ReWOO 计划节点到 `TestStep` 的转换

#### 4.2 Plan-and-Execute 集成（执行层）
- [x] 增加"有状态执行"能力，支持 `AuthContext` 传递
- [x] 支持在每步请求中复用认证信息（Cookie/Token/Headers）
- [x] 增加安全测试执行模式的 Prompt
- [x] 实现 Orchestrator 到 Plan-and-Execute 的调用适配
- [x] 返回结构化执行结果（包含关键摘要）

#### 4.3 LLM-Compiler 集成（脚本层）
- [x] 增加安全测试脚本生成专用 Prompt 模式
- [x] 支持根据 API schema/示例生成 fuzz 模板
- [x] 支持根据失败记录迭代生成新 payload
- [x] 实现 Orchestrator 到 LLM-Compiler 的调用适配
- [x] 返回代码/payload 列表及用途说明

---

### 任务 5：前端 UI 与交互
**状态**: completed

#### 5.1 不新增模式入口，复用现有 AI 助手页面
- [x] 在 `AIChat.vue` 中识别 Orchestrator 会话类型
- [x] 当会话由 Orchestrator 驱动时，加载 `OrchestratorStepDisplay` 组件
- [x] 保持现有界面布局，不增加新的 tab/模式按钮

#### 5.2 新增 OrchestratorStepDisplay 组件
- [x] 创建 `src/components/MessageParts/OrchestratorStepDisplay.vue`
- [x] 实现顶部概要区域：
  - 显示 task_kind / primary_target / stage
  - 显示关键指标（发现漏洞数/高危数量等）
- [x] 实现步骤列表展示：
  - 按序号展示每个 OrchestratorStep
  - 显示 sub_agent_kind / short_summary / status / risk_impact
  - 支持步骤状态图标和颜色标识
- [x] 实现详情折叠功能：
  - 点击步骤可展开子 Agent 详情
  - 嵌入或链接到 ReActStepDisplay / ReWOOStepDisplay 等组件
- [x] 支持多种 task_kind 的差异化展示

#### 5.3 创建 useOrchestratorMessage composable
- [x] 创建 `src/composables/useOrchestratorMessage.ts`
- [x] 实现 Orchestrator 消息解析逻辑
- [x] 实现步骤数据提取和格式化
- [x] 提供给 OrchestratorStepDisplay 使用

#### 5.4 集成到 AIChat
- [x] 在 `AIChat.vue` 中导入 OrchestratorStepDisplay
- [x] 根据消息类型动态渲染对应组件
- [x] 保持与现有消息展示的一致性

---

### 任务 6：日志与编译验证
**状态**: completed

#### 6.1 日志支持
- [x] 为 Orchestrator 添加统一日志标签（`orchestrator`）
- [x] 为子 Agent 调用添加日志标签（`sub_agent=rewoo/plan_exec/compiler`）
- [x] 记录关键决策点和状态转换
- [x] 记录子 Agent 调用的输入/输出摘要

#### 6.2 编译检查
- [x] 执行 Rust 后端编译检查：`cd src-tauri && cargo check`
- [x] 执行前端编译检查：`npm run build` 或 `yarn build`
- [x] 确保无编译错误
- [x] 不实际运行测试流程（按用户规则）

---

## 四、技术细节

### 1. 安全任务类型与步骤映射

#### Web/API 渗透测试
- **阶段**：Recon → Login → APIMapping → VulnScan → Exploit → Report
- **典型步骤**：
  - Recon: 信息收集、子域枚举、技术栈识别
  - Login: 登录流程测试、会话管理
  - APIMapping: 接口枚举、参数识别
  - VulnScan: XSS/SQLi/IDOR/权限等漏洞测试
  - Exploit: PoC 构造、深度利用
  - Report: 结果整理、风险评估

#### 取证分析
- **阶段**：LogCollection → TimelineReconstruction → IOCExtraction → BehaviorAnalysis → Report
- **典型步骤**：
  - LogCollection: 日志收集、数据源识别
  - TimelineReconstruction: 时间线重建、事件关联
  - IOCExtraction: 威胁指标提取
  - BehaviorAnalysis: 行为模式分析、攻击链还原

#### CTF 解题
- **阶段**：ChallengeAnalysis → VulnIdentification → PayloadCrafting → FlagExtraction → Writeup
- **典型步骤**：
  - ChallengeAnalysis: 题目分析、类型识别
  - VulnIdentification: 漏洞点定位
  - PayloadCrafting: Exploit 编写
  - FlagExtraction: Flag 获取

#### 逆向工程
- **阶段**：BinaryLoading → StaticAnalysis → DynamicAnalysis → Deobfuscation → BehaviorSummary
- **典型步骤**：
  - BinaryLoading: 文件加载、格式识别
  - StaticAnalysis: 静态分析、控制流分析
  - DynamicAnalysis: 动态调试、行为监控
  - Deobfuscation: 反混淆、代码还原

### 2. Orchestrator 调度策略

Orchestrator 引擎采用**两阶段调度模型**：

#### 阶段 1：Planning（规划）
- 使用 ReWOO 子 Agent 生成完整的安全测试计划
- 输入：用户的安全测试目标 + Orchestrator Planning Prompt
- 输出：ReWOO 标准 JSON 计划（包含 plan_summary 和 steps）
- 计划中的每个 step 会指定使用哪个子 Agent（通过 args 元数据）

#### 阶段 2：Execution（执行）
- 按照 ReWOO 计划的依赖关系，顺序调度子 Agent
- 每个 step 根据其 tool 和 args，路由到对应的子 Agent：
  - **ReWOO**：用于需要进一步细化规划的步骤
  - **Plan-and-Execute**：用于线性执行链（登录、扫描、测试等）
  - **LLM-Compiler**：用于生成脚本、payload、工具
- 维护 TestSession 状态，记录每个步骤的执行结果
- 自动清理资源（浏览器、代理等）

### 3. 认证上下文传递

```rust
pub struct AuthContext {
    pub cookies: HashMap<String, String>,
    pub headers: HashMap<String, String>,
    pub tokens: HashMap<String, String>, // Bearer, API Key, etc.
    pub credentials: Option<Credentials>,
}

pub struct Credentials {
    pub username: String,
    pub password: String,
}
```

在 Plan-and-Execute 执行时，每个 HTTP 请求自动附加 `AuthContext` 中的信息。

### 4. 前端数据结构

```typescript
interface OrchestratorSession {
  taskKind: 'WebPentest' | 'APIPentest' | 'Forensics' | 'CTF' | 'ReverseEngineering' | 'OtherSecurity';
  primaryTarget: string;
  stage: string;
  summary: string;
  findings: Finding[];
  steps: OrchestratorStep[];
}

interface OrchestratorStep {
  id: string;
  index: number;
  subAgentKind: 'ReWOO' | 'PlanAndExecute' | 'LLMCompiler' | 'Other';
  stepType: string;
  shortSummary: string;
  riskImpact: 'None' | 'Info' | 'Low' | 'Medium' | 'High' | 'Critical';
  status: 'pending' | 'running' | 'completed' | 'failed';
  startedAt?: string;
  finishedAt?: string;
  detailRefs?: string[];
}

interface Finding {
  id: string;
  location: string; // URL/接口/文件路径
  method?: string; // HTTP 方法
  riskLevel: 'Info' | 'Low' | 'Medium' | 'High' | 'Critical';
  title: string;
  description: string;
  evidence: string; // 请求/响应片段
  reproductionSteps?: string[];
}
```

---

## 五、实现顺序

1. **任务 2**：先定义领域模型，确保数据结构清晰
2. **任务 1**：搭建 Orchestrator 基础和子 Agent 接口
3. **任务 3**：编写 Prompt 和工具定义
4. **任务 4**：集成三大子 Agent
5. **任务 5**：前端 UI 实现
6. **任务 6**：日志和编译验证

---

## 六、与 ReAct 引擎的关系

### 架构定位
- **Orchestrator 引擎**：专注于安全测试场景，独立的执行引擎
- **ReAct 引擎**：通用推理和工具调用，适用于普通对话、代码辅助等
- **关系**：并列存在，互不依赖，只共享底层基础设施

### 任务路由
在 AI 命令层（`ai_commands.rs`）根据任务类型选择引擎：
- 包含"安全/渗透/漏洞/取证/ctf/逆向"等关键词 → Orchestrator 引擎
- 其他任务 → ReAct 引擎或其他引擎

### 共享基础设施
- AI 服务（AiService / AiServiceManager）
- 工具系统（FrameworkToolAdapter / MCP）
- 数据库服务（DatabaseService）
- 前端消息传递（ordered_message）

## 七、后续扩展方向

1. **插件系统集成**：允许 Orchestrator 调用自定义安全测试插件
2. **报告生成**：自动生成结构化安全测试报告（Markdown/PDF）
3. **并发控制**：支持多个测试步骤并发执行（在安全的前提下）
4. **测试回放**：记录完整测试过程，支持回放和审计
5. **知识库集成**：与 RAG 系统结合，利用历史漏洞知识优化测试策略
6. **动态计划调整**：根据执行结果动态调整后续测试计划

---

## 八、注意事项

1. **不过度设计**：只实现必需功能，避免过度抽象
2. **兼容性**：不考虑向后兼容，专注当前架构
3. **日志语言**：所有日志使用英语
4. **注释风格**：简洁清晰，说明意图即可
5. **文件拆分**：单文件超过 1000 行时按功能拆分
6. **测试策略**：只验证编译通过，不实际运行测试

---

## 九、更新日志

- **2025-11-18**：
  - 初始版本，完成整体架构设计和任务拆分
  - ✅ 任务 1 完成：后端 Orchestrator Agent 基础搭建
  - ✅ 任务 2 完成：安全任务领域模型与状态管理
  - ✅ 任务 3 完成：Orchestrator Prompt & ReAct 工具定义
  - ✅ 任务 4 完成：与 ReWOO/Plan-and-Execute/LLM-Compiler 的集成
  - ✅ 任务 5 完成：前端 UI 与交互
  - ✅ 任务 6 完成：日志与编译验证
  - **编译状态**：✅ Rust 后端编译通过，✅ 前端编译通过

- **2025-11-18 (下午)**：
  - 🔄 架构重构：将 Orchestrator 从"基于 ReAct"改为"独立引擎"
  - 更新文档，移除所有 ReAct 依赖描述
  - 明确 Orchestrator 与 ReAct 的并列关系
  - 优化两阶段调度模型（Planning + Execution）

## 十、实现总结

### 已完成的核心组件

#### 后端 (Rust)
1. **领域模型** (`src-tauri/src/models/security_testing.rs`)
   - `SecurityTaskKind`: 支持 Web/API 渗透、取证、CTF、逆向等
   - `TestSession`: 会话状态管理
   - `TestStep`: 步骤跟踪
   - `Finding`: 安全发现记录
   - `AuthContext`: 认证上下文传递

2. **会话管理器** (`src-tauri/src/managers/security_test_manager.rs`)
   - 会话生命周期管理
   - 步骤和发现的增删改查
   - 统计信息查询

3. **子 Agent 接口** (`src-tauri/src/agents/orchestrator/sub_agent_interface.rs`)
   - 统一的请求/响应结构
   - `SubAgentExecutor` trait
   - 支持 ReWOO、Plan-and-Execute、LLM-Compiler

4. **Orchestrator 引擎** (`src-tauri/src/engines/orchestrator/`)
   - `engine_adapter.rs`: 核心适配器
   - `tools.rs`: 工具定义（6个工具）
   - `prompt.md`: 详细的系统 Prompt
   - `sub_agents/`: 三个子 Agent 执行器

#### 前端 (Vue/TypeScript)
1. **Composable** (`src/composables/useOrchestratorMessage.ts`)
   - 消息解析和类型判断
   - 标签和颜色映射
   - 状态图标管理

2. **展示组件** (`src/components/MessageParts/OrchestratorStepDisplay.vue`)
   - 会话概要展示
   - 步骤列表展示
   - 风险等级可视化
   - 详细输出折叠

3. **集成** (`src/components/AIChat.vue`)
   - Orchestrator 消息检测
   - 自动路由到对应组件
   - 与现有架构无缝集成

### 架构特点

1. **独立引擎架构**：与 ReAct 完全解耦，专注安全测试场景
2. **两阶段调度模型**：Planning（规划）+ Execution（执行）
3. **模块化设计**：每个组件职责清晰，易于扩展
4. **类型安全**：Rust 和 TypeScript 都有完整的类型定义
5. **可扩展性**：
   - 新增安全任务类型只需扩展枚举
   - 新增子 Agent 只需实现 `SubAgentExecutor` trait
   - 前端组件复用现有展示逻辑
6. **用户体验**：
   - 统一的消息格式
   - 清晰的视觉层次
   - 实时状态更新

### 下一步工作（可选）

1. **动态计划执行**：根据 ReWOO 计划的 steps 动态调度子 Agent（目前是硬编码两步）
2. **状态工具实现**：实现 update_session_state / record_finding / update_auth_context 的实际逻辑
3. **计划解析优化**：完善 ReWOO 计划到 TestStep 的映射
4. **测试**：编写单元测试和集成测试
5. **持久化增强**：优化会话数据持久化机制
6. **报告生成**：自动生成结构化安全测试报告

