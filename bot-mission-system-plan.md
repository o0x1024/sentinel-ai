# Bot Mission 系统方案

## 背景

Bot 控制台需要支持长期运行、定时执行、持续监控、周期报告和告警类任务。这些任务的来源不应该只限于 Bot 渠道，还应该包括应用内 UI、AI Assistant 对话、系统事件、Webhook、文件导入以及本地/手动入口。

典型请求包括：

- "每天 9 点给我生成最新 AI 资讯新闻。"
- "监控 XX 公司官网是否新增功能。"
- "接下来 10 天持续观察某行业趋势，最后给我一份总结。"
- "每天运行这个项目的测试，失败就把错误摘要发给我。"
- "如果某资产出现新增端口或证书变化，立即提醒我。"

这些请求不应该被实现成某个领域的专用模板，也不应该被实现成一段持续 10 天或更久的普通助手对话。它们应该被表示为由应用运行时持久化管理的 Mission。

## 架构约束

sentinel-ai 是 Tauri 桌面应用，不是 7x24 运行的服务端。所有设计必须遵守以下约束：

1. **桌面生命周期**：用户随时可能关闭应用或合上笔记本。Scheduler 必须是 best-effort 语义，不能假设持续在线。
2. **资源有限**：桌面应用不能像服务端一样并行跑大量 Mission run。必须有全局并发控制。
3. **单数据库引擎**：Mission 数据层只针对 SQLite 实现，避免现有 3x match（PostgreSQL/SQLite/MySQL）模式导致的代码膨胀。如有 PostgreSQL 需求，用条件编译 feature flag 隔离。
4. **Mission 是唯一 Bot 调度入口**：旧 Bot schedule 调度链已移除，长期、周期、订阅和投递类任务统一进入 Mission。

## 产品目标

构建一个通用 Mission 层，用来调度和监督长期任务，同时复用当前已有的 Agent Management、ToolConfig、Agent 运行时、执行任务账本、harness 事件以及 Bot 投递基础设施。

核心原则：

```text
Mission = 长期目标、触发规则、状态、证据和投递策略。
Agent Profile = 执行身份、模型、行为准则、上下文策略和工具权限。
Tool Runtime = 实际能力。
Harness / execution_tasks = 执行完成真相。
Artifact / Observation = 证据链。
Delivery = 通知路由。
```

Mission 不能变成一套平行的智能体系统，不能变成新的权限系统，也不能变成模板引擎。

## 非目标

- 不创建 Mission 专属的独立工具权限系统。
- 不绕过 Agent Management 或 ToolConfig。
- 当 Mission 需要缺失能力时，不静默启用工具。
- 不把领域专用模板作为主要抽象。
- 不把 LLM 最后一段文本作为完成真相。
- 不把长期运行状态只保存在 prompt 上下文里。
- 除非用户显式选择，否则不使用 fallback 数据源或替代工具。
- 不在还没有真实 Mission 运行前过早引入 Operator Registry 抽象。

## 设计决策与修正

### 1. Mission 和 Agent 的边界

Mission 只保存长期任务事实：目标、来源、触发规则、状态、运行记录、步骤、产物、观察结果、投递策略、失败策略、预算。

执行身份和能力必须来自：`assistantProfileId`、Profile 模型绑定、Profile 工具配置、Profile 上下文引擎模式。

每次运行应该持久化本次使用的 profile、model 和 tool config 快照，用于审计；但 Mission 本身不拥有独立工具权限。

### 2. Source 和 Delivery 独立路由

任务来源不等于投递目标。

```ts
interface MissionSource {
  kind: 'bot' | 'app' | 'assistant_conversation' | 'system_event' | 'webhook' | 'import' | 'manual'
  ref: Record<string, unknown>
}

interface MissionDeliveryPolicy {
  primary: MissionDeliveryTarget
  onSuccess: 'none' | 'summary' | 'full'
  onChange: 'none' | 'immediate' | 'summary'
  onFailure: 'none' | 'immediate'
  quietHours?: unknown
}

interface MissionDeliveryTarget {
  kind: 'bot' | 'assistant_conversation' | 'app_notification' | 'email' | 'webhook'
  ref: Record<string, unknown>
}
```

### 3. 独立的 Mission 状态机

```text
Mission: draft / active / paused / blocked / completed / failed / archived
Run: queued / running / succeeded / partial / failed / cancelled / timed_out
Step: pending / running / succeeded / failed / skipped / blocked
```

