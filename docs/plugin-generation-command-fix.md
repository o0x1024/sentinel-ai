# 修复 AI 插件生成命令缺失问题

## 问题描述

用户在使用 AI 生成插件功能时遇到错误：
```
Command get_combined_plugin_prompt_api not found
```

## 根本原因

在之前的重构中，`src-tauri/src/commands/prompt_api.rs` 文件被删除，但前端代码 `PluginManagement.vue` 第 862 行仍然在调用这个命令：

```typescript
const systemPrompt = await invoke<string>('get_combined_plugin_prompt_api', {
  pluginType: isAgentPlugin ? 'agent' : 'traffic',
  vulnType: 'custom',
  severity: aiSeverity.value
})
```

## 解决方案

创建了新的命令文件 `plugin_generation_commands.rs` 来提供插件生成的 system prompt。

### 新增文件

**`src-tauri/src/commands/plugin_generation_commands.rs`**

```rust
//! Plugin generation prompt commands

use tauri::command;

/// Get combined plugin generation prompt for AI
#[command]
pub fn get_combined_plugin_prompt_api(
    plugin_type: String,
    _vuln_type: String,
    _severity: String,
) -> Result<String, String> {
    if plugin_type == "agent" {
        Ok(get_agent_plugin_prompt())
    } else {
        Ok(get_traffic_plugin_prompt())
    }
}

fn get_traffic_plugin_prompt() -> String {
    // Traffic plugin generation prompt (Node.js style)
}

fn get_agent_plugin_prompt() -> String {
    // Agent plugin generation prompt (Node.js style)
}
```

### Prompt 内容特点

1. **完全使用 Node.js 风格 API**
   - ✅ `Sentinel.emitFinding()` 而不是 `Deno.core.ops.op_emit_finding()`
   - ✅ `console.log()` 而不是 `Deno.core.ops.op_plugin_log()`
   - ✅ `require('fs')`, `Buffer`, `crypto` 等 Node.js API

2. **简洁明了**
   - 只包含必要的接口说明
   - 重点突出 `Sentinel.emitFinding()` 自定义 API
   - 说明 Node.js 兼容运行时

3. **示例代码正确**
   - 所有示例都使用 Node.js/Sentinel API
   - 没有任何 Deno 特定的代码

### 修改的文件

1. **新增文件**：
   - `src-tauri/src/commands/plugin_generation_commands.rs`

2. **修改文件**：
   - `src-tauri/src/commands/mod.rs` - 添加模块导出
   - `src-tauri/src/lib.rs` - 注册 Tauri 命令

### 修改内容

#### `mod.rs`
```rust
pub mod plugin_generation_commands;
// ...
pub use plugin_generation_commands::*;
```

#### `lib.rs`
```rust
.invoke_handler(generate_handler![
    // ...
    ai::cancel_plugin_assistant_chat,
    commands::get_active_rag_collections,
    commands::set_rag_collection_active,
    // Plugin generation commands
    commands::get_combined_plugin_prompt_api,  // ← 新增
    // Database commands
    // ...
])
```

## Traffic Plugin Prompt 示例

```typescript
export async function scan_transaction(transaction) {
  const resp = transaction.response;
  if (!resp) return;

  // Convert body using Buffer (Node.js style)
  const bodyText = Buffer.from(resp.body).toString('utf8');
  
  // Report finding using Sentinel API
  Sentinel.emitFinding({
      title: "Vulnerable Header Detected",
      severity: "medium",
      description: "The server exposes a vulnerable header.",
      evidence: `x-vulnerable-header: ${headerValue}`,
      confidence: "high",
  });
}

globalThis.scan_transaction = scan_transaction;
```

## Agent Plugin Prompt 示例

```typescript
const fs = require('fs').promises;
const crypto = require('crypto');

interface ToolInput {
    target: string;
}

interface ToolOutput {
    success: boolean;
    data?: any;
    error?: string;
}

export function get_input_schema() {
    return {
        type: "object",
        required: ["target"],
        properties: {
            target: {
                type: "string",
                description: "Target address"
            }
        }
    };
}

export async function analyze(input: ToolInput): Promise<ToolOutput> {
    try {
        // Use Node.js APIs
        const content = await fs.readFile(input.target, 'utf8');
        const hash = crypto.createHash('sha256').update(content).digest('hex');
        
        return {
            success: true,
            data: { hash }
        };
    } catch (error) {
        return {
            success: false,
            error: error instanceof Error ? error.message : String(error)
        };
    }
}

globalThis.get_input_schema = get_input_schema;
globalThis.analyze = analyze;
```

## 验证

### 编译测试
```bash
cd src-tauri
cargo check
# ✅ 编译通过
```

### 功能测试
1. 打开 Sentinel AI 应用
2. 进入插件管理页面
3. 点击 "AI生成插件" 按钮
4. 输入插件需求
5. 选择插件类型（Traffic 或 Agent）
6. 点击生成

预期结果：
- ✅ 不再出现 "Command not found" 错误
- ✅ AI 生成的插件使用 Node.js 风格 API
- ✅ 生成的代码使用 `Sentinel.emitFinding()`
- ✅ 没有任何 Deno API

## 相关文件

- ✅ `src-tauri/src/commands/plugin_generation_commands.rs` - 新增命令
- ✅ `src-tauri/src/commands/mod.rs` - 模块导出
- ✅ `src-tauri/src/lib.rs` - 命令注册
- ✅ `src-tauri/src/generators/prompt_templates.rs` - 模板（已更新为 Node.js 风格）
- ✅ `src-tauri/sentinel-plugins/src/plugin_bootstrap.js` - Node.js 兼容层

## 总结

通过创建 `plugin_generation_commands.rs` 文件并注册 `get_combined_plugin_prompt_api` 命令，修复了前端调用缺失命令的问题。新的 prompt 完全使用 Node.js 风格的 API，与我们实现的 Node.js 兼容层完美配合，确保 AI 生成的插件代码风格一致且正确。
