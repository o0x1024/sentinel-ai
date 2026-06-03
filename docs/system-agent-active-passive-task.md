# 系统 Agent 主动类/被动类改造任务文档

更新日期：2026-04-04

## 1. 背景与目标

当前仓库里已经存在多条与 AI 相关但彼此分散的能力：

- 工作流中的 AI 生成
- 插件管理中的 AI 生成插件/修复插件
- 流量分析中的自动插件扫描
- Agent 执行器、工具路由、执行日志与子 Agent 能力

这些能力本质上都属于“系统级 AI 能力”，但目前没有统一抽象：

- 有的是用户点击按钮手动触发
- 有的是后台事件驱动
- 有的是独立生成器
- 有的是普通 Agent 对话能力

本任务的目标是把这类能力统一为“系统 Agent”体系，并以顶层二分法分类：

- 主动类 Agent：用户手动触发
- 被动类 Agent：系统事件自动触发

最终效果：

- 用户可以在统一界面管理系统 Agent
- 工作流 AI 生成、插件 AI 生成等能力收敛到主动类 Agent
- 流量分析中的安全风险发现收敛到被动类 Agent
- prompt、工具绑定、预算、运行日志、执行状态统一管理

## 2. 顶层分类定义

### 2.1 主动类 Agent

定义：

- 由用户显式触发
- 用户知道它正在运行
- 允许用户在 UI 中编辑输入
- 适合生成、设计、修复、手动审计类任务

首批归类：

- `workflow_designer_agent`
- `workflow_optimizer_agent`
- `traffic_plugin_generator_agent`
- `agent_plugin_generator_agent`
- `plugin_fix_agent`
- `manual_traffic_audit_agent`

### 2.2 被动类 Agent

定义：

- 由系统事件自动触发
- 后台无感执行
- 必须有预算、节流、冷却、并发限制
- 输出风险标记、假设、验证任务或自动发现

首批归类：

- `traffic_logic_triage`
- `traffic_active_verifier`
- `finding_dedup_agent`
- `finding_prioritizer_agent`

## 3. 当前能力映射

### 3.1 需要迁入主动类 Agent 的现有能力

- 工作流 AI 生成
- 插件管理中的 AI 生成插件
- 插件失败后的 AI 修复

### 3.2 需要迁入被动类 Agent 的现有能力

- 流量分析中的越权/逻辑漏洞自动分诊
- 流量分析中的主动验证
- findings 去重、聚合、优先级判断

### 3.3 可复用的现有底座

- Agent 执行器：`src-tauri/src/agents/`
- 工具路由：`src-tauri/src/agents/tool_router.rs`
- HTTP 网关中的 Agent 执行入口：`src-tauri/src/services/http_gateway.rs`
- AI 服务管理：`src-tauri/src/services/ai_manager.rs`
- 流量扫描管线：`src-tauri/sentinel-traffic/src/scanner.rs`
- 插件扫描接口：`scan_transaction`

## 4. 统一数据模型

建议新增统一系统 Agent Profile。

### 4.1 SystemAgentProfile

字段建议：

- `id`
- `name`
- `mode`，值为 `active | passive`
- `capability`，值为 `designer | generator | triage | verifier | reviewer`
- `enabled`
- `trigger_mode`，值为 `manual | event | scheduled | hybrid`
- `trigger_events_json`
- `base_prompt_id`
- `prompt_patch`
- `input_schema_json`
- `output_schema_json`
- `required_tools_json`
- `optional_tools_json`
- `budget_json`
- `cooldown_secs`
- `max_concurrency`
- `risk_level`
- `visibility`
- `created_at`
- `updated_at`

### 4.2 SystemAgentRun

字段建议：

- `id`
- `profile_id`
- `trigger_event`
- `status`
- `input_summary_json`
- `output_json`
- `error`
- `started_at`
- `finished_at`

### 4.3 SystemAgentBinding

字段建议：

- `id`
- `profile_id`
- `event_name`
- `filter_json`
- `priority`
- `enabled`

