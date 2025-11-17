# 插件生成增量保存优化

## 问题描述

之前的插件生成流程是等所有插件都生成完毕后才批量保存到数据库，这导致：
1. 如果生成过程中断，已生成的插件会丢失
2. 用户无法实时看到生成进度
3. 内存占用较高（所有插件都在内存中）

## 解决方案

### 1. 新增 `generate_with_callback` 方法

在 `AdvancedPluginGenerator` 中新增了 `generate_with_callback` 方法，支持在每个插件生成后立即调用回调函数：

```rust
pub async fn generate_with_callback<F, Fut>(
    &self, 
    request: PluginGenerationRequest,
    mut on_plugin_generated: F
) -> Result<Vec<GeneratedPlugin>> 
where
    F: FnMut(GeneratedPlugin) -> Fut,
    Fut: std::future::Future<Output = Result<()>>,
```

### 2. 修改工具执行流程

在 `GenerateAdvancedPluginTool::execute` 中：
- 使用 `generate_with_callback` 替代原来的 `generate`
- 每生成一个插件就立即保存到数据库
- 如果插件状态为 `Approved`，立即启用并加载

### 3. 静态方法支持

为了在闭包中调用，添加了静态版本的保存方法：
- `save_plugin_to_db_static`
- `enable_and_load_plugin_static`

## 优势

1. **实时保存**：每个插件生成后立即写入数据库，避免数据丢失
2. **更好的进度反馈**：用户可以实时看到每个插件的生成和保存状态
3. **降低内存占用**：不需要在内存中保存所有插件
4. **更好的错误处理**：单个插件保存失败不影响其他插件

## 相关文件

- `src-tauri/src/generators/advanced_generator.rs`
- `src-tauri/src/tools/generator_tools.rs`

## 日志示例

```
[INFO] Generated plugin: XSS Detection Plugin for xss
[INFO] Saved plugin xss_plugin_123 to database immediately after generation
[INFO] Auto-approved and loaded plugin xss_plugin_123
[INFO] Generated plugin: SQLi Detection Plugin for sqli
[INFO] Saved plugin sqli_plugin_456 to database immediately after generation
[INFO] Successfully generated and saved 2 plugins
```

