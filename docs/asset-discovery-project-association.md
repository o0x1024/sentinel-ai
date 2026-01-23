# 资产发现与项目关联说明

## ✅ 项目关联已实现

插件运行后发现的资产**已经自动关联到项目**，无需额外配置。

## 🔄 完整流程

### 1. 用户操作流程

```
用户选择项目
  ↓
点击"发现资产"按钮
  ↓
选择插件（如 plugin__subdomain_enumerator）
  ↓
填写输入参数（如 domain: example.com）
  ↓
勾选"自动导入资产"
  ↓
点击"开始发现"
  ↓
插件执行并发现资产
  ↓
自动导入到当前项目
```

### 2. 数据流

```typescript
// 前端：DiscoverAssetsModal.vue
const result = await invoke('monitor_discover_and_import_assets', {
  request: {
    program_id: props.selectedProgram.id,  // ✅ 项目ID
    scope_id: null,                         // 可选：范围ID
    plugin_id: form.plugin_id,              // 插件名称
    plugin_input: pluginInput,              // 插件输入参数
    auto_import: form.auto_import,          // 是否自动导入
  }
})
```

```rust
// 后端：monitor_commands.rs
pub async fn monitor_discover_and_import_assets(
    request: MonitorDiscoverAssetsRequest,
) -> Result<MonitorDiscoverAssetsResponse, String> {
    
    // 1. 执行插件
    let tool_result = tool_server.execute(&request.plugin_id, request.plugin_input).await;
    
    // 2. 解析插件输出
    if let Some(subdomains) = data.get("subdomains") {
        
        // 3. 为每个发现的资产创建数据库记录
        for subdomain in subdomains {
            let asset = BountyAssetRow {
                id: Uuid::new_v4().to_string(),
                program_id: request.program_id.clone(),  // ✅ 关联到项目
                scope_id: request.scope_id.clone(),      // ✅ 关联到范围（可选）
                asset_type: "domain".to_string(),
                canonical_url: format!("https://{}", subdomain),
                hostname: Some(subdomain.to_string()),
                labels_json: Some(["monitor-discovered"]),  // 标记来源
                // ... 其他字段
            };
            
            // 4. 保存到数据库
            db_service.create_bounty_asset(asset).await?;
        }
    }
}
```

### 3. 数据库存储

资产保存在 `bounty_assets` 表中：

```sql
CREATE TABLE bounty_assets (
    id TEXT PRIMARY KEY,
    program_id TEXT NOT NULL,        -- ✅ 项目ID（外键）
    scope_id TEXT,                   -- ✅ 范围ID（可选）
    asset_type TEXT NOT NULL,        -- 资产类型：domain, url, ip, port
    canonical_url TEXT NOT NULL,     -- 规范化URL
    hostname TEXT,                   -- 主机名
    labels_json TEXT,                -- 标签：["monitor-discovered"]
    metadata_json TEXT,              -- 元数据：来源、发现时间等
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    -- ... 其他字段
    
    FOREIGN KEY (program_id) REFERENCES bounty_programs(id)
);
```

## 📊 项目关联的好处

### 1. 自动分类

- 每个项目的资产独立管理
- 不同项目的资产不会混淆
- 可以按项目查看所有资产

### 2. 标签标识

发现的资产会自动添加标签：
- `monitor-discovered` - 标记为监控发现
- 可以根据标签筛选资产来源

### 3. 元数据记录

资产的 `metadata_json` 字段记录：
```json
{
  "source": "monitor_task",
  "plugin_id": "plugin__subdomain_enumerator",
  "discovered_at": "2026-01-23T10:30:00Z",
  "discovery_method": "automated"
}
```

## 🔍 如何查看关联的资产

### 方法 1：资产表面管理

```
BugBounty → 资产表面 → 选择项目
```

会显示该项目的所有资产，包括：
- 手动添加的资产
- 监控发现的资产（带 `monitor-discovered` 标签）
- 工作流发现的资产

### 方法 2：变更事件

```
BugBounty → 变更事件 → 选择项目
```

会显示资产变更历史，包括：
- 新资产发现事件
- 资产状态变更
- 资产属性变更

### 方法 3：数据库查询

```sql
-- 查询某个项目的所有资产
SELECT * FROM bounty_assets 
WHERE program_id = 'your-program-id';

-- 查询监控发现的资产
SELECT * FROM bounty_assets 
WHERE program_id = 'your-program-id'
  AND labels_json LIKE '%monitor-discovered%';

-- 查询特定插件发现的资产
SELECT * FROM bounty_assets 
WHERE program_id = 'your-program-id'
  AND metadata_json LIKE '%plugin__subdomain_enumerator%';
```

