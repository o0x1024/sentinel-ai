/**
 * Sentinel AI 被动扫描插件类型定义
 * 
 * 插件接口规范：
 * 1. 导出 get_metadata() 函数返回插件元数据
 * 2. 导出 scan_request() 函数扫描 HTTP 请求
 * 3. 导出 scan_response() 函数扫描 HTTP 响应
 * 4. 可选：导出 init() 函数进行初始化
 */

// ============================================================
// 核心类型
// ============================================================

/**
 * 插件元数据
 */
export interface PluginMetadata {
  /** 插件 ID（唯一标识，建议格式：作者.类别.名称） */
  id: string;
  /** 插件名称 */
  name: string;
  /** 版本号（语义化版本） */
  version: string;
  /** 作者 */
  author?: string;
  /** 漏洞类别（sqli/xss/sensitive-info/csrf 等） */
  category: string;
  /** 默认严重等级 */
  default_severity: Severity;
  /** 标签（可用于筛选） */
  tags?: string[];
  /** 描述 */
  description?: string;
}

/**
 * 严重等级
 */
export type Severity = "critical" | "high" | "medium" | "low" | "info";

/**
 * 置信度
 */
export type Confidence = "high" | "medium" | "low";

// ============================================================
// 请求/响应上下文
// ============================================================

/**
 * HTTP 请求上下文
 */
export interface RequestContext {
  /** 请求 ID（UUID） */
  id: string;
  /** HTTP 方法 */
  method: string;
  /** 完整 URL */
  url: string;
  /** 请求头（键值对） */
  headers: Record<string, string>;
  /** 请求体（Uint8Array，最多 2MB） */
  body: Uint8Array;
  /** Content-Type */
  content_type?: string;
  /** 查询参数 */
  query_params: Record<string, string>;
  /** 是否 HTTPS */
  is_https: boolean;
  /** 时间戳（ISO 8601 字符串） */
  timestamp: string;
}

/**
 * HTTP 响应上下文
 */
export interface ResponseContext {
  /** 关联的请求 ID */
  request_id: string;
  /** HTTP 状态码 */
  status: number;
  /** 响应头 */
  headers: Record<string, string>;
  /** 响应体（Uint8Array，最多 2MB） */
  body: Uint8Array;
  /** Content-Type */
  content_type?: string;
  /** 时间戳（ISO 8601 字符串） */
  timestamp: string;
}

/**
 * 组合上下文（用于 scan_response）
 */
export interface CombinedContext {
  /** 请求上下文 */
  request: RequestContext;
  /** 响应上下文 */
  response: ResponseContext;
}

// ============================================================
// 漏洞发现（Finding）
// ============================================================

/**
 * 漏洞发现输出
 */
export interface Finding {
  /** 漏洞类型（与 category 对应） */
  vuln_type: string;
  /** 严重等级 */
  severity: Severity;
  /** 标题（简短描述） */
  title: string;
  /** 详细描述 */
  description: string;
  /** 证据片段（不脱敏，原始数据） */
  evidence: string;
  /** 位置（param:xxx / header:xxx / body / response:body 等） */
  location: string;
  /** 置信度 */
  confidence: Confidence;
  /** CWE 标签（可选，如 CWE-89） */
  cwe?: string;
  /** OWASP Top 10 标签（可选，如 A03:2021） */
  owasp?: string;
  /** 修复建议 */
  remediation?: string;
}

// ============================================================
// 插件接口（必须实现）
// ============================================================

/**
 * 获取插件元数据（必须实现）
 * 
 * 该函数在插件加载时调用一次，用于注册插件信息
 */
export function get_metadata(): PluginMetadata;

/**
 * 扫描 HTTP 请求（必须实现）
 * 
 * @param ctx 请求上下文
 * @returns Finding 数组（没有发现返回空数组）
 * 
 * 使用场景：
 * - 检测请求参数中的注入符号（SQL 注入、XSS）
 * - 检测敏感参数（密码明文传输）
 * - 检测危险 HTTP 方法（PUT/DELETE）
 */
export function scan_request(ctx: RequestContext): Finding[];

/**
 * 扫描 HTTP 响应（必须实现）
 * 
 * @param ctx 组合上下文（包含请求和响应）
 * @returns Finding 数组（没有发现返回空数组）
 * 
 * 使用场景：
 * - 响应中的数据库错误信息（SQL 注入确认）
 * - 反射型 XSS（参数 → 响应体）
 * - 敏感信息泄露（JWT、API Key、证书）
 * - 安全头缺失（CSP、X-Frame-Options）
 */
export function scan_response(ctx: CombinedContext): Finding[];

/**
 * 初始化插件（可选实现）
 * 
 * @param config 配置对象（可选）
 * 
 * 使用场景：
 * - 加载自定义规则文件
 * - 初始化正则表达式缓存
 * - 建立外部服务连接（如威胁情报库）
 */
export function init(config?: Record<string, any>): void;

// ============================================================
// 工具函数（类型声明，实现见 plugin-utils.ts）
// ============================================================

/**
 * 解码请求体为字符串（UTF-8）
 */
export function decodeBody(body: Uint8Array): string;

/**
 * 解析 JSON 请求/响应体
 */
export function parseJSON(body: Uint8Array): any;

/**
 * 解析表单数据（application/x-www-form-urlencoded）
 */
export function parseFormData(body: Uint8Array): Record<string, string>;

/**
 * 检测反射点（参数 → 响应体）
 */
export function hasReflection(paramValue: string, responseBody: string): boolean;

/**
 * 截断字符串（用于证据展示）
 */
export function truncate(str: string, maxLen?: number): string;
