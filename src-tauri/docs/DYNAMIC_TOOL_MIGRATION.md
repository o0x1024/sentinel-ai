# 动态工具调用迁移指南

本文档详细说明如何将现有的硬编码工具调用迁移到统一的动态工具调用系统。

## 概述

当前系统存在以下问题：
1. 硬编码的工具调用（如 `plan_and_execute_agent.rs` 和 `llm_compiler_scheduler_agent.rs`）
2. 多套工具管理系统并存
3. 扩展性差，添加新工具需要修改多处代码
4. 缺乏统一的工具接口和参数验证

新的统一工具系统解决了这些问题，提供了：
- 统一的工具接口
- 动态工具注册和发现
- 自动参数验证和类型转换
- 内置工具和MCP工具的无缝集成
- 执行历史和统计信息

## 迁移步骤

### 1. 引入统一工具系统

在需要使用工具的模块中添加导入：

```rust
use crate::tools::{
    UnifiedToolManager, BuiltinToolProvider, McpAdapter, 
    DynamicToolAdapter, ContextualToolAdapter, ToolExecutionContext
};
```

### 2. 初始化工具管理器

```rust
// 创建统一工具管理器
let mut unified_manager = UnifiedToolManager::new_with_defaults();

// 注册内置工具提供者
let builtin_provider = BuiltinToolProvider::new();
unified_manager.register_provider("builtin".to_string(), Box::new(builtin_provider)).await?;

// 注册MCP工具提供者（如果可用）
if let Ok(mcp_manager) = McpToolManager::new().await {
    let mcp_adapter = McpAdapter::new(Arc::new(mcp_manager));
    let mcp_provider = mcp_adapter.create_provider();
    unified_manager.register_provider("mcp".to_string(), Box::new(mcp_provider)).await?;
}

// 刷新所有提供者
unified_manager.refresh_all_providers().await?;

// 创建动态工具适配器
let tool_adapter = DynamicToolAdapter::new(Arc::new(unified_manager));
```

### 3. 替换硬编码的工具调用

#### 原来的方式（plan_and_execute_agent.rs L727-738）：

```rust
match step.action.as_str() {
    "nmap_scan" => {
        let target = resolve_variables(&step.target, variable_values)?;
        let ports = step.parameters.get("ports")
            .and_then(|v| v.as_str())
            .unwrap_or("1-1000");
        execute_nmap_scan(&target, Some(ports)).await?
    }
    "service_scan" => {
        let target = resolve_variables(&step.target, variable_values)?;
        execute_service_scan(&target).await?
    }
    "nuclei_scan" => {
        let target = resolve_variables(&step.target, variable_values)?;
        let templates = step.parameters.get("templates")
            .and_then(|v| v.as_str());
        execute_nuclei_scan(&target, templates).await?
    }
    // ... 更多硬编码的工具调用
    _ => return Err(anyhow::anyhow!("Unknown action: {}", step.action))
}
```

#### 新的动态方式：

```rust
// 解析目标和参数
let target = resolve_variables(&step.target, variable_values)?;
let mut params = HashMap::new();

for (key, value) in &step.parameters {
    let resolved_value = resolve_variable_value(value, variable_values)?;
    params.insert(key.clone(), resolved_value);
}

// 使用动态工具适配器执行
let result = tool_adapter.execute_by_action(&step.action, &target, Some(params)).await?;

if result.success {
    result.output
} else {
    return Err(anyhow::anyhow!("Tool execution failed: {}", 
        result.error.unwrap_or("Unknown error".to_string())));
}
```

### 4. 替换LLM编译器调度器中的工具调用

#### 原来的方式（llm_compiler_scheduler_agent.rs L233-271）：

```rust
pub async fn execute_port_scan_task(&self, target: &str, port_range: Option<&str>) -> Result<Value> {
    // 硬编码的端口扫描实现
    let scan_id = self.tool_manager.start_scan("port_scan", json!({
        "target": target,
        "port_range": port_range.unwrap_or("1-65535")
    })).await?;
    
    // 等待扫描完成
    tokio::time::sleep(Duration::from_secs(5)).await;
    
    let result = self.tool_manager.get_scan_result(&scan_id).await?;
    Ok(result)
}
```

#### 新的动态方式：

