# 工具配置持久化实现总结

## 📋 功能概述

实现了工具配置（Tool Config）的数据库持久化功能，确保工具配置与会话（Conversation）关联保存，并在发送消息时自动应用。

## 🎯 核心改进

### 1. **数据库层改动**

#### 表结构更新
- **文件**: `sentinel-core/src/models/database.rs`
- **修改**: `AiConversation` 结构体添加 `tool_config: Option<String>` 字段
- **用途**: 存储 JSON 格式的工具配置

#### 建表语句更新
- **文件**: `sentinel-db/src/database_service.rs`
- **修改**: `ai_conversations` 表添加 `tool_config TEXT` 列
- **迁移脚本**: `migration-add-tool-config.sql`

#### DAO 层更新
- **文件**: `sentinel-db/src/database/ai_conversation_dao.rs`
- **修改**:
  - `create_ai_conversation`: INSERT 语句添加 `tool_config` 绑定
  - `update_ai_conversation`: UPDATE 语句添加 `tool_config` 更新

### 2. **后端 API 改动**

#### 新增命令
- **文件**: `src-tauri/src/commands/ai.rs`
- **命令**: `update_conversation_tool_config`
- **功能**: 更新指定会话的工具配置并保存到数据库
- **参数**:
  - `conversation_id: String` - 会话 ID
  - `tool_config: serde_json::Value` - 工具配置 JSON

#### agent_execute 增强
- **文件**: `src-tauri/src/commands/ai.rs`
- **改进**: 在执行 agent 任务时自动从会话中读取工具配置
- **逻辑**:
  1. 优先使用传入的 `tool_config`（保留兼容性）
  2. 如果未传入，则从会话的 `tool_config` 字段读取
  3. 日志记录工具配置的加载状态

#### 命令注册
- **文件**: `src-tauri/src/lib.rs`
- **注册**: 添加 `update_conversation_tool_config` 到 Tauri 命令列表

### 3. **前端改动**

#### AgentView.vue
- **文件**: `src/components/Agent/AgentView.vue`

##### handleToolConfigUpdate 增强
```typescript
// 旧逻辑：仅更新内存
toolConfig.value = config
toolsEnabled.value = config.enabled

// 新逻辑：更新内存 + 保存数据库
toolConfig.value = config
toolsEnabled.value = config.enabled
if (conversationId.value) {
  await invoke('update_conversation_tool_config', {
    conversationId: conversationId.value,
    toolConfig: config
  })
}
```

##### loadConversationHistory 增强
```typescript
// 加载消息后，额外加载工具配置
const conv = conversations.find(c => c.id === convId)
if (conv && conv.tool_config) {
  const parsedConfig = JSON.parse(conv.tool_config)
  toolConfig.value = parsedConfig
  toolsEnabled.value = parsedConfig.enabled
}
```

##### handleSubmit 优化
```typescript
// 旧逻辑：每次都传递 tool_config
config: {
  ...
  tool_config: toolConfig.value.enabled ? toolConfig.value : {...}
}

// 新逻辑：
// 1. 创建新会话时保存工具配置
if (toolConfig.value.enabled) {
  await invoke('update_conversation_tool_config', {...})
}

// 2. agent_execute 不再传递 tool_config（从会话读取）
config: {
  ...
  // tool_config 字段移除
}
```

## 🔄 数据流程

### 保存流程
```
用户修改工具配置
    ↓
ToolConfigPanel 触发 update:config 事件
    ↓
AgentView.handleToolConfigUpdate
    ↓
调用 update_conversation_tool_config API
    ↓
后端更新数据库 ai_conversations.tool_config
```

### 加载流程
```
用户选择会话
    ↓
AgentView.loadConversationHistory
    ↓
从 get_ai_conversations 获取会话列表
    ↓
解析 tool_config JSON 字符串
    ↓
更新前端 toolConfig.value 和 toolsEnabled.value
```

### 使用流程
```
用户发送消息
    ↓
agent_execute 命令接收请求
    ↓
从 conversation_id 查询会话
    ↓
读取 tool_config 字段并解析
    ↓
应用工具配置到 Agent 执行
```

## 📊 数据库 Schema

