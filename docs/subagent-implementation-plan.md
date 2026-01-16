## 方案1实施计划（Subagent）

目标：在现有 Agent 架构上新增 subagent 工具与执行器，复用流式执行链路，保证可控并发与工具隔离。

### 实施步骤
1. **新增 subagent 工具定义**
   - 位置：`src-tauri/sentinel-tools/src/buildin_tools/subagent_tool.rs`
   - 内容：`SubagentToolArgs`、`SubagentToolOutput`、`set_subagent_executor`

2. **注册 subagent 到 ToolServer**
   - 位置：`src-tauri/sentinel-tools/src/tool_server.rs`
   - 逻辑：用 `DynamicToolBuilder` 注册 `subagent_run`

3. **实现 subagent 执行器**
   - 位置：`src-tauri/src/agents/subagent_executor.rs`
   - 逻辑：
     - 保存父执行上下文（模型、provider、默认工具配置）
     - 执行子任务时复用 `execute_agent`
     - 默认工具策略：未指定时使用全部工具，且禁用 `subagent_run` 防递归
     - 通过 Tauri 事件发送 `subagent:start/done/error`

4. **初始化 subagent 执行器**
   - 位置：`src-tauri/src/lib.rs`
   - 与 `tenth_man_executor` 一致：设置 AppHandle 并初始化

5. **工具路由元数据**
   - 位置：`src-tauri/src/agents/tool_router.rs`
   - 添加 subagent 工具元数据（不做过度策略，仅用于可见性）

### 事件约定（前端对接）
- `subagent:start`：`{ execution_id, parent_execution_id, role, task }`
- `subagent:done`：`{ execution_id, parent_execution_id, success, output }`
- `subagent:error`：`{ execution_id, parent_execution_id, error }`

### 风险与控制
- 递归调用：默认禁用 `subagent_run`
- 资源消耗：限制 `max_iterations`、`timeout_secs`
- 并发控制：预留 `Semaphore` 扩展点（本次不实现并发控制）