不从 `bot_execution_runs.status` 推断 Mission 状态。Bot execution run 只是某次 Mission run 的执行证据。

### 4. 证据链

同时持久化原始证据和结构化发现：

- Artifact：原始或半原始材料（HTML、截图、搜索结果、日志片段、命令输出、文件、报告、JSON payload）。
- Observation：从 artifacts 派生出的结构化判断（`changed=true`、`new_button_detected`、`test_failed`、`port_added`）。

报告必须基于 observations 生成，observations 必须能回溯到 artifacts。

### 5. LLM 与确定性服务的职责边界

确定性服务负责：抓取、快照、Diff、规则匹配、退出码检查、预算检查、状态迁移、success criteria 评估。

LLM 负责：自然语言解析为 Mission draft、步骤计划草案、证据摘要、最终报告撰写、面向用户的解释。

### 6. 确定性 Success Criteria

```ts
interface MissionSuccessCriteria {
  rules: SuccessRule[]
  aggregation: 'all' | 'any'
}

type SuccessRule =
  | { kind: 'artifact_produced'; artifactType: string; minCount: number }
  | { kind: 'observation_present'; observationType: string }
  | { kind: 'observation_absent'; observationType: string }
  | { kind: 'exit_code'; operator: string; expectedCode: number }
  | { kind: 'content_changed'; artifactType: string }
  | { kind: 'content_unchanged'; artifactType: string }
  | { kind: 'task_ledger_complete' }
```

### 7. 权限三层控制

```text
Agent Profile ToolConfig: 硬能力边界。
Mission Approval: 用户批准该 Mission 可自动运行哪些高风险操作。
Runtime Guard: 每次运行前和每个步骤执行前检查高风险操作。
```

高风险操作需要显式批准：`run_shell`、`port_scan`、`write_file`、`send_external_webhook`、`authenticated_browser_action`、任何可能修改外部系统的操作。

### 8. 桌面调度器语义

Mission Scheduler 是 best-effort desktop scheduler：

- missed-run 策略：`skip`（默认）/ `run_once_on_startup` / `run_all_missed`。
- 全局并发：`max_concurrent_runs: 1`（默认），超出排队。
- 应用启动时：扫描 due missions、恢复 stale running runs、释放过期锁。

持久化字段：`next_run_at`、上次运行 id、错过运行策略、锁持有者、锁过期时间、重试次数、失败次数。

### 9. Run Checkpoint 协议

每个 step 完成后持久化 checkpoint。应用重启时 stale running run 可选择：

- `resume_from_checkpoint`：从上次完成的 step 继续。
- `restart`：重新开始当前 run。
- `mark_failed`：标记失败并触发 failure policy。

Checkpoint 包含：已完成 steps、已生产 artifacts、当前 step 上下文。

### 10. 跨 Run 上下文连续性

```ts
interface MissionContextStrategy {
  mode: 'stateless' | 'incremental' | 'cumulative'
  contextWindow?: {
    maxRuns: number
    maxTokens: number
    summarizeOlder: boolean
  }
  completionCondition?: {
    kind: 'run_count' | 'date' | 'observation_match' | 'manual'
    value: unknown
  }
  synthesisOnComplete: boolean
}
```

- `stateless`：每次 run 独立（适合每日新闻）。
- `incremental`：注入上次 run 的 observations 摘要（适合监控）。
- `cumulative`：注入所有历史 observations（适合趋势分析）。

### 11. Artifact 存储策略

```ts
interface ArtifactStoragePolicy {
  backend: 'filesystem'
  // 路径：{app_data}/missions/{mission_id}/runs/{run_id}/artifacts/{artifact_id}.{ext}
  retention: {
    maxAge: Duration
    maxArtifactsPerMission: number
    maxTotalBytes: number
    onExceed: 'reject' | 'evict_oldest' | 'compress_oldest'
  }
  hashUsage: 'dedup' | 'change_detection' | 'both'
}
```

默认存储在 Tauri `app_data_dir` 下的文件系统，数据库只存元数据和 URI。清理在每次 run 结束后异步执行。

### 12. 预算策略

```ts
interface MissionBudget {
  maxRunsPerDay: number
  maxToolCallsPerRun: number
  maxModelTokensPerRun: number
  maxArtifactsBytes: number
  timeoutSeconds: number
}
```

