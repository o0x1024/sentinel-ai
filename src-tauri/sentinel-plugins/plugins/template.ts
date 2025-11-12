/**
 * Sentinel AI 被动扫描插件模板
 * 
 * 使用方法：
 * 1. 复制此文件作为新插件的起点
 * 2. 修改 get_metadata() 返回的元数据
 * 3. 实现 scan_request() 和 scan_response() 逻辑
 * 4. 将文件放入插件目录：~/.sentinel-ai/plugins/
 * 
 * 注意：
 * - 插件在 Deno Core 环境中运行，支持所有 ES2022 特性
 * - 不支持 Node.js 特定 API（如 fs、process）
 * - 使用 Deno.core.ops.op_emit_finding() 报告漏洞（不要 return）
 * - 使用 Deno.core.ops.op_plugin_log() 输出日志
 */

// ============================================================
// 类型定义（从 plugin-types.d.ts） 
// ============================================================

interface PluginMetadata {
  id: string;
  name: string;
  version: string;
  author?: string;
  category: string;
  default_severity: "critical" | "high" | "medium" | "low" | "info";
  tags?: string[];
  description?: string;
}

interface RequestContext {
  id: string;
  method: string;
  url: string;
  headers: Record<string, string>;
  body: Uint8Array;
  content_type?: string;
  query_params: Record<string, string>;
  is_https: boolean;
  timestamp: string;
}

interface ResponseContext {
  request_id: string;
  status: number;
  headers: Record<string, string>;
  body: Uint8Array;
  content_type?: string;
  timestamp: string;
}

interface CombinedContext {
  request: RequestContext;
  response: ResponseContext;
}

// 用于 op_emit_finding 的简化结构（无需提供所有字段）
interface Finding {
  vuln_type: string;
  severity: "critical" | "high" | "medium" | "low" | "info";
  confidence: "high" | "medium" | "low";
  url: string;
  method?: string;
  param_name?: string;
  param_value?: string;
  evidence?: string;
  description?: string;
}

// 声明 Deno ops API
declare namespace Deno {
  namespace core {
    namespace ops {
      function op_emit_finding(finding: Finding): boolean;
      function op_plugin_log(level: string, message: string): void;
    }
  }
}

// ============================================================
// 插件元数据（必须实现）
// ============================================================

function get_metadata(): PluginMetadata {
  return {
    id: "example.template", // 建议格式：作者.类别.名称
    name: "Template Plugin",
    version: "1.0.0",
    author: "Sentinel AI",
    category: "example",
    default_severity: "medium",
    tags: ["demo", "template"],
    description: "This is a template plugin for demonstration purposes.",
  };
}

// ============================================================
// 初始化函数（可选）
// ============================================================

function init(config?: Record<string, any>): void {
  // 可选：执行初始化逻辑
  // 例如：加载配置、编译正则表达式、建立连接等
  Deno.core.ops.op_plugin_log("info", "Template plugin initialized");
}

// ============================================================
// 扫描请求（必须实现）
// ============================================================

