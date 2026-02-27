# Team 并入主链任务文档

## 1. 背景与目标

- 目标：将 Agent Team 能力并入 AI 助手主链路，不再依赖独立入口。
- 要求：
  - 不破坏现有 AI 助手主链路。
  - 用户可在输入区打开 Team 模式。
  - Team 支持用户可编排的串行/并行执行。
  - 白板承载共享记忆（摘要），原始输出可归档并检索。

## 2. 分阶段任务拆分

### Phase 1: 数据模型与存储扩展（串行/并行编排基础）

- [x] Team Session 增加编排计划字段（JSON）。
- [x] Team Session 增加计划版本字段。
- [x] 同步更新：
  - Rust `agent_team` 模型与请求结构。
  - 前端 `agentTeam` 类型定义。
  - PostgreSQL migration。
  - SQLite runtime schema 与兼容升级逻辑。
  - PostgreSQL/SQLite 仓储 create/get/update 查询链路。

### Phase 2: 主链路入口接入 Team 模式（InputArea 开关）

- [x] 在 `InputAreaComponent.vue` 增加 Team 开关按钮与事件。
- [x] 在 `AgentView.vue` 接收 Team 模式状态并路由发送逻辑。
- [x] 保持原 `agent_execute` 主链路不变（开关关闭时完全不受影响）。

### Phase 3: Team 运行视图并入 AgentView

- [x] 将模板库/白板/产物能力逐步并入 `AgentView`（而非独立 `AgentTeamView` 页面）。
- [x] 统一事件总线与状态管理，避免双套消息/会话状态。
- [x] 白板条目 `resolve`（已解决）链路接通（后端命令 + 主链路白板面板）。
- [x] 白板摘要 + 原始归档检索链路接通。

### Phase 4: 用户可编排串并行执行

- [x] 定义并校验编排计划 DSL（串行、并行）基础版。
- [x] 引擎按计划执行 Team 成员节点（替代固定阶段顺序）。
- [x] 失败恢复与重试策略。

### Phase 8: 模板化编排最佳实践与恢复策略 Presets

- [x] 编排页内置 Best Practices 编排预设（需求到交付 / 安全审计矩阵 / 故障处置链路）。
- [x] 内置恢复策略预设（保守 / 平衡 / 激进），可一键同步到 step 重试参数。
- [x] 恢复策略预设同步写入 `state_machine`（`no_human_input_policy`、`human_intervention_timeout_secs`、`max_human_interventions`）。
- [x] 预设与模板库打通（支持将当前预设演化为模板资产）。

## 3. 当前进度更新

### 2026-02-27

- 已完成 Phase 1（数据层）：
  - `orchestration_plan`、`plan_version` 已打通模型/仓储/迁移/类型。
  - SQLite 旧库已支持启动自动补列。
  - `cargo check` 通过。
- 已完成 Phase 2（主链路入口接入）：
  - InputArea 已新增 Team 开关并持久化状态。
  - AgentView 已支持 Team 模式消息路由（创建/复用 Team 会话并启动/续跑）。
  - Team 状态事件已接入主链路执行态（可停止 Team 运行）。
- 已完成 Phase 3（运行视图并入，第一批）：
  - AgentView 右侧栏新增 Team 工作台（模板库 / 白板 / 产物 / 时间线 / 对比）。
  - Team 事件（state/round/artifact）已在主链路侧栏联动刷新数据。
  - Team 模式与原 WebExplorer/Todo/Terminal/HTML 面板互斥切换。
- 已完成 Phase 4（第一批，DSL 校验）：
  - 新增 `agent_team/orchestration.rs`，实现 `orchestration_plan` 规范化与校验。
  - 在创建/更新 Team Session 时统一执行校验，阻断非法计划落库。