超出预算时 run 进入 `blocked` 或 `partial`，并给出清晰原因和用户可执行的修复建议。

### 13. 失败恢复语义

```ts
interface MissionFailurePolicy {
  retryCount: number
  retryDelaySeconds: number
  afterFailure: 'continue_next' | 'pause' | 'block_until_user_action'
}
```

### 14. 创建校验

```text
自然语言 -> MissionDraft -> 确定性校验 -> 用户确认 -> active Mission
```

校验内容：目标非空、触发规则有效、source 已捕获、delivery target 可解析、Agent Profile 存在、预算有效、高风险操作已被批准。缺失关键字段时先追问。

### 15. 所有权隔离

```sql
-- missions 表增加显式隔离字段
owner_kind TEXT NOT NULL  -- 'bot_peer' | 'user' | 'system'
owner_ref TEXT NOT NULL   -- bot: "{transport}:{account_id}:{peer_type}:{peer_id}"
                          -- user: user_id
                          -- system: "system"

CREATE INDEX idx_missions_owner ON missions(owner_kind, owner_ref, status);
```

### 16. UI 原则

Bot Console Mission 标签页只展示：Mission 列表、状态、下次运行时间、绑定的 Agent Profile、最新结果、失败原因、运行记录、步骤账本、按需查看 artifacts 和 observations、暂停/恢复/立即运行/归档。深层 Agent 配置跳转到 Agent Management。

## 最终架构

```text
Sources
  Bot / App / AI Assistant / System Event / Webhook / Import / Manual

Mission Core
  Mission / Run / Step / Artifact / Observation / Delivery / Lock
  Owner isolation (owner_kind + owner_ref)

Agent Binding
  Assistant Profile / Model / Context Engine / ToolConfig Snapshot

Mission Scheduler (best-effort desktop)
  Cron / Interval / Manual / Event / Recovery / Locking
  missed-run policy / concurrency control / checkpoint protocol

Mission Runner
  Creates Run -> executes via Agent runtime -> records evidence
  Cross-run context injection (stateless / incremental / cumulative)

Existing Agent Runtime
  execute_agent_turn / execution_tasks / harness events / checkpoints

Evidence Chain
  Artifacts (filesystem) -> Observations -> Success Criteria evaluation

Delivery
  Bot / Assistant Conversation / App Notification / Webhook
```

## 建议数据模型

### `missions`

- `id`
- `title`
- `objective`
- `status`
- `owner_kind`
- `owner_ref`
- `source_json`
- `delivery_policy_json`
- `assistant_profile_id`
- `trigger_json`
- `step_plan_json`
- `success_criteria_json`
- `context_strategy_json`
- `budget_json`
- `failure_policy_json`
- `missed_run_policy`
- `next_run_at`
- `last_run_at`
- `last_error`
- `run_count`
- `created_at`
- `updated_at`

### `mission_runs`

- `id`
- `mission_id`
- `run_index`
- `status`
- `trigger_kind`
- `started_at`
- `completed_at`
- `agent_execution_id`
- `bot_execution_run_id`
- `assistant_profile_snapshot_json`
- `tool_config_snapshot_json`
- `checkpoint_json`
- `context_injected_json`
- `result_summary`
- `error_message`
- `created_at`
- `updated_at`

### `mission_steps`

- `id`
- `run_id`
- `step_index`
- `description`
- `status`
- `input_json`
- `output_json`
- `error_message`
- `started_at`
- `completed_at`

### `mission_artifacts`

- `id`
- `mission_id`
- `run_id`
- `step_id`
- `artifact_type`
- `storage_kind`
- `uri`
- `size_bytes`
- `content_hash`
- `metadata_json`
- `created_at`

### `mission_observations`

- `id`
- `mission_id`
- `run_id`
- `step_id`
- `observation_type`
- `severity`
- `title`
- `summary`
- `data_json`
- `artifact_ids_json`
- `created_at`

### `mission_deliveries`

- `id`
- `mission_id`
- `run_id`
- `target_json`
- `status`
- `message_id`
- `payload_json`
- `error_message`
- `created_at`
- `updated_at`

### `mission_locks`

- `mission_id`
- `run_id`
- `lock_owner`
- `expires_at`
- `created_at`

## 实施计划

### Phase 0: 数据层准备

目标：Mission 新表只写一份 SQLite 实现，避免 3x match 代码膨胀。

