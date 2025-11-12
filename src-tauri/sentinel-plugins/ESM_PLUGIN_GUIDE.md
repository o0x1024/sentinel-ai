# ESM/TypeScript 插件开发指南

## 概述

Sentinel AI 插件引擎现在支持完整的 ES Modules 和 TypeScript！你可以使用现代 JavaScript/TypeScript 特性编写插件。

## 特性支持

### ✅ 支持的特性

- **TypeScript 语法**：类型标注、接口、泛型等
- **ES Modules**：`import`/`export` 语句
- **async/await**：异步函数
- **现代 JS**：箭头函数、解构、模板字符串、可选链等
- **自动转译**：TypeScript 自动编译为 JavaScript

### ❌ 不支持的特性

- **外部依赖**：无法 `import` 第三方 npm 包（未来可能支持）
- **Node.js API**：不支持 `fs`、`path` 等 Node 内置模块
- **Deno 特定 API**：`Deno.readTextFile` 等（仅支持我们提供的 `Sentinel` API）

## 插件结构

### 基础模板

```typescript
// 1. 类型定义（可选，会被自动移除）
interface RequestContext {
    id: string;
    method: string;
    url: string;
    headers: Record<string, string>;
    body: number[];
    // ...
}

// 2. 导出扫描函数（必须使用 snake_case 命名）
export function scan_request(ctx: RequestContext): void {
    // 扫描请求逻辑
    
    // 使用 Sentinel API 上报发现
    (globalThis as any).Sentinel.emitFinding({
        title: '发现问题',
        description: '详细描述',
        severity: 'high',
        vuln_type: 'xss',
        confidence: 'medium',
    });
}

export function scan_response(args: { request: RequestContext; response: ResponseContext }): void {
    // 扫描响应逻辑
}

// 3. 将函数暴露到全局（重要！）
(globalThis as any).scan_request = scan_request;
(globalThis as any).scan_response = scan_response;
```

### 关键要点

1. **函数命名**：必须使用 `scan_request` 和 `scan_response`（snake_case）
2. **全局暴露**：必须在文件末尾将函数挂载到 `globalThis`
3. **上报方式**：使用 `Sentinel.emitFinding()` 上报，不要用 `return`
4. **类型安全**：可以使用 TypeScript 类型，引擎会自动移除

## Sentinel API

### emitFinding(finding: Finding)

上报一个安全发现。

```typescript
interface Finding {
    title: string;              // 标题
    description: string;         // 详细描述
    severity: 'critical' | 'high' | 'medium' | 'low' | 'info';
    vuln_type: string;          // 漏洞类型，如 'xss', 'sqli', 'sensitive_info'
    confidence: 'high' | 'medium' | 'low';
    
    // 可选字段
    request?: {
        method: string;
        url: string;
    };
    response?: {
        status: number;
    };
    cwe?: string;               // CWE 编号
    owasp?: string;             // OWASP 分类
    remediation?: string;       // 修复建议
}

// 使用示例
(globalThis as any).Sentinel.emitFinding({
    title: 'XSS 漏洞',
    description: '在响应中发现未转义的用户输入',
    severity: 'high',
    vuln_type: 'xss',
    confidence: 'high',
    cwe: 'CWE-79',
    owasp: 'A03:2021',
    remediation: '对所有用户输入进行 HTML 转义',
});
```

## 实用工具函数

### 字节数组转字符串

```typescript
function bytesToUtf8(bytes: number[]): string {
    try {
        return new TextDecoder('utf-8', { fatal: false }).decode(new Uint8Array(bytes));
    } catch {
        return String.fromCharCode(...bytes);
    }
}

// 使用
const bodyText = bytesToUtf8(ctx.body);
```

### 正则匹配示例

```typescript
function detectSensitiveInfo(text: string): void {
    const patterns = [
        { name: '手机号', regex: /\b1[3-9]\d{9}\b/g },
        { name: '邮箱', regex: /\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}\b/g },
    ];

    for (const { name, regex } of patterns) {
        const matches = text.match(regex);
        if (matches) {
            (globalThis as any).Sentinel.emitFinding({
                title: `检测到${name}`,
                description: `发现 ${matches.length} 个${name}`,
                severity: 'medium',
                vuln_type: 'sensitive_info',
                confidence: 'medium',
            });
        }
    }
}
```

## 从旧格式迁移

### 旧格式（脚本模式，已不推荐）

```javascript
// ❌ 旧的写法
function scan_request(ctx) {
    // ...
}
```

### 新格式（ESM/TS 模式）

```typescript
// ✅ 新的写法
export function scan_request(ctx: RequestContext): void {
    // ...
}

// 挂载到全局
(globalThis as any).scan_request = scan_request;
```

## 完整示例

查看 `plugins/esm-template.ts` 获取完整的模板示例。

## 调试技巧

### 1. 使用 console.log

```typescript
export function scan_request(ctx: RequestContext): void {
    console.log('[Plugin] Processing:', ctx.method, ctx.url);
    // 日志会输出到 Rust 的 tracing 系统
}
```

### 2. 错误处理

```typescript
export function scan_request(ctx: RequestContext): void {
    try {
        const text = bytesToUtf8(ctx.body);
        // 处理逻辑
    } catch (error) {
        console.error('[Plugin] Error:', error);
    }
}
```

### 3. 条件检测

```typescript
export function scan_response(args: { request: RequestContext; response: ResponseContext }): void {
    const { response } = args;
    
    // 检查必要条件
    if (!response || !response.body || response.body.length === 0) {
        return;
    }
    
    // 只处理 HTML 响应
    const contentType = response.headers['content-type'] || '';
    if (!contentType.includes('text/html')) {
        return;
    }
    
    // 继续处理...
}
```

## 常见问题

### Q: 为什么必须挂载到 globalThis？

A: 因为 ES Module 的导出在模块作用域内，引擎需要从全局作用域调用函数。未来版本可能会直接支持模块导出。

### Q: 可以使用 npm 包吗？

A: 目前不支持。未来可能添加对常用库的支持。

### Q: TypeScript 编译错误怎么办？

A: 检查语法是否正确。如果使用了不支持的特性（如装饰器的特定写法），请简化代码。

### Q: 如何测试插件？

A: 
1. 在数据库中创建插件
2. 启用插件
3. 启动被动扫描代理
4. 发送测试流量
5. 查看日志和前端的发现列表

## 性能优化

1. **避免大量正则匹配**：对超大响应体进行采样或限制匹配范围
2. **早期返回**：快速过滤不需要处理的请求/响应
3. **缓存编译结果**：正则表达式可以定义在模块顶层

```typescript
// ✅ 好的做法：正则在模块级别定义
const PHONE_REGEX = /\b1[3-9]\d{9}\b/g;

export function scan_request(ctx: RequestContext): void {
    const text = bytesToUtf8(ctx.body);
    const matches = text.match(PHONE_REGEX);
    // ...
}
```

## 下一步

- 查看 `plugins/esm-template.ts` 完整模板
- 阅读 `plugin-types.d.ts` 了解完整类型定义
- 参考内置插件了解最佳实践
