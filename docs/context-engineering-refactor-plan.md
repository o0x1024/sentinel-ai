# Context Engineering 重构计划

目标：完成上下文工程的完整重构，覆盖上下文裁剪/压缩、上下文分层、Agent/Subagent 隔离、断点续跑与可恢复状态，并对现有 `agents` 模块进行结构化拆分。

## 计划与进度

1. 设计并落地 Context Engineering 模块骨架（分层模型、策略、Builder、Checkpoint、Tool Digest）
   - 状态：完成
2. 将主 Agent 上下文构建迁移到 Context Engineering（替换 system prompt 拼接逻辑）
   - 状态：完成
3. 强化 Tool Context 裁剪与结构化摘要，并在工具结果回流时更新 RunState
   - 状态：完成
4. Subagent 隔离策略改造（最小继承、任务 brief、禁用污染层）
   - 状态：完成
5. 拆分 `executor.rs`（>1000 行）为模块化文件并完成编译自洽
   - 状态：完成
6. 补充断点续跑数据表与恢复注入策略（RunState -> Layer D）
   - 状态：完成

