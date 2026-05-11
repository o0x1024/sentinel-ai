# Bot Mission System — 实施任务

> 基于 `bot-mission-system-plan.md` 的修正方案，按依赖顺序拆分为可执行任务。

## Phase 0: 数据层准备

### T-0.1 创建 Mission 数据模块骨架
- [ ] 在 `sentinel-db/src/database_service/` 下新建 `mission.rs`
- [ ] 在 `mod.rs` 中注册模块
- [ ] 只实现 SQLite 路径，不使用 3x match 模式
- [ ] 如需 PostgreSQL，使用 `#[cfg(feature = "postgres")]` 条件编译
- **文件**: `sentinel-db/src/database_service/mission.rs`, `sentinel-db/src/database_service/mod.rs`
- **验收**: 模块编译通过，不引入 PostgreSQL/MySQL 分支

---

## Phase 1: Mission Core

### T-1.1 创建 Mission 数据表
- [ ] 在 `mission.rs` 中实现 `create_mission_schema()` — SQLite DDL
- [ ] 表：`missions`、`mission_runs`、`mission_steps`、`mission_artifacts`、`mission_observations`、`mission_deliveries`、`mission_locks`
- [ ] `missions` 表必须包含 `owner_kind`、`owner_ref` 隔离字段
- [ ] 创建索引：`idx_missions_owner(owner_kind, owner_ref, status)`、`idx_missions_status_next_run(status, next_run_at)`
- **验收**: 应用启动后表自动创建，schema 正确

### T-1.2 定义 Mission Rust 数据模型
- [ ] 在 `sentinel-db/src/models/` 或 `sentinel-core/src/models/` 下定义 Mission 相关 struct
- [ ] `Mission`、`MissionRun`、`MissionStep`、`MissionArtifact`、`MissionObservation`、`MissionDelivery`、`MissionLock`
- [ ] 所有 struct 实现 `Serialize`、`Deserialize`、`FromRow`
- [ ] 定义 `MissionStatus`、`MissionRunStatus`、`MissionStepStatus` 枚举
- **验收**: 模型编译通过，可用于数据库读写

### T-1.3 实现 Mission CRUD 服务方法
- [ ] `create_mission()`
- [ ] `get_mission(id)`
- [ ] `list_missions(owner_kind, owner_ref, status, limit)`
- [ ] `update_mission_status(id, new_status)` — 带状态迁移校验
- [ ] `update_mission_fields()` — 更新 trigger、delivery、budget 等
- [ ] `delete_mission(id)` — 仅 draft/archived 状态可删
- **验收**: 所有方法单元测试通过

### T-1.4 实现 Mission 状态迁移校验
- [ ] 定义合法状态迁移表：
  - `draft -> active | archived`
  - `active -> paused | blocked | completed | failed | archived`
  - `paused -> active | archived`
  - `blocked -> active | archived`
  - `failed -> active | archived`
  - `completed -> archived`
- [ ] `update_mission_status()` 拒绝非法迁移并返回清晰错误
- **验收**: 非法迁移返回错误，不修改数据

### T-1.5 实现 Tauri Commands
- [ ] `mission_create` — 从结构化 JSON 输入创建 Mission
- [ ] `mission_list` — 支持 owner 过滤、状态过滤、分页
- [ ] `mission_get` — 返回 Mission 详情 + 最近 runs
- [ ] `mission_pause` / `mission_resume`
- [ ] `mission_archive`
- [ ] `mission_run_now` — 手动触发一次 run
- [ ] 在 `src/lib.rs` 中注册所有 commands
- **文件**: `src-tauri/src/commands/mission_commands.rs`, `src-tauri/src/lib.rs`
- **验收**: 前端可调用所有 commands，数据持久化

---

## Phase 2: Agent Binding

### T-2.1 Mission 创建时校验 Agent Profile
- [ ] `mission_create` 时校验 `assistant_profile_id` 存在
- [ ] 加载 profile 的 model、context engine mode、ToolConfig
- [ ] 如果 profile 不存在，拒绝创建
- **验收**: 使用不存在的 profile id 创建 Mission 会失败