## 🎯 范围关联（可选）

如果项目定义了多个范围（Scope），可以指定资产属于哪个范围：

### 前端传入 scope_id

```typescript
const result = await invoke('monitor_discover_and_import_assets', {
  request: {
    program_id: props.selectedProgram.id,
    scope_id: selectedScope?.id,  // ✅ 指定范围ID
    // ...
  }
})
```

### 用途

- **按范围组织资产**：不同的子域、IP段等
- **权限控制**：不同范围可能有不同的测试权限
- **优先级管理**：核心范围 vs 边缘范围

## 📝 资产去重

系统会自动检查资产是否已存在：

```rust
// 检查资产是否已存在
if db_service.get_bounty_asset_by_canonical_url(&request.program_id, &canonical_url)
    .await?
    .is_some()
{
    continue; // 跳过已存在的资产
}
```

**去重规则**：
- 同一项目内
- 相同的 `canonical_url`
- 已存在则跳过，不重复创建

## 🔄 资产更新

对于已存在的资产，某些插件会更新其状态：

### HTTP Prober 示例

```rust
// 更新存活状态
if let Some(hosts) = data.get("hosts") {
    for host in hosts {
        // 查找已存在的资产
        if let Some(existing_asset) = db_service
            .get_bounty_asset_by_canonical_url(&request.program_id, &url)
            .await?
        {
            // 更新元数据
            let mut metadata = existing_asset.metadata_json;
            metadata["is_alive"] = true;
            metadata["last_checked_at"] = Utc::now();
            metadata["last_status_code"] = status_code;
            
            // 保存更新
            db_service.update_bounty_asset(existing_asset.id, metadata).await?;
        }
    }
}
```

## 🎨 UI 显示

### 资产列表

```
┌─────────────────────────────────────────────────────────┐
│ 资产表面 - 项目：Example Bug Bounty Program            │
├─────────────────────────────────────────────────────────┤
│ 🔍 筛选：[monitor-discovered]                           │
├─────────────────────────────────────────────────────────┤
│ ✅ api.example.com                                      │
│    类型：domain  |  标签：monitor-discovered            │
│    发现时间：2026-01-23 10:30                           │
│    来源：plugin__subdomain_enumerator                   │
├─────────────────────────────────────────────────────────┤
│ ✅ www.example.com                                      │
│    类型：domain  |  标签：monitor-discovered            │
│    发现时间：2026-01-23 10:30                           │
│    来源：plugin__subdomain_enumerator                   │
└─────────────────────────────────────────────────────────┘
```

### 统计信息

```
项目统计
├─ 总资产数：156
├─ 监控发现：45
├─ 手动添加：98
└─ 工作流发现：13
```

## 🚀 最佳实践

### 1. 使用标签组织

```typescript
// 为不同来源的资产添加不同标签
labels: [
  "monitor-discovered",      // 监控发现
  "subdomain-enum",          // 子域名枚举
  "high-priority",           // 高优先级
  "production",              // 生产环境
]
```

### 2. 定期清理

```sql
-- 删除长期未活跃的资产
DELETE FROM bounty_assets
WHERE program_id = 'your-program-id'
  AND last_checked_at < datetime('now', '-90 days')
  AND labels_json LIKE '%inactive%';
```

### 3. 资产分级

```typescript
// 根据资产重要性设置优先级分数
priority_score: {
  "api.*": 10,      // API 端点高优先级
  "admin.*": 10,    // 管理后台高优先级
  "www.*": 5,       // 主站中优先级
  "test.*": 1,      // 测试环境低优先级
}
```

## 📋 总结

### ✅ 已实现的功能

1. **自动项目关联** - 发现的资产自动关联到当前项目
2. **范围支持** - 可选的范围（Scope）关联
3. **自动去重** - 避免重复创建相同资产
4. **标签标识** - 自动添加 `monitor-discovered` 标签
5. **元数据记录** - 记录来源、发现时间等信息
6. **状态更新** - 支持更新已存在资产的状态

### 🎯 使用流程

1. 选择项目
2. 点击"发现资产"
3. 选择插件并填写参数
4. 勾选"自动导入资产"
5. 点击"开始发现"
6. 资产自动导入到当前项目

### 📊 查看资产

- **资产表面** - 查看项目的所有资产
- **变更事件** - 查看资产发现历史
- **数据库** - 直接查询 `bounty_assets` 表

**一切都是自动的，无需手动关联！** ✨