## 5. 运行时设计

新增 `SystemAgentRuntime`，职责如下：

- 监听系统事件
- 根据 binding 匹配需要运行的系统 Agent
- 做预算控制、冷却控制、并发控制
- 构造输入 payload
- 调用现有 Agent 执行器
- 记录运行日志与产物
- 对被动类 Agent 提供后台 Worker 能力

注意：

- 被动类 Agent 是“以 Agent Profile 形式存在的系统 Worker”
- 不应该直接做成普通聊天 Agent
- 也不应该做成无法配置的硬编码后台线程

## 6. 事件模型

被动类 Agent 首批建议接入以下事件：

- `traffic.cluster.ready`
- `traffic.auth_context.changed`
- `traffic.workflow.detected`
- `traffic.passive_signal.created`
- `traffic.hypothesis.ready`

其中流量安全发现首批重点：

- `traffic.cluster.ready`
- `traffic.workflow.detected`
- `traffic.hypothesis.ready`

## 7. 流量分析中的被动类 Agent 方案

### 7.1 被动链路拆分

流量分析中的无感安全发现按三层执行：

1. 轻量被动画像
2. AI 分诊
3. 自动验证

### 7.2 轻量被动画像

放在 `ScanPipeline` 后：

- 提取身份指纹
- 提取资源边界字段
- 提取动作类型
- 提取响应特征
- 对事务做聚类

这层不调用 AI，只做快速确定性计算。

### 7.3 AI 分诊 Agent

首批核心被动类 Agent：

- `traffic_logic_triage`

职责：

- 从事务簇和行为上下文中生成 hypothesis
- 判断越权、流程异常、状态机问题与业务逻辑漏洞候选点
- 生成结构化测试计划

### 7.4 自动验证 Agent

首批：

- `traffic_active_verifier`

职责：

- 基于 hypothesis 执行小范围安全验证
- 调用重放链路
- 比较响应差异
- 生成证据
- 写入现有 `traffic_vuln`

## 8. 工作流与插件生成中的主动类 Agent 方案

### 8.1 工作流生成

主动类 Agent：

- `workflow_designer_agent`
- `workflow_optimizer_agent`

工具建议：

- 读取可用插件列表
- 读取插件输入输出 schema
- 校验 workflow 结构

输出：

- 结构化 workflow JSON

### 8.2 插件生成

主动类 Agent：

- `traffic_plugin_generator_agent`
- `agent_plugin_generator_agent`
- `plugin_fix_agent`

工具建议：

- 读取模板
- 读取 few-shot 示例
- 运行校验器
- 运行静态测试

输出：

- 插件代码
- 元数据
- 修复建议或修复后的代码

## 9. 权限与安全策略

### 9.1 主动类 Agent

- 可以开放更多工具
- 允许用户补充 prompt patch
- 允许手动输入任务参数

### 9.2 被动类 Agent

- 默认不允许自由修改 base prompt
- 必须具备预算限制
- 必须具备冷却时间
- 必须具备并发限制
- 默认只能使用白名单工具
- 主动验证类 Agent 必须额外受目标范围限制

## 10. 前端产品形态

新增统一页面：`System Agents`

页面分三部分：

### 10.1 Profiles

- Agent 列表
- 分类
- 启用状态
- 最近运行
- 最近错误

### 10.2 Configuration

- prompt patch
- 工具绑定
- 预算
- 冷却
- 并发
- 事件绑定

### 10.3 Runs

- 运行日志
- 输入摘要
- 输出摘要
- 耗时
- 错误
- 产物

同时在业务页面做轻量联动：

- 工作流生成入口改走主动类 Agent
- 插件管理中的 AI 生成入口改走主动类 Agent
- 流量记录与漏洞面板显示被动类 Agent 产生的风险结果

## 11. 分阶段任务拆分

### Phase 1：系统 Agent 数据模型与存储

- [ ] 新增 `system_agent_profiles`
- [ ] 新增 `system_agent_bindings`
- [ ] 新增 `system_agent_runs`
- [ ] 新增必要 migration
- [ ] 新增 Rust 类型定义与 DB 仓储读写链路

