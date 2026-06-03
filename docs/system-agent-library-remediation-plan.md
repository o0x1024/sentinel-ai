# 智能体库整改与演进方案

更新时间：2026-04-08

## 1. 结论

当前“智能体库”更准确的定位是：

- 主动类能力里，只有一部分算真正的 Agent
- 被动类能力本质上更接近分析器、判定器、验证 Worker
- 当前页面更像“系统 AI 能力配置中心”
- 不是可扩展、可复用、可发布的真正智能体库

核心矛盾不在于“Agent 不够多”，而在于：

1. 产品抽象与实现抽象不一致
2. 配置模型与运行时契约不一致
3. 同一 Profile 在不同入口下能力不一致
4. 版本、调试、治理链路还不够闭环

因此整改顺序应当是：

1. 先修平台契约
2. 再做库化
3. 最后补治理与运营

## 2. 第一性原理下的重新分类

从第一性原理看，“Agent”至少应满足以下特征中的大部分：

- 面向开放任务而非固定单步判定
- 具备多步推理和阶段性决策
- 可能需要选择工具而不是只做文本分类
- 能根据上下文调整执行路径

按这个标准，当前能力应拆成两类：

### 2.1 真正的 Agent

更适合保留 Agent 抽象的能力：

- `workflow_designer_agent`
- `traffic_plugin_generator_agent`
- `plugin_fix_agent`
- `manual_traffic_audit_agent`

这些能力的共同点：

- 用户显式发起
- 任务开放度高
- 往往需要多步生成或修复
- 可能需要工具、上下文注入、结构化输出

### 2.2 不应强行叫 Agent 的能力

更准确应归类为分析器 / 判定器 / Worker 的能力：

- `traffic_logic_triage`
- `traffic_active_verifier`

这些能力的共同点：

- 输入高度结构化
- 目标相对固定
- 输出通常是分类、假设、验证结果
- 更接近“受约束的系统任务执行单元”
- 不一定需要 tool use，也不一定需要完整 agent executor

### 2.3 设计原则

建议后续明确采用双轨模型：

1. 主动类：`Agent`
   - 面向开放任务
   - 可使用工具
   - 可多步推理
   - UI 上保留“智能体”语义
2. 被动类：`Analyzer / Verifier / Worker`
   - 面向固定系统任务
   - 默认不走通用 tool use
   - 优先使用结构化输入、固定 prompt、确定性后处理
   - UI 上不必强行叫“智能体”

## 3. 当前主要问题

### 3.1 产品名实不符

现状：

- 页面名称是“智能体库”
- 实际页面只承载 System Agent 配置与运行查看
- 新增真正新类型 Agent 仍需改前端 registry 和后端 prompt 分支

影响：

- 用户会误以为这里可以像插件库一样新增和管理 Agent 资产
- 实际只能编辑少数内置 Agent 的参数

### 3.2 配置字段存在但不生效

现状：

- `input_schema`
- `output_schema`
- `trigger_events`
- `budget`

这些字段已经进入存储模型，但运行时主链路并未完整消费。

影响：

- UI 显示的配置不等于真实执行约束
- Profile 失去“自描述”意义

### 3.3 同一 Profile 在不同入口下表现不一致

现状：

- `workflow_designer_agent`
- `traffic_plugin_generator_agent`
- `plugin_fix_agent`

走的是外部任务入口

- `traffic_logic_triage`
- `traffic_active_verifier`

走的是 `SystemAgentRuntime`

问题：

- 外部入口会补虚拟工具上下文
- Runtime 主链路不会补同类上下文

影响：

- 同一个 Profile 的工具能力与上下文能力并不稳定

### 3.4 被动 Agent 的调试入口失真

现状：

- 手动运行被动 Agent，只能得到 run 结果
- 不会走完整 finding 持久化和后续 verifier 链路

影响：

- “调试成功”不代表“真实自动链路成功”

### 3.5 版本体系偏噪声化

现状：

- 自动保存频繁
- 每次保存都写 profile version

影响：

- 版本记录更像输入日志
- 不利于审查、比较、回滚、发布

## 4. 目标定义

整改后的系统应分成两类目标，而不是继续维护单一“Agent 库”幻觉。

### 4.1 主动类 Agent 的目标定义

任意主动类 Agent Profile 至少应完整定义：

- 身份：`id/name/description`
- 类型：`mode/capability/trigger_mode`
- 输入输出契约：`input_schema/output_schema`
- 提示词契约：`base_prompt/prompt_patch`
- 工具契约：`required/optional/forbidden`
- 运行约束：`budget/cooldown/concurrency/safety`
- 事件契约：`bindings/trigger_events`

### 4.2 被动类 Analyzer / Worker 的目标定义

任意被动类能力至少应完整定义：

