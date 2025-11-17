# 插件审核命令修复总结

## 🐛 问题描述

用户在使用插件审核功能时遇到错误：
```
Command get_plugins_for_review not found
```

## 🔍 问题分析

1. **前端调用了不存在的命令**：`PluginManagement.vue` 中调用了 `get_plugins_for_review` 命令
2. **后端缺少命令实现**：`plugin_review_commands.rs` 中没有实现该命令
3. **数据库方法缺失**：`DatabaseService` 中没有从 `plugin_registry` 表查询数据的方法
4. **命令未注册**：即使有实现，也没有在 Tauri 的 `invoke_handler` 中注册

## ✅ 修复内容

### 1. 添加后端命令 (plugin_review_commands.rs)

```rust
/// Get plugins for review (from plugin_registry table)
#[tauri::command]
pub async fn get_plugins_for_review(
    db: State<'_, Arc<DatabaseService>>,
) -> Result<PluginReviewResponse, String> {
    log::info!("Getting plugins for review from plugin_registry");
    
    // Query from plugin_registry table
    match db.get_plugins_from_registry().await {
        Ok(plugins) => {
            log::info!("Found {} plugins in registry", plugins.len());
            Ok(PluginReviewResponse {
                success: true,
                message: format!("Found {} plugins", plugins.len()),
                data: Some(serde_json::to_value(&plugins).unwrap_or(serde_json::json!([]))),
            })
        }
        Err(e) => {
            log::error!("Failed to get plugins from registry: {}", e);
            Ok(PluginReviewResponse {
                success: false,
                message: format!("Failed to get plugins: {}", e),
                data: Some(serde_json::json!([])),
            })
        }
    }
}
```

### 2. 添加数据库查询方法 (database.rs)

```rust
/// Get all plugins from plugin_registry table for review
pub async fn get_plugins_from_registry(&self) -> Result<Vec<serde_json::Value>> {
    let pool = self.get_pool()?;
    
    let rows = sqlx::query(
        r#"
        SELECT 
            id as plugin_id,
            name as plugin_name,
            version,
            author,
            main_category,
            category as vuln_type,
            description,
            default_severity,
            tags,
            enabled,
            plugin_code as code,
            quality_score,
            validation_status as status,
            created_at as generated_at,
            updated_at
        FROM plugin_registry
        ORDER BY created_at DESC
        "#
    )
    .fetch_all(pool)
    .await?;
    
    // 处理每一行数据并转换为 JSON 格式
    // ... (详见代码)
    
    Ok(plugins)
}
```

**查询逻辑：**
- 从 `plugin_registry` 表查询所有插件
- 映射字段名称以匹配前端期望的格式
- 构造 `quality_breakdown` 和 `validation` 嵌套对象
- 按创建时间倒序排列

### 3. 注册命令到 Tauri (lib.rs)

```rust
.invoke_handler(generate_handler![
    // ... 其他命令
    
    // Plugin review commands (Plan B)
    commands::plugin_review_commands::get_plugins_for_review, // 新添加
    commands::plugin_review_commands::list_generated_plugins,
    commands::plugin_review_commands::get_plugin_detail,
    commands::plugin_review_commands::approve_plugin,
    commands::plugin_review_commands::reject_plugin,
    commands::plugin_review_commands::review_update_plugin_code,
    
    // ... 其他命令
])
```

### 4. 更新前端调用方式 (PluginManagement.vue)

**修改所有审核相关命令的调用方式：**

```typescript
// 修改前 (使用泛型类型)
const response = await invoke<CommandResponse<ReviewPlugin[]>>('get_plugins_for_review')
if (response.success && response.data) {
  reviewPlugins.value = response.data
}

// 修改后 (使用 any 类型以适配后端返回格式)
const response: any = await invoke('get_plugins_for_review')
if (response.success && response.data) {
  reviewPlugins.value = Array.isArray(response.data) ? response.data : []
}
```

**更新的方法：**
- `refreshReviewPlugins()` - 加载插件列表
- `approvePlugin()` - 批准插件
- `rejectPlugin()` - 拒绝插件
- `approveSelected()` - 批量批准
- `rejectSelected()` - 批量拒绝
- `deleteReviewPlugin()` - 删除插件
- `saveReviewEdit()` - 保存代码编辑

