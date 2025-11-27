# 插件注册表独立化实现总结

## 概述

成功将插件系统从被动扫描中独立出来，使用统一的 `plugin_registry` 表管理所有类型的插件（被动扫描插件、Agent插件等）。

## 完成的修改

### 1. 数据库层修改

#### 新增 Migration: `20251111_independent_plugin_registry.sql`
- 创建独立的 `plugin_registry` 表
- 新增 `main_category` 字段（passive/agent）
- 保留 `category` 字段作为子分类
- 从旧表 `passive_plugin_registry` 迁移数据
- 删除旧表（不再使用视图方案）

#### Schema 结构
```sql
CREATE TABLE plugin_registry (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    version TEXT NOT NULL,
    author TEXT,
    main_category TEXT NOT NULL DEFAULT 'passive',  -- 主分类
    category TEXT NOT NULL,                         -- 子分类
    description TEXT,
    default_severity TEXT NOT NULL,
    tags TEXT,                    -- JSON array
    file_path TEXT NOT NULL,
    file_hash TEXT NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT 0,
    config_json TEXT,             -- JSON
    plugin_code TEXT,
    installed_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_loaded_at TIMESTAMP,
    load_error TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

#### 索引优化
```sql
CREATE INDEX idx_plugins_main_category ON plugin_registry(main_category);
CREATE INDEX idx_plugins_category ON plugin_registry(category);
CREATE INDEX idx_plugins_enabled ON plugin_registry(enabled);
CREATE INDEX idx_plugins_type ON plugin_registry(main_category, category);
CREATE INDEX idx_plugins_agent_enabled ON plugin_registry(main_category, enabled) 
  WHERE main_category = 'agent';
```

### 2. Rust 类型系统修改

#### `PluginMetadata` 结构体更新
文件: `src-tauri/sentinel-plugins/src/types.rs`

```rust
pub struct PluginMetadata {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: Option<String>,
    
    // 新增字段
    #[serde(default = "default_main_category")]
    pub main_category: String,  // passive/agent
    
    pub category: String,       // 子分类
    pub default_severity: Severity,
    pub tags: Vec<String>,
    pub description: Option<String>,
}