### T-2.2 Run 创建时快照 Agent 配置
- [ ] 创建 `mission_run` 时，把当前 profile、model、ToolConfig 序列化写入快照字段
- [ ] 快照是只读的运行时证据，后续 profile 变更不影响历史 run
- **验收**: run 详情可以展示创建时的 profile/model/tool config

### T-2.3 工具能力校验
- [ ] Mission 创建或 run 执行前，检查 task 所需工具是否被 ToolConfig 允许
- [ ] 如果缺少必需工具，Mission 进入 `blocked` 状态，附 blocked reason
- **验收**: 禁用工具后，Mission 给出清晰阻塞原因

---

## Phase 3: Scheduler 和桌面恢复

### T-3.1 实现 Mission Scheduler Loop
- [ ] 在应用启动时创建后台 tokio task
- [ ] 每 30 秒扫描 `missions` 表中 `status = 'active' AND next_run_at <= now()`
- [ ] 尊重全局并发限制（`max_concurrent_runs`，默认 1）
- [ ] 超出并发时排队，不拒绝
- **文件**: `src-tauri/src/services/mission_scheduler.rs`
- **验收**: cron Mission 按时触发

### T-3.2 实现持久化锁
- [ ] 触发 run 前在 `mission_locks` 中写入锁（mission_id, lock_owner, expires_at）
- [ ] run 完成或失败后释放锁
- [ ] 应用启动时释放所有过期锁
- **验收**: 同一 Mission 不会同时有两个 running run

### T-3.3 实现 missed-run 处理
- [ ] Mission 字段 `missed_run_policy`：`skip`（默认）/ `run_once_on_startup` / `run_all_missed`
- [ ] 应用启动时扫描 `next_run_at < now()` 的 active Missions
- [ ] 按 policy 决定是跳过、执行一次、还是补执行所有错过的
- [ ] 执行后更新 `next_run_at` 到下一个触发时间
- **验收**: 关闭应用 → 错过运行 → 重启后按 policy 正确处理

### T-3.4 实现 stale run 恢复
- [ ] 应用启动时扫描 `mission_runs` 中 `status = 'running'` 且无有效锁的记录
- [ ] 根据 `checkpoint_json` 决定恢复策略
- [ ] 默认 `mark_failed`，记录中断原因
- **验收**: 应用崩溃后重启，stale run 被正确标记

### T-3.5 与 bot_schedules 共存
- [ ] Mission scheduler loop 同时处理 `bot_schedules` 的 due items
- [ ] 或：保留现有 bot_schedules scheduler，Mission scheduler 独立运行
- [ ] 确保不会出现两个 scheduler 竞争同一个 schedule
- **验收**: 现有 bot_schedules 功能不受影响

---

## Phase 4: Runner 接入 Agent Runtime

### T-4.1 实现 Mission Runner
- [ ] 触发时创建 `mission_run`，关联 `agent_execution_id`
- [ ] 构建 `AgentExecuteParams`（从 profile 快照派生）
- [ ] 注入跨 run 上下文（如果 context_strategy 不是 stateless）
- [ ] 调用 `execute_agent_turn()` 执行
- [ ] 将 harness events、execution_tasks 作为 run evidence
- **文件**: `src-tauri/src/services/mission_runner.rs`
- **验收**: Mission run 可以通过现有 Agent runtime 执行

### T-4.2 实现 Run Checkpoint
- [ ] 每个 step 完成后更新 `mission_runs.checkpoint_json`
- [ ] Checkpoint 记录：已完成 step indices、已生产 artifact ids、当前 step 上下文
- **验收**: run 中途中断后，checkpoint 数据完整

### T-4.3 实现 Bot Execution 关联
- [ ] 当 Mission source 是 bot 时，同时创建 `bot_execution_runs` 记录
- [ ] `mission_runs.bot_execution_run_id` 指向对应的 bot execution
- [ ] Bot Console 的 execution 列表能展示来自 Mission 的 runs
- **验收**: Mission run 和 bot execution run 双向可达

