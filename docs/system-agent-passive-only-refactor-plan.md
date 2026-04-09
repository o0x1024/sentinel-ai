# System Agent 被动化改造方案

更新时间：2026-04-08

## 1. 决策

建议采纳以下产品与架构决策：

- 删除“主动 Agent”在 `system_agents` 体系中的存在
- `system_agents` 只保留后台、事件驱动、受运行时约束的被动智能体
- 不再在 `system_agents` 内部区分“主动/被动”
- 原主动能力迁回各自业务域，改名为：
  - AI 任务
  - AI 生成器
  - Prompt 工具
  - Assistant Action

结论：

以后 `System Agent = 被动系统智能体`

## 2. 为什么这样更合理

从第一性原理看，真正的 Agent 至少需要具备：

- 后台运行能力
- 事件触发能力
- 预算、冷却、并发等运行约束
- 安全策略与执行边界
- 结果能进入系统闭环，而不仅是返回一次性文本

当前仓库中真正满足以上条件的，是：

- `traffic_logic_triage`
- `traffic_active_verifier`

而这些“主动 Agent”：

- `workflow_designer_agent`
- `traffic_plugin_generator_agent`
- `plugin_fix_agent`
- `manual_traffic_audit_agent`

本质上只是手动触发的一次性 AI 任务，不应继续挂在 System Agent 抽象下。

## 3. 改造后的边界定义

## 3.1 保留在 `system_agents` 的能力

仅保留：

- 被动 triage
- 被动 verifier
- 后续可能新增的被动 dedup/prioritizer/monitor 类智能体

判断标准：

- 必须由事件触发
- 必须走 runtime 调度
- 必须受预算/并发/冷却/安全策略约束
- 必须有系统级闭环输出

## 3.2 移出 `system_agents` 的能力

迁出：

- 工作流生成
- 插件生成
- 插件修复
- 手动流量审计

这些能力应该进入各自业务域：

- 工作流设计域
- 插件生成域
- 安全助手域

它们可以继续使用 LLM、prompt、上下文注入，甚至继续复用 agent executor。
但它们不再共享 `SystemAgentProfile` 这套系统智能体模型。

## 4. 数据模型裁剪建议

## 4.1 `system_agent_profiles` 未来只服务被动智能体

建议保留字段：

- `id`
- `name`
- `description`
- `capability`
- `enabled`
- `llm_provider_override`
- `llm_model_override`
- `base_prompt_id`
- `prompt_patch`
- `input_schema_json`
- `output_schema_json`
- `required_tools_json`
- `optional_tools_json`
- `forbidden_tools_json`
- `budget_json`
- `safety_policy_json`
- `cooldown_secs`
- `max_concurrency`
- `risk_level`
- `visibility`
- `created_at`
- `updated_at`

建议删除字段：

- `mode`
- `trigger_mode`
- `trigger_events_json`

原因：

- `mode` 永远都是被动
- `trigger_mode` 永远都是事件触发
- `trigger_events_json` 与 `bindings` 语义重复，真正生效的是 binding

## 4.2 `SystemAgentBinding` 继续保留

保留：

- `event_name`
- `filter_json`
- `priority`
- `enabled`

因为在纯被动模型下，binding 就是系统智能体的真实触发契约。

## 4.3 `SystemAgentRun` 保留

保留：

- `trigger_event`
- `status`
- `input_summary_json`
- `output_json`
- `error`
- `started_at`
- `finished_at`

因为运行记录仍然是 passive runtime 的核心观测面。

## 5. 前端裁剪建议

## 5.1 页面重命名

当前名称：

- 智能体库

建议改为：

- 系统智能体
- 被动智能体

更推荐：

- 系统智能体

原因：

- 对用户来说更自然
- 不必再在文案层面强调“主动/被动”
- 技术上实际只剩被动系统 worker

涉及文件：

- `src/views/AgentManagement.vue`
- `src/main.ts`
- `src/i18n/locales/sidebar/zh.ts`
- `src/i18n/locales/agents/zh.ts`
- 其他相关 i18n 文案

## 5.2 删除主动 Agent 的 UI registry 定义

可删除的前端定义：

- `manual_traffic_audit_agent`
- `workflow_designer_agent`
- `traffic_plugin_generator_agent`
- `plugin_fix_agent`

保留：

- `traffic_logic_triage`
- `traffic_active_verifier`

涉及文件：

- `src/components/Settings/system-agent/systemAgentRegistry.ts`

## 5.3 调试面板只保留事件调试

当前调试面板同时支持：

- 手动运行
- 派发测试事件

在纯被动模型下，建议改为：

- 派发测试事件
- 模拟完整链路
- 查看最近队列/运行状态

建议删除：

- “手动运行”按钮
- 与主动任务相关的手动输入模板语义

涉及文件：

- `src/components/Settings/system-agent/SystemAgentDebugPanel.vue`
- `src/components/Settings/system-agent/useSystemAgentSettingsController.ts`

## 5.4 删掉与主动任务相关的面板语义

保留：

- 工具绑定
- 安全策略
- 高级设置
- 运行记录
- 最近发现
- 行为上下文

弱化或重构：

- Prompt patch 面板仍可保留
- 但文案应明确这是“后台被动分析策略补充”

## 6. 后端裁剪建议

## 6.1 删除主动 Agent 种子

应从默认种子中移除：