实现：
- 评估现有 bot schema 的多数据库使用情况，确认 Mission 模块只需 SQLite。
- 在 sentinel-db 中建立 Mission 专属模块（`mission.rs`），只实现 SQLite 路径。
- 如需 PostgreSQL 兼容，用 `#[cfg(feature = "postgres")]` 隔离。

验收：
- Mission 数据层代码量可控（单路径实现）。
- 不影响现有 bot/agent 表的多数据库兼容。

### Phase 1: Mission Core

目标：创建持久化 Mission 生命周期。

实现：
- 新增 `missions`、`mission_runs`、`mission_steps`、`mission_artifacts`、`mission_observations`、`mission_deliveries`、`mission_locks` 表（SQLite only）。
- `missions` 表包含 `owner_kind`、`owner_ref` 隔离字段。
- 新增数据库服务方法。
- 新增 Tauri commands：create、list、get、pause、resume、archive、run now。
- 增加状态迁移校验。

验收：
- 可以从结构化输入创建 Mission。
- Mission 状态在应用重启后仍然存在。
- 可以按 owner 查询 Mission 列表和详情。
- 非法状态迁移会被拒绝。

### Phase 2: Agent Binding

目标：把 Mission 接入已有 Agent Management 和 ToolConfig。

实现：
- 校验和运行时加载 Assistant Profile。
- 从 profile 派生 model、context engine mode 和 ToolConfig。
- 每次 run 持久化 profile、model、ToolConfig 快照。
- 当所需工具不可用时，阻塞创建或执行。

验收：
- Mission 不能使用被选中 Agent Profile 禁用的工具。
- 缺少能力时给出清晰 blocked reason。
- run 详情展示实际使用的快照。

### Phase 3: Scheduler 和桌面恢复

目标：让 Mission 可以可靠地在桌面应用中跨时间运行。

实现：
- 持久化 `next_run_at`、`missed_run_policy`。
- 增加 scheduler loop 扫描 due Missions。
- 增加带过期时间的持久化锁。
- 实现 missed-run handling（skip / run_once_on_startup / run_all_missed）。
- 增加全局并发控制（`max_concurrent_runs`，默认 1）。
- 增加 retry 和 timeout handling。
- 应用启动时恢复 stale running runs（支持 resume/restart/mark_failed）。
- Mission scheduler 只扫描 Mission，不再处理旧 Bot schedule 表。

验收：
- 应用重启后，到期 Mission 按 missed-run policy 处理。
- 同一个 Mission tick 不会重复运行。
- 超时 run 会被清晰标记。
- failure policy 会被应用。
- 全局并发不超过限制。

### Phase 4: Runner 接入 Agent Runtime

目标：复用现有 Agent 执行路径和完成真相。

实现：
- 每次 Mission 触发时创建 `mission_run`。
- 用自然语言 task 直接驱动 Agent（通过现有 ToolConfig 调用工具，不需要 Operator 中介层）。
- 创建或关联标准 Agent execution id。
- 当 source/delivery 与 Bot 相关时，关联 `bot_execution_runs`。
- 将 `execution_tasks`、harness events 和 checkpoints 作为 run evidence。
- 每个 step 完成后持久化 checkpoint。
- 只有 success criteria 通过后，run 才能标记成功。

验收：
- Mission run 详情可以打开底层 Agent execution。
- 单纯最后一条 assistant message 不能让 run 成功。
- 未完成的 task ledger 会产生 `partial` 或 `failed`。
- 应用关闭后重启，stale run 能正确恢复。

### Phase 5: Artifact 存储和 Observation 证据链

目标：让 Mission 输出可审计，存储策略明确。

实现：
- 实现 filesystem-based artifact storage（`{app_data}/missions/{mission_id}/...`）。
- 存储 raw artifacts，记录 content hash、size_bytes 和 metadata。
- 实现 retention policy（maxAge、maxArtifactsPerMission、maxTotalBytes、onExceed）。
- 存储 observations，引用 artifact ids。
- 根据 observations 生成摘要。
- 强制报告基于已持久化 observations 生成。

验收：
- 网站监控可以展示上次和本次快照引用。
- 新闻摘要可以展示 source URLs 和 timestamps。
- Artifact 清理策略在 run 结束后自动执行。
- content_hash 支持变化检测。

### Phase 6: Success Criteria 和跨 Run 上下文