### T-4.4 Run 完成判断
- [ ] run 完成后执行 success criteria 确定性评估
- [ ] 根据评估结果设置 run status（succeeded / partial / failed）
- [ ] 触发 failure policy（retry / continue_next / pause / block）
- [ ] 更新 Mission 的 `next_run_at`、`last_run_at`、`run_count`
- **验收**: success criteria 评估不依赖 LLM 文本

---

## Phase 5: Artifact 和 Observation 证据链

### T-5.1 实现 Artifact 文件存储
- [ ] 存储路径：`{app_data_dir}/missions/{mission_id}/runs/{run_id}/artifacts/{artifact_id}.{ext}`
- [ ] 写入文件后在 `mission_artifacts` 表记录元数据（uri、size_bytes、content_hash）
- [ ] 实现 `save_mission_artifact(mission_id, run_id, step_id, type, data)` 方法
- **验收**: artifact 文件可写入和读取

### T-5.2 实现 Artifact Retention 清理
- [ ] run 结束后异步执行清理
- [ ] 按 retention policy 删除过期/超额 artifacts
- [ ] 同步删除数据库中的元数据记录
- **验收**: 超过配额后自动清理最旧 artifacts

### T-5.3 实现 Observation 存储
- [ ] `save_mission_observation(mission_id, run_id, step_id, type, severity, data, artifact_ids)`
- [ ] Observation 引用产生它的 artifact ids
- [ ] 支持按 mission_id 和 observation_type 查询
- **验收**: observation 可追溯到源 artifacts

### T-5.4 实现 content_hash 变化检测
- [ ] 同类型 artifact 写入时，与上一个同类型 artifact 的 hash 比较
- [ ] 如果 hash 不同，自动生成 `content_changed` observation
- **验收**: 网站快照变化时自动产生 observation

---

## Phase 6: Success Criteria 和跨 Run 上下文

### T-6.1 实现 Success Criteria 评估引擎
- [ ] 解析 `success_criteria_json` 为 `MissionSuccessCriteria` struct
- [ ] 实现每种 `SuccessRule` 的确定性评估：
  - `artifact_produced` — 查 mission_artifacts 表
  - `observation_present` / `observation_absent` — 查 mission_observations 表
  - `content_changed` / `content_unchanged` — 查 content_hash
  - `task_ledger_complete` — 查 execution_tasks
- [ ] 按 aggregation（all / any）聚合结果
- **验收**: 各种 rule 类型评估正确

### T-6.2 实现跨 Run 上下文注入
- [ ] 解析 `context_strategy_json` 为 `MissionContextStrategy`
- [ ] `stateless`：不注入任何历史
- [ ] `incremental`：查询上一次 run 的 observations，序列化为摘要注入 task prompt
- [ ] `cumulative`：查询 contextWindow.maxRuns 范围内的所有 observations，按 maxTokens 限制截断
- [ ] 超过窗口的历史，如果 `summarizeOlder=true`，用 LLM 生成摘要
- **验收**: "10 天趋势" 场景，run 10 的 prompt 包含前 9 天的 observations

### T-6.3 实现 Completion Condition 和 Synthesis Run
- [ ] `run_count`：Mission 的 run_count 达到目标值时 status -> completed
- [ ] `date`：当前时间超过目标日期时 status -> completed
- [ ] `observation_match`：出现匹配的 observation 时 status -> completed
- [ ] `manual`：只能手动完成
- [ ] `synthesisOnComplete=true` 时，完成前自动追加一个 synthesis run
- **验收**: run_count=10 的 Mission 在第 10 次 run 后自动完成

---

## Phase 7: Delivery 层

### T-7.1 实现 Delivery Target 抽象
- [ ] `deliver_to_bot()` — 通过现有 Bot gateway 发送消息
- [ ] `deliver_to_assistant()` — 写入 AI Assistant conversation
- [ ] `deliver_to_app_notification()` — 通过 Tauri 事件推送
- [ ] `deliver_to_webhook()` — HTTP POST
- **文件**: `src-tauri/src/services/mission_delivery.rs`
- **验收**: 各 target 可独立测试