### Phase 2：SystemAgentRuntime

- [ ] 新增系统 Agent 运行时
- [ ] 接入现有 agent 执行器
- [ ] 接入工具白名单与预算控制
- [ ] 实现 manual/event 两种 trigger mode

### Phase 3：主动类 Agent 首批迁移

- [ ] 工作流 AI 生成功能迁移到 `workflow_designer_agent`
- [ ] 插件 AI 生成功能迁移到 `traffic_plugin_generator_agent`
- [ ] 插件修复迁移到 `plugin_fix_agent`
- [ ] 保持旧入口 UI 不变，仅替换底层执行路径

### Phase 4：被动类 Agent 首批接入流量分析

- [ ] 在 `ScanPipeline` 后新增事务画像与聚类
- [ ] 发出 `traffic.cluster.ready`
- [ ] 接入 `traffic_idor_triage`
- [ ] 接入 `traffic_logic_triage`

### Phase 5：被动类主动验证闭环

- [ ] 新增 `traffic_active_verifier`
- [ ] 接入重放与响应差异验证
- [ ] 将结果写入现有 `traffic_vuln`
- [ ] 前端漏洞面板显示来源为被动类 Agent

### Phase 6：前端系统 Agent 管理页

- [ ] 新增 `System Agents` 列表页
- [ ] 新增 Profile 配置页
- [ ] 新增 Runs 日志页
- [ ] 展示主动类/被动类分类

## 12. 文件落地建议

### 12.1 后端 Rust

建议新增目录：

- `src-tauri/src/agents/system_profiles/`
- `src-tauri/src/agents/system_runtime/`
- `src-tauri/src/agents/system_events/`

建议新增文件：

- `src-tauri/src/agents/system_profiles/types.rs`
- `src-tauri/src/agents/system_profiles/repository.rs`
- `src-tauri/src/agents/system_runtime/runtime.rs`
- `src-tauri/src/agents/system_runtime/dispatcher.rs`
- `src-tauri/src/agents/system_runtime/budget.rs`
- `src-tauri/src/agents/system_runtime/cooldown.rs`
- `src-tauri/src/agents/system_events/traffic_events.rs`
- `src-tauri/src/agents/system_events/workflow_events.rs`

流量侧建议新增：

- `src-tauri/src/services/traffic_ai/context_fingerprint.rs`
- `src-tauri/src/services/traffic_ai/behavior_clustering.rs`
- `src-tauri/src/services/traffic_ai/hypothesis_builder.rs`
- `src-tauri/src/services/traffic_ai/verification_scheduler.rs`

### 12.2 前端 Vue/TS

建议新增页面与组件：

- `src/views/SystemAgentsView.vue`
- `src/components/SystemAgents/SystemAgentList.vue`
- `src/components/SystemAgents/SystemAgentProfileEditor.vue`
- `src/components/SystemAgents/SystemAgentRunsPanel.vue`

建议新增类型与 composable：

- `src/types/systemAgent.ts`
- `src/composables/useSystemAgents.ts`

### 12.3 文档

- [x] 新增本任务文档
- [ ] 后续补充运行时设计文档
- [ ] 后续补充被动类 Agent 风险控制说明

## 13. 验收标准

### 13.1 主动类 Agent

- 工作流 AI 生成可通过统一系统 Agent Profile 执行
- 插件 AI 生成可通过统一系统 Agent Profile 执行
- 插件修复可通过统一系统 Agent Profile 执行
- 用户可查看运行日志与 prompt patch

### 13.2 被动类 Agent

- 系统可在无用户交互时自动触发流量分诊
- 越权/逻辑漏洞 hypothesis 可写入中间结果
- 主动验证成功后可写入现有漏洞发现面板
- 不阻塞主抓包链路

### 13.3 统一管理

- 系统 Agent 页面可看到主动类与被动类 Agent
- 用户可启停 Agent
- 用户可编辑允许修改的配置项
- 运行日志可追踪

