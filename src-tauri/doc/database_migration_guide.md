# 数据库迁移执行指南

## 问题描述

如果遇到以下错误：
```
Failed to create plugin in database: 数据库错误: Failed to register plugin with code: 
error returned from database: (code: 1) no such table: plugin_registry
```

这说明数据库迁移尚未执行，`plugin_registry` 表还不存在。

## 解决方案

### 方案1: 自动迁移（推荐）

1. **重启应用**
   - 关闭当前应用
   - 重新启动应用
   - 应用启动时会自动执行所有未运行的迁移

2. **查看日志确认**
   检查应用日志，应该看到：
   ```
   Running database migrations...
   Executing migration: 20251111_independent_plugin_registry.sql
   Migration completed: 20251111_independent_plugin_registry.sql
   All migrations completed successfully
   ```

### 方案2: 手动执行迁移

如果自动迁移失败，可以手动执行SQL：

1. **找到数据库文件**
   ```bash
   # macOS
   find ~/Library/Application\ Support -name "sentinel-ai.db"
   
   # 或查看应用日志中的 "Database path" 信息
   ```

2. **执行迁移SQL**
   ```bash
   # 进入migrations目录
   cd src-tauri/migrations
   
   # 执行20251111迁移
   sqlite3 <数据库路径> < 20251111_independent_plugin_registry.sql
   ```

3. **验证表已创建**
   ```bash
   sqlite3 <数据库路径> ".schema plugin_registry"
   ```

   应该看到：
   ```sql
   CREATE TABLE plugin_registry (
       id TEXT PRIMARY KEY,
       name TEXT NOT NULL,
       version TEXT NOT NULL,
       author TEXT,
       main_category TEXT NOT NULL DEFAULT 'passive',
       category TEXT NOT NULL,
       ...
   );
   ```

### 方案3: 删除数据库重新初始化

⚠️ **警告：这会删除所有现有数据！**

```bash
# 备份当前数据库
cp <数据库路径> <数据库路径>.backup

# 删除数据库
rm <数据库路径>

# 重启应用，会自动创建新数据库并执行所有迁移
```

## 迁移系统工作原理

1. **迁移文件位置**: `src-tauri/migrations/`
2. **执行顺序**: 按文件名排序执行（YYYYMMDD格式保证顺序）
3. **迁移跟踪**: 使用 `_migrations` 表记录已执行的迁移
4. **自动执行**: 每次应用启动时会自动执行未运行的迁移

## 相关迁移文件

- `20251105_passive_scan_schema.sql` - 创建被动扫描相关表（包括旧的passive_plugin_registry）
- `20251106_add_plugin_code.sql` - 为passive_plugin_registry添加plugin_code字段
- `20251111_independent_plugin_registry.sql` - **创建独立的plugin_registry表并迁移数据**

## 检查迁移状态

```bash
# 查看已执行的迁移
sqlite3 <数据库路径> "SELECT * FROM _migrations ORDER BY executed_at;"

# 应该看到包含：
# 20251105_passive_scan_schema.sql
# 20251106_add_plugin_code.sql
# 20251111_independent_plugin_registry.sql
```

## 排查问题

如果迁移仍然失败：

1. **查看应用日志**
   - 搜索 "Migration failed" 或 "Failed to execute statement"
   - 查看具体的SQL错误信息

2. **检查文件权限**
   ```bash
   ls -la src-tauri/migrations/
   # 确保migration文件可读
   ```

3. **手动测试SQL**
   ```bash
   # 复制migration文件内容
   # 分段执行测试哪个语句失败
   sqlite3 <数据库路径>
   sqlite> -- 粘贴SQL语句
   ```

4. **查看数据库状态**
   ```bash
   sqlite3 <数据库路径>
   sqlite> .tables
   sqlite> .schema
   ```

## 联系支持

如果问题仍然存在，请提供：
- 应用日志（包含迁移相关日志）
- 数据库schema（`.schema` 输出）
- 已执行的迁移列表（`SELECT * FROM _migrations`）
