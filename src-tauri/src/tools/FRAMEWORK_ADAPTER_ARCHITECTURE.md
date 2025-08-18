# 框架适配器架构设计文档

## 概述

本文档描述了为三个AI框架（Plan & Execute、ReWOO、LLM Compiler）重构tools模块的长期方案。重构目标是提供统一、高效、可扩展的工具调用接口。

## 架构设计

### 1. 核心接口

#### FrameworkToolAdapter
统一的框架适配器接口，为不同框架提供一致的工具调用抽象：

```rust
#[async_trait]
pub trait FrameworkToolAdapter: Send + Sync + std::fmt::Debug {
    fn adapter_name(&self) -> &str;
    async fn execute_tool(&self, call: UnifiedToolCall) -> Result<UnifiedToolResult>;
    async fn execute_tools_batch(&self, calls: Vec<UnifiedToolCall>) -> Vec<Result<UnifiedToolResult>>;
    async fn list_available_tools(&self) -> Vec<String>;
    async fn get_tool_info(&self, tool_name: &str) -> Option<ToolInfo>;
    async fn is_tool_available(&self, tool_name: &str) -> bool;
    async fn validate_tool_call(&self, tool_name: &str, call: &UnifiedToolCall) -> Result<()>;
}
```

#### EngineToolAdapter
为LLM Compiler提供向后兼容的接口：

```rust
#[async_trait]
pub trait EngineToolAdapter: Send + Sync + std::fmt::Debug {
    async fn execute_tool(&self, call: UnifiedToolCall) -> Result<UnifiedToolResult>;
    async fn list_available_tools(&self) -> Vec<String>;
    async fn get_tool_info(&self, tool_name: &str) -> Option<ToolInfo>;
}
```

### 2. 框架专用适配器

#### PlanAndExecuteAdapter
- 特点：保守的并发控制（最多5个并发调用）
- 执行模式：通常顺序执行步骤
- 超时设置：300秒默认超时
- 重试策略：最多2次重试，2秒基础延迟

#### ReWOOAdapter
- 特点：支持变量替换和步骤间数据传递
- 执行模式：顺序执行，支持#E变量传递
- 超时设置：180秒默认超时
- 重试策略：最多1次重试，1秒基础延迟
- 特殊功能：`substitute_variables()` 方法处理ReWOO特有的变量占位符

#### LLMCompilerAdapter
- 特点：支持高并发（最多10个并发调用）
- 执行模式：支持并行执行
- 超时设置：120秒默认超时
- 重试策略：最多3次重试，500毫秒基础延迟
- 兼容性：同时实现FrameworkToolAdapter和EngineToolAdapter

### 3. 基础适配器

#### BaseFrameworkAdapter
提供所有适配器的通用功能：

- **缓存机制**：智能缓存成功的工具调用结果（5分钟过期）
- **并发控制**：通过信号量控制最大并发数
- **重试机制**：支持指数退避的重试策略
- **超时处理**：支持工具级和调用级超时设置
- **错误处理**：统一的错误处理和日志记录

### 4. 工厂和管理

#### AdapterFactory
负责创建不同类型的适配器实例：

```rust
pub struct AdapterFactory {
    pub fn create_adapter(&self, framework_type: FrameworkType) -> Arc<dyn FrameworkToolAdapter>;
    pub fn create_engine_adapter(&self) -> Arc<dyn EngineToolAdapter>;
    pub fn create_base_adapter(&self, config: AdapterConfig) -> Arc<dyn FrameworkToolAdapter>;
}
```

#### GlobalAdapterManager
全局单例管理器，负责：

- 适配器的注册和获取
- 生命周期管理
- 统计信息收集
- 预初始化所有框架适配器

### 5. 配置系统

#### AdapterConfig
支持灵活的适配器配置：

```rust
pub struct AdapterConfig {
    pub framework_type: FrameworkType,
    pub cache_enabled: bool,
    pub max_concurrent_calls: usize,
    pub default_timeout: Duration,
    pub retry_policy: RetryPolicy,
}
```

