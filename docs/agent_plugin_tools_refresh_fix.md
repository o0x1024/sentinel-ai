# Agent插件工具在ReAct任务模式下不显示的问题修复

## 问题描述

当在UI中创建或修改Agent类型插件后，在任务模式（使用ReAct执行器）下，新添加或修改的Agent插件工具不会出现在系统提示的"可用工具"列表中。

## 根本原因

### 工具注册和使用流程

1. **工具注册流程**：
   ```
   数据库 → AgentPluginProvider.get_tools() → 
   UnifiedToolManager.register_provider() → 
   tool_registry (HashMap<String, Arc<dyn UnifiedTool>>)
   ```

2. **工具使用流程**：
   ```
   ReAct Executor → framework_adapter.list_available_tools() →
   UnifiedToolManager.list_tools() → 读取 tool_registry →
   返回工具列表 → 构建系统提示
   ```

3. **问题所在**：
   - AgentPluginProvider在应用启动时注册一次（`src-tauri/src/lib.rs:287`）
   - 注册时会调用`get_tools()`读取数据库中的Agent插件
   - 工具列表被缓存在`tool_registry`中
   - **当用户创建/修改/启用/禁用Agent插件后，`tool_registry`没有更新**
   - ReAct执行器读取的仍然是旧的工具列表

### 缺失的刷新机制

以下命令修改了Agent插件但没有刷新工具系统：
- `create_plugin_in_db()` - 创建新插件
- `update_plugin_code()` - 更新插件代码
- `enable_plugin()` - 启用插件
- `disable_plugin()` - 禁用插件

## 解决方案

### 修改内容

在以下三个命令中添加工具系统刷新逻辑：

#### 1. `create_plugin_in_db()` 
```rust
// 如果是Agent插件，刷新全局工具系统
if main_category == "agent" {
    if let Ok(tool_system) = crate::tools::get_global_tool_system() {
        if let Err(e) = tool_system.refresh_all().await {
            tracing::warn!("Failed to refresh tool system after creating agent plugin: {}", e);
        } else {
            tracing::info!("Tool system refreshed successfully after creating agent plugin");
        }
    }
}
```

#### 2. `update_plugin_code()`
```rust
// 查询插件的main_category
let main_category: Option<String> = sqlx::query_scalar(
    "SELECT main_category FROM plugin_registry WHERE id = ?"
)
.bind(&plugin_id)
.fetch_optional(db.pool())
.await?;

// 如果是Agent插件，刷新全局工具系统
if let Some(cat) = main_category {
    if cat == "agent" {
        if let Ok(tool_system) = crate::tools::get_global_tool_system() {
            tool_system.refresh_all().await?;
        }
    }
}
```

#### 3. `enable_plugin()` 和 `disable_plugin()`
同样的逻辑应用到这两个命令

### 工作原理

1. **刷新触发**：当Agent插件被创建/修改/启用/禁用时，调用`tool_system.refresh_all()`
2. **刷新过程**：
   ```rust
   // UnifiedToolManager::refresh_all_providers()
   for (provider_name, provider) in &self.providers {
       provider.refresh().await?; // 刷新提供者
       let tools = provider.get_tools().await?; // 重新读取工具列表
       let mut registry = self.tool_registry.write().await;
       for tool in tools {
           registry.insert(tool.name().to_string(), tool); // 更新注册表
       }
   }
   ```
3. **工具可见**：刷新后，`tool_registry`包含最新的Agent插件工具
4. **系统提示更新**：下次ReAct执行器构建系统提示时，会获取到更新后的工具列表

### 只刷新Agent插件的原因

代码中检查`main_category == "agent"`来决定是否刷新工具系统，这是因为：
- **Passive插件**：属于被动扫描系统，不需要注册到工具系统
- **Agent插件**：需要在AI任务中作为工具调用，必须刷新工具注册表
- **性能考虑**：避免不必要的刷新操作

## 测试验证

### 测试步骤

1. **创建Agent插件**：
   - 在UI中创建一个新的Agent插件
   - 设置main_category为"agent"
   - 保存插件代码

2. **验证刷新日志**：
   ```
   Plugin created in database: <plugin_id>
   Tool system refreshed successfully after creating agent plugin
   ```