```rust
pub async fn execute_port_scan_task(&self, target: &str, port_range: Option<&str>) -> Result<Value> {
    let mut params = HashMap::new();
    if let Some(port_range) = port_range {
        params.insert("ports".to_string(), json!(port_range));
    }
    
    let result = self.tool_adapter.execute_scan_task("port_scan", target, Some(params)).await?;
    
    if result.success {
        Ok(result.output)
    } else {
        Err(anyhow::anyhow!("Port scan failed: {}", 
            result.error.unwrap_or("Unknown error".to_string())))
    }
}
```

### 5. 更新结构体定义

#### 原来的结构体：

```rust
pub struct PlanAndExecuteAgent {
    llm_client: Arc<dyn LlmClient>,
    tool_manager: Arc<ToolManager>,
    // ...
}

pub struct LlmCompilerSchedulerAgent {
    llm_client: Arc<dyn LlmClient>,
    tool_manager: Arc<ToolManager>,  // 这个字段导致编译错误
    // ...
}
```

#### 新的结构体：

```rust
pub struct PlanAndExecuteAgent {
    llm_client: Arc<dyn LlmClient>,
    tool_adapter: DynamicToolAdapter,
    // ...
}

pub struct LlmCompilerSchedulerAgent {
    llm_client: Arc<dyn LlmClient>,
    tool_adapter: DynamicToolAdapter,
    // ...
}
```

### 6. 更新构造函数

```rust
impl PlanAndExecuteAgent {
    pub async fn new(llm_client: Arc<dyn LlmClient>) -> Result<Self> {
        // 初始化统一工具管理器
        let mut unified_manager = UnifiedToolManager::new_with_defaults();
        
        // 注册工具提供者
        let builtin_provider = BuiltinToolProvider::new();
        unified_manager.register_provider("builtin".to_string(), Box::new(builtin_provider)).await?;
        
        if let Ok(mcp_manager) = McpToolManager::new().await {
            let mcp_adapter = McpAdapter::new(Arc::new(mcp_manager));
            let mcp_provider = mcp_adapter.create_provider();
            unified_manager.register_provider("mcp".to_string(), Box::new(mcp_provider)).await?;
        }
        
        unified_manager.refresh_all_providers().await?;
        
        let tool_adapter = DynamicToolAdapter::new(Arc::new(unified_manager));
        
        Ok(Self {
            llm_client,
            tool_adapter,
            // ...
        })
    }
}
```

## 迁移的好处

### 1. 代码简化
- 消除了大量的硬编码match语句
- 统一的错误处理
- 减少重复代码

### 2. 扩展性提升
- 新工具只需注册到提供者，无需修改调用代码
- 支持运行时动态添加工具
- MCP工具自动发现和集成

### 3. 类型安全
- 统一的参数验证
- 自动类型转换
- 编译时检查

### 4. 可观测性
- 执行历史记录
- 工具使用统计
- 性能监控

### 5. 测试友好
- 工具可以轻松模拟
- 单元测试更简单
- 集成测试更可靠

## 兼容性考虑

### 1. 渐进式迁移
- 可以逐步迁移，新旧系统可以并存
- 保持现有API的兼容性
- 逐个文件进行迁移

### 2. 配置迁移
- 现有的工具配置可以自动转换
- 保持用户配置的兼容性

### 3. 数据迁移
- 执行历史可以迁移到新格式
- 保持数据的连续性

## 实施计划

### 阶段1：基础设施（已完成）
- [x] 创建统一工具接口
- [x] 实现内置工具提供者
- [x] 实现MCP工具适配器
- [x] 创建动态工具适配器

### 阶段2：核心代理迁移
- [ ] 迁移 `plan_and_execute_agent.rs`
- [ ] 迁移 `llm_compiler_scheduler_agent.rs`
- [ ] 更新相关测试

### 阶段3：其他组件迁移
- [ ] 迁移其他使用工具的代理
- [ ] 更新前端接口
- [ ] 更新文档

### 阶段4：优化和清理
- [ ] 移除旧的工具管理代码
- [ ] 性能优化
- [ ] 完善监控和日志

## 注意事项

1. **向后兼容性**：确保迁移过程中不破坏现有功能
2. **性能影响**：新系统可能有轻微的性能开销，需要监控
3. **错误处理**：确保错误信息的一致性和可读性
4. **日志记录**：保持详细的执行日志用于调试
5. **测试覆盖**：确保所有迁移的功能都有充分的测试

## 示例代码

完整的迁移示例请参考：
- `src/agents/dynamic_plan_execute_example.rs` - 动态执行示例
- `src/tools/dynamic_adapter.rs` - 动态适配器实现
- `docs/DYNAMIC_TOOL_CALLING_DESIGN.md` - 设计文档