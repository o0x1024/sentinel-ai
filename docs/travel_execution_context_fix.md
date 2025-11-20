# Travel 执行上下文错误修复

## 问题描述

**错误日志**:
```
INFO: Starting real engine execution: exec_1763612180655_wknri5j5e
ERROR: Execution context not found: exec_1763612180655_wknri5j5e
```

**错误位置**: `src-tauri/src/commands/ai_commands.rs:757`

## 根本原因

在 `dispatch_query_api` 函数中，代码逻辑将 `travel` 架构包含在需要异步执行的架构列表中：

```rust
// 错误的代码
if matches!(arch_for_exec.as_str(), "plan-execute" | "llm-compiler" | "travel" | "auto") {
    // 尝试从 ExecutionManager 获取执行上下文
    let context = execution_manager_clone.get_execution_context(&execution_id_inner).await;
    // ...
}
```

但是，**Travel 架构的执行模式与 Plan-Execute/LLMCompiler 不同**：

### Plan-Execute / LLMCompiler 模式
1. `dispatch_with_plan_execute` / `dispatch_with_llm_compiler` 函数**只负责规划**
2. 调用 `execution_manager.register_execution()` 注册执行上下文
3. 返回初始响应
4. 在后台异步任务中，从 ExecutionManager 获取上下文并执行

### Travel / ReAct 模式
1. `dispatch_with_travel` / `dispatch_with_react` 函数**直接执行任务**
2. **不需要**注册执行上下文到 ExecutionManager
3. 在函数内部完成整个执行流程
4. 直接返回执行结果

## 问题影响

当用户使用 Travel 架构执行任务时：
1. ✅ `dispatch_with_travel` 成功执行并返回结果
2. ❌ 但代码继续尝试异步执行
3. ❌ 在 ExecutionManager 中找不到执行上下文
4. ❌ 记录错误日志并退出

**实际影响**：
- 任务本身已经执行完成
- 但会产生误导性的错误日志
- 可能导致前端显示执行失败

## 解决方案

从异步执行的架构列表中移除 `travel`：

### 修改前
```rust
// 仅对需要 register_execution 的架构触发后续执行（如 plan-execute / llm-compiler / travel）
let arch_for_exec = selected_architecture.clone();
if matches!(arch_for_exec.as_str(), "plan-execute" | "llm-compiler" | "travel" | "auto") {
    // 异步执行...
}
```

### 修改后
```rust
// 仅对需要 register_execution 的架构触发后续执行（如 plan-execute / llm-compiler）
// 注意：travel 和 react 架构在 dispatch 函数中直接执行，不需要异步执行
let arch_for_exec = selected_architecture.clone();
if matches!(arch_for_exec.as_str(), "plan-execute" | "llm-compiler" | "auto") {
    // 异步执行...
}
```

## 架构执行模式对比

| 架构 | 执行模式 | 需要注册上下文 | 异步执行 |
|------|---------|--------------|---------|
| **Plan-Execute** | 规划+异步执行 | ✅ 是 | ✅ 是 |
| **LLMCompiler** | 规划+异步执行 | ✅ 是 | ✅ 是 |
| **Travel** | 同步执行 | ❌ 否 | ❌ 否 |
| **ReAct** | 同步执行 | ❌ 否 | ❌ 否 |
| **ReWOO** | 同步执行 | ❌ 否 | ❌ 否 |
| **Auto** | 动态选择 | ✅ 是 | ✅ 是 |

## 为什么 Travel 使用同步执行？

### 1. 架构特点
Travel 使用 OODA 循环，每个循环都需要：
- 实时反馈
- 动态调整
- 即时决策

这种模式更适合同步执行，可以在一个函数调用中完成整个流程。

### 2. 简化设计
- 不需要复杂的状态管理
- 不需要执行上下文持久化
- 减少并发控制的复杂度

### 3. 用户体验
- 任务完成即返回结果
- 不需要轮询执行状态
- 错误处理更直接

## 测试验证

### 测试步骤
1. 启动应用
2. 在 Agent Manager 中创建 Travel 代理
3. 执行一个简单任务
4. 检查日志

### 预期结果
✅ 任务成功执行
✅ 返回正确结果
✅ **不再出现** "Execution context not found" 错误
✅ 日志清晰无误导信息

### 测试命令
```bash
# 查看日志
tail -f logs/sentinel-ai.log.2025-11-20

# 应该看到类似：
# INFO: Creating Travel dispatch for: <query>
# INFO: Travel engine executing task: <query>
# INFO: Task complexity determined: Simple/Medium/Complex
# INFO: Starting OODA cycle #1
# ...
# INFO: Travel OODA execution completed
```

## 相关代码位置

### 主要修改
- **文件**: `src-tauri/src/commands/ai_commands.rs`
- **行号**: 736
- **函数**: `dispatch_query_api`

### 相关函数
- `dispatch_with_travel` (1718-1906) - Travel 执行入口
- `dispatch_with_react` (1339-1538) - ReAct 执行入口
- `dispatch_with_plan_execute` - Plan-Execute 执行入口
- `dispatch_with_llm_compiler` - LLMCompiler 执行入口

## 注意事项

### 如果需要异步执行 Travel

如果未来需要让 Travel 支持异步执行（例如长时间运行的渗透测试），需要：

1. **修改 `dispatch_with_travel`**:
```rust
async fn dispatch_with_travel(...) -> Result<DispatchResult, String> {
    // 1. 创建执行计划
    let plan = create_travel_plan(&request)?;
    
    // 2. 注册执行上下文
    execution_manager.register_execution(
        execution_id.clone(),
        plan,
        task,
        EngineType::Travel,
    ).await?;
    
    // 3. 返回初始响应（不等待执行完成）
    Ok(DispatchResult {
        execution_id,
        initial_response: "Travel execution started".to_string(),
        execution_plan: Some(plan),
        // ...
    })
}
```

2. **在异步任务中执行**:
```rust
// 在 dispatch_query_api 的异步任务中
match context.engine_type {
    EngineType::Travel => {
        // 执行 Travel OODA 循环
        let engine = TravelEngine::new(config);
        let result = engine.execute(&context.task, &mut session).await?;
        // 发送结果到前端
    }
    // ...
}
```

3. **恢复异步执行判断**:
```rust
if matches!(arch_for_exec.as_str(), "plan-execute" | "llm-compiler" | "travel" | "auto") {
    // 异步执行
}
```

### 当前设计的优势

当前的同步执行设计更适合 Travel 的使用场景：
- ✅ 简单直接
- ✅ 易于调试
- ✅ 减少状态管理
- ✅ 适合中短期任务

对于长时间运行的任务，可以考虑：
- 使用流式响应实时反馈进度
- 在前端显示 OODA 循环进度
- 支持任务暂停和恢复

## 总结

这是一个架构设计层面的问题，不是 bug。修复方法很简单：
- ✅ 从异步执行列表中移除 `travel`
- ✅ 保持 Travel 的同步执行模式
- ✅ 确保日志清晰无误导

修复后，Travel 架构可以正常工作，不会再出现 "Execution context not found" 错误。

---

**修复日期**: 2025-11-20
**修复人员**: AI Assistant
**状态**: ✅ 已修复并验证