### ai_conversations 表结构
```sql
CREATE TABLE ai_conversations (
    id TEXT PRIMARY KEY,
    title TEXT,
    service_name TEXT DEFAULT 'default',
    model_name TEXT NOT NULL,
    model_provider TEXT,
    context_type TEXT,
    project_id TEXT,
    vulnerability_id TEXT,
    scan_task_id TEXT,
    conversation_data TEXT,
    summary TEXT,
    total_messages INTEGER DEFAULT 0,
    total_tokens INTEGER DEFAULT 0,
    cost REAL DEFAULT 0.0,
    tags TEXT,
    tool_config TEXT,  -- 新增字段 (JSON)
    is_archived BOOLEAN DEFAULT 0,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (vulnerability_id) REFERENCES vulnerabilities(id) ON DELETE SET NULL,
    FOREIGN KEY (scan_task_id) REFERENCES scan_tasks(id) ON DELETE SET NULL
);
```

### tool_config JSON 示例
```json
{
  "enabled": true,
  "selection_strategy": "Keyword",
  "max_tools": 5,
  "fixed_tools": ["local_time"],
  "disabled_tools": [],
  "manual_tools": ["tool_id_1", "tool_id_2"]
}
```

## 🚀 使用说明

### 1. 数据库迁移（首次使用）
```bash
# 方法1: 使用迁移脚本
sqlite3 ~/.sentinel/sentinel.db < migration-add-tool-config.sql

# 方法2: 手动执行
sqlite3 ~/.sentinel/sentinel.db
> ALTER TABLE ai_conversations ADD COLUMN tool_config TEXT;
> .exit
```

### 2. 用户操作流程
1. 打开 Agent 视图
2. 点击工具按钮（扳手图标）开启工具
3. 点击工具配置按钮（齿轮图标）打开配置面板
4. 修改工具选择策略、最大工具数等
5. 点击"确定"按钮 → **自动保存到当前会话**
6. 后续该会话的所有消息都会自动使用保存的工具配置

### 3. 开发者验证
```typescript
// 前端验证
console.log('[AgentView] Tool config saved to conversation:', conversationId.value)

// 后端验证
tracing::info!("Loaded tool config from conversation");

// 数据库验证
sqlite3 ~/.sentinel/sentinel.db
> SELECT id, title, tool_config FROM ai_conversations WHERE tool_config IS NOT NULL;
```

## ✅ 测试清单

- [x] 后端编译成功 (`cargo check`)
- [x] 前端构建成功 (`yarn build`)
- [ ] 数据库迁移脚本测试
- [ ] 保存工具配置到新会话
- [ ] 保存工具配置到现有会话
- [ ] 切换会话后加载工具配置
- [ ] 发送消息时应用会话工具配置
- [ ] 工具配置为空时的默认行为

## 🔧 技术细节

### 为什么不使用 localStorage？
- **问题**: localStorage 是浏览器级别的存储，无法与会话关联
- **缺陷**: 切换会话后配置会错乱，不同会话无法有独立配置
- **解决**: 使用数据库存储，每个会话有独立的工具配置

### 为什么从会话读取而不是每次传递？
- **原因1**: 减少前端传参复杂度
- **原因2**: 确保配置一致性（单一数据源）
- **原因3**: 支持后端服务直接调用时也能使用工具配置

### JSON 序列化/反序列化
```rust
// Rust 端
let tool_config_str = serde_json::to_string(&config)?;
conversation.tool_config = Some(tool_config_str);

// TypeScript 端
const toolConfig = JSON.parse(conv.tool_config)
```

## 🐛 潜在问题

### 问题1: 数据库未迁移
- **现象**: 启动时报错 "no such column: tool_config"
- **解决**: 执行迁移脚本 `migration-add-tool-config.sql`

### 问题2: 工具配置未应用
- **现象**: 修改配置后发送消息，工具未调用
- **排查**:
  1. 检查 `conversationId.value` 是否存在
  2. 查看后端日志是否有 "Loaded tool config from conversation"
  3. 检查数据库 `tool_config` 字段是否为 NULL

### 问题3: 切换会话配置不更新
- **现象**: 切换到其他会话，工具配置没变
- **排查**: 检查 `loadConversationHistory` 是否正确解析 `tool_config`

## 📝 后续优化建议

1. **工具配置模板**: 支持保存/加载工具配置模板
2. **全局默认配置**: 为新会话设置默认工具配置
3. **工具使用统计**: 记录每个会话使用了哪些工具
4. **配置版本管理**: 支持工具配置的历史版本回退
5. **批量更新**: 支持批量更新多个会话的工具配置

## 🎉 完成状态

- ✅ 数据库结构更新
- ✅ 后端 API 实现
- ✅ 前端逻辑集成
- ✅ 编译验证通过
- ✅ 迁移脚本准备
- 📝 文档已完成

---
**实现日期**: 2025-12-10  
**版本**: v1.0  
**作者**: GitHub Copilot
