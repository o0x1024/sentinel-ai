# Orchestrator 架构消息分发修复

## 问题描述

用户在使用 Orchestrator agent 发送消息时遇到错误：
```
Unsupported architecture: orchestrator
```

## 根本原因

虽然之前已经在多个地方添加了 Orchestrator 支持（如 `AgentEngine` 枚举、`ArchitectureType` 枚举、数据库映射等），但在消息分发逻辑中缺少对 Orchestrator 架构的处理。

## 修复内容

### 1. 添加 Orchestrator 引擎导入

**文件**: `src-tauri/src/commands/ai_commands.rs`

```rust
use crate::engines::orchestrator::engine_adapter::OrchestratorEngineAdapter;
```

### 2. 在主分发逻辑中添加 Orchestrator 分支

**文件**: `src-tauri/src/commands/ai_commands.rs`

在 `dispatch_agent_task` 函数的 match 语句中添加：

```rust
"orchestrator" => {
    dispatch_with_orchestrator(
        execution_id.clone(),
        dispatch_req,
        (*ai_service_manager).clone(),
        (*db_service).clone(),
        (*execution_manager).clone(),
        app_clone.clone(),
    ).await
}
```

### 3. 在自动架构选择中添加 Orchestrator 支持

**文件**: `src-tauri/src/commands/ai_commands.rs`

在 `dispatch_with_auto` 函数的 match 语句中添加：

```rust
"orchestrator" => dispatch_with_orchestrator(execution_id, request, ai_service_manager, db_service, execution_manager, app).await,
```

### 4. 创建 dispatch_with_orchestrator 函数

**文件**: `src-tauri/src/commands/ai_commands.rs`

实现完整的 Orchestrator 分发逻辑：

```rust
async fn dispatch_with_orchestrator(
    execution_id: String,
    request: DispatchQueryRequest,
    ai_service_manager: Arc<AiServiceManager>,
    db_service: Arc<DatabaseService>,
    execution_manager: Arc<crate::managers::ExecutionManager>,
    app: AppHandle,
) -> Result<DispatchResult, String> {
    // 创建 SecurityTestManager
    let session_manager = Arc::new(crate::managers::SecurityTestManager::new());
    
    // 创建 Orchestrator 引擎
    let mut engine = OrchestratorEngineAdapter::new(session_manager.clone());
    
    // 确保子代理已注册
    engine.ensure_sub_agents_registered(
        ai_service_manager.clone(),
        db_service.clone(),
        Some(app.clone()),
    ).await.map_err(|e| format!("Failed to register sub-agents: {}", e))?;
    
    // 创建执行计划
    let plan = engine.create_plan(&task).await
        .map_err(|e| format!("Failed to create Orchestrator plan: {}", e))?;
    
    // 注册执行上下文和引擎实例
    let engine_instance = crate::managers::EngineInstance::Orchestrator(Box::new(engine));
    execution_manager.register_execution(
        execution_id.clone(),
        crate::managers::EngineType::Orchestrator,
        plan.clone(),
        task,
        engine_instance,
    ).await.map_err(|e| format!("Failed to register execution: {}", e))?;
    
    // 返回分发结果
    Ok(DispatchResult {
        execution_id,
        initial_response: "已启动Orchestrator安全测试编排引擎，引擎实例已注册，准备真实执行...".to_string(),
        execution_plan: Some(execution_plan),
        estimated_duration: plan.estimated_duration,
        selected_architecture: "Orchestrator".to_string(),
    })
}
```

### 5. 添加异步执行支持

**文件**: `src-tauri/src/commands/ai_commands.rs`

更新异步执行的架构匹配条件：

```rust
if matches!(arch_for_exec.as_str(), "plan-execute" | "llm-compiler" | "orchestrator" | "auto") {
    // 异步开始执行
}
```

### 6. 修复 ensure_sub_agents_registered 方法

**文件**: `src-tauri/src/engines/orchestrator/engine_adapter.rs`

将私有方法改为公开方法，并添加必要的参数：

```rust
pub async fn ensure_sub_agents_registered(
    &self,
    _ai_service_manager: Arc<crate::services::AiServiceManager>,
    _db_service: Arc<crate::services::database::DatabaseService>,
    _app_handle: Option<tauri::AppHandle>,
) -> Result<()> {
    // 注册子代理执行器
    // ...
    Ok(())
}
```

## 修复的文件清单

### 核心分发逻辑
1. `src-tauri/src/commands/ai_commands.rs` - 添加分发逻辑和 dispatch_with_orchestrator 函数
2. `src-tauri/src/engines/orchestrator/engine_adapter.rs` - 修复方法可见性

### 提示词管理
3. `src-tauri/src/commands/prompt_api.rs` - 添加 orchestrator 到架构类型映射
4. `src-tauri/src/utils/prompt_resolver.rs` - 添加 Orchestrator 的阶段映射

## 详细修改说明

### 3. prompt_api.rs 修改

在 `map_engine_to_arch` 函数中添加 orchestrator 和 react 的映射：

```rust
fn map_engine_to_arch(engine: &str) -> ArchitectureType {
    match engine {
        "rewoo" => ArchitectureType::ReWOO,
        "llm-compiler" => ArchitectureType::LLMCompiler,
        "orchestrator" => ArchitectureType::Orchestrator,  // 新增
        "react" => ArchitectureType::ReAct,                // 新增
        _ => ArchitectureType::PlanExecute,
    }
}
```

### 4. prompt_resolver.rs 修改

在 `to_architecture_stage` 方法中添加 Orchestrator 的阶段映射：

```rust
(CanonicalStage::Planner, ArchitectureType::Orchestrator) => Some(StageType::Planning),
(CanonicalStage::Executor, ArchitectureType::Orchestrator) => Some(StageType::Execution),
(CanonicalStage::Evaluator, ArchitectureType::Orchestrator) => Some(StageType::Evaluation),
```

这样 Orchestrator 就可以使用动态提示词系统，从数据库中加载和管理其提示词模板。

## 测试验证

编译成功，无错误：
```bash
cd src-tauri && cargo check
# Finished `dev` profile [unoptimized] target(s) in 1m 16s
```

## 使用方式

现在用户可以：

1. 在 Agent Manager 中创建 Orchestrator 类型的 agent
2. 在 AI 助手中选择该 agent 发送消息
3. Orchestrator 会自动协调 ReWOO、Plan-and-Execute、LLM-Compiler 等子代理完成安全测试任务

## 相关文档

- [Orchestrator 实现计划](./orchestrator_agent_implementation_plan.md)
- [Orchestrator 快速开始](./orchestrator_quick_start.md)
- [Orchestrator 使用指南](./orchestrator_usage_guide.md)
- [Orchestrator 设置完成](./orchestrator_setup_complete.md)

## 完成时间

2025-11-18

