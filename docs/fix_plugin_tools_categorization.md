# 修复插件工具分类问题

## 问题描述

在 AgentManager 的新增/编辑 Agent 界面中，"可用工具"部分的**插件工具**错误地出现在了**内置工具**分组中，而不是独立的"插件工具"分组。

## 根本原因

后端 `list_unified_tools_grouped` 函数的分类逻辑存在问题：

```rust
// 旧代码（有问题）
for t in tools.into_iter() {
    let is_mcp = t.metadata.tags.iter().any(|x| x == "mcp");
    // ...
    
    if is_mcp {
        groups.entry(key).or_default().push(item);
    } else {
        builtin.push(item);  // ❌ 所有非 MCP 工具都被归类为 builtin
    }
}
```

**问题分析**:
- 该函数只检查工具是否有 `mcp` 标签
- 所有**非 MCP 工具**都被归类到 `builtin` 数组
- **插件工具**（名称以 `plugin::` 开头）也被当作内置工具返回

## 解决方案

在分类逻辑中添加插件工具检测，将插件工具从返回结果中排除：

```rust
// 新代码（修复后）
for t in tools.into_iter() {
    // 跳过插件工具（名称以 plugin:: 开头或包含 plugin 标签）
    // 插件工具应该通过 list_plugins 接口单独管理
    let is_plugin = t.name.starts_with("plugin::") || t.metadata.tags.iter().any(|x| x == "plugin");
    if is_plugin {
        continue;  // ✅ 跳过插件工具
    }
    
    let is_mcp = t.metadata.tags.iter().any(|x| x == "mcp");
    // ...
}
```

**修复逻辑**:
1. 检测工具是否为插件工具（`plugin::` 前缀或 `plugin` 标签）
2. 如果是插件工具，直接跳过，不加入 `builtin` 或 `mcp` 分组
3. 插件工具通过前端的 `loadPluginTools()` 函数单独加载（调用 `list_plugins` 接口）

## 工具分类体系

修复后，工具系统的三层分类清晰明确：

```
┌─────────────────────────────────────────┐
│  list_unified_tools_grouped 接口        │
├─────────────────────────────────────────┤
│  ├─ builtin: 内置工具                    │
│  │   - playwright_navigate              │
│  │   - playwright_click                 │
│  │   - ...                              │
│  │                                       │
│  └─ mcp: MCP 工具分组                    │
│      ├─ connection: "server1"           │
│      │   └─ tools: [...]                │
│      └─ connection: "server2"           │
│          └─ tools: [...]                │
└─────────────────────────────────────────┘

┌─────────────────────────────────────────┐
│  list_plugins 接口（独立）               │
├─────────────────────────────────────────┤
│  插件工具列表                            │
│  - plugin::test_1                       │
│  - plugin::custom_tool                  │
│  - ...                                  │
└─────────────────────────────────────────┘
```

## 前端集成

前端 `AgentManager.vue` 已经正确实现了三分组展示：

```vue
<!-- 内置工具 -->
<div>
  <label v-for="tool in builtinTools" ...>
    {{ tool.title || tool.name }}
  </label>
</div>

<!-- MCP 工具分组 -->
<div v-for="group in mcpToolGroups" ...>
  <label v-for="tool in group.tools" ...>
    {{ tool.title || tool.name }}
  </label>
</div>

<!-- 插件工具分组 -->
<div>
  <label v-for="plugin in pluginTools" ...>
    {{ plugin.metadata.name }}
  </label>
</div>
```

**数据加载流程**:
```typescript
// 1. 加载内置工具 + MCP 工具
const loadTools = async () => {
  const grouped = await invoke('list_unified_tools_grouped')
  builtinTools.value = grouped.builtin  // ✅ 只包含真正的内置工具
  mcpToolGroups.value = grouped.mcp
}

// 2. 单独加载插件工具
const loadPluginTools = async () => {
  const response = await invoke('list_plugins')
  pluginTools.value = response.data.filter(
    p => p.metadata.category === 'agentTools' && p.status === 'Enabled'
  )
}
```

## 修改的文件

- **后端**: `/Users/a1024/code/ai/sentinel-ai/src-tauri/src/commands/ai_commands.rs`
  - 函数: `list_unified_tools_grouped`
  - 修改: 添加插件工具检测和跳过逻辑

## 测试验证

### 验证步骤

1. **启动应用**
   ```bash
   npm run dev
   ```

2. **打开 Agent 管理**
   - 点击 "新增 Agent" 或编辑现有 Agent
   
3. **检查可用工具分组**
   - ✅ "内置工具" 分组应该只包含真正的内置工具
   - ✅ "MCP: xxx" 分组应该包含对应连接的 MCP 工具
   - ✅ "插件工具" 分组应该独立显示，包含 agentTools 类型的插件

4. **验证工具选择**
   - 选择插件工具后，应该以 `plugin::` 前缀存储到 `agent.tools.allow` 数组
   - 保存 Agent 后重新编辑，插件工具应该正确回显勾选状态

### 预期结果

**修复前** ❌:
```
内置工具
  ├─ playwright_navigate
  ├─ playwright_click
  ├─ plugin::test_1          // ❌ 错误地出现在这里
  └─ plugin::custom_tool      // ❌ 错误地出现在这里

MCP: server1
  └─ mcp_tool_1

插件工具
  ├─ test_1                   // ✓ 正确位置
  └─ custom_tool              // ✓ 正确位置
```

**修复后** ✅:
```
内置工具
  ├─ playwright_navigate     // ✓ 只有真正的内置工具
  └─ playwright_click

MCP: server1
  └─ mcp_tool_1

插件工具
  ├─ test_1                  // ✓ 所有插件都在这里
  └─ custom_tool
```

## 影响范围

### 修改影响

- ✅ **无破坏性改动**: 只是改变了工具分类，不影响工具执行
- ✅ **向后兼容**: 已保存的 Agent 配置中的 `plugin::xxx` 工具名依然有效
- ✅ **UI 体验提升**: 工具分类更清晰，避免混淆

### 相关功能

- Agent 新增/编辑工具选择 ✅
- Agent 详情查看工具列表 ✅
- 工具执行（ReAct/Plan-Execute 等引擎）✅ 不受影响
- 插件管理 ✅ 不受影响

## 相关代码

### 后端

- `/Users/a1024/code/ai/sentinel-ai/src-tauri/src/commands/ai_commands.rs`
  - `list_unified_tools_grouped()` - 工具分组接口

### 前端

- `/Users/a1024/code/ai/sentinel-ai/src/views/AgentManager.vue`
  - `loadTools()` - 加载内置工具和 MCP 工具
  - `loadPluginTools()` - 加载插件工具
  - 模板中的三分组展示逻辑

## 未来改进

1. **后端统一接口**: 考虑在 `list_unified_tools_grouped` 返回值中增加 `plugins` 字段，将三种工具统一到一个接口返回
   ```rust
   pub struct GroupedToolsResponse {
       pub builtin: Vec<SimpleToolInfo>,
       pub mcp: Vec<McpToolGroup>,
       pub plugins: Vec<SimpleToolInfo>,  // 新增
   }
   ```

2. **工具标签规范**: 统一工具的标签系统
   - `builtin` 标签用于内置工具
   - `mcp` 标签用于 MCP 工具
   - `plugin` 标签用于插件工具

3. **性能优化**: 考虑缓存工具列表，减少重复查询
