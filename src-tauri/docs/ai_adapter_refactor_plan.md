# AI模块精简重构计划

## 当前模块分析

### 现有文件结构
- `core.rs` - 核心trait和管理器
- `error.rs` - 错误处理
- `mod.rs` - 模块导出
- `multi_model_manager.rs` - 多模型管理器（复杂的负载均衡功能）
- `provider_adapter.rs` - 提供商适配器（兼容性层）
- `raw_message.rs` - 原始消息处理
- `types.rs` - 类型定义
- `utils.rs` - 工具函数
- `providers/` - 各种AI提供商实现

### 冗余和兼容性功能识别

1. **multi_model_manager.rs** - 过于复杂的多模型管理
   - 负载均衡策略（轮询、加权轮询、最少连接等）
   - 智能模型选择器
   - 性能跟踪器
   - 健康检查
   - 这些功能对单机桌面应用过于复杂

2. **provider_adapter.rs** - 兼容性适配器
   - `exec_chat` 和 `exec_chat_stream` 方法
   - 原始请求和标准请求之间的转换
   - 这是为了兼容TipaiClient而设计的兼容层

3. **core.rs中的send_chat相关方法**
   - `send_chat_request` 和 `send_chat_stream`
   - 需要用 `send_raw_chat` 替代

4. **utils.rs中的复杂工具**
   - HTTP客户端包装器的请求响应记录功能
   - 过于复杂的验证函数

## 精简方案

### 第一阶段：删除冗余模块
1. 删除 `multi_model_manager.rs` - 多模型管理过于复杂
2. 删除 `provider_adapter.rs` - 兼容性适配器
3. 简化 `utils.rs` - 保留基本工具函数

### 第二阶段：简化核心功能
1. 修改 `core.rs`：
   - 删除 `send_chat_request` 和 `send_chat_stream` 方法
   - 添加 `send_raw_chat` 方法
   - 简化 `AiAdapterManager`

2. 合并文件：
   - 将 `raw_message.rs` 的核心功能合并到 `types.rs`
   - 将简化后的工具函数合并到需要的模块中

### 第三阶段：更新模块导出
1. 更新 `mod.rs` 移除已删除模块的导出
2. 确保编译通过

## 实施步骤

1. ✅ 分析现有模块结构
2. ⏳ 删除 `multi_model_manager.rs`
3. ⏳ 删除 `provider_adapter.rs`
4. ⏳ 简化 `core.rs`，用 `send_raw_chat` 替代 `send_chat`
5. ⏳ 简化 `utils.rs`
6. ⏳ 合并 `raw_message.rs` 到 `types.rs`
7. ⏳ 更新 `mod.rs`
8. ⏳ 修复编译错误
9. ⏳ 测试功能完整性

## 预期结果

精简后的AI模块将包含：
- `core.rs` - 简化的核心功能，只包含 `send_raw_chat`
- `types.rs` - 合并了原始消息处理的类型定义
- `error.rs` - 错误处理（保持不变）
- `utils.rs` - 简化的工具函数
- `providers/` - AI提供商实现（保持不变）
- `mod.rs` - 更新的模块导出

这样可以大大简化代码结构，去除不必要的复杂性，更适合单机桌面应用的需求。