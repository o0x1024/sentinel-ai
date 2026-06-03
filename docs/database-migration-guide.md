# 数据库迁移指南

## 概述

Sentinel AI 现在支持多种数据库类型，包括 SQLite、PostgreSQL 和 MySQL。您可以轻松地在不同数据库之间切换和迁移数据。

**当前功能状态**:
- ✅ **数据迁移**: 完全支持从SQLite迁移到PostgreSQL/MySQL
- ✅ **数据导出/导入**: 支持JSON和SQL格式
- ✅ **连接测试**: 支持测试各种数据库连接
- ⚠️ **应用运行**: 当前应用仍基于SQLite运行，数据库切换功能正在开发中
- 📝 **配置保存**: 迁移后的数据库配置会保存，但重启后仍使用SQLite

**使用建议**:
目前数据库迁移功能主要用于：
1. 数据备份和恢复
2. 将SQLite数据导出到PostgreSQL/MySQL进行分析
3. 在不同环境间同步数据

如需完全切换到PostgreSQL/MySQL并在应用中使用，需要等待下一版本的架构升级。

## 支持的数据库类型

- **SQLite** (默认) - 适合单机部署，无需额外配置
- **PostgreSQL** - 适合高并发场景，支持高级特性
- **MySQL** - 广泛使用，兼容性好

## 准备工作

### PostgreSQL 准备步骤

如果要迁移到PostgreSQL，需要先创建数据库：

```sql
-- 连接到PostgreSQL服务器
psql -U postgres

-- 创建数据库
CREATE DATABASE sentinel_ai;

-- 创建用户（可选，如果不使用默认用户）
CREATE USER sentinel_user WITH PASSWORD 'your_password';

-- 授予权限
GRANT ALL PRIVILEGES ON DATABASE sentinel_ai TO sentinel_user;

-- 退出
\q
```

### MySQL 准备步骤

如果要迁移到MySQL，需要先创建数据库：

```sql
-- 连接到MySQL服务器
mysql -u root -p

-- 创建数据库
CREATE DATABASE sentinel_ai CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;

-- 创建用户（可选，如果不使用默认用户）
CREATE USER 'sentinel_user'@'localhost' IDENTIFIED BY 'your_password';

-- 授予权限
GRANT ALL PRIVILEGES ON sentinel_ai.* TO 'sentinel_user'@'localhost';

-- 刷新权限
FLUSH PRIVILEGES;

-- 退出
EXIT;
```

## 使用步骤

### 1. 测试数据库连接

在迁移前，请先测试目标数据库的连接：

1. 打开 `设置` → `数据库` 页面
2. 选择目标数据库类型（PostgreSQL 或 MySQL）
3. 填写连接信息：
   - 主机地址 (默认: localhost)
   - 端口 (PostgreSQL: 5432, MySQL: 3306)
   - 数据库名称
   - 用户名和密码
4. 点击 `测试连接` 按钮
5. 确认连接成功

### 2. 数据库迁移

**重要：在迁移前请先备份当前数据库！**

1. 点击 `创建备份` 按钮备份当前数据
2. 确认目标数据库已创建（**不需要手动创建表结构，系统会自动创建**）
3. 点击 `数据库迁移` 按钮
4. 确认迁移操作
5. 等待迁移完成（系统会：
   - 自动清理目标数据库中现有的表和相关对象（如果有）
   - 使用多重清理策略确保完全清理
   - 自动创建完整的表结构
   - 转换并迁移所有数据
   - 可能需要几分钟）
6. 重启应用以使用新数据库

**注意**：迁移过程会自动完成以下操作：
- 在目标数据库中自动清理和创建所有必要的表结构
- 在导入数据前强制清空每个表（使用TRUNCATE/DELETE确保无数据残留）
- 自动转换数据类型以适配目标数据库（SQLite → PostgreSQL/MySQL）
- 迁移所有数据并保持数据完整性
- 如果任何步骤失败，会立即报错并停止（确保数据一致性）

### 3. 数据导出/导入

如果只需要导出数据用于备份或分析：

