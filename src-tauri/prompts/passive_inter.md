## 插件接口规范（必需结构）

生成的插件必须实现以下 TypeScript 接口：

```typescript
// 插件元数据
function get_metadata(): PluginMetadata {
    return {
        id: "unique_plugin_id",           // 唯一插件 ID
        name: "插件名称",                  // 插件显示名称
        version: "1.0.0",                  // 版本号
        author: "AI Generated",            // 作者
        main_category: "passive",          // 主分类（固定为 passive）
        category: "vuln_type",             // 漏洞类型，例如："sqli", "xss", "file_upload", "command_injection", "path_traversal", "xxe", "ssrf", "idor", "auth_bypass", "info_leak", "csrf"
        description: "详细描述",           // 插件功能描述
        default_severity: "critical",      // 默认严重程度："critical" | "high" | "medium" | "low"
        tags: ["tag1", "tag2"],           // 标签数组
    };
}

// 分析 HTTP 请求（可选但推荐实现）
// 注意：如果不使用 await，返回类型为 void；如果使用 await，必须改为 async 并返回 Promise<void>
export function scan_request(ctx: RequestContext): void {
    // 分析请求参数、请求头、请求体
    // 如果检测到漏洞，调用 Deno.core.ops.op_emit_finding() 上报
}

// 分析 HTTP 响应（必需实现）
// 注意：如果不使用 await，返回类型为 void；如果使用 await，必须改为 async 并返回 Promise<void>
export function scan_response(ctx: CombinedContext): void {
    // 同时分析请求和响应
    // 检查响应中的漏洞指示器
    // 如果检测到漏洞，调用 Deno.core.ops.op_emit_finding() 上报
}

// **关键**：必须将函数导出到 globalThis，插件引擎才能调用
// 如果缺少这些导出，插件会报 "Function not found" 错误
// 使用直接赋值，不要使用类型转换，以确保正确执行
globalThis.get_metadata = get_metadata;
globalThis.scan_request = scan_request;
globalThis.scan_response = scan_response;

// 当检测到漏洞时上报发现
Deno.core.ops.op_emit_finding({
    vuln_type: "sqli",              // 漏洞类型
    severity: "critical",            // 严重程度
    confidence: "high",              // 置信度："high" | "medium" | "low"
    url: ctx.request.url,           // 请求 URL
    method: ctx.request.method,     // 请求方法
    param_name: "paramName",        // 参数名称
    param_value: "paramValue",      // 参数值
    evidence: "证据文本",            // 证据
    description: "漏洞描述",         // 描述
    cwe: "CWE-89",                  // CWE 编号
    owasp: "A03:2021",              // OWASP 分类
    remediation: "修复建议",         // 修复建议
});
```

**上下文对象接口**：
```typescript
// 请求上下文
interface RequestContext {
    request_id: string;              // 请求 ID
    url: string;                     // 请求 URL
    method: string;                  // 请求方法（GET, POST 等）
    headers: Record<string, string>; // 请求头
    query_params: Record<string, string>; // 查询参数
    body: number[];                  // 请求体（UTF-8 字节数组）
    timestamp: string;               // 时间戳
}

// 响应上下文
interface ResponseContext {
    request_id: string;              // 请求 ID
    status: number;                  // HTTP 状态码
    headers: Record<string, string>; // 响应头
    body: number[];                  // 响应体（UTF-8 字节数组）
    timestamp: string;               // 时间戳
}

// 组合上下文（包含请求和响应）
interface CombinedContext {
    request: RequestContext;         // 请求上下文
    response: ResponseContext;       // 响应上下文
}
```

**可用的内置 API**：

插件可以访问以下内置 API：

1. **Fetch API** - 发起 HTTP 请求：
```typescript
// ⚠️ 重要：使用 fetch 必须在 async 函数中，并且返回类型为 Promise<void>
export async function scan_response(ctx: CombinedContext): Promise<void> {
    try {
const response = await fetch('https://example.com/api', {
    method: 'POST',
    headers: {
        'Content-Type': 'application/json',
    },
    body: JSON.stringify({ key: 'value' }),
            timeout: 5000, // 可选，默认 30000ms
});

const data = await response.json();
        // 或者: const text = await response.text();
    } catch (error) {
        Deno.core.ops.op_plugin_log('error', `请求失败: ${error}`);
    }
}
```

2. **TextDecoder/TextEncoder** - 字节编解码：
```typescript
// 解码请求/响应体
const decoder = new TextDecoder();
const bodyText = decoder.decode(new Uint8Array(ctx.request.body));

// 编码文本为字节
const encoder = new TextEncoder();
const bytes = encoder.encode("文本内容");
```

3. **URL/URLSearchParams** - URL 解析：
```typescript
// 解析 URL
const url = new URL(ctx.request.url);
const params = new URLSearchParams(url.search);
const userId = params.get('user_id');

// 遍历所有参数
params.forEach((value, key) => {
    console.log(`${key} = ${value}`);
});
```

4. **日志记录** - 调试输出：
```typescript
// 记录日志（级别：'info', 'warn', 'error', 'debug'）
Deno.core.ops.op_plugin_log('info', '正在处理请求...');
Deno.core.ops.op_plugin_log('warn', '检测到可疑模式');
Deno.core.ops.op_plugin_log('error', `错误信息: ${error.message}`);
```

**关键注意事项**：

1. **async/await 规则**：
   - 如果函数内使用 `await`，必须声明为 `async` 且返回 `Promise<T>`
   - 示例：`export async function scan_response(ctx: CombinedContext): Promise<void>`

2. **字节数组处理**：
   - 请求体和响应体是 `number[]` 类型（UTF-8 字节）
   - 使用 `TextDecoder` 转换为字符串

3. **错误处理**：
   - 所有可能失败的操作都应包装在 `try-catch` 块中
   - 使用日志 API 记录错误信息

4. **globalThis 导出**：
   - 必须在文件末尾导出所有三个函数
   - 缺少导出会导致运行时错误