3. **检查工具列表**：
   - 调用`unified_list_tools`命令
   - 确认新插件工具出现在列表中

4. **测试ReAct执行**：
   - 创建一个使用ReAct框架的任务
   - 检查系统提示中的"可用工具"部分
   - 确认新插件工具包含在内

5. **测试禁用/启用**：
   - 禁用Agent插件，验证工具从列表中移除
   - 重新启用，验证工具再次出现

## 相关文件

### 修改的文件
- `src-tauri/src/commands/passive_scan_commands.rs`
  - `create_plugin_in_db()` - 添加创建后刷新
  - `update_plugin_code()` - 添加更新后刷新
  - `enable_plugin()` - 添加启用后刷新
  - `disable_plugin()` - 添加禁用后刷新

### 相关文件（未修改）
- `src-tauri/src/lib.rs` - AgentPluginProvider注册位置
- `src-tauri/src/tools/agent_plugin_provider.rs` - Agent插件工具提供者
- `src-tauri/src/tools/unified_manager.rs` - 统一工具管理器
- `src-tauri/src/tools/framework_adapters.rs` - 框架适配器实现
- `src-tauri/src/engines/react/executor.rs` - ReAct执行器
- `src-tauri/sentinel-tools/src/manager.rs` - UnifiedToolManager核心实现

## 技术架构图

```
┌─────────────────────────────────────────────────────────────┐
│                      用户操作                                │
│  创建/修改/启用/禁用 Agent 插件                              │
└─────────────────────┬───────────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────────┐
│              Tauri Commands 层                              │
│  create_plugin_in_db / update_plugin_code                   │
│  enable_plugin / disable_plugin                             │
│                                                              │
│  ✓ 修改数据库                                               │
│  ✓ 检查 main_category == "agent"                           │
│  ✓ 调用 tool_system.refresh_all() ← 新增逻辑               │
└─────────────────────┬───────────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────────┐
│                 ToolSystem 层                               │
│  refresh_all() → manager.refresh_all_providers()            │
└─────────────────────┬───────────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────────┐
│            UnifiedToolManager 层                            │
│  遍历所有 providers:                                        │
│    - BuiltinToolProvider                                    │
│    - McpToolProvider                                        │
│    - PassiveToolProvider                                    │
│    - AgentPluginProvider ← 关键                            │
│                                                              │
│  对每个 provider:                                           │
│    1. provider.refresh()                                    │
│    2. provider.get_tools() ← 重新读取数据库                │
│    3. 更新 tool_registry                                    │
└─────────────────────┬───────────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────────┐
│          AgentPluginProvider 层                             │
│  get_tools() {                                              │
│    SELECT * FROM plugin_registry                            │
│    WHERE main_category = 'agent' AND enabled = 1            │
│    返回更新后的工具列表                                     │
│  }                                                           │
└─────────────────────┬───────────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────────┐
│              tool_registry 更新                             │
│  HashMap<String, Arc<dyn UnifiedTool>>                      │
│  包含最新的 Agent 插件工具                                  │
└─────────────────────┬───────────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────────┐
│              ReAct Executor 使用                            │
│  framework_adapter.list_available_tools()                   │
│    → UnifiedToolManager.list_tools()                        │
│    → 读取 tool_registry                                     │
│    → 构建系统提示的"可用工具"部分                          │
└─────────────────────────────────────────────────────────────┘
```

## 注意事项

1. **刷新性能**：刷新操作会重新读取数据库并更新所有provider的工具，在插件数量很多时可能有轻微延迟
2. **错误处理**：刷新失败只记录警告日志，不影响主要操作（创建/更新/启用/禁用）的成功
3. **仅针对Agent插件**：Passive插件修改不会触发刷新，因为它们不需要注册到工具系统
4. **并发安全**：使用RwLock保护tool_registry，支持并发读取和独占写入

## 总结

此修复确保了Agent插件的动态更新能够及时反映到AI任务执行系统中，解决了工具列表缓存导致的不一致问题。通过在关键操作后自动刷新工具注册表，用户无需重启应用即可使用新创建或修改的Agent插件工具。
