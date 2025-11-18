## 输出格式

只返回 TypeScript 插件代码，包裹在 markdown 代码块中：

```typescript
// 你的完整插件代码

// 1. 插件元数据函数
function get_metadata(): PluginMetadata {
    return {
        id: "unique_plugin_id",
        name: "插件名称",
        version: "1.0.0",
        author: "AI Generated",
        main_category: "passive",
        category: "sqli",  // 根据实际漏洞类型修改
        description: "插件功能描述",
        default_severity: "high",
        tags: ["sql", "injection"],
    };
}

// 2. 请求分析函数（如果不使用 await，保持 void；如果使用 await，改为 async 并返回 Promise<void>）
export function scan_request(ctx: RequestContext): void {
    try {
        // 分析请求的检测逻辑
        // 示例：检查危险参数
    } catch (error) {
        Deno.core.ops.op_plugin_log('error', `Request scan error: ${error}`);
    }
}

// 3. 响应分析函数（如果不使用 await，保持 void；如果使用 await，改为 async 并返回 Promise<void>）
export function scan_response(ctx: CombinedContext): void {
    try {
        // 解码响应体
        const decoder = new TextDecoder();
        const responseBody = decoder.decode(new Uint8Array(ctx.response.body));
        
        // 分析响应的检测逻辑
        // 示例：检查 SQL 错误特征
        
        // 如果检测到漏洞，上报发现
        // Deno.core.ops.op_emit_finding({ ... });
    } catch (error) {
        Deno.core.ops.op_plugin_log('error', `Response scan error: ${error}`);
    }
}

// 4. **关键**：必须导出到 globalThis
globalThis.get_metadata = get_metadata;
globalThis.scan_request = scan_request;
globalThis.scan_response = scan_response;
```

**必需要求**：

1. **包含详细注释**：解释检测逻辑和原理
2. **使用正确的 TypeScript 类型**：所有变量和函数都要有类型注解
3. **优雅处理边界情况和错误**：所有可能失败的操作都用 `try-catch` 包裹
4. **仅在置信度合理时上报发现**：避免低置信度的误报
5. **包含 CWE 和 OWASP 引用**（如适用）
6. **检测模式具有通用性**：不要仅针对特定网站，要能在不同网站上工作
7. **彻底验证模式以避免误报**：使用多个条件组合判断
8. **必须包含 globalThis 导出**：在文件末尾，否则会报 "Function not found" 错误

**语法检查（生成前必须确认）**：

- ✅ 如果函数内使用 `await`，必须声明为 `async` 并返回 `Promise<void>`
- ✅ 如果不使用 `await`，保持普通函数签名返回 `void`
- ✅ 所有 `try-catch` 块正确闭合
- ✅ 所有字符串引号正确闭合
- ✅ 所有花括号 `{}` 正确配对
- ✅ globalThis 导出位于文件末尾

**错误示例 - 会导致 AwaitInFunction 错误**：
```typescript
// ❌ 错误：在非 async 函数中使用 await
export function scan_response(ctx: CombinedContext): void {
    const response = await fetch(url);  // 错误！
}
```

**正确示例**：
```typescript
// ✅ 正确：async 函数可以使用 await
export async function scan_response(ctx: CombinedContext): Promise<void> {
    try {
        const response = await fetch(url);
        const data = await response.json();
    } catch (error) {
        Deno.core.ops.op_plugin_log('error', `Fetch failed: ${error}`);
    }
}
```

现在生成插件代码。

