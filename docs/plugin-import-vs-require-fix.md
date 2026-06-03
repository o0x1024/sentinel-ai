# 修复插件 ES6 Import 导致的执行失败

## 问题描述

用户生成的插件测试失败，错误信息：
```
Failed to create plugin executor: Plugin execution failed: 
Failed to get isolate handle: channel is empty and sending half is closed
```

## 问题代码

```javascript
// ❌ 错误：使用 ES6 import 语法
import * as fs from 'fs/promises';

export async function analyze(input) {
    const content = await fs.readFile(targetPath, encoding);
    // ...
}
```

## 根本原因

### 1. 模块系统不匹配

插件系统的 Node.js 兼容层实现了 **CommonJS 的 `require()`**，但没有实现 **ES6 的模块解析器**。

```javascript
// ✅ 已实现：CommonJS require
globalThis.require = function(moduleName) {
    if (moduleName === 'fs' || moduleName === 'node:fs') {
        return { /* fs module implementation */ };
    }
    // ...
}

// ❌ 未实现：ES6 module resolution
// import from 'fs/promises' 需要模块解析器能找到并加载 'fs/promises'
```

### 2. 插件引擎的限制

虽然插件引擎支持 **ESM 语法**（`export function`），但是：
- ✅ 支持 `export` 导出函数
- ✅ 支持 `async/await` 等现代 JS 特性
- ❌ 不支持 `import from` 外部模块
- ✅ 只支持 `require()` 导入模块

### 3. 为什么会失败

当插件尝试 `import * as fs from 'fs/promises'` 时：
1. Deno Core 的模块加载器尝试解析 `'fs/promises'` 模块
2. 找不到该模块（因为我们只注册了 `require()` 函数）
3. 模块加载失败，导致插件引擎崩溃
4. 出现 "channel is empty" 错误（V8 isolate 已关闭）

## 解决方案

### 正确的写法

```javascript
// ✅ 正确：使用 require()
const fs = require('fs').promises;
const crypto = require('crypto');
const path = require('path');

// ✅ 仍然可以使用 export（ESM 导出语法）
export async function analyze(input) {
    const content = await fs.readFile(targetPath, 'utf8');
    return { success: true, data: { content } };
}

// ✅ 必须导出到 globalThis
globalThis.analyze = analyze;
```

### 修复后的完整插件

```javascript
/**
 * @plugin local_system_auditor
 * @name Local System File Auditor
 */

// ✅ 使用 require 导入模块
const fs = require('fs').promises;

interface ToolInput {
    filePath?: string;
}

interface ToolOutput {
    success: boolean;
    data?: { content: string; path: string };
    error?: string;
}

export function get_input_schema() {
    return {
        type: "object",
        properties: {
            filePath: {
                type: "string",
                description: "File path to read",
                default: "/etc/passwd"
            }
        }
    };
}

export async function analyze(input: ToolInput): Promise<ToolOutput> {
    const targetPath = input.filePath || '/etc/passwd';

    try {
        // ✅ 使用 require 导入的 fs
        const content = await fs.readFile(targetPath, 'utf8');

        Sentinel.emitFinding({
            title: 'File Read Successful',
            description: `Read ${targetPath}`,
            severity: 'info',
            confidence: 'high',
            evidence: content.substring(0, 200)
        });

        return {
            success: true,
            data: { path: targetPath, content }
        };
    } catch (error: any) {
        return {
            success: false,
            error: error.message
        };
    }
}

// ✅ 导出到 globalThis
globalThis.get_input_schema = get_input_schema;
globalThis.analyze = analyze;
```

## 更新的 Prompt 模板

### 1. `plugin_generation_commands.rs`

添加了明确的警告：

```rust
**IMPORTANT**: Use `require()` for importing modules, NOT ES6 `import` statements:
```typescript
// ✅ CORRECT - Use require()
const fs = require('fs').promises;
const crypto = require('crypto');

// ❌ WRONG - Do NOT use import
import * as fs from 'fs/promises';  // This will fail!
```
```

### 2. `prompt_templates.rs`

同样添加了警告：

```rust
**IMPORTANT**: Use `require()` for modules, NOT ES6 `import`:
```typescript
// ✅ CORRECT
const fs = require('fs').promises;

// ❌ WRONG
import * as fs from 'fs/promises';  // Will fail!
```
```

## 支持的模块导入方式

### ✅ 支持的写法

```javascript
// CommonJS require
const fs = require('fs').promises;
const crypto = require('crypto');
const path = require('path');
const http = require('http');

// 解构导入
const { readFile, writeFile } = require('fs').promises;

// 导入整个模块
const fsModule = require('fs');
```

### ❌ 不支持的写法

```javascript
// ES6 import - 不支持！
import * as fs from 'fs/promises';
import { readFile } from 'fs/promises';
import fs from 'fs';
import crypto from 'crypto';

// 动态 import - 不支持！
const fs = await import('fs/promises');
```

### ✅ 但是可以使用 export

```javascript
// ✅ 可以使用 export 导出
export function get_input_schema() { }
export async function analyze(input) { }

// ✅ 也可以使用 export default
export default function analyze(input) { }

// ⚠️ 但必须同时导出到 globalThis
globalThis.analyze = analyze;
```

## 为什么这样设计？

### 1. 简化实现
- `require()` 可以通过简单的函数实现
- 不需要复杂的模块解析器
- 不需要文件系统查找

### 2. 安全性
- 可以精确控制哪些模块可用
- 避免加载任意文件
- 沙箱化更容易实现

### 3. 性能
- 无需文件系统 I/O
- 模块立即可用
- 启动速度快

## 验证

### 编译测试
```bash
cd src-tauri
cargo check
# ✅ 编译通过
```

### 功能测试
1. 使用修复后的插件代码
2. 保存并测试插件
3. 应该能正常执行

## 相关文件

- ✅ `src-tauri/src/commands/plugin_generation_commands.rs` - 添加警告
- ✅ `src-tauri/src/generators/prompt_templates.rs` - 添加警告
- ✅ `scripts/readfile_fixed.js` - 修复后的示例
- ✅ `src-tauri/sentinel-plugins/src/plugin_bootstrap.js` - require() 实现

## 最佳实践

### 插件开发者

1. **总是使用 `require()`** 导入 Node.js 模块
2. **可以使用 `export`** 导出函数（ESM 语法）
3. **必须导出到 `globalThis`** 让引擎能调用
4. **使用 TypeScript 类型** 提高代码质量

### 示例模板

```typescript
// ✅ 推荐的插件模板
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
        properties: {
            target: { type: "string", description: "Target" }
        },
        required: ["target"]
    };
}

export async function analyze(input: ToolInput): Promise<ToolOutput> {
    try {
        // Your logic here
        return { success: true, data: {} };
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

## 总结

- ❌ **不要使用** `import from` 语法导入模块
- ✅ **使用** `require()` 导入 Node.js 模块
- ✅ **可以使用** `export` 导出函数
- ✅ **必须导出** 到 `globalThis`
- 📝 **已更新** prompt 模板添加明确警告