- 已完成 Phase 4（第二批，按计划执行）：
  - `AgentTeamEngine.start_run` 增加按 `orchestration_plan` 执行分支，无计划时继续走原固定流程。
  - 新增 `agent / serial / parallel` 递归执行器，支持用户定义串行与并行混合节点。
  - `agent` 节点按 `member` 绑定会话成员执行，复用主链能力（角色上下文、工具调用、流式事件、执行记忆、白板沉淀）。
  - 计划执行异常会触发 `agent_team:orchestration_fallback` 并回退旧固定流程，保证可用性。
  - `orchestration.rs` 校验增强为递归校验（含嵌套 children 与全局唯一 step id）。
- 已完成 Phase 4（第三批，失败恢复与重试）：
  - `agent` 编排节点支持 `retry.max_attempts` 与 `retry.backoff_ms`（含兼容字段）重试配置。
  - 节点失败会发出 `agent_team:orchestration_step_retry` 事件并按 backoff 重试。
  - 新增 `state_machine.orchestration_runtime` checkpoint（最近 step、轮次、尝试次数、错误）用于故障定位与后续恢复。
- 验证：
  - `cargo check` 通过。
  - `npm run -s type-check` 当前仅剩项目内既有 `LlmSecurityPanel.vue` 无关类型错误。
- 已完成 Phase 5（第一批，主链路编排编辑入口）：
  - 在 `AgentView` Team 工作台新增“编排”Tab。
  - 支持会话级 `orchestration_plan` 的加载、JSON 编辑、保存。
  - 支持“用当前计划启动”与“重试运行”入口（不影响原主链路发送逻辑）。
  - 展示 `state_machine.orchestration_runtime` 检查点（最近步骤/状态/尝试/错误）用于快速排障。
- 已完成 Phase 5（第二批：从指定 step 恢复入口）：
  - 后端执行引擎支持读取 `state_machine.orchestration_runtime.resume_from_step_id`。
  - 当 `resume_from_step_id` 命中 step 时可继续执行；未命中则回退从首 step 执行并发事件告警。
  - 引擎在消费恢复指令后会清理 `resume_from_step_id` 并写入 `resume_consumed_*` 元数据。
  - `AgentView` 编排页新增“从 Step 恢复”输入与执行按钮，支持用户手动指定恢复节点。
- 已完成 Phase 5（第三批：可视化节点编辑器）：
  - `AgentView` 编排页新增可视化节点编辑器（支持顶层节点和一层 children 的增删改与顺序调整）。
  - 支持 `agent / serial / parallel` 节点类型切换，并可编辑 `member / phase / instruction / retry` 关键字段。
  - 可视化编辑与 JSON 编辑器双向同步，保留高级用户直接改 JSON 能力。
- 已完成 Phase 6（第一批：递归化 + 同层拖拽）：
  - 新增 `TeamOrchestrationStepTreeEditor.vue` 递归组件，支持深层 `children` 可视化编辑。
  - 每层步骤支持同层拖拽排序（HTML5 drag/drop）与上下移动按钮。
  - `AgentView` 编排页改为基于递归组件渲染，保留 JSON 高级编辑器并保持双向同步。
- 已完成 Phase 6（第二批：跨层路径操作 + 细粒度恢复）：
  - 递归编辑器新增跨层路径操作：`promote`（提升一层）和 `nest`（嵌入前置节点）按钮。
  - 编排页恢复输入支持 step 自动补全（基于当前编排树展开）。
  - 后端恢复能力升级为递归查找任意层级 `step_id`，按命中路径恢复后续执行。
- 已完成 Phase 6（第三批：跨层拖拽落点 + 路径可视化/快捷恢复）：
  - 递归编辑器支持拖拽落点模式：拖到节点头部可按同层前置重排，拖到节点内容区可嵌入为子节点。
  - `AgentView` 增加路径级 move 算法（`sourcePath/targetPath/mode`）统一处理跨层拖拽结果。
  - 编排页展示节点路径（如 `1.2.1`），并提供恢复 step 快捷按钮与最近运行路径显示。
