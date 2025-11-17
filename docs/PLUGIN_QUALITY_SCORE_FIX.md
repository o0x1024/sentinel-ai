# AI生成插件Quality Score保存问题修复

## 问题描述

AI生成的插件`quality_score`和`validation_status`字段没有正确保存到数据库中。

## 问题分析

### 根本原因

在`generator_tools.rs`中保存插件时采用了两步操作：

1. **第一步**：调用`register_plugin_with_code()`插入插件基本信息
   - 该方法的INSERT语句**不包含**`quality_score`和`validation_status`字段
   - 导致这两个字段在数据库中为NULL

2. **第二步**：执行UPDATE语句更新`quality_score`和`validation_status`
   - 虽然代码存在，但分两步操作存在以下问题：
     - 可能因为事务问题导致UPDATE未执行
     - 增加了数据库操作次数
     - 代码逻辑不够清晰

### 相关代码位置

- `src-tauri/sentinel-passive/src/database.rs:572-616` - 插件注册方法
- `src-tauri/src/tools/generator_tools.rs:457-483` - 插件保存逻辑

## 解决方案

### 1. 扩展数据库方法

在`sentinel-passive/src/database.rs`中新增方法：

```rust
/// Register plugin with code and optional quality score
pub async fn register_plugin_with_code_and_quality(
    &self, 
    plugin: &PluginMetadata, 
    plugin_code: &str,
    quality_score: Option<f64>,
    validation_status: Option<&str>
) -> Result<()>
```

**改进点：**
- 在INSERT时直接包含`quality_score`和`validation_status`字段
- 一次数据库操作完成所有字段的设置
- 保持向后兼容（原`register_plugin_with_code`方法内部调用新方法）

### 2. 修改插件保存逻辑

在`src/tools/generator_tools.rs`中：

**修改前：**
```rust
// 先插入基本信息
db_service.register_plugin_with_code(&metadata, &plugin.code).await?;

// 再更新quality_score和validation_status
sqlx::query("UPDATE plugin_registry SET quality_score = ?, validation_status = ? WHERE id = ?")
    .bind(plugin.quality_score as f64)
    .bind(status_str)
    .bind(&plugin.plugin_id)
    .execute(db_service.pool())
    .await?;
```

**修改后：**
```rust
// 一次操作完成所有字段的插入
db_service.register_plugin_with_code_and_quality(
    &metadata, 
    &plugin.code,
    Some(plugin.quality_score as f64),
    Some(status_str)
).await?;
```

## 修改文件清单

1. `src-tauri/sentinel-passive/src/database.rs`
   - 新增`register_plugin_with_code_and_quality()`方法
   - 修改原`register_plugin_with_code()`为调用新方法

2. `src-tauri/src/tools/generator_tools.rs`
   - 修改`save_plugin_to_db_static()`方法
   - 移除单独的UPDATE操作
   - 改为调用新的数据库方法

## 验证方法

1. 启动应用并生成新插件
2. 查询数据库验证`quality_score`和`validation_status`字段是否正确保存：

```sql
SELECT id, name, quality_score, validation_status 
FROM plugin_registry 
WHERE id LIKE 'ai_gen_%' 
ORDER BY created_at DESC 
LIMIT 10;
```

3. 在前端插件管理界面查看插件详情，确认质量分数显示正常

## 影响范围

- ✅ 不影响现有插件数据
- ✅ 向后兼容（原API仍可用）
- ✅ 仅影响新生成的插件
- ✅ 无需数据库迁移

## 测试建议

1. 生成新的被动扫描插件，验证`quality_score`正确保存
2. 检查插件复用逻辑是否正常（依赖`quality_score >= 70`的查询）
3. 验证插件审批流程中的质量分数显示

## 相关文档

- [插件复用优化文档](./PLUGIN_REUSE_OPTIMIZATION.md)
- [插件复用条件说明](./PLUGIN_REUSE_CRITERIA.md)

