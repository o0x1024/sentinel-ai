## 目标与范围
- 将现有 Vue/Tauri 聊天助手升级为“Cursor Agent”风格的 ReAct 循环：简单任务直连工具执行，复杂任务先规划生成 todo，按计划分步推进。
- 引入持久化状态与记忆：记录 plan/todo、决策日志、修改与检查点、已运行命令与结果，支持回滚与审计。
- 基于仓库现状精简提示词体系，沿用现有工具与存储能力，不做过度重构。

## 架构总览
- 前端：Vue 3 + Pinia。核心页面 `src/components/AIChat.vue`、展示组件 `src/components/MessageParts/ReActStepDisplay.vue`。
- 后端：Tauri(Rust)。命令聚合 `src-tauri/src/commands/mod.rs`，工具系统 `src-tauri/src/tools/*`，AI 流出 `src-tauri/src/commands/ai.rs`。
- 数据：SQLite via `sqlx`。已存在 Plan-and-Execute 仓库 `sentinel-db/src/database/plan_execute_repository.rs`，工具执行记录 `tool_execution_dao.rs`，对话存储 `ai_conversation_dao.rs`。

## 任务流（ReAct 循环）
- 最外层 ReAct 循环
  - 入口：用户消息 → 分类器判定“简单/复杂”。
  - 简单任务：直接选择合适工具执行（统一工具系统），返回结果；必要时少量总结。
  - 复杂任务：先用 LLM 生成高层 plan 与 todo 列表；建立执行会话，分步推进，每步产出“思考/行动/观察”，更新 todo 状态。
- 子任务策略
  - 简单子任务：直接工具或直接回答。
  - 复杂子任务：局部计划 + 子 todo；按步执行并合并到母会话。
- 完成收束
  - 所有 todo 置为 completed 后，汇总产出、关键决策与证据链，写入会话结果并回显。

## 状态与记忆模块
- 前端 Pinia store：`src/stores/agent_memory.ts`
  - 会话级状态：`current_plan`、`todo_list`、`decision_log`、`checkpoints`、`change_log`、`command_runs`。
  - 方法（遵循下划线命名）：`init_session_state`、`save_plan`、`append_todo`、`mark_todo_in_progress`、`mark_todo_completed`、`append_decision`、`create_checkpoint`、`append_change`、`append_command_run`、`rollback_to_checkpoint`。
- 后端持久化（无新表优先，尽量复用现有仓库）
  - 使用 `execution_sessions` 存储会话主状态（context/metadata 扩展字段承载 plan/todo/决策聚合）。
  - 使用 `tool_executions` 存储命令与工具运行记录（输出、错误、资源、工件）。
  - 修改与检查点：记录到 `execution_sessions.context` 的结构化字段，并可选地将重要补丁/文件快照存储为“工件”。

## 后端命令扩展
- 新增命令组 `react_loop_commands`
  - `start_react_session`：基于用户目标生成 `plan` 与 `todo`，创建 `execution_session`。
  - `execute_simple_task`：选择工具并执行，写入 `tool_executions` 与消息。
  - `execute_next_step`：按当前会话 `todo` 推进一步；更新 `current_step`、`progress`、`step_results`、检查是否需要子计划。
  - `mark_todo_status`：更新指定 todo 状态（pending/in_progress/completed/cancelled）。
  - `create_checkpoint`/`rollback_checkpoint`：生成/回滚检查点；必要时附带文件快照/补丁元数据。
  - `finalize_session`：收束总结、统计与归档。

## 前端集成与UI变更
- `AIChat.vue`
  - 新增“会话进度条 + todo 列表”区域；支持状态切换与查看决策日志、命令记录。
  - 新增“回滚”入口：选择检查点回滚（调用后端命令）。
  - 消息管线接入 ReAct 循环：在复杂任务场景中以流式展示 Thought/Action/Observation。
- `ReActStepDisplay.vue`
  - 支持显示每步绑定的 todo 项与状态徽标，展示工具输出的关键信息摘要。

## 决策记录与可审计性
- 决策日志结构：时间戳、输入、备选方案、选择理由、风险评估、关联证据（代码/命令输出引用）。
- UI 可折叠查看；后端保存于 `execution_sessions.context.decision_log`，关键信息落入 `metadata` 便于检索。

## 检查点与回滚
- 检查点携带：文件路径集与快照标识、补丁摘要、关联 todo、原因说明。
- 回滚策略：优先“补丁逆向”与“快照恢复”；保持幂等与原子性；失败时提供部分回滚与提示。

## 安全与约束
- 工具执行遵守白名单/黑名单与分类规则（沿用 `unified_tools` 与 MCP 管理）。
- 命令非交互且最小权限；长任务后台运行并跟踪进度。
- 绝不提交密钥或泄露敏感信息；输出做脱敏与截断。

## 精简版系统提示词（参考 docs/cursor_prompt.md）
- 目标：仅保留与本项目相关的高价值规则，减少冗余。
- 核心要点
  - 语义检索优先：高层问题用“搜索代理”；精确匹配用 `grep`；已知文件直接读取。
  - 任务管理：复杂任务必须生成 plan/todo；一步一更，实时标记状态；完成即总结与归档。
  - 引用规范：展示现有代码用“代码引用块”，新提案用“语言标记代码块”；不要混用。
  - 工具约束：提案命令必须非交互，加 `--yes` 之类；长跑任务后台执行并跟踪。
- 交付：新增 `docs/cursor_agent_prompt_min.md`，仅 150–300 行，围绕上述四类规则与本项目工具集。

## 数据结构与类型
- 新增前端类型 `src/types/agent.ts`
  - `agent_session_state`、`agent_todo_item`、`agent_decision_item`、`agent_checkpoint`、`agent_change_item`、`agent_command_run`。
  - 字段命名统一采用下划线风格，兼容后端 JSON 序列化。

## 迭代步骤
1) 后端：新增 `react_loop_commands`，打通会话/步骤/工具执行与持久化。
2) 前端：新增 `agent_memory` store，接入聊天消息管线与 UI；实现 todo/决策/命令/检查点视图。
3) 提示词：落地 `cursor_agent_prompt_min.md`，绑定到 Agent 管理或聊天初始化流程。
4) 验证：端到端演示两类任务（简单工具、复杂计划），覆盖执行、状态标记、回滚、总结归档。

## 验证与度量
- 端到端用例：
  - 简单：调用某 MCP 工具检索，输出并归档。
  - 复杂：生成计划、分步运行工作流工具，记录命令与输出、回滚一次、最终总结。
- 指标：成功率、平均执行时间、todo 完成率；复用 `get_execution_statistics`。

## 交付物
- 新增/修改的前后端文件列表（store、命令、类型、组件变更）。
- 精简版系统提示词文件。
- 演示用会话数据与操作手册（短文档）。

## 风险与回避
- React vs ReAct：本仓库为 Vue；此“react 循环”解释为 ReAct 推理-行动循环，不引入 React 框架。
- 数据膨胀：将大输出做摘要与分片存储，必要时作为工件引用。
- 回滚一致性：优先文件级快照与补丁反向，限制跨工具副作用。