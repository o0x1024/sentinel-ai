# Sentinel-AI 统一联动方案（LLM 三大架构 × Agent × Prompt × AI 助手）

本文档梳理当前实现的问题，并给出一套“前后端一体”的最小完备方案，统一串联 Plan-and-Execute / LLMCompiler / ReWOO 三大架构，与 Agent、Prompt 系统和前端 AI 助手流式 UI 的协作方式。

## 1. 现状与问题

- 架构分层已具备：三种引擎各有 `engine_adapter` 与核心组件，但存在接口不一致与事件输出不统一的问题。
- 事件流问题：前端通过 `ai_stream_message` 统一消费，但后端在执行开始前缺少 `PlanUpdate`，导致计划不可视化；`stop_execution` 未真正取消执行（已修复）。
- Prompt 解析：已实现统一解析器 `PromptResolver`，但运行期模板 ID 透传路径不完全一致（现已在 P&E 转换步骤中注入 `prompt_template_executor_id`）。
- 进度/取消：P&E 未实现 `cancel_execution`（已补齐，调用 `Executor.cancel()`）。

## 2. 统一交互协议（后端 → 前端）

后端统一通过 `ai_stream_message` 事件发送以下消息：

- `PlanUpdate`：附带 `execution_plan`（包含步骤、依赖、参数）。触发时机：`start_execution` 拿到上下文后立即发送。
- `ToolUpdate`：工具执行进展/结果，由执行器实时发送。
- `Content`：LLM 增量内容（如需要）。
- `FinalResult`：最终聚合结果文本。
- `Error`：错误详情。

前端 `useEventListeners.ts` 已支持上述类型的统一处理。

## 3. 统一调度入口

- 前端：`AIChat.vue` 在 `/task` 模式调用 `dispatch_scenario_task`，后台根据 Agent Profile 选择架构（或 Auto），并注册引擎实例到 `ExecutionManager`。
- 后端：`dispatch_intelligent_query` 负责创建计划、注册 `EngineInstance`，随后异步 `start_execution(execution_id)`。
- 执行开始即发送 `PlanUpdate`（已实现）。

## 4. Prompt 统一解析与注入

- 解析入口：各架构 Planner 使用 `PromptResolver` 解析对应阶段模板。
- 运行期参数：通过 `request.options.prompt_ids.*` 注入，P&E 在步骤转换时为 `LlmCall` 注入 `prompt_template_executor_id`，执行器读取并优先使用该模板。

## 5. 取消/清理策略

- 前端：`stopExecution()` 同时调用 `stop_execution` 与 `cancel_ai_stream`。
- 后端：`stop_execution` 通过 `ExecutionManager.stop_execution` 调用具体引擎的 `cancel_execution` 并清理上下文（已实现）。
- P&E：实现了 `cancel_execution()` → `Executor.cancel()`。
- LLMCompiler/ReWOO：已有 `cancel_pending_tasks()` 等接口，可继续按需扩展。

## 6. 最小改动已落地

- 增加 `PlanUpdate` 下发：`start_execution` 在执行前发送计划 JSON，前端可直接展示。
- 修复 `stop_execution`：真正调用 `ExecutionManager` 停止执行并清理。
- 为 Plan-and-Execute 补齐 `cancel_execution`。

## 7. 前端展示建议（已兼容现状）

- 在 `AIChat.vue` 中接收 `PlanUpdate` 后，将 `executionPlan` 绑定到当前助手消息，使用现有步骤面板组件渲染即可。
- `ToolUpdate`/`FinalResult` 已由监听器合并到同一条助手消息，保证用户视角的一致性。

## 8. 后续可选增强（保持最小化改动原则）

- 统一计划与步骤类型：抽象到公共 `engines::types`，在各引擎转换层适配。
- 进度轮询 API：基于 `ExecutionManager.get_execution_progress`，前端显示进度条。
- Prompt 版本固定：在 `PromptResolver` 中启用 `pinned_versions` 检查。
- 失败重试/回滚：在 P&E 的 `Replanner` 中根据 `EnhancedExecutionFeedback` 动态调整。

---

本方案以“极小改动实现统一体验”为目标：保持既有模块不变，统一事件、取消与计划可视化的关键链路，前端无需大改即可获得稳定的三架构联动体验。


