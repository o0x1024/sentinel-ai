# 插件 API 参考文档

本文档描述了 Sentinel AI 插件系统中可用的所有内置 API 和功能。

## 概述

插件运行在 Deno Core 环境中，可以访问以下内置 API：

1. **Fetch API** - 发起 HTTP 请求
2. **TextDecoder/TextEncoder** - 字节编码/解码
3. **URL/URLSearchParams** - URL 解析
4. **日志 API** - 调试输出
5. **漏洞发现 API** - 报告安全问题

---

## 1. Fetch API

### 描述
允许插件向外部服务发起 HTTP 请求。这对于验证漏洞、查询外部数据库或调用第三方 API 非常有用。

### 语法
```typescript
const response = await fetch(url: string, options?: FetchOptions): Promise<Response>
```

### 参数

**url** (string, 必需)
- 要请求的 URL

**options** (object, 可选)
- `method`: HTTP 方法 (GET, POST, PUT, DELETE, PATCH, HEAD)，默认 "GET"
- `headers`: 请求头对象，如 `{ 'Content-Type': 'application/json' }`
- `body`: 请求体（字符串）
- `timeout`: 超时时间（毫秒），默认 30000 (30秒)

### 返回值

返回一个 Promise，resolve 为 Response 对象：

```typescript
interface Response {
    ok: boolean;           // 状态码 200-299 时为 true
    status: number;        // HTTP 状态码
    headers: Record<string, string>;  // 响应头
    text(): Promise<string>;          // 获取响应文本
    json(): Promise<any>;             // 解析 JSON 响应
    body: string;                     // 响应体文本
}
```

### 示例

#### 基本 GET 请求
```typescript
export async function scan_response(ctx: CombinedContext): Promise<void> {
    try {
        const response = await fetch('https://api.example.com/check');
        const data = await response.json();
        
        if (data.vulnerable) {
            Deno.core.ops.op_emit_finding({
                vuln_type: "info_leak",
                severity: "high",
                confidence: "high",
                url: ctx.request.url,
                evidence: `External API confirmed vulnerability: ${JSON.stringify(data)}`,
            });
        }
    } catch (error) {
        Deno.core.ops.op_plugin_log('error', `Fetch failed: ${error.message}`);
    }
}
```

#### POST 请求
```typescript
const response = await fetch('https://api.example.com/verify', {
    method: 'POST',
    headers: {
        'Content-Type': 'application/json',
        'Authorization': 'Bearer token123',
    },
    body: JSON.stringify({
        url: ctx.request.url,
        param: suspiciousParam,
    }),
    timeout: 5000,
});

const result = await response.json();
```

#### SSRF 验证示例
```typescript
// 检测并验证 SSRF 漏洞
export async function scan_request(ctx: RequestContext): void {
    const url = new URL(ctx.url);
    const targetUrl = url.searchParams.get('url');
    
    if (targetUrl && isInternalIP(targetUrl)) {
        try {
            // 尝试访问内部 URL
            const response = await fetch(targetUrl, { timeout: 3000 });
            
            if (response.ok) {
                Deno.core.ops.op_emit_finding({
                    vuln_type: "ssrf",
                    severity: "critical",
                    confidence: "high",
                    url: ctx.url,
                    param_name: "url",
                    param_value: targetUrl,
                    evidence: `Successfully accessed internal URL: ${targetUrl}, status: ${response.status}`,
                    description: "Server-Side Request Forgery vulnerability allows access to internal resources",
                    cwe: "CWE-918",
                    owasp: "A10:2021",
                });
            }
        } catch (e) {
            // 网络错误也可能表明 SSRF 尝试
            Deno.core.ops.op_plugin_log('debug', `SSRF attempt detected but connection failed: ${e.message}`);
        }
    }
}

function isInternalIP(url: string): boolean {
    const patterns = [
        /^https?:\/\/(127\.|10\.|172\.(1[6-9]|2[0-9]|3[01])\.|192\.168\.)/,
        /^https?:\/\/localhost/,
        /169\.254\.169\.254/, // AWS metadata
    ];
    return patterns.some(p => p.test(url));
}
```