- 输入事件契约
- 固定 prompt 或固定策略
- 输出结构
- 安全边界
- 队列 / 节流 / 重试 / 死信策略
- finding / evidence 持久化策略

被动类能力不强制要求：

- 通用 tool use
- 通用 agent executor
- 多轮自主规划

### 4.3 单一执行语义

无论从哪里触发同一个 Profile：

- 使用同一套 prompt 合成规则
- 使用同一套工具注入规则
- 使用同一套输入校验规则
- 使用同一套输出规范化规则

### 4.4 可治理

至少具备：

- 草稿与发布
- 版本对比
- 回滚
- 运行可追踪
- 失败可诊断

## 5. 分阶段整改方案

## Phase 1：抽象纠偏

目标：

- 先停止把所有系统 AI 能力都建模为 Agent

### 5.1 明确双轨模型

需要做：

- 主动类继续保留 `Agent Profile`
- 被动类改称：
  - `Passive Analyzer`
  - `Verifier Worker`
  - 或 `System Worker`
- UI 文案、文档、接口注释统一修正

验收标准：

- 文档和页面中不再混淆“主动智能体”和“被动系统分析器”

## Phase 2：平台契约纠偏

目标：

- 消除“配置看起来存在，但运行时不生效”的问题

### 5.2 统一运行前校验

需要做：

- 在执行前对 `input_schema` 做校验
- 对输出做 JSON 结构校验
- 对不符合 schema 的结果做标准化错误返回

涉及文件：

- `src-tauri/src/services/system_agents/runtime.rs`
- `src-tauri/src/services/system_agents/prompts.rs`
- `src-tauri/src/commands/ai_system_agent_support.rs`

验收标准：

- 手动触发和事件触发都能执行同一套 schema 校验
- schema 配置错误时，UI 能看到明确报错

### 5.3 预算字段真正生效

需要做：

- 定义 `budget_json` 的正式结构
- 先落地最小可用项：
  - `maxRunsPerHour`
  - `maxTokensPerRun`
  - `maxFailuresPerHour`

涉及文件：

- `src-tauri/src/services/system_agents/runtime.rs`
- `src-tauri/src/commands/system_agent_commands.rs`

验收标准：

- 超预算时 run 被拒绝或排队
- run 记录中可见预算拒绝原因

### 5.4 主动类才保留 tool use，且统一注入

需要做：

- 只对主动类 Agent 保留 tool use 相关抽象
- 把“虚拟工具”从外部入口特例改成主动类通用机制
- 被动类默认不走通用工具注入，除非明确证明需要

涉及文件：

- `src-tauri/src/commands/ai_system_agent_support.rs`
- `src-tauri/src/services/system_agents/runtime.rs`
- `src-tauri/src/services/system_agents/tool_policy.rs`

验收标准：

- 主动类 Profile 无论从设置页、工作流生成还是插件修复触发，虚拟工具上下文一致
- 被动类默认不再承诺通用 tool use 语义

### 5.5 被动分析器调试链路对齐

需要做：

- 新增“模拟事件回放”调试模式
- 被动 Agent 调试默认走事件驱动链路，而不是简化直跑链路

涉及文件：

- `src/components/Settings/system-agent/SystemAgentDebugPanel.vue`
- `src/components/Settings/system-agent/useSystemAgentSettingsController.ts`
- `src-tauri/src/commands/system_agent_commands.rs`
- `src-tauri/src/services/system_agents/runtime.rs`

验收标准：

- 调试被动分析器时可以选择：
  - 仅运行模型
  - 模拟完整事件链路
- 模拟完整链路时能落 finding 和派发后续事件

## Phase 3：主动类 Agent 库化改造

目标：

- 只把真正需要 Agent 抽象的主动类能力做成库

### 5.6 去除基于固定 profile id 的前端硬编码

需要做：

- 把 `systemAgentRegistry.ts` 中的大量 ID 分支改成：
  - 通用 metadata 渲染
  - 少量 capability adapter 扩展

建议模型：

- `profile metadata`
- `capability adapter`
- `ui schema`

涉及文件：

- `src/components/Settings/system-agent/systemAgentRegistry.ts`
- `src/components/Settings/SystemAgentSettings.vue`
- `src/components/Settings/system-agent/useSystemAgentSettingsController.ts`

验收标准：

- 新增一个普通 active/passive profile 时，不需要再改 registry 主体

### 5.7 Prompt 解析从“固定分支”升级为“可配置模板”

需要做：

- `resolve_base_prompt` 不再只靠 profile id switch
- 引入 prompt 模板注册层
- Profile 引用模板时走模板仓库

涉及文件：

- `src-tauri/src/services/system_agents/prompts.rs`
- 后续可新增 `src-tauri/src/services/system_agents/prompt_templates.rs`

验收标准：

- 新增 profile 可通过配置绑定 prompt template
- 不必再修改 `prompts.rs` 主分支

