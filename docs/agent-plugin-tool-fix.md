# Agent 工具插件显示修复

## 问题描述

Agent 工具插件没有显示在工具配置面板的手动选择模式中。

## 根本原因

后端代码在加载插件时使用了错误的字段名：
- **错误**：使用 `status` 字段检查插件是否启用
- **正确**：应该使用 `enabled` 字段（数据库表中的实际字段名）

## 数据库表结构

```sql
CREATE TABLE plugin_registry (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    version TEXT NOT NULL,
    author TEXT,
    main_category TEXT NOT NULL DEFAULT 'passive',  -- 'passive' 或 'agent'
    category TEXT NOT NULL,
    description TEXT,
    default_severity TEXT NOT NULL,
    tags TEXT,
    enabled BOOLEAN NOT NULL DEFAULT 0,  -- ← 关键字段
    config_json TEXT,
    plugin_code TEXT,
    installed_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_loaded_at TIMESTAMP,
    load_error TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    quality_score REAL,
    validation_status TEXT
)
```

## 修复内容

### 文件：`src-tauri/src/agents/tool_router.rs`

**修改前**：
```rust
let status = plugin.get("status").and_then(|v| v.as_str()).unwrap_or("");
let main_category = plugin.get("main_category").and_then(|v| v.as_str()).unwrap_or("");

if status == "enabled" && main_category == "agent" {
    // 加载插件
}
```

**修改后**：
```rust
let enabled = plugin.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false);
let main_category = plugin.get("main_category").and_then(|v| v.as_str()).unwrap_or("");

if enabled && main_category == "agent" {
    // 加载插件
}
```

## 加载条件

Agent 插件会被加载为 AI 工具，需要同时满足：
1. ✅ `enabled = true` (1)
2. ✅ `main_category = 'agent'`

## 测试步骤

### 1. 检查数据库中的 agent 插件

```bash
sqlite3 "/Users/a1024/Library/Application Support/sentinel-ai/database.db" \
  "SELECT id, name, enabled, main_category FROM plugin_registry WHERE main_category = 'agent';"
```

### 2. 启用一个 agent 插件（如果需要）

```bash
sqlite3 "/Users/a1024/Library/Application Support/sentinel-ai/database.db" \
  "UPDATE plugin_registry SET enabled = 1 WHERE id = 'your_plugin_id';"
```

或者在前端的插件管理页面启用插件。

### 3. 重启应用

重启应用后，工具路由器会重新加载插件。

### 4. 验证插件工具显示

1. 打开 Agent 对话界面
2. 点击工具按钮（🔧）旁边的设置图标（⚙️）
3. 选择"手动选择"策略
4. 点击"🧩 工具插件"分类按钮
5. 应该能看到启用的 agent 插件

## 日志验证

启动应用后，查看日志中是否有：

```
Loaded X plugin tools
```

如果 X > 0，说明插件已成功加载。

## 当前数据库状态

```
Next.js exp              | enabled=1 | main_category=agent  ✅ 会显示
Sec_es_ip               | enabled=0 | main_category=agent  ❌ 不会显示
```

## 工具配置面板显示效果

```
┌─────────────────────────────────────────┐
│ 工具配置                          [X]   │
├─────────────────────────────────────────┤
│ ☑ 启用工具调用                          │
│                                         │
│ 工具选择策略: [手动选择 ▼]              │
│                                         │
│ 选择工具:                        [🔄]   │
│ ┌─────────────────────────────────────┐ │
│ │ [全部] [🧩 工具插件] [网络] [系统]  │ │
│ ├─────────────────────────────────────┤ │
│ │ ☐ Next.js exp        [插件]        │ │  ← agent 插件
│ │   Agent plugin tool                 │ │
│ │                                     │ │
│ │ ☐ port_scan          [网络]        │ │
│ │   Scan TCP ports...                 │ │
│ └─────────────────────────────────────┘ │
└─────────────────────────────────────────┘
```

## 相关文件

- `src-tauri/src/agents/tool_router.rs` - 工具路由器（已修复）
- `src/components/Agent/ToolConfigPanel.vue` - 工具配置面板 UI
- `src/views/PluginManagement.vue` - 插件管理页面

## 注意事项

1. **插件必须启用**：只有 `enabled = true` 的插件才会显示
2. **必须是 agent 类型**：只有 `main_category = 'agent'` 的插件才会作为工具
3. **需要重启**：修改插件状态后需要重启应用才能生效
4. **前端启用**：推荐在插件管理页面启用插件，而不是直接修改数据库

## 如何在插件管理页面启用插件

1. 打开"插件管理"页面
2. 找到 agent 类型的插件（通常在"agents"分类下）
3. 点击插件卡片上的"启用"按钮
4. 重启应用
5. 在工具配置面板中应该就能看到该插件了

## 总结

✅ **问题已修复**：将 `status` 字段改为 `enabled` 字段  
✅ **类型正确**：使用 `as_bool()` 而不是 `as_str()`  
✅ **逻辑正确**：同时检查 `enabled` 和 `main_category`  
✅ **UI 已优化**：添加了"工具插件"专属分类按钮  

现在只要插件被启用（`enabled = true`）且类型为 agent（`main_category = 'agent'`），就会自动显示在工具配置面板中。
