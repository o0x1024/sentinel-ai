/**
 * ESM/TypeScript 插件模板
 * 
 * 支持：
 * - TypeScript 类型标注
 * - ES Modules (import/export)
 * - async/await
 * - 现代 JavaScript 特性
 */

// 类型定义（可选，会被自动移除）
interface RequestContext {
    id: string;
    method: string;
    url: string;
    headers: Record<string, string>;
    body: number[];
    content_type?: string;
    query_params: Record<string, string>;
    is_https: boolean;
    timestamp: string;
}

interface ResponseContext {
    request_id: string;
    status: number;
    headers: Record<string, string>;
    body: number[];
    content_type?: string;
    timestamp: string;
}

interface Finding {
    title: string;
    description: string;
    severity: 'critical' | 'high' | 'medium' | 'low' | 'info';
    vuln_type: string;
    confidence: 'high' | 'medium' | 'low';
    request?: {
        method: string;
        url: string;
    };
    response?: {
        status: number;
    };
    cwe?: string;
    owasp?: string;
    remediation?: string;
}

// 辅助函数：字节数组转 UTF-8 字符串
function bytesToUtf8(bytes: number[]): string {
    try {
        return new TextDecoder('utf-8', { fatal: false }).decode(new Uint8Array(bytes));
    } catch {
        return String.fromCharCode(...bytes);
    }
}

// 导出扫描请求函数（必须导出到 globalThis 以便引擎调用）
export function scan_request(ctx: RequestContext): void {
    if (!ctx || !ctx.body || ctx.body.length === 0) {
        return;
    }

    const bodyText = bytesToUtf8(ctx.body);

    // 示例：检测敏感信息
    const patterns = [
        { name: '手机号', regex: /\b1[3-9]\d{9}\b/g, severity: 'medium' as const },
        { name: '身份证号', regex: /\b\d{17}[\dXx]\b/g, severity: 'high' as const },
        { name: '邮箱地址', regex: /\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}\b/g, severity: 'low' as const },
    ];

    for (const pattern of patterns) {
        const matches = bodyText.match(pattern.regex);
        if (matches && matches.length > 0) {
            // 使用全局 Sentinel API 上报发现
            (globalThis as any).Sentinel.emitFinding({
                title: `请求体包含${pattern.name}`,
                description: `在请求体中检测到 ${matches.length} 个${pattern.name}，示例: ${matches.slice(0, 3).join(', ')}`,
                severity: pattern.severity,
                vuln_type: 'sensitive_info_leak',
                confidence: 'medium',
                request: {
                    method: ctx.method,
                    url: ctx.url,
                },
            });
        }
    }
}

// 导出扫描响应函数
export function scan_response(args: { request: RequestContext; response: ResponseContext }): void {
    const { request, response } = args;
    
    if (!response || !response.body || response.body.length === 0) {
        return;
    }

    const bodyText = bytesToUtf8(response.body);

    // 示例：检测响应中的敏感信息
    const patterns = [
        { name: 'API密钥', regex: /\b[A-Za-z0-9]{32,}\b/g, severity: 'high' as const },
        { name: '邮箱地址', regex: /\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}\b/g, severity: 'medium' as const },
    ];

    for (const pattern of patterns) {
        const matches = bodyText.match(pattern.regex);
        if (matches && matches.length > 0) {
            (globalThis as any).Sentinel.emitFinding({
                title: `响应体可能泄露${pattern.name}`,
                description: `在响应中检测到 ${matches.length} 个可能的${pattern.name}`,
                severity: pattern.severity,
                vuln_type: 'sensitive_info_leak',
                confidence: 'medium',
                request: {
                    method: request.method,
                    url: request.url,
                },
                response: {
                    status: response.status,
                },
            });
        }
    }
}

// 将导出的函数挂载到 globalThis，以便引擎调用
// （ModuleLoader 执行后，导出会在模块作用域，需要手动暴露到全局）
(globalThis as any).scan_request = scan_request;
(globalThis as any).scan_response = scan_response;