### 5.8 增加真正的 Agent 资产操作

需要做：

- 新建主动类 Agent Profile
- 克隆主动类 Agent Profile
- 删除主动类 Agent Profile
- 草稿保存
- 发布启用

当前后端已有部分接口，但前端没有完成资产化操作面板。

涉及文件：

- `src/components/Settings/system-agent/SystemAgentToolbarPanel.vue`
- `src/components/Settings/system-agent/SystemAgentListPanel.vue`
- `src/components/Settings/system-agent/useSystemAgentSettingsController.ts`

验收标准：

- 用户可从 UI 新建和克隆 profile
- 系统默认 profile 与用户自定义 profile 区分明确

## Phase 4：治理与可运营性

目标：

- 从“能跑”升级为“可维护、可审计、可迭代”

### 5.9 版本治理升级

需要做：

- 区分自动保存版本与发布版本
- 增加版本标签
- 增加版本 diff
- 支持回滚到指定版本

涉及文件：

- `src-tauri/sentinel-db/src/database_service/system_agent.rs`
- `src/components/Settings/system-agent/SystemAgentVersionsPanel.vue`

验收标准：

- 能清晰看到：
  - 谁改了什么
  - 哪个版本在运行
  - 能否回滚

### 5.10 SOP 与运营知识持久化

现状问题：

- SOP 现在保存在前端 localStorage

需要做：

- 改为后端持久化
- 支持团队共享
- 与 finding 命中、误报反馈关联

涉及文件：

- `src/components/Settings/system-agent/systemAgentSopCatalog.ts`
- `src/components/Settings/system-agent/SystemAgentSopInsightsPanel.vue`
- 需要新增后端持久化接口

### 5.11 反馈闭环

需要做：

- 误报反馈反向影响 profile 迭代
- 版本发布前展示：
  - 最近误报率
  - 最近验证成功率
  - 近窗口执行失败率

涉及文件：

- `src/components/SecurityCenter/VulnerabilitiesPanel.vue`
- `src/components/Settings/system-agent/SystemAgentStatsCards.vue`
- `src/components/Settings/system-agent/SystemAgentRecentFindingsPanel.vue`

## 6. 推荐实施优先级

### P0：必须先做

1. 明确主动 / 被动双轨模型
2. 主动类统一虚拟工具注入
3. 补齐 schema 校验
4. 补齐预算执行
5. 修正被动类调试链路

原因：

- 这是运行时契约问题
- 不解决就继续扩功能，会放大系统不一致

### P1：紧接着做

1. 去除前端按 profile id 硬编码
2. Prompt 模板化
3. 主动类 Agent 的新建/克隆/发布

原因：

- 这一步做完，才能勉强称为“库”

### P2：后续增强

1. 版本治理升级
2. SOP 持久化
3. 反馈驱动的运营闭环

## 7. 最小可交付方案

如果只做一轮 MVP，建议范围如下：

### MVP 范围

- 明确被动类不再默认视为真 Agent
- 主动类 Runtime 与外部入口共用虚拟工具注入器
- `input_schema/output_schema` 生效
- `budget_json` 至少支持 `maxRunsPerHour`
- 被动分析器调试增加“模拟完整链路”
- 前端支持克隆主动类 profile

### MVP 完成后的结果

- 智能体库仍然不是完全体
- 但已经从“配置台”升级到“可复用 Profile 平台雏形”

## 8. 暂不建议现在做的事

以下事项不建议现在优先：

- 继续新增更多被动“agent”类型
- 继续扩更多 passive 事件种类
- 继续堆更多 prompt patch 面板能力

原因：

- 基础契约还未统一
- 先扩能力只会增加维护复杂度

## 9. 下一步实施建议

建议直接进入第一轮工程整改，顺序如下：

1. 先把主动类 / 被动类抽象彻底分开
2. 为主动类抽出统一的 virtual tool context builder
3. 在 runtime 中接入 input/output schema 校验
4. 在 runtime 中接入预算执行
5. 改造被动类调试页，增加“模拟完整事件链路”
6. 再开始处理前端 registry 去硬编码

## 10. 参考文件

- `src/views/AgentManagement.vue`
- `src/components/Settings/SystemAgentSettings.vue`
- `src/components/Settings/system-agent/systemAgentRegistry.ts`
- `src/components/Settings/system-agent/useSystemAgentSettingsController.ts`
- `src-tauri/src/services/system_agents/runtime.rs`
- `src-tauri/src/services/system_agents/prompts.rs`
- `src-tauri/src/services/system_agents/tool_policy.rs`
- `src-tauri/src/commands/system_agent_commands.rs`
- `src-tauri/src/commands/ai_system_agent_support.rs`
- `src-tauri/sentinel-db/src/database_service/system_agent.rs`