- `manual_traffic_audit_agent`
- `workflow_designer_agent`
- `traffic_plugin_generator_agent`
- `plugin_fix_agent`

保留：

- `traffic_logic_triage`
- `traffic_active_verifier`

涉及文件：

- `src-tauri/src/services/system_agents/seed.rs`

## 6.2 删除对主动/被动二分的运行时判断依赖

当前 runtime 中很多地方有：

- `profile.mode != "passive"`
- `profile.mode == "passive"`

在纯被动模型下，可逐步改为：

- 默认所有 system agent 都是 runtime passive worker

涉及文件：

- `src-tauri/src/services/system_agents/runtime.rs`

## 6.3 删除对主动类 Profile 的命令入口耦合

当前以下业务入口仍借用 system agent profile：

- 工作流生成
- 插件生成
- 插件修复

这些入口应从 `SystemAgentProfile` 脱钩，改为独立配置源。

涉及文件：

- `src-tauri/src/commands/ai.rs`
- `src-tauri/src/services/system_agents/plugin_fix.rs`
- `src-tauri/src/commands/ai_system_agent_support.rs`

## 6.4 将外部主动能力重命名为 Task/Generator

建议重构命名：

- `workflow_designer_agent` -> `workflow_designer_task`
- `traffic_plugin_generator_agent` -> `traffic_plugin_generator_task`
- `plugin_fix_agent` -> `plugin_fix_task`
- `manual_traffic_audit_agent` -> `manual_traffic_audit_task`

注意：

- 这一步未必需要立即改所有代码标识
- 但产品与配置层必须先脱离 `SystemAgentProfile`

## 7. 迁移后的业务结构建议

## 7.1 新结构

建议形成两条清晰主线：

### A. System Agents

定位：

- 系统级被动智能体

特点：

- 事件驱动
- 有预算与安全策略
- 有系统闭环

### B. AI Tasks

定位：

- 用户显式触发的一次性 AI 能力

特点：

- 以 prompt 为主
- 可选少量上下文提供器
- 不进入 system runtime 调度模型

## 7.2 不建议继续共享同一张 Profile 表

原因：

- passive runtime worker 与 one-shot prompt task 的字段需求差异太大
- 强行共享只会制造更多空字段、假字段和无效配置

建议：

- `system_agent_profiles` 只服务被动系统智能体
- 主动 AI 任务另建配置源，哪怕一开始只是业务内静态配置

## 8. 具体删改清单

## 8.1 第一批必须改

1. 页面和文案重命名
2. 种子删除主动 Agent
3. 前端 registry 删除主动 Agent 定义
4. 调试页去掉“手动运行”
5. 业务入口与 `SystemAgentProfile` 脱钩

## 8.2 第二批结构清理

1. 数据模型移除 `mode`
2. 数据模型移除 `trigger_mode`
3. 数据模型移除 `trigger_events_json`
4. runtime 移除主动/被动分支判断

## 8.3 第三批迁移收尾

1. 工作流生成改挂到独立 AI Task 配置
2. 插件生成改挂到独立 AI Task 配置
3. 插件修复改挂到独立 AI Task 配置
4. 文档与使用说明统一更新

## 9. 推荐实施顺序

### Step 1：产品与前端先纠偏

- 改名
- 去掉主动 Agent 的展示
- 只展示 passive profiles

原因：

- 先让用户看到的概念是对的

### Step 2：后端种子与业务依赖脱钩

- 删主动 Agent 种子
- 工作流/插件/修复入口不再从 system agent profile 读配置

原因：

- 避免页面删了但后端还继续把主动能力塞进这套模型

### Step 3：数据库与模型裁剪

- 去掉冗余字段
- 迁移数据
- 清理 runtime 判断逻辑

原因：

- 这是破坏性更强的一步，放在依赖脱钩之后更稳

## 10. 风险

### 10.1 风险一：工作流生成和插件修复暂时失去可配 prompt patch

解决方式：

- 在各自业务域建立最小配置入口
- 不必继续借用 system agent profile

### 10.2 风险二：历史 profile 数据需要迁移

解决方式：

- 先隐藏主动 profile
- 再做数据库清理

### 10.3 风险三：部分代码仍使用旧 profile id

解决方式：

- 先脱钩配置读取
- 后续再逐步改内部常量名

## 11. 最终建议

建议直接采纳以下原则：

- `System Agent` 只表示被动系统智能体
- 主动一次性 AI 能力不再叫 Agent
- 不再维护“主动/被动 Agent 二元模型”
- 在 System Agent 域内默认只有一种：事件驱动后台智能体

这是最干净、最稳定、最符合当前代码真实运行方式的收敛方向。

## 12. 参考文件

- `src/views/AgentManagement.vue`
- `src/components/Settings/SystemAgentSettings.vue`
- `src/components/Settings/system-agent/systemAgentRegistry.ts`
- `src/components/Settings/system-agent/SystemAgentDebugPanel.vue`
- `src/components/Settings/system-agent/useSystemAgentSettingsController.ts`
- `src-tauri/src/services/system_agents/seed.rs`
- `src-tauri/src/services/system_agents/runtime.rs`
- `src-tauri/src/commands/ai.rs`
- `src-tauri/src/services/system_agents/plugin_fix.rs`
- `src-tauri/src/commands/ai_system_agent_support.rs`