目标：确定性判断 run 成功，支持跨 run 的信息累积。

实现：
- 实现 `MissionSuccessCriteria` schema 和确定性评估引擎。
- 实现 `MissionContextStrategy`（stateless / incremental / cumulative）。
- 实现 context window 和 summarizeOlder。
- 实现 completionCondition（run_count / date / observation_match / manual）。
- 实现 synthesis run 自动触发。

验收：
- "10 天趋势观察" 场景可以跨 run 累积 observations 并自动生成最终总结。
- 每日新闻场景每次 run 独立。
- success criteria 评估不依赖 LLM。

### Phase 7: Delivery 层

目标：让结果路由独立于任务来源。

实现：
- 实现 delivery targets：Bot conversation、AI Assistant conversation、App notification、Webhook。
- 针对 success、change 和 failure 应用 delivery policy。
- 存储 delivery attempts 和 failures。

验收：
- 从应用 UI 创建的 Mission 可以投递到 Bot。
- 从 Bot 创建的 Mission 可以只投递应用内通知。
- 失败的 delivery 可见且可重试。

### Phase 8: Bot Console UI

目标：在不挤占现有 Bot Console 的情况下展示 Mission 工作流。

实现：
- 新增 `Missions` 标签页。
- 新增 Mission 列表（status、next run、profile、latest result、failure count）。
- 新增详情视图（trigger、delivery policy、runs、steps、artifacts、observations、linked executions）。
- 增加 pause、resume、run now、archive。
- 增加创建弹窗：结构化输入，展示校验后的 draft。
- Agent Profile 编辑跳转到 Agent Management。

验收：
- Bot Console 主界面仍聚焦 sessions、executions、schedules 和 Missions。
- 详细 artifacts 和 debug logs 只按需展示。
- Bot Console 不重复实现 Agent 配置。

### Phase 9: Mission Planner

目标：把自然语言转换为经过校验的 MissionDraft。

实现：
- 增加 planner prompt contract，输出 `MissionDraft` 结构化 JSON。
- 校验 trigger、source、delivery、profile、step plan、risk 和 budget。
- 对缺失的必要字段先追问。

验收：
- "每天 9 点给我生成 AI 资讯" 能生成带 trigger 和 delivery 的 draft。
- "监控这个网站" 在缺少 URL 时会追问。
- Planner 在校验通过前不能创建 active Mission。

### Phase 10: Operator Registry（按需）

目标：从实际运行经验中提取稳定的步骤契约。

前置条件：已有多个真实 Mission 在运行，且观察到重复的执行模式需要稳定化。

实现：
- 定义 operator registry metadata（id、requiredTools、inputSchema、outputSchema、riskLevel）。
- 将 operators 映射到现有 tool ids。
- 增加 input 和 output schema 校验。

验收：
- Operators 从实际执行模式中提取，不是凭空设计。
- 没有所需工具权限时，operator 不能执行。

### Phase 11: 验证场景

不要把这些做成模板，只作为端到端验证用例：

1. 每日 AI 新闻摘要 — 覆盖 search/fetch/dedup/summarize/delivery。
2. 网站变化监控 — 覆盖 fetch/snapshot/diff/observation/change notification。
3. 每日项目测试运行 — 覆盖 shell/log artifact/failure summary/delivery。

验收：
- 三个场景都走同一套 Mission Core。
- 三个场景都使用 Agent Profile 和 ToolConfig。
- 三个场景都能产生 artifacts、observations、run history 和 delivery records。
- failure 和 blocked 状态可见且可处理。

## 关键验收规则

- Mission 状态必须持久化，并能跨重启恢复。
- Mission 执行必须始终使用 Agent Profile。
- 工具权限必须始终来自 ToolConfig。
- 缺少工具时必须以清晰原因阻塞 Mission。
- 每次 run 都必须有 step ledger。
- 每个有意义的结果都必须有 artifact 或 observation 证据。
- 完成判断必须基于 success criteria 确定性评估，不能只看最后文本。
- Source 和 delivery 必须独立。
- 所有权隔离必须通过显式字段，不通过 JSON 解析。
- Bot Console 不重复实现 Agent Management。
- 桌面调度器使用 best-effort 语义，missed-run 默认 skip。
- Artifact 存储在文件系统，有明确的 retention policy。
- 不静默引入兼容或 fallback 路径。
