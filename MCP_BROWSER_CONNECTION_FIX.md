# MCP Browser Connection 修复报告

## 问题描述

在执行Plan-and-Execute任务时，MCP工具调用失败，错误信息为：
```
ERROR: MCP tool 'browser_navigate' execution failed: No MCP client session for connection: browser
```

## 问题分析

通过日志分析和代码审查，发现问题的根本原因：

1. **工具名称映射错误**：
   - 系统从playwright MCP服务器成功获取了`browser_navigate`、`browser_click_text`、`browser_screenshot`等工具
   - 但在执行时，`split_connection_and_tool`方法错误地将`browser_navigate`解析为：
     - `connection_name` = `browser`
     - `actual_tool_name` = `navigate`
   - 而实际应该是：
     - `connection_name` = `playwright`
     - `actual_tool_name` = `browser_navigate`

2. **工具名称构造缺陷**：
   - MCP服务在获取外部工具时，直接使用了原始工具名称（如`browser_navigate`）
   - 没有添加连接名前缀（应该是`playwright_browser_navigate`）

## 修复方案

在`src-tauri/src/services/mcp.rs`中修改工具名称构造逻辑：

```rust
// 修复前
let tool_info = crate::tools::ToolInfo {
    id: rmcp_tool.name.to_string(),
    name: rmcp_tool.name.to_string(),
    // ...
}

// 修复后
let tool_name_with_prefix = format!("{}_{}", server_name, rmcp_tool.name);
let tool_info = crate::tools::ToolInfo {
    id: tool_name_with_prefix.clone(),
    name: tool_name_with_prefix,
    // ...
}
```

## 修复效果

修复后，从playwright连接获取的工具将被正确命名为：
- `playwright_browser_navigate`
- `playwright_browser_click_text`
- `playwright_browser_screenshot`
- 等等

这样`split_connection_and_tool`方法就能正确解析：
- `connection_name` = `playwright`
- `actual_tool_name` = `browser_navigate`

## 验证步骤

1. 构建项目：`cargo build --manifest-path src-tauri/Cargo.toml` ✅
2. 启动应用：`cargo tauri dev` ✅
3. 测试MCP工具执行功能

## 文件修改

- `/Users/a1024/code/sentinel-ai/src-tauri/src/services/mcp.rs`：第183-187行

## 影响范围

此修复影响所有外部MCP连接的工具命名，确保工具能够正确路由到对应的MCP服务器连接。

## 测试建议

建议测试以下场景：
1. 创建包含browser工具的Plan-and-Execute任务
2. 验证工具执行不再出现"No MCP client session"错误
3. 确认playwright和其他MCP服务器的工具都能正常执行
