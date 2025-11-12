# MCP工具刷新功能完善文档

## 问题描述

1. **Tools.vue**: MCP服务器重新安装/连接后，Agent编辑和新增中的可用工具列表没有自动更新
2. **AgentManager.vue**: 缺少实时刷新机制和手动刷新功能

## 解决方案

### 1. 后端事件发射 (src-tauri/src/commands/mcp.rs)

在MCP服务器状态变化时发射事件，通知前端更新：

#### 修改的命令：

1. **`add_child_process_mcp_server`**
   - 添加 `app_handle: tauri::AppHandle` 参数
   - 连接成功后发射 `mcp:tools-changed` 事件
   ```rust
   let _ = app_handle_clone.emit("mcp:tools-changed", &serde_json::json!({
       "action": "server_connected",
       "server": name_clone
   }));
   ```

2. **`mcp_disconnect_server`**
   - 添加 `app_handle: tauri::AppHandle` 参数
   - 断开连接成功后发射 `mcp:tools-changed` 事件
   ```rust
   let _ = app_handle.emit("mcp:tools-changed", &serde_json::json!({
       "action": "server_disconnected",
       "server": connection_id
   }));
   ```

3. **`mcp_delete_server_config`**
   - 添加 `app_handle: tauri::AppHandle` 参数
   - 删除服务器配置后发射 `mcp:tools-changed` 事件
   ```rust
   let _ = app_handle.emit("mcp:tools-changed", &serde_json::json!({
       "action": "server_deleted",
       "server": config.name.clone()
   }));
   ```

### 2. Tools.vue 事件监听

在组件挂载时添加事件监听器：

```typescript
// 监听MCP工具变更事件
listen('mcp:tools-changed', async (event) => {
  console.log('MCP tools changed event received:', event.payload);
  // 刷新连接列表和工具列表
  await fetchConnections();
  await fetchBuiltinTools();
});
```

### 3. AgentManager.vue 完善

#### 3.1 导入必要模块
```typescript
import { listen } from '@tauri-apps/api/event'
```

#### 3.2 添加状态变量
```typescript
const isRefreshingTools = ref(false)
```

#### 3.3 打开弹窗时刷新工具
```typescript
const openCreateModal = async () => {
  // ... 其他代码
  // 刷新工具列表以获取最新数据
  await loadTools()
  await loadPluginTools()
  showEditModal.value = true
}

const editAgent = async (agent: AgentProfile) => {
  // ... 其他代码
  // 刷新工具列表以获取最新数据
  await loadTools()
  await loadPluginTools()
  showEditModal.value = true
}
```

#### 3.4 添加事件监听
```typescript
onMounted(async () => {
  await Promise.all([loadAgents(), loadTools(), loadPluginTools(), loadPromptTemplates(), loadPromptGroups()])
  
  // 监听插件变化事件
  listen('plugin:changed', async () => {
    await loadPluginTools()
  })
  
  // 监听MCP工具变更事件
  listen('mcp:tools-changed', async () => {
    console.log('AgentManager: MCP tools changed, refreshing tools...')
    await loadTools()
    await loadPluginTools()
  })
})
```

#### 3.5 添加手动刷新功能
```typescript
// 手动刷新工具列表
const refreshTools = async () => {
  isRefreshingTools.value = true
  try {
    await Promise.all([loadTools(), loadPluginTools()])
  } finally {
    isRefreshingTools.value = false
  }
}
```

#### 3.6 UI改进 - 添加刷新按钮
```html
<div class="flex items-center gap-2 pb-2">
  <button type="button" class="btn btn-xs" @click="expandAllTools">展开全部</button>
  <button type="button" class="btn btn-xs" @click="collapseAllTools">折叠全部</button>
  <button type="button" class="btn btn-xs btn-outline btn-primary" 
          @click="refreshTools" :disabled="isRefreshingTools">
    <i :class="['fas', 'fa-sync-alt', { 'fa-spin': isRefreshingTools }]"></i>
    <span class="ml-1">刷新工具</span>
  </button>
</div>
```

## 功能特性

### 自动刷新机制
1. **MCP服务器连接时**: 自动刷新Tools.vue和AgentManager.vue的工具列表
2. **MCP服务器断开时**: 自动刷新工具列表
3. **MCP服务器删除时**: 自动刷新工具列表
4. **插件状态变化时**: 自动刷新插件工具列表

### 手动刷新
- AgentManager工具选择区域新增"刷新工具"按钮
- 按钮带有loading状态指示（旋转图标）
- 可随时手动刷新获取最新工具列表

### 主动刷新时机
- 打开Agent创建弹窗时
- 打开Agent编辑弹窗时

## 用户体验改进

1. **实时性**: MCP服务器状态变化时自动更新，无需手动刷新页面
2. **可见性**: 提供明确的刷新按钮和loading状态
3. **一致性**: Tools页面和AgentManager页面同步更新
4. **可靠性**: 支持手动刷新作为兜底方案

## 与AI助手联动

根据项目要求，所有模块都应该与AI助手进行联动和结合。本次更新确保了：

1. **工具可用性实时同步**: AI助手可以立即使用新安装的MCP工具
2. **Agent配置准确性**: 创建/编辑Agent时看到的工具列表始终是最新的
3. **插件工具支持**: 包含了agentTools类型的插件工具刷新

## 测试建议

1. 在Tools页面安装新的MCP服务器，检查AgentManager是否自动更新
2. 断开MCP服务器，验证工具列表是否正确移除相关工具
3. 在编辑Agent时手动点击"刷新工具"按钮，验证功能正常
4. 测试插件工具的启用/禁用是否触发刷新

## 相关文件

- `/Users/a1024/code/ai/sentinel-ai/src-tauri/src/commands/mcp.rs`
- `/Users/a1024/code/ai/sentinel-ai/src/views/Tools.vue`
- `/Users/a1024/code/ai/sentinel-ai/src/views/AgentManager.vue`