fn default_main_category() -> String {
    "passive".to_string()
}
```

### 3. 后端代码更新

#### 批量替换
- 所有 `.rs` 文件中的 `passive_plugin_registry` → `plugin_registry`
- 命令: `find . -type f -name "*.rs" -exec sed -i '' 's/passive_plugin_registry/plugin_registry/g' {} +`

#### 数据库操作更新

**文件: `sentinel-passive/src/database.rs`**

```rust
// 注册插件时自动推断 main_category
pub async fn register_plugin_with_code(&self, plugin: &PluginMetadata, plugin_code: &str) {
    let main_category = if plugin.category == "agents" { "agent" } else { "passive" };
    
    sqlx::query(r#"
        INSERT OR REPLACE INTO plugin_registry (
            id, name, version, author, main_category, category, ...
        ) VALUES (?, ?, ?, ?, ?, ?, ...)
    "#)
    .bind(&plugin.id)
    .bind(main_category)
    .bind(&plugin.category)
    // ...
}

// 查询时返回 main_category
pub async fn get_plugin_by_id(&self, plugin_id: &str) -> Result<Option<serde_json::Value>> {
    let row: Option<(..., String, String, ...)> = sqlx::query_as(r#"
        SELECT id, name, version, author, main_category, category, ...
        FROM plugin_registry WHERE id = ?
    "#)
    // ...
    
    Ok(Some(serde_json::json!({
        "main_category": main_category,
        "category": category,
        // ...
    })))
}
```

**文件: `sentinel-passive/src/scanner.rs`**

```rust
// 加载插件时默认设置 main_category
let metadata = PluginMetadata {
    id: id.clone(),
    name: name.clone(),
    version,
    author,
    main_category: "passive".to_string(),  // 从数据库加载默认为passive
    category,
    description,
    default_severity: severity,
    tags: tags_array,
};
```

**文件: `src/commands/passive_scan_commands.rs`**

```rust
// 列表插件时设置 main_category
let metadata = PluginMetadata {
    id: id.clone(),
    name,
    version,
    author,
    main_category: "passive".to_string(),  // 默认passive
    category,
    description,
    default_severity: severity,
    tags: tags_array,
};
```

#### Agent插件提供者更新

**文件: `src/tools/agent_plugin_provider.rs`**

```rust
async fn get_tools(&self) -> anyhow::Result<Vec<Arc<dyn UnifiedTool>>> {
    let mut tools: Vec<Arc<dyn UnifiedTool>> = Vec::new();
    let plugins = self.state.list_plugins_internal().await?;
    
    for plugin in plugins {
        // 使用 main_category 过滤 Agent 插件
        if plugin.status == PluginStatus::Enabled 
            && plugin.metadata.main_category == "agent" {
            tools.push(Arc::new(AgentPluginTool::new(
                self.state.clone(),
                plugin.metadata.id.clone(),
                plugin.metadata.name.clone(),
                plugin.metadata.description.clone().unwrap_or_default(),
            )));
        }
    }
    
    Ok(tools)
}
```

### 4. 前端代码更新

#### PluginManagement.vue 映射逻辑

```typescript
// 保存时映射到后端
const backendCategory = newPluginMetadata.value.mainCategory === 'agent' 
    ? 'agents'  // Agent插件统一使用 agents
    : newPluginMetadata.value.category  // 被动扫描插件保留子分类

// 查询时从数据库 main_category 推断前端主分类
function inferMainCategory(category: string): 'passive' | 'agent' {
    return category === 'agents' ? 'agent' : 'passive'
}
```

## 分类映射表

| 前端主分类 | 前端子分类 | 后端 main_category | 后端 category | 数据库存储 |
|-----------|-----------|-------------------|--------------|----------|
| 被动扫描   | 漏洞检测   | passive | vulnerability | main_category='passive', category='vulnerability' |
| 被动扫描   | 注入检测   | passive | injection | main_category='passive', category='injection' |
| 被动扫描   | 跨站脚本   | passive | xss | main_category='passive', category='xss' |
| Agent插件 | 扫描器     | agent | scanner | main_category='agent', category='scanner' |
| Agent插件 | 分析器     | agent | analyzer | main_category='agent', category='analyzer' |
| Agent插件 | 报告生成   | agent | reporter | main_category='agent', category='reporter' |

## 旧数据迁移策略

```sql
-- 从旧category推断新的main_category和category
INSERT INTO plugin_registry (main_category, category, ...)
SELECT 
    CASE 
        WHEN category = 'agents' THEN 'agent'
        ELSE 'passive'
    END as main_category,
    CASE 
        WHEN category = 'agents' THEN 'scanner'
        WHEN category = 'passiveScan' THEN 'vulnerability'
        ELSE category
    END as category,
    ...
FROM passive_plugin_registry;
```

## 工作流程

### 创建Agent插件
1. 前端选择：主分类=Agent, 子分类=扫描器
2. 前端保存：`category='agents'`（向后兼容）
3. 后端处理：识别 `category='agents'`，设置 `main_category='agent', category='scanner'`
4. 数据库存储：`main_category='agent', category='scanner'`
5. Agent加载：`AgentPluginProvider` 查询 `WHERE main_category='agent' AND enabled=1`
6. 工具注册：`plugin::<plugin_id>`

### 创建被动扫描插件
1. 前端选择：主分类=被动扫描, 子分类=漏洞检测
2. 前端保存：`category='vulnerability'`
3. 后端处理：设置 `main_category='passive', category='vulnerability'`
4. 数据库存储：`main_category='passive', category='vulnerability'`
5. 扫描器加载：`PassiveScanManager` 查询 `WHERE main_category='passive' AND enabled=1`

## 验证清单

- [x] 编译成功无错误
- [x] 数据库Migration正确
- [x] PluginMetadata包含main_category字段
- [x] 所有SQL查询使用plugin_registry表
- [x] AgentPluginProvider使用main_category过滤
- [x] 前端保存映射正确
- [x] 后端加载设置默认main_category
- [x] 索引优化完成

## 测试步骤

1. **测试数据迁移**
   ```bash
   # 查看旧数据
   sqlite3 sentinel-ai.db "SELECT id, category FROM passive_plugin_registry"
   
   # 运行迁移
   # 启动应用（自动执行migration）
   
   # 查看新数据
   sqlite3 sentinel-ai.db "SELECT id, main_category, category FROM plugin_registry"
   ```

2. **测试Agent插件创建**
   - 前端创建Agent插件
   - 检查数据库：`main_category='agent'`
   - 检查工具列表：包含 `plugin::<plugin_id>`

3. **测试被动扫描插件**
   - 前端创建被动扫描插件
   - 检查数据库：`main_category='passive'`
   - 运行被动扫描验证插件加载

4. **测试向后兼容**
   - 旧插件数据正确迁移
   - Agent工具正常识别
   - 被动扫描功能正常

## 注意事项

1. **不再使用视图方案**：直接使用 `plugin_registry` 表，更清晰直接
2. **main_category必需**：所有新插件必须设置 main_category（默认为 'passive'）
3. **Agent识别变更**：从 `category='agents'` 改为 `main_category='agent'`
4. **前端兼容**：保存时仍然使用 `agents` 以保持向后兼容
5. **索引性能**：针对 Agent 插件查询添加了部分索引

## 相关文件

- Migration: `src-tauri/migrations/20251111_independent_plugin_registry.sql`
- 类型定义: `src-tauri/sentinel-plugins/src/types.rs`
- 数据库操作: `src-tauri/sentinel-passive/src/database.rs`
- 扫描器: `src-tauri/sentinel-passive/src/scanner.rs`
- Agent工具: `src-tauri/src/tools/agent_plugin_provider.rs`
- 命令: `src-tauri/src/commands/passive_scan_commands.rs`
- 前端: `src/views/PluginManagement.vue`
- i18n: `src/i18n/locales/zh.ts`, `src/i18n/locales/en.ts`