## 14. 当前推荐实施顺序

优先级建议：

1. Phase 1
2. Phase 2
3. Phase 3
4. Phase 4
5. Phase 6
6. Phase 5

理由：

- 先统一模型与运行时
- 再把已有主动类能力迁进去，快速产生收益
- 再接流量被动类 Agent
- 最后补主动验证闭环

## 15. 多角色补充分析

### 15.1 产品经理视角

当前方案还需要补齐产品闭环，而不仅仅是技术架构。

建议补充：

- 风险对象分层：
  - `风险线索`
  - `待验证假设`
  - `已验证漏洞`
- 被动类 Agent 打扰策略：
  - 静默标记
  - 弱提醒
  - 强告警
- 可解释性展示：
  - 为什么被判定为可疑
  - 基于哪些流量
  - 做了哪些自动验证
  - 当前置信度来源
- 运营指标：
  - 发现率
  - 误报率
  - 自动验证成功率
  - 平均发现耗时
  - 平均 token 消耗
  - 平均每个 Agent 成本

### 15.2 安全架构师视角

当前方案还需要补齐权限边界与安全控制。

建议补充：

- Agent 权限分层：
  - 只读
  - 可重放
  - 可变异请求
  - 可并发主动验证
- 目标授权控制：
  - 只允许对白名单目标执行主动验证
  - 支持 project/workspace/host/path 级别 scope guard
- Prompt 注入与数据投毒防护：
  - HTTP 响应内容视为不可信输入
  - 不允许把响应正文直接当系统指令
- 证据链要求：
  - 原始事务
  - 聚类结果
  - hypothesis
  - 验证步骤
  - 最终结论
- 插件供应链治理：
  - 插件签名/审批
  - 版本锁定
  - 风险工具禁用策略

### 15.3 渗透测试工程师视角

当前方案还需要补齐真实攻防场景里的“跨事务分析”和“可验证性”。

建议补充：

- 上下文建模：
  - 身份画像
  - 角色切换
  - 租户上下文
  - 业务状态机
- 变异模板库：
  - 对象 ID 替换
  - token/session 交换
  - 参数缺失
  - 重复提交
  - 顺序颠倒
  - 并发双发
- 低误报主动验证门槛：
  - 哪些 hypothesis 可自动验证
  - 哪些仅标记待人工复核
- 结果聚合与去重：
  - 同根因漏洞合并
  - 同接口多命中聚合
- 人工接管能力：
  - hypothesis 一键转手动审计
  - 自动验证失败后一键送 Repeater/Intruder

### 15.4 QA 专家视角

当前方案还需要补齐测试基线、影子模式与回归体系。

建议补充：

- 基准样本集：
  - 正常业务样本
  - 已知越权样本
  - 已知逻辑漏洞样本
  - 高噪音无漏洞样本
- 回归测试分层：
  - Prompt 回归
  - Tool binding 回归
  - Schema 回归
  - 插件输出回归
  - UI 呈现回归
- 影子模式：
  - 被动类 Agent 先只运行不对用户显示
  - 收集误报/漏报后再正式开启
- 稳定性测试：
  - 高频流量
  - 大历史记录
  - AI 超时
  - 插件失败
  - 重放失败
- 可复现性：
  - 同样输入 + 同样 profile 版本 + 同样 prompt 版本时结果尽量稳定

## 16. 建议新增的核心模块

### 16.1 风险生命周期

建议新增统一风险状态：

- `suspicious`
- `hypothesis`
- `verifying`
- `verified`
- `dismissed`

说明：

- 被动类 Agent 首先产出 `suspicious/hypothesis`
- 自动验证成功后转为 `verified`
- 人工确认误报后转为 `dismissed`

### 16.2 Agent Profile Versioning

建议对以下内容显式版本化：

- base prompt
- prompt patch
- tool binding
- input schema
- output schema

目的：

- 保证结果可追踪
- 支持回归测试
- 支持误报定位