function scan_request(ctx: RequestContext): void {
  // 示例 1: 检测 URL 中的敏感关键词
  const sensitiveKeywords = ["admin", "debug", "test", "backup"];
  for (const keyword of sensitiveKeywords) {
    if (ctx.url.toLowerCase().includes(keyword)) {
      Deno.core.ops.op_emit_finding({
        vuln_type: "sensitive-path",
        severity: "low",
        confidence: "medium",
        url: ctx.url,
        method: ctx.method,
        evidence: ctx.url,
        description: `The URL contains a potentially sensitive keyword: "${keyword}".`,
      });
    }
  }

  // 示例 2: 检测查询参数中的可疑值
  for (const [key, value] of Object.entries(ctx.query_params)) {
    // SQL 注入符号
    if (/['";]|--|\bOR\b|\bUNION\b/i.test(value)) {
      Deno.core.ops.op_emit_finding({
        vuln_type: "sqli",
        severity: "high",
        confidence: "medium",
        url: ctx.url,
        method: ctx.method,
        param_name: key,
        param_value: truncate(value, 100),
        evidence: `SQL injection pattern detected`,
        description: `The parameter "${key}" contains characters/keywords commonly used in SQL injection attacks.`,
      });
    }

    // XSS 符号
    if (/<script|onerror|onclick|javascript:/i.test(value)) {
      Deno.core.ops.op_emit_finding({
        vuln_type: "xss",
        severity: "high",
        confidence: "medium",
        url: ctx.url,
        method: ctx.method,
        param_name: key,
        param_value: truncate(value, 100),
        evidence: `XSS pattern detected`,
        description: `The parameter "${key}" contains HTML/JavaScript code.`,
      });
    }
  }
}

// ============================================================
// 扫描响应（必须实现）
// ============================================================

function scan_response(ctx: CombinedContext): void {
  const responseBody = decodeBody(ctx.response.body);

  // 示例 1: 检测数据库错误信息（SQL 注入确认）
  const dbErrors = [
    { pattern: /mysql_fetch/i, db: "MySQL" },
    { pattern: /You have an error in your SQL syntax/i, db: "MySQL" },
    { pattern: /ORA-\d{5}/i, db: "Oracle" },
    { pattern: /PostgreSQL.*ERROR/i, db: "PostgreSQL" },
    { pattern: /Microsoft SQL Server/i, db: "MSSQL" },
  ];

  for (const { pattern, db } of dbErrors) {
    const match = responseBody.match(pattern);
    if (match) {
      Deno.core.ops.op_emit_finding({
        vuln_type: "sqli",
        severity: "critical",
        confidence: "high",
        url: ctx.request.url,
        method: ctx.request.method,
        evidence: truncate(match[0], 200),
        description: `The response contains database error messages from ${db}, indicating a potential SQL injection vulnerability.`,
      });
    }
  }

  // 示例 2: 检测敏感信息泄露
  const sensitivePatterns = [
    {
      name: "JWT Token",
      pattern: /eyJ[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+/,
    },
    { name: "API Key", pattern: /(?:api[_-]?key|apikey)["\s:=]+([a-zA-Z0-9_-]{20,})/i },
    { name: "AWS Access Key", pattern: /AKIA[0-9A-Z]{16}/ },
    { name: "Private Key", pattern: /-----BEGIN (?:RSA |EC |DSA )?PRIVATE KEY-----/ },
  ];

  for (const { name, pattern } of sensitivePatterns) {
    const match = responseBody.match(pattern);
    if (match) {
      Deno.core.ops.op_emit_finding({
        vuln_type: "sensitive-info",
        severity: "high",
        confidence: "high",
        url: ctx.request.url,
        method: ctx.request.method,
        evidence: truncate(match[0], 100),
        description: `The response body contains ${name}, which should not be exposed.`,
      });
    }
  }

  // 示例 3: 检测缺失的安全响应头
  const securityHeaders = [
    { name: "Content-Security-Policy", severity: "medium" as const },
    { name: "X-Frame-Options", severity: "medium" as const },
    { name: "X-Content-Type-Options", severity: "low" as const },
    { name: "Strict-Transport-Security", severity: "medium" as const },
  ];

  for (const { name, severity } of securityHeaders) {
    if (!ctx.response.headers[name] && !ctx.response.headers[name.toLowerCase()]) {
      Deno.core.ops.op_emit_finding({
        vuln_type: "missing-security-header",
        severity,
        confidence: "high",
        url: ctx.request.url,
        method: ctx.request.method,
        evidence: `Header "${name}" not found in response`,
        description: `The response is missing the "${name}" header, which could expose users to attacks.`,
      });
    }
  }
}

// ============================================================
// 工具函数
// ============================================================

function decodeBody(body: Uint8Array): string {
  return new TextDecoder("utf-8").decode(body);
}

function truncate(str: string, maxLen: number = 200): string {
  return str.length > maxLen ? str.slice(0, maxLen) + "..." : str;
}

// ============================================================
// 导出（供 Deno Core 调用）
// ============================================================

// 在 Deno Core 环境中，将函数注册到全局作用域
globalThis.get_metadata = get_metadata;
globalThis.init = init;
globalThis.scan_request = scan_request;
globalThis.scan_response = scan_response;
