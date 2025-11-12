# 插件注册表迁移说明

## 概述

将插件系统从被动扫描中独立出来，创建统一的插件注册表 `plugin_registry`，支持多种插件类型。

## 数据库结构变更

### 旧结构 (passive_plugin_registry)
```sql
CREATE TABLE passive_plugin_registry (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    category TEXT NOT NULL,  -- 单层分类: agentTools/vulnerability/injection/xss
    ...
);
```

### 新结构 (plugin_registry)
```sql
CREATE TABLE plugin_registry (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    main_category TEXT NOT NULL,  -- 主分类: passive/agent
    category TEXT NOT NULL,        -- 子分类: vulnerability/injection/xss/scanner/analyzer/reporter
    ...
);
```

## 分类映射关系

### 前端 → 后端存储

| 前端主分类 | 前端子分类 | 后端 main_category | 后端 category | 说明 |
|-----------|-----------|-------------------|--------------|------|
| 被动扫描 | 漏洞检测 | passive | vulnerability | 被动扫描插件 |
| 被动扫描 | 注入检测 | passive | injection | 被动扫描插件 |
| 被动扫描 | 跨站脚本 | passive | xss | 被动扫描插件 |
| Agent插件 | 扫描器 | agent | scanner | Agent工具插件 |
| Agent插件 | 分析器 | agent | analyzer | Agent工具插件 |
| Agent插件 | 报告生成 | agent | reporter | Agent工具插件 |

### 旧数据迁移

| 旧 category | 新 main_category | 新 category | 说明 |
|------------|-----------------|------------|------|
| agentTools | agent | scanner | Agent插件默认为扫描器 |
| passiveScan | passive | vulnerability | 被动扫描默认为漏洞检测 |
| vulnerability | passive | vulnerability | 保持不变 |
| injection | passive | injection | 保持不变 |
| xss | passive | xss | 保持不变 |
| custom | passive | custom | 保持不变 |

## 向后兼容性

### 视图映射
创建了 `passive_plugin_registry` 视图指向 `plugin_registry` 表：

```sql
CREATE VIEW passive_plugin_registry AS
SELECT 
    id, name, version, author,
    CASE 
        WHEN main_category = 'agent' THEN 'agentTools'
        ELSE category
    END as category,
    ...
FROM plugin_registry;
```

### INSTEAD OF 触发器
为视图创建了 INSERT/UPDATE/DELETE 触发器，自动处理分类转换：

- **INSERT**: 自动将 `agentTools` 转换为 `main_category='agent', category='scanner'`
- **UPDATE**: 自动更新分类映射
- **DELETE**: 直接删除底层数据

### 现有代码兼容性
✅ 所有现有的 SQL 查询继续工作，无需修改：
```rust
// 这些查询会自动工作
sqlx::query("SELECT * FROM passive_plugin_registry WHERE id = ?")
sqlx::query("INSERT INTO passive_plugin_registry (...) VALUES (...)")
sqlx::query("UPDATE passive_plugin_registry SET enabled = ? WHERE id = ?")
```

## Agent插件工具注册

`AgentPluginProvider` 会自动识别并注册所有 `main_category = 'agent'` 的插件：

```rust
// src-tauri/src/tools/agent_plugin_provider.rs
for plugin in plugins {
    if plugin.status == Enabled 
        && plugin.metadata.category == "agentTools" {  // 视图会返回 "agentTools"
        tools.push(AgentPluginTool::new(...));
    }
}
```

工具命名格式：`plugin::<plugin_id>`

## 迁移步骤

### 自动迁移
运行应用时自动执行 `20251111_independent_plugin_registry.sql`：

1. 创建 `plugin_registry` 表
2. 从 `passive_plugin_registry` 迁移数据
3. 删除旧表，创建同名视图
4. 创建 INSTEAD OF 触发器

### 手动验证
```sql
-- 查看新表结构
SELECT * FROM plugin_registry LIMIT 5;

-- 查看视图数据（应该和旧表一致）
SELECT * FROM passive_plugin_registry LIMIT 5;

-- 检查Agent插件
SELECT id, name, main_category, category 
FROM plugin_registry 
WHERE main_category = 'agent';

-- 检查被动扫描插件
SELECT id, name, main_category, category 
FROM plugin_registry 
WHERE main_category = 'passive';
```

## 前端变更

### PluginManagement.vue
- ✅ 新增 `mainCategory` 字段
- ✅ 分层分类选择器（主分类 + 子分类）
- ✅ 保存时自动映射为后端格式
- ✅ 加载时自动转换为前端格式

### 映射函数
```typescript
// 推断主分类
function inferMainCategory(category: string): 'passive' | 'agent' {
  return category === 'agentTools' ? 'agent' : 'passive'
}

// 转换子分类
function convertToSubCategory(category: string): string {
  if (category === 'agentTools') return 'scanner'
  if (category === 'passiveScan') return 'vulnerability'
  return category
}

// 保存时映射回后端
const backendCategory = mainCategory === 'agent' ? 'agentTools' : category
```

## 测试清单

- [ ] 创建新的被动扫描插件
- [ ] 创建新的Agent插件
- [ ] 编辑现有插件
- [ ] 启用/禁用插件
- [ ] 删除插件
- [ ] 验证Agent工具注册
- [ ] 验证被动扫描功能
- [ ] 验证数据迁移正确性

## 注意事项

1. **主分类是必需的**: 所有插件必须有 `main_category`（默认为 'passive'）
2. **视图限制**: 通过视图修改数据时，触发器会自动处理分类转换
3. **索引优化**: 新增了 `(main_category, category)` 联合索引
4. **Agent工具识别**: 基于视图返回的 `category = 'agentTools'` 进行识别
5. **数据一致性**: 迁移过程中保持 plugin_id 不变，保证外键引用有效

## 回滚方案

如果需要回滚到旧结构：

```sql
-- 1. 删除视图和触发器
DROP VIEW IF EXISTS passive_plugin_registry;
DROP TRIGGER IF EXISTS passive_plugin_registry_insert;
DROP TRIGGER IF EXISTS passive_plugin_registry_update;
DROP TRIGGER IF EXISTS passive_plugin_registry_delete;

-- 2. 从 plugin_registry 重建旧表
CREATE TABLE passive_plugin_registry AS
SELECT 
    id, name, version, author,
    CASE 
        WHEN main_category = 'agent' THEN 'agentTools'
        ELSE category
    END as category,
    description, default_severity, tags, file_path, file_hash,
    enabled, config_json, plugin_code, installed_at, last_loaded_at,
    load_error, created_at, updated_at
FROM plugin_registry;

-- 3. 删除新表
DROP TABLE plugin_registry;
```