- 已完成 Phase 7（第一批：step 级执行可观测性）：
  - 后端在 `orchestration_runtime` 新增 `step_stats` 与 `summary` 聚合（尝试次数、成功失败、平均/最近耗时、最慢 step）。
  - 失败时记录 `suggested_resume_step_id`，成功时记录 `last_success_step_id`，用于恢复决策。
  - 前端编排页新增“执行可观测性”和“恢复建议”面板，支持按热点 step 一键设为恢复点。
- 已完成 Phase 7（第二批：失败模式分类与自动恢复策略）：
  - 后端新增失败模式分类（timeout / llm_provider / permission / tool_execution / input_validation / member_mapping / unknown）。
  - 在 `orchestration_runtime.failure_modes` 聚合模式频次、最近失败 step、最近错误与对应恢复 hint。
  - 后端自动生成 `orchestration_runtime.recovery_suggestions`，前端优先读取并展示。
  - 编排页新增“失败模式分布”可视化卡片，辅助快速定位高频故障模式。
- 已完成 Phase 3（补充：白板 resolve 闭环）：
  - 新增 `agent_team_resolve_blackboard_entry` 命令，支持按 `session_id + entry_id` 标记白板条目为已解决。
  - PostgreSQL/SQLite/runtime 仓储层已补齐 `resolve_blackboard_entry`。
  - `AgentView` 白板面板“解决”按钮已改为真实调用并刷新白板数据。
- 已完成 Phase 3（补充：白板原始归档检索）：
  - 新增 `agent_team_get_blackboard_entry_archive` 命令，支持按白板条目检索关联消息归档。
  - PostgreSQL/SQLite/runtime 仓储层新增 `get_blackboard_entry_archive`（优先同轮次，空结果时回退会话最近消息）。
  - `AgentView` / `AgentTeamBlackboardPanel` 新增“归档”入口，可展开查看原始消息明细与检索范围。
- 已完成 Phase 8（第一批：预设化编排与恢复策略）：
  - `AgentView` 编排页新增 3 组编排预设（需求到交付 / 安全审计矩阵 / 故障处置链路），可一键生成串并行混合骨架。
  - 新增 3 组恢复策略预设（保守 / 平衡 / 激进），可一键同步全部 `agent` 节点 `retry` 参数。
  - 应用恢复策略时会同步更新会话 `state_machine` 的 no-human-input policy 与 human-intervention timeout 配置。
- 已完成 Phase 8（第二批：预设与模板库打通）：
  - `AgentView` 编排页新增“沉淀为模板”入口，可将当前编排计划、恢复策略和成员快照保存为模板。
  - 模板保存时会把 `orchestration_plan/plan_version` 写入 `default_rounds_config`，恢复策略写入 `default_tool_policy.state_machine`。
  - `CreateTeamFromTemplateModal` 在创建会话时会自动读取模板中的编排计划与恢复策略并透传到 `createSession`。
- 已完成 Phase 9（第一批：图形化拖拽编排可用化）：
  - `TeamOrchestrationStepTreeEditor` 升级为流程画布式交互（节点类型徽标、执行摘要、可视化层级、拖拽嵌入提示）。
  - 节点配置改为“配置节点”分组化表单（ID/类型/Phase/Instruction/member/retry），降低上手成本。
  - `AgentView` 编排页新增“快速上手”引导卡片，明确“预设 -> 拖拽配置 -> 保存运行”路径。
- 已完成 Phase 9（第二批：输入区模板选择并入主链）：
  - `InputAreaComponent` 在 Team 模式下新增“Team 模板”下拉，位置与模型选择器并列。
  - `AgentView` 新增模板列表加载/缓存/选择状态（含空模板自动 seed 内置模板兜底）。
  - Team 发送链路改为优先使用当前选择模板创建会话，不再固定使用首个模板。
  - 模板库更新后会同步刷新输入区模板选项，确保创建新会话时可直接使用最新模板。
