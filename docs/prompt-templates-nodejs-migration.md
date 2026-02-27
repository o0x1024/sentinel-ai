# Prompt 模板 Node.js 迁移修复

## 问题描述

用户报告使用 AI 生成的插件中包含大量 Deno 方法（如 `Deno.readTextFile`, `Deno.core.ops.op_plugin_log` 等），而理论上应该全部使用 Node.js 风格的 API。

## 根本原因

虽然我们在 `prompt_api.rs` 中更新了 prompt 模板，但实际的插件生成流程使用的是 `prompt_templates.rs` 文件中的模板，该文件中仍然包含大量 Deno API 的示例代码。

## 修复的文件

### 1. `src-tauri/src/generators/prompt_templates.rs`

#### 修复前：
```rust
// Emit finding when vulnerability is detected
Deno.core.ops.op_emit_finding({
    vuln_type: "sqli",
    severity: "critical",
    // ...
});

// Logging
Deno.core.ops.op_plugin_log('info', 'Processing request...');
```

#### 修复后：
```rust
// Emit finding when vulnerability is detected
Sentinel.emitFinding({
    title: "SQL Injection Detected",
    vuln_type: "sqli",
    severity: "critical",
    // ...
});

// Logging
console.log('Processing request...');
```

#### 修改的具体位置：

1. **`build_plugin_template()` 函数** (第 769-867 行)
   - 将 `Deno.core.ops.op_emit_finding` 改为 `Sentinel.emitFinding`
   - 将 `Deno.core.ops.op_plugin_log` 改为 `console.log`
   - 添加 Node.js API 示例（`require('fs')`, `Buffer`, `crypto` 等）
   - 移除所有 Deno 特定的 API 引用

2. **修复提示信息** (第 169、178、309 行)
   - 将提示从 `Deno.core.ops.op_emit_finding()` 改为 `Sentinel.emitFinding()`
   - 更新错误提示信息

### 2. `src-tauri/src/generators/validator.rs`

#### 修复位置：
- 测试代码中的示例（第 281-285 行）
- 将 `Deno.core.ops.op_emit_finding` 改为 `Sentinel.emitFinding`

### 3. `src-tauri/src/generators/templates/agent_plugin_generation.txt`

#### 修复位置：
- 日志示例（第 221-225 行）
- 将 `Deno.core.ops.op_plugin_log` 改为 `console.log` / `console.error`

## 修改后的 API 示例

### Traffic 插件模板

```typescript
export function scan_transaction(ctx: HttpTransaction): void {
    // Convert body to string using Buffer
    const bodyText = Buffer.from(ctx.request.body).toString('utf8');
    
    // Emit finding
    Sentinel.emitFinding({
        title: 'SQL Injection Detected',
        severity: 'critical',
        confidence: 'high',
        evidence: bodyText,
    });
}

globalThis.scan_transaction = scan_transaction;
```

### Agent 插件模板

```typescript
const fs = require('fs').promises;
const crypto = require('crypto');

export async function analyze(input: ToolInput): Promise<ToolOutput> {
    // File operations
    const content = await fs.readFile(input.path, 'utf8');
    
    // Crypto operations
    const hash = crypto.createHash('sha256').update(content).digest('hex');
    
    // HTTP requests
    const response = await fetch('https://api.example.com');
    const data = await response.json();
    
    return { success: true, data: { hash } };
}

globalThis.analyze = analyze;
```

## 验证

### 编译测试
```bash
cd src-tauri
cargo check --package sentinel-ai
# ✅ 编译通过
```

### 预期效果

现在 AI 生成的插件应该：
1. ✅ 使用 `Sentinel.emitFinding()` 而不是 `Deno.core.ops.op_emit_finding()`
2. ✅ 使用 `console.log()` 而不是 `Deno.core.ops.op_plugin_log()`
3. ✅ 使用 Node.js API（`require('fs')`, `Buffer`, `crypto`）
4. ✅ 不包含任何 Deno 特定的 API

## 相关文件

- ✅ `src-tauri/src/commands/prompt_api.rs` - 已在之前更新
- ✅ `src-tauri/src/generators/prompt_templates.rs` - 本次修复
- ✅ `src-tauri/src/generators/validator.rs` - 本次修复
- ✅ `src-tauri/src/generators/templates/agent_plugin_generation.txt` - 本次修复
- ✅ `src-tauri/sentinel-plugins/src/plugin_bootstrap.js` - Node.js 兼容层（已实现）

## 后续测试建议

1. **生成新插件测试**：
   ```
   在 UI 中使用 AI 生成一个新的 Traffic 插件
   检查生成的代码是否使用 Sentinel.emitFinding()
   检查是否没有任何 Deno.* 的引用
   ```

2. **验证插件执行**：
   ```
   加载生成的插件
   测试插件是否能正常执行
   验证 Sentinel.emitFinding() 是否正常工作
   ```

3. **检查日志输出**：
   ```
   确认插件使用 console.log() 而不是 Deno.core.ops.op_plugin_log()
   验证日志是否正常输出
   ```

## 总结

通过修复 `prompt_templates.rs` 和相关模板文件，现在 AI 生成的插件将完全使用 Node.js 风格的 API，不再包含任何 Deno 特定的方法。这与我们实现的 Node.js 兼容层完美配合，确保：

- 🎯 **一致性**：生成的代码风格与文档一致
- 🎯 **兼容性**：使用标准 Node.js API，LLM 更容易理解
- 🎯 **可维护性**：减少混淆，降低维护成本
- 🎯 **用户体验**：生成的代码更符合开发者预期