#### RetryPolicy
详细的重试策略配置：

```rust
pub struct RetryPolicy {
    pub max_retries: u32,
    pub base_delay_ms: u64,
    pub max_delay_ms: u64,
    pub backoff_multiplier: f64,
}
```

## 性能优化

### 1. 缓存机制
- **结果缓存**：相同参数的工具调用结果缓存5分钟
- **智能键值**：基于工具名称、参数和上下文版本生成缓存键
- **内存管理**：限制缓存大小，自动清理过期条目

### 2. 并发控制
- **框架特化**：每个框架根据特点设置不同的并发限制
- **信号量机制**：使用tokio信号量控制并发数量
- **批量执行**：支持高效的批量工具调用

### 3. 重试和容错
- **指数退避**：智能的重试延迟计算
- **错误分类**：区分不同类型的错误，制定相应策略
- **超时处理**：多层次的超时控制（工具级、调用级、框架级）

## 使用方式

### 1. 初始化
```rust
// 在lib.rs中初始化
let tool_manager = tool_system.get_manager();
crate::tools::initialize_global_adapter_manager(tool_manager).await?;
```

### 2. 获取适配器
```rust
// 获取框架特定适配器
let adapter = crate::tools::get_framework_adapter(FrameworkType::ReWOO).await?;

// 获取引擎适配器（LLM Compiler兼容）
let engine_adapter = crate::tools::get_engine_adapter().await?;
```

### 3. 执行工具调用
```rust
let call = UnifiedToolCall {
    id: Uuid::new_v4().to_string(),
    tool_name: "nmap".to_string(),
    parameters: tool_params,
    timeout: Some(Duration::from_secs(300)),
    context: HashMap::new(),
    retry_count: 0,
};

let result = adapter.execute_tool(call).await?;
```

## 向后兼容性

### 1. 现有接口保持
- LLM Compiler的EngineToolAdapter接口完全兼容
- 现有的UnifiedToolCall和UnifiedToolResult类型保持不变
- 全局函数`get_global_engine_adapter()`继续可用

### 2. 渐进迁移
- 新功能使用新的框架适配器接口
- 现有代码可以继续使用旧接口
- 提供迁移指南和最佳实践

## 扩展性

### 1. 新框架支持
添加新框架只需：
1. 实现FrameworkToolAdapter接口
2. 在FrameworkType枚举中添加新类型
3. 在AdapterFactory中添加创建逻辑

### 2. 自定义适配器
支持创建自定义适配器配置：
```rust
let config = AdapterConfigBuilder::new(FrameworkType::Custom)
    .max_concurrent_calls(20)
    .cache_enabled(false)
    .build();
let adapter = factory.create_base_adapter(config);
```

## 测试策略

### 1. 单元测试
- 每个适配器的独立测试
- 缓存机制测试
- 重试逻辑测试
- 并发控制测试

### 2. 集成测试
- 端到端工具调用测试
- 框架间兼容性测试
- 性能压力测试

### 3. 回归测试
- 确保现有功能正常工作
- 验证向后兼容性
- 性能回归检测

## 监控和调试

### 1. 日志记录
- 结构化日志记录所有工具调用
- 性能指标收集
- 错误和异常追踪

### 2. 统计信息
- 工具调用成功率
- 平均执行时间
- 缓存命中率
- 并发使用情况

### 3. 调试工具
- 适配器状态查询
- 缓存内容查看
- 执行历史追踪

## 总结

这个长期重构方案提供了：

1. **统一接口**：三个框架使用一致的工具调用抽象
2. **性能优化**：缓存、并发控制、重试机制
3. **框架特化**：每个框架的专门优化
4. **向后兼容**：现有代码无需修改
5. **易于扩展**：支持新框架和自定义配置
6. **生产就绪**：完整的错误处理、监控和调试支持

通过这个架构，tools模块可以更好地支持三个框架的不同需求，同时提供出色的性能和可维护性。