- 已完成 Phase 9（第三批：编排入口迁移到模板编辑）：
  - `AgentView` Team 工作台已移除“编排”Tab 与旧编排面板入口。
  - `AgentTeamSettings`（模板新建/编辑）新增编排配置区：`plan_version` + `orchestration_plan(JSON)`。
  - 模板保存时写入 `default_rounds_config.orchestration_plan / plan_version`，会话创建时继续自动透传。
  - 模板复制行为已保留 `default_rounds_config/default_tool_policy`，避免丢失编排配置。
- 已完成 Phase 10（第一批：工作流式拖拽编排）：
  - 新增 `TeamWorkflowCanvasEditor`：支持从角色库拖拽角色节点到画布、节点拖动、节点连线依赖、节点属性编辑。
  - `AgentTeamSettings` 集成工作流画布，编排默认通过图形化方式完成，JSON 模式降级为高级编辑。
  - 模板保存时会将 `workflow_v2(nodes/edges)` 编译为 `orchestration_plan` 并写入 `default_rounds_config`。
  - 兼容旧模板：无 `workflow_v2` 时会尝试从 `orchestration_plan` 回填为可编辑工作流。
- 已完成 Phase 10（第一批补充：画布可用性修复）：
  - 模板编辑弹窗宽度扩展为 `min(96vw,1200px)`，保障流程画布编辑空间。
  - 画布拖拽兼容增强：同时写入/读取 `application/x-workflow-member` 与 `text/plain`。
  - 角色节点库增加“点击添加/双击添加”兜底入口，避免仅依赖拖拽。
- 已完成 Phase 10（第一批补充：拖拽稳定性修复）：
  - 角色拖拽增加同源状态兜底（`draggingPaletteMember`），`drop` 失败时可由 `mouseup` 在画布落点补建节点。
  - 节点拖动改为“拖动中仅更新本地坐标，`mouseup` 后一次性同步”，避免每帧同步导致的位移抖动/短距离拖动问题。
- 已完成 Phase 10（第一批补充：Team 运行态收敛）：
  - `AgentView` 增加 `agent_team_get_run_status` 轮询兜底（2s），在事件漏收时仍可同步 Team 会话真实状态。
  - Team 状态更新统一走 `applyTeamState`，避免重复提示，并确保从运行态及时回收到非运行态。
- 已完成 Phase 10（第一批补充：仅工作流编排执行）：
  - `AgentTeamEngine.start_run` 强制要求 `orchestration_plan`，缺失时直接失败并发出 `agent_team:orchestration_required`。
  - `execute_orchestration_plan` 执行失败时不再回退旧四阶段，改为直接失败并发出 `agent_team:orchestration_failed`。
  - 清理旧默认流实现（`run_default_flow/propose/challenge/decide` 及其遗留工具函数），后端执行路径统一为编排主链路。
- 下一步：Phase 10（第二批：条件分支/容器节点与模板差异对比）。

## 4. 验收标准（阶段性）

### Phase 1 验收

- `create/get/update/list session` 返回包含 `orchestration_plan` 和 `plan_version`。
- 新老库可兼容（旧库启动后自动补列）。
- 编译通过。

## 5. 约束

- 仅修改本目标相关文件，不处理无关脏改动。

## 6. 清理进度

### 2026-02-27（旧入口与冗余组件清理）

- [x] 移除独立 AI Team 路由入口 `/ai-team`（`main.ts`）。
- [x] 移除侧边栏 AI Team 菜单项（`Sidebar.vue`）。
- [x] 删除已无入口引用的旧页面与旧容器组件：
  - `src/views/AITeamAssistant.vue`
  - `src/components/Agent/AgentTeamView.vue`
  - `src/components/Agent/TeamSessionList.vue`
- [x] 清理侧边栏文案键 `sidebar.aiTeam`（中/英文）。