### 16.3 Shadow Mode

建议被动类 Agent 支持影子模式：

- 正常执行
- 正常写运行日志
- 不对用户产生告警
- 可在内部面板中查看效果

### 16.4 Evidence Graph

建议新增证据图关联：

- request/response
- cluster
- hypothesis
- verification run
- vulnerability/finding

目的：

- 可解释
- 可追溯
- 可回放

### 16.5 Scope Guard

主动验证前必须做目标范围校验：

- workspace
- project
- target
- host
- path

### 16.6 Evaluation Suite

建议建立专门评估集：

- 越权评估集
- 逻辑漏洞评估集
- 正常业务对照集
- 高噪音负样本集

### 16.7 Feedback Loop

建议支持用户反馈：

- 标记误报
- 标记确认
- 标记风险等级过高/过低

后续可用于：

- prompt patch 优化
- tool 选择优化
- hypothesis 过滤优化

## 17. 增补任务看板

### P0：必须补齐

- [ ] 风险生命周期模型
- [ ] 目标授权控制（scope guard）
- [ ] 证据链与 evidence graph
- [ ] 被动类 Agent 的 shadow mode
- [ ] 被动类 Agent 的预算/冷却/并发控制
- [ ] Prompt 注入与不可信流量输入边界

### P1：高优先级

- [ ] Agent profile versioning
- [ ] 评估基线与自动评分
- [ ] 变异模板库
- [ ] 结果聚合与去重
- [ ] hypothesis 一键转人工审计
- [ ] 主动验证结果与漏洞面板联动增强

### P2：增强项

- [ ] 用户反馈学习闭环
- [ ] 成本/效果分析面板
- [ ] Agent 误报率趋势统计
- [ ] 更细粒度的工具权限编排
- [ ] 更灵活的系统 Agent 可视化配置

## 18. 推荐实施调整

在原先的实施顺序基础上，建议调整为：

1. Phase 1
2. Phase 2
3. P0 风险控制基础设施
4. Phase 3
5. Phase 4
6. Phase 6
7. Phase 5
8. P1 评测与去重
9. P2 反馈与优化

原因：

- 如果没有 P0，系统 Agent 尤其是被动类 Agent 很容易造成误报、越权主动探测或不可解释结果
- 先补风险控制基础设施，再接流量被动分析，落地更稳

## 19. 当前方案不足与整改方案

结合当前实现状态与后续目标，当前方案的主要不足和对应整改方向如下。

### 19.1 行为语义层缺失

不足：

- 当前主要依赖代理流量和从流量里反推的 `authContext / principalContext / recentSequence / clusterSummary`
- 系统看到的是“请求发生了什么”，不是“用户做了什么”
- 对审批、支付、退款、工单流转等强流程场景，语义信息仍然不足

整改方案：

- 行为特征采集采用双层模式，而不是单一路径：
  - `L1 默认模式`：代理侧弱行为推断
  - `L2 增强模式`：浏览器扩展行为采集
- 默认情况下，系统必须能够仅依赖代理流量完成逻辑漏洞分析，不把浏览器行为采集作为前置依赖
- 浏览器扩展只作为增强语义输入，用于提高复杂场景下的流程理解和检出率
- 新增 `BehaviorSession` 抽象，把同一用户、同一页面、同一时间窗口内的行为和流量组织成一个会话
- 第一阶段先支持“弱行为模式”：
  - 即使没有浏览器埋点，也按时间窗口、身份上下文、路径变化和流量序列构造近似行为链
- 第二阶段再接入真实浏览器行为事件：
  - 页面 URL
  - tab/frame
  - 点击动作
  - 输入字段
  - 提交动作

建议新增：

- `src-tauri/src/services/system_agents/behavior_session.rs`
- `src-tauri/src/services/system_agents/behavior_linker.rs`

前端产品要求：

- 在 Agent 管理或流量分析设置中，提供明显的行为特征来源切换方式
- 至少支持：
  - `默认：代理侧弱行为推断`
  - `增强：浏览器扩展行为采集`