### T-7.2 实现 Delivery Policy 路由
- [ ] run 完成后根据 `delivery_policy_json` 决定是否投递
- [ ] `onSuccess` / `onChange` / `onFailure` 分别处理
- [ ] quietHours 期间延迟投递
- **验收**: success-only policy 下失败不触发投递

### T-7.3 实现 Delivery 持久化
- [ ] 每次投递尝试写入 `mission_deliveries` 表
- [ ] 记录 status（pending / sent / failed）、error_message
- [ ] 支持 UI 查看和手动重试
- **验收**: 失败的 delivery 可见且可重试

---

## Phase 8: Bot Console UI

### T-8.1 新增 Missions 标签页
- [ ] 在 Bot Console 中新增 `Missions` tab
- [ ] Mission 列表：status badge、title、next run、profile name、latest result、failure count
- [ ] 支持按 owner（当前 peer）过滤
- **验收**: 列表正确展示当前 peer 的 Missions

### T-8.2 Mission 详情视图
- [ ] 展示：objective、trigger、delivery policy、status、budget usage
- [ ] Runs 列表：status、started_at、duration、result_summary
- [ ] Run 详情：steps、artifacts（按需展开）、observations、linked execution
- [ ] 操作按钮：pause / resume / run now / archive
- **验收**: 详情页信息完整，操作可用

### T-8.3 Mission 创建弹窗
- [ ] 结构化输入表单：objective、trigger（cron）、delivery target、profile 选择、budget
- [ ] 提交前执行确定性校验，展示校验结果
- [ ] 校验通过后创建 draft，用户确认后激活
- **验收**: 创建流程完整，校验拦截无效输入

---

## Phase 9: Mission Planner

### T-9.1 Planner Prompt Contract
- [ ] 设计 system prompt，要求 LLM 输出 `MissionDraft` 结构化 JSON
- [ ] 定义 MissionDraft schema（objective、trigger、delivery、profile、budget）
- [ ] 实现 JSON 解析和校验
- **验收**: 自然语言输入产生有效的 MissionDraft

### T-9.2 缺失字段追问
- [ ] 如果 MissionDraft 缺少必要字段（URL、trigger 时间等），生成追问消息
- [ ] 不创建不完整的 Mission
- **验收**: "监控这个网站" 在缺少 URL 时追问

---

## Phase 10: Operator Registry（按需）

### T-10.1 前置条件评估
- [ ] 至少有 3 个不同类型的 Mission 稳定运行 2 周以上
- [ ] 识别出重复的执行模式（如 fetch -> diff -> observe 出现 3 次以上）
- [ ] 如果没有明确的重复模式，跳过此阶段

### T-10.2 Operator 定义和注册
- [ ] 从实际执行日志中提取 operator 定义（id、requiredTools、inputSchema、outputSchema、riskLevel）
- [ ] 将 operators 映射到现有 tool ids
- [ ] 增加 input/output schema 校验

---

## Phase 11: 验证场景

### T-11.1 每日 AI 新闻摘要
- [ ] 创建 Mission：cron 每日 9:00，context_strategy=stateless
- [ ] 验证：search/fetch/summarize/delivery 全流程
- [ ] 验证：artifact 存储、observation 生成、delivery 到 Bot

### T-11.2 网站变化监控
- [ ] 创建 Mission：cron 每小时，context_strategy=incremental
- [ ] 验证：fetch/snapshot/content_hash diff/observation/onChange delivery
- [ ] 验证：无变化时不投递，有变化时立即通知

### T-11.3 每日项目测试运行
- [ ] 创建 Mission：cron 每日 8:00，success_criteria 包含 exit_code 检查
- [ ] 验证：shell 执行/log artifact/failure observation/onFailure delivery
- [ ] 验证：测试全部通过时 run=succeeded，有失败时 run=failed