#### 导出数据
1. 点击 `导出数据` 按钮
2. 选择导出格式（JSON 或 SQL）
3. 选择保存位置
4. 等待导出完成

#### 导入数据
1. 点击 `导入数据` 按钮
2. 选择 JSON 格式的数据文件
3. 确认导入操作
4. 等待导入完成

## 配置说明

### PostgreSQL 配置示例

```json
{
  "db_type": "postgresql",
  "host": "localhost",
  "port": 5432,
  "database": "sentinel_ai",
  "username": "postgres",
  "password": "your_password",
  "enable_ssl": false,
  "max_connections": 10,
  "query_timeout": 30
}
```

### MySQL 配置示例

```json
{
  "db_type": "mysql",
  "host": "localhost",
  "port": 3306,
  "database": "sentinel_ai",
  "username": "root",
  "password": "your_password",
  "enable_ssl": false,
  "max_connections": 10,
  "query_timeout": 30
}
```

### SQLite 配置示例

```json
{
  "db_type": "sqlite",
  "path": "/path/to/database.db",
  "enable_wal": true,
  "max_connections": 10,
  "query_timeout": 30
}
```

## 注意事项

1. **备份**: 在任何迁移操作前，务必备份当前数据库
2. **数据库创建**: 目标数据库实例必须已经创建（例如在PostgreSQL中创建database），但**不需要手动创建表结构**，系统会自动创建
3. **权限**: 确保数据库用户有足够的权限（CREATE、INSERT、UPDATE、DELETE、CREATE TABLE）
4. **网络**: PostgreSQL 和 MySQL 需要确保网络连接正常
5. **SSL**: 生产环境建议启用 SSL 连接
6. **性能**: 大数据量迁移可能需要较长时间，请耐心等待
7. **数据类型转换**: 系统会自动将SQLite的数据类型转换为PostgreSQL/MySQL对应的类型，包括日期时间格式的转换
8. **时间戳处理**: SQLite中的日期时间字符串会被自动转换为PostgreSQL的TIMESTAMP类型

## 故障排除

### 连接失败

- 检查数据库服务是否运行
- 验证主机地址和端口是否正确
- 确认用户名和密码是否正确
- 检查防火墙设置

### 迁移失败

- 确保目标数据库有足够的存储空间
- 检查数据库用户权限（必须有CREATE TABLE、DROP TABLE权限）
- 确认目标数据库已经创建（例如在PostgreSQL中：`CREATE DATABASE sentinel_ai;`）
- 查看应用日志了解详细错误信息
- 如果部分数据迁移失败，可以尝试使用导出/导入功能
- 常见错误：
  - "relation does not exist"：确保目标数据库已创建，系统会自动创建表结构
  - "duplicate key value violates unique constraint"：系统会自动清理现有表并重建，请重试迁移
  - 如果迁移失败，可以先手动清空目标数据库，然后重试

### 性能问题

- 对于大数据量，建议先导出为文件，然后在非高峰时段导入
- 可以适当增加 `max_connections` 和 `query_timeout` 配置
- PostgreSQL 建议启用连接池和适当的内存配置

## API 参考

### 测试连接

```typescript
await invoke('test_db_connection', { 
  config: {
    db_type: 'postgresql',
    host: 'localhost',
    port: 5432,
    // ...
  }
})
```

### 数据库迁移

```typescript
await invoke('migrate_database', {
  targetConfig: {
    db_type: 'postgresql',
    // ...
  }
})
```

### 导出数据

```typescript
// 导出为 JSON
await invoke('export_db_to_json', {
  outputPath: '/path/to/export.json'
})

// 导出为 SQL
await invoke('export_db_to_sql', {
  outputPath: '/path/to/export.sql'
})
```

### 导入数据

```typescript
await invoke('import_db_from_json', {
  inputPath: '/path/to/import.json'
})
```

## 相关文档

- [PostgreSQL 官方文档](https://www.postgresql.org/docs/)
- [MySQL 官方文档](https://dev.mysql.com/doc/)
- [SQLite 官方文档](https://www.sqlite.org/docs.html)