- UI 必须明确说明：
  - 默认模式无需安装扩展
  - 增强模式需要浏览器扩展支持
  - 增强模式是可选增强，而不是系统正常工作的必要条件

### 19.2 triage 与 verifier 之间缺少结构化验证计划

不足：

- 当前 `traffic_logic_triage -> finding -> traffic_active_verifier` 已通
- 但 verifier 更像“收到 finding 后做一次最小安全重放”
- 对逻辑漏洞来说，缺少结构化测试计划和多策略验证

整改方案：

- 在 triage 输出中新增 `verificationPlan`
- 计划内容至少包括：
  - 目标请求或请求组
  - 变异策略
  - 身份切换策略
  - 顺序/跳步/重复/并发策略
  - 预期信号
- verifier 改为优先消费 `verificationPlan`，而不是只依赖 finding 首条证据
- 当前实现状态：
  - 已新增 `verificationPlan` 输出约束
  - 已新增 `src-tauri/src/services/system_agents/verification_plan.rs`
  - 已新增 `src-tauri/src/services/system_agents/verification_strategy.rs`
  - verifier 已优先按 `targetRequestId` 和 `verificationPlan` 选择基线请求
  - 已支持保守执行：
    - `replay_as_is`
    - `repeat_action`
    - `swap_identity`
    - `swap_resource_reference`
    - `skip_prerequisite`
    - `reorder_sequence`
    - `concurrent_submit`
  - `manual_review` 和更复杂的身份切换策略仍保留为后续项

建议新增：

- `src-tauri/src/services/system_agents/verification_plan.rs`
- `src-tauri/src/services/system_agents/verification_strategy.rs`

### 19.3 SOP/skill 尚未进入主链路

不足：

- 项目已有 skill 基础设施，但逻辑漏洞检测还没有把 skill 当成正式知识源
- 业务经验目前更多散落在 prompt patch 和实现细节里，不够工程化

整改方案：

- 把 SOP 做成 skill，而不是硬编码规则
- 但不直接让 Agent 毫无约束地自己猜 skill，而是采用：
  - 基础行为/流量归纳
  - 候选 skill 推荐
  - Agent 按需加载 skill
- skill 主要承载：
  - 场景说明
  - 关键不变量
  - 风险信号
  - 推荐测试策略
  - 输出格式要求

建议新增：

- `src-tauri/src/services/system_agents/skill_recommendation.rs`
- `src-tauri/src/services/system_agents/logic_skill_context.rs`

当前实现状态：

- 已新增候选 skill 推荐层
- 已新增 `logicSkillContext`
- 已接入 `traffic_logic_triage` payload：
  - `skillRecommendations`
  - `logicSkillContext`
- 已内置首批逻辑漏洞 skills：
  - `payment-flow`
  - `approval-workflow`
  - `resource-ownership`
  - `single-use-consumption`
- 当前仍属于“推荐 + 上下文增强”模式，尚未做真正的动态 skill 执行编排

### 19.4 逻辑漏洞检测仍偏“模型驱动”，缺少通用不变量层

不足：

- 现在 finding 是否落库仍高度依赖 triage 输出的 `riskType / confidence`
- 缺少业务无关的通用逻辑约束模型

整改方案：

- 增加通用不变量层，不直接判断“这是支付/审批”，而是先判断：
  - `ownership_invariant`
  - `state_prerequisite_invariant`
  - `single_use_invariant`
  - `role_separation_invariant`
  - `sequence_invariant`
  - `concurrency_invariant`
- triage 先消费“过程图 + 不变量命中”，再结合 skill 先验生成 hypothesis

建议新增：

- `src-tauri/src/services/system_agents/process_graph.rs`
- `src-tauri/src/services/system_agents/logic_invariants.rs`

### 19.5 runtime 可靠性仍偏轻量

不足：

- 当前 runtime 已有冷却、并发、排队，但仍是内存态调度
- 没有持久化任务队列、失败重试、死信箱和优先级治理

整改方案：