---

## 2. TextDecoder / TextEncoder

### 描述
用于在字节数组和字符串之间进行转换。插件接收的 HTTP body 是字节数组格式。

### TextDecoder

#### 语法
```typescript
const decoder = new TextDecoder(encoding?: string);
const text = decoder.decode(input: Uint8Array): string;
```

#### 示例
```typescript
export function scan_response(ctx: CombinedContext): void {
    const decoder = new TextDecoder('utf-8');
    const requestBody = decoder.decode(new Uint8Array(ctx.request.body));
    const responseBody = decoder.decode(new Uint8Array(ctx.response.body));
    
    // 分析文本内容
    if (responseBody.includes('SQL syntax error')) {
        // 检测到 SQL 错误
    }
}
```

### TextEncoder

#### 语法
```typescript
const encoder = new TextEncoder();
const bytes = encoder.encode(input: string): Uint8Array;
```

#### 示例
```typescript
const text = "Hello, World!";
const bytes = new TextEncoder().encode(text);
```

---

## 3. URL / URLSearchParams

### URL

#### 描述
解析和操作 URL。

#### 语法
```typescript
const url = new URL(input: string, base?: string);
```

#### 属性
- `href`: 完整 URL
- `protocol`: 协议 (http:, https:)
- `host`: 主机名和端口
- `hostname`: 主机名
- `port`: 端口
- `pathname`: 路径
- `search`: 查询字符串 (包含 ?)
- `hash`: 锚点 (包含 #)
- `origin`: 源 (protocol + host)
- `searchParams`: URLSearchParams 对象

#### 示例
```typescript
export function scan_request(ctx: RequestContext): void {
    const url = new URL(ctx.url);
    
    console.log(url.hostname);  // "example.com"
    console.log(url.pathname);  // "/api/users"
    console.log(url.search);    // "?id=123&name=test"
    
    // 访问查询参数
    const userId = url.searchParams.get('id');
    const userName = url.searchParams.get('name');
}
```

### URLSearchParams

#### 描述
解析和操作 URL 查询参数。

#### 方法
- `get(name)`: 获取参数值
- `getAll(name)`: 获取所有同名参数值
- `has(name)`: 检查参数是否存在
- `append(name, value)`: 添加参数
- `toString()`: 转换为查询字符串
- `forEach(callback)`: 遍历参数

#### 示例
```typescript
export function scan_request(ctx: RequestContext): void {
    const url = new URL(ctx.url);
    const params = url.searchParams;
    
    // 检查 SQL 注入模式
    params.forEach((value, key) => {
        if (containsSQLPattern(value)) {
            Deno.core.ops.op_emit_finding({
                vuln_type: "sqli",
                severity: "high",
                confidence: "medium",
                url: ctx.url,
                param_name: key,
                param_value: value,
                evidence: `SQL injection pattern detected in parameter: ${key}=${value}`,
            });
        }
    });
}

function containsSQLPattern(value: string): boolean {
    const patterns = [
        /(\bOR\b|\bAND\b).*=.*=/i,
        /UNION.*SELECT/i,
        /'\s*OR\s*'1'\s*=\s*'1/i,
    ];
    return patterns.some(p => p.test(value));
}
```

---

## 4. 日志 API

### 描述
输出调试和诊断信息。

### 语法
```typescript
Deno.core.ops.op_plugin_log(level: string, message: string): void
```

### 参数
- **level**: 日志级别 - "error", "warn", "info", "debug"
- **message**: 日志消息

### 示例
```typescript
export function scan_request(ctx: RequestContext): void {
    Deno.core.ops.op_plugin_log('info', `Processing request: ${ctx.url}`);
    
    try {
        // 插件逻辑
        Deno.core.ops.op_plugin_log('debug', 'Checking for SQL injection patterns');
    } catch (error) {
        Deno.core.ops.op_plugin_log('error', `Plugin error: ${error.message}`);
    }
}
```

---

## 5. 漏洞发现 API

### 描述
报告检测到的安全漏洞。

### 语法
```typescript
Deno.core.ops.op_emit_finding(finding: Finding): void
```

### Finding 对象

```typescript
interface Finding {
    vuln_type: string;        // 必需: 漏洞类型
    severity: string;         // 必需: 严重程度 (critical, high, medium, low, info)
    confidence: string;       // 必需: 置信度 (high, medium, low)
    url: string;              // 必需: 漏洞 URL
    method?: string;          // HTTP 方法
    param_name?: string;      // 参数名
    param_value?: string;     // 参数值
    evidence: string;         // 必需: 证据
    description?: string;     // 漏洞描述
    title?: string;           // 标题
    cwe?: string;             // CWE 编号
    owasp?: string;           // OWASP 分类
    remediation?: string;     // 修复建议
}
```

### 示例

#### 基本用法
```typescript
Deno.core.ops.op_emit_finding({
    vuln_type: "xss",
    severity: "high",
    confidence: "high",
    url: ctx.request.url,
    method: ctx.request.method,
    param_name: "comment",
    param_value: "<script>alert(1)</script>",
    evidence: "Unescaped user input reflected in response",
    description: "Cross-Site Scripting vulnerability allows execution of arbitrary JavaScript",
    cwe: "CWE-79",
    owasp: "A03:2021",
    remediation: "Sanitize and encode all user input before rendering in HTML",
});
```

#### SQL 注入检测
```typescript
export function scan_response(ctx: CombinedContext): void {
    const decoder = new TextDecoder();
    const responseBody = decoder.decode(new Uint8Array(ctx.response.body));
    
    // 检测数据库错误消息
    const sqlErrors = [
        /You have an error in your SQL syntax/i,
        /mysql_fetch_array\(\)/i,
        /PostgreSQL.*ERROR/i,
        /Warning.*mysql_/i,
    ];
    
    for (const pattern of sqlErrors) {
        if (pattern.test(responseBody)) {
            Deno.core.ops.op_emit_finding({
                vuln_type: "sqli",
                severity: "critical",
                confidence: "high",
                url: ctx.request.url,
                method: ctx.request.method,
                evidence: `Database error message detected: ${responseBody.match(pattern)?.[0]}`,
                description: "SQL Injection vulnerability - database error messages exposed",
                cwe: "CWE-89",
                owasp: "A03:2021",
                remediation: "Use parameterized queries and input validation",
            });
            break;
        }
    }
}
```

---

## 完整插件示例

以下是一个完整的 XSS 检测插件示例，展示了如何使用多个 API：

```typescript
// 插件元数据
function get_metadata() {
    return {
        id: "xss-detector-advanced",
        name: "Advanced XSS Detector",
        version: "1.0.0",
        author: "AI Generated",
        main_category: "passive",
        category: "xss",
        description: "Detects reflected and stored XSS vulnerabilities",
        default_severity: "high",
        tags: ["xss", "injection", "client-side"],
    };
}

// 扫描请求
export function scan_request(ctx: RequestContext): void {
    try {
        Deno.core.ops.op_plugin_log('debug', `Scanning request: ${ctx.url}`);
        
        const url = new URL(ctx.url);
        const params = url.searchParams;
        
        // 检查查询参数中的 XSS 模式
        params.forEach((value, key) => {
            if (containsXSSPattern(value)) {
                Deno.core.ops.op_plugin_log('info', `Potential XSS in parameter: ${key}`);
            }
        });
    } catch (error) {
        Deno.core.ops.op_plugin_log('error', `scan_request error: ${error.message}`);
    }
}

// 扫描响应
export function scan_response(ctx: CombinedContext): void {
    try {
        const decoder = new TextDecoder();
        const responseBody = decoder.decode(new Uint8Array(ctx.response.body));
        const url = new URL(ctx.request.url);
        
        // 检查参数是否被反射到响应中
        url.searchParams.forEach((value, key) => {
            if (containsXSSPattern(value) && responseBody.includes(value)) {
                // 检查是否被正确编码
                const isEncoded = isProperlyEncoded(value, responseBody);
                
                if (!isEncoded) {
                    Deno.core.ops.op_emit_finding({
                        vuln_type: "xss",
                        severity: "high",
                        confidence: "high",
                        url: ctx.request.url,
                        method: ctx.request.method,
                        param_name: key,
                        param_value: value,
                        evidence: `Unencoded XSS payload reflected in response: ${value}`,
                        description: "Reflected Cross-Site Scripting vulnerability",
                        cwe: "CWE-79",
                        owasp: "A03:2021",
                        remediation: "Encode all user input before rendering in HTML context",
                    });
                }
            }
        });
    } catch (error) {
        Deno.core.ops.op_plugin_log('error', `scan_response error: ${error.message}`);
    }
}

// 辅助函数
function containsXSSPattern(value: string): boolean {
    const patterns = [
        /<script[^>]*>.*?<\/script>/i,
        /javascript:/i,
        /on\w+\s*=/i,  // onclick=, onerror=, etc.
        /<iframe/i,
        /<svg.*onload/i,
    ];
    return patterns.some(p => p.test(value));
}

function isProperlyEncoded(original: string, html: string): boolean {
    const encoded = original
        .replace(/&/g, '&amp;')
        .replace(/</g, '&lt;')
        .replace(/>/g, '&gt;')
        .replace(/"/g, '&quot;')
        .replace(/'/g, '&#x27;');
    
    return html.includes(encoded);
}

// 导出到 globalThis (必需!)
globalThis.get_metadata = get_metadata;
globalThis.scan_request = scan_request;
globalThis.scan_response = scan_response;
```

---

## 最佳实践

### 1. 错误处理
始终使用 try-catch 包裹插件逻辑：

```typescript
export function scan_response(ctx: CombinedContext): void {
    try {
        // 插件逻辑
    } catch (error) {
        Deno.core.ops.op_plugin_log('error', `Plugin error: ${error.message}`);
    }
}
```

### 2. 性能考虑
- 使用 `timeout` 参数限制 fetch 请求时间
- 避免在每个请求中进行大量外部 API 调用
- 使用日志级别 "debug" 输出详细信息

### 3. 避免误报
- 设置合理的 `confidence` 级别
- 验证检测模式的准确性
- 提供清晰的 `evidence` 说明

### 4. 安全性
- 不要在日志中输出敏感信息
- 谨慎使用 fetch API，避免造成 SSRF
- 验证外部 API 响应的可信度

---

## 限制和注意事项

1. **网络访问**: fetch API 可以访问任何 URL，但应谨慎使用以避免性能问题
2. **超时**: 默认 30 秒超时，建议根据场景调整
3. **内存**: 插件运行在隔离环境中，避免存储大量数据
4. **异步**: 所有扫描函数都支持 async/await
5. **globalThis 导出**: 必须将函数导出到 globalThis，否则插件引擎无法调用

---

## 故障排查

### 问题: "Function not found" 错误
**解决方案**: 确保在插件末尾添加 globalThis 导出：
```typescript
globalThis.get_metadata = get_metadata;
globalThis.scan_request = scan_request;
globalThis.scan_response = scan_response;
```

### 问题: "ReferenceError: URL is not defined"
**解决方案**: 这个问题已在最新版本中修复。如果仍然出现，请更新插件引擎。

### 问题: Fetch 请求超时
**解决方案**: 调整 timeout 参数或检查网络连接：
```typescript
const response = await fetch(url, { timeout: 5000 }); // 5秒超时
```

### 问题: 无法解析 body
**解决方案**: 使用 TextDecoder 正确解码：
```typescript
const decoder = new TextDecoder();
const text = decoder.decode(new Uint8Array(ctx.request.body));
```