**错误处理改进：**
- 将 `response.error` 改为 `response.message` (匹配后端返回格式)
- 添加数组类型检查：`Array.isArray(response.data)`
- 失败时设置空数组：`reviewPlugins.value = []`

## 📊 数据格式

### plugin_registry 表结构

| 字段名 | 类型 | 说明 |
|--------|------|------|
| id | TEXT | 插件ID |
| name | TEXT | 插件名称 |
| version | TEXT | 版本 |
| author | TEXT | 作者 |
| main_category | TEXT | 主分类 |
| category | TEXT | 子分类(漏洞类型) |
| description | TEXT | 描述 |
| default_severity | TEXT | 默认严重程度 |
| tags | TEXT | 标签(JSON) |
| enabled | BOOLEAN | 是否启用 |
| plugin_code | TEXT | 插件代码 |
| quality_score | REAL | 质量评分 |
| validation_status | TEXT | 验证状态 |
| created_at | DATETIME | 创建时间 |
| updated_at | DATETIME | 更新时间 |

### 返回数据格式

```json
{
  "success": true,
  "message": "Found 5 plugins",
  "data": [
    {
      "plugin_id": "sqli_detector_001",
      "plugin_name": "SQL Injection Detector",
      "code": "export async function analyze(...) { ... }",
      "description": "Detects SQL injection vulnerabilities",
      "vuln_type": "sqli",
      "quality_score": 85.0,
      "quality_breakdown": {
        "syntax_score": 90.0,
        "logic_score": 85.0,
        "security_score": 80.0,
        "code_quality_score": 85.0
      },
      "validation": {
        "is_valid": true,
        "syntax_valid": true,
        "has_required_functions": true,
        "security_check_passed": true,
        "errors": [],
        "warnings": []
      },
      "status": "PendingReview",
      "generated_at": "2025-11-13T10:30:00Z",
      "model": "AI Generated"
    }
  ]
}
```

## 🔧 修改的文件

1. **src-tauri/src/commands/plugin_review_commands.rs**
   - 添加 `get_plugins_for_review` 命令

2. **src-tauri/src/services/database.rs**
   - 添加 `get_plugins_from_registry` 方法

3. **src-tauri/src/lib.rs**
   - 注册 `get_plugins_for_review` 到 `invoke_handler`

4. **src/views/PluginManagement.vue**
   - 更新所有审核命令的调用方式
   - 修正错误字段名 (`response.error` → `response.message`)
   - 添加数组类型检查

## ✨ 功能验证

### 测试步骤

1. **启动应用**
   ```bash
   npm run tauri dev
   ```

2. **访问插件管理**
   - 打开应用
   - 进入"插件管理"页面
   - 点击"插件审核"Tab

3. **验证功能**
   - ✅ 页面正常加载，不再显示 "Command not found" 错误
   - ✅ 如果数据库中有插件，应该能看到插件列表
   - ✅ 统计卡片显示正确的数字
   - ✅ 可以查看插件详情
   - ✅ 可以批准/拒绝插件
   - ✅ 可以批量操作

### 预期行为

- **有数据时**：显示插件列表和详细信息
- **无数据时**：显示"暂无待审核的插件"提示
- **加载失败时**：显示错误提示并记录到控制台

## 🎯 关键改进

1. **完整的命令实现**：从数据库查询到前端展示的完整链路
2. **错误处理**：统一的错误处理机制和用户提示
3. **类型安全**：使用 `any` 类型处理后端返回的灵活格式
4. **数据验证**：添加数组类型检查，防止运行时错误
5. **日志记录**：添加详细的日志记录便于调试

## 📝 注意事项

1. **数据库初始化**：确保 `plugin_registry` 表已创建（在 Plan B 实现中已添加）
2. **字段映射**：注意数据库字段名与前端期望字段名的映射
3. **类型转换**：SQL 查询结果需要正确转换为 JSON 格式
4. **空值处理**：使用 `try_get().ok()` 处理可能为空的字段

## 🚀 后续优化建议

1. **缓存机制**：对插件列表进行缓存，减少数据库查询
2. **分页加载**：当插件数量很多时，实现分页功能
3. **实时更新**：使用事件监听机制，插件状态变更时自动刷新
4. **错误重试**：网络或数据库错误时自动重试
5. **性能优化**：优化 SQL 查询，添加索引

## 📅 修复时间

2025-11-13

## 👤 修复者

AI Assistant