- 第二阶段把 passive triage / verifier 调度从纯内存队列升级为持久化任务模型
- 先支持：
  - 待执行
  - 运行中
  - 完成
  - 失败
  - 重试次数
- 再逐步补死信和优先级

建议新增：

- `src-tauri/src/services/system_agents/job_queue.rs`
- `src-tauri/src/services/system_agents/job_scheduler.rs`

### 19.6 证据链尚未覆盖“行为 -> 流量 -> finding -> 验证”

不足：

- 当前已能保存 triage context 和 verification evidence
- 但如果后续引入浏览器行为，目前还没有完整证据图谱

整改方案：

- 扩展 evidence graph，把以下节点关联起来：
  - 行为事件
  - 行为会话
  - 流量请求/响应
  - 过程图
  - skill 命中
  - hypothesis
  - verification run
  - finding

建议新增：

- `src-tauri/src/services/system_agents/evidence_graph.rs`

## 20. 下一阶段实施 Plan

### P0：行为与过程抽象

- [x] 新增 `BehaviorSession`
- [x] 支持弱行为模式的会话归纳
- [x] 明确行为特征来源分层：
  - [x] `L1 默认模式：代理侧弱行为推断`
  - [x] `L2 增强模式：浏览器扩展行为采集`
- [x] 新增 `process_graph`
- [x] 从流量画像和聚类生成过程图摘要

### P0.5：行为特征来源切换

- [x] 在前端提供明显的行为特征来源切换入口
- [x] 默认选中 `代理侧弱行为推断`
- [x] 浏览器扩展模式显示安装与连接状态
- [x] 在未启用扩展时，系统仍完整工作

### P1：通用逻辑约束层

- [ ] 新增 `logic_invariants`
- [ ] 定义首批通用不变量：
  - [ ] 归属约束
  - [ ] 前置状态约束
  - [ ] 角色分离约束
  - [ ] 单次消费约束
  - [ ] 顺序约束
  - [x] 并发约束
- [x] triage 输入接入不变量命中结果

### P2：SOP skill 接入

- [x] 新增候选 skill 推荐层
- [x] skill 推荐输入接入行为链和过程图
- [x] triage 支持“候选 skill 列表”
- [x] 首批新增逻辑漏洞 SOP skills：
  - [x] `payment-flow`
  - [x] `approval-workflow`
  - [x] `resource-ownership`
  - [x] `single-use-consumption`

### P3：结构化验证计划

- [x] triage 输出 `verificationPlan`
- [x] verifier 消费结构化计划而不是只看首条证据
- [x] 首批验证动作支持：
  - [x] 改参
  - [x] 换身份
  - [x] 跳步
  - [x] 重复提交
  - [x] 乱序
  - [x] 并发测试

### P4：运行时可靠性增强

- [x] passive triage/verifier 队列持久化
- [x] 失败重试机制
- [x] 死信与错误归因
- [x] 调度优先级治理

### P5：证据图与产品化展示

- [x] evidence graph 覆盖行为 -> 流量 -> finding -> 验证
- [x] 安全中心展示命中的 skills
- [x] 安全中心展示过程图与不变量命中
- [x] Agent 管理页展示 skill 使用效果和命中统计

## 21. 调整后的总实施顺序

建议整体顺序调整为：

1. 已完成的系统 Agent 底座继续保持
2. P0 行为与过程抽象
3. P0.5 行为特征来源切换
4. P1 通用逻辑约束层
5. P2 SOP skill 接入
6. P3 结构化验证计划
7. P4 runtime 可靠性增强
8. P5 证据图与产品展示

原因：

- 先把“行为”和“过程”抽象出来，才能让逻辑漏洞检测脱离单纯的请求级判断
- 先明确默认行为来源和增强来源的切换方式，才能避免后续实现被浏览器扩展绑死
- 先有通用不变量，再引入 skill 先验，能避免 SOP 变成硬编码规则
- verifier 只有消费结构化计划，才能真正从“最小重放”升级为“逻辑漏洞验证器”
