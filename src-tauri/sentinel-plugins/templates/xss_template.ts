// XSS (Cross-Site Scripting) Detection Template
// Auto-generated plugin template for XSS detection

interface PluginMetadata {
  id: string;
  name: string;
  version: string;
  author?: string;
  main_category: string;
  category: string;
  description?: string;
  default_severity: string;
  tags: string[];
}

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

interface CombinedContext {
  request: RequestContext;
  response: ResponseContext;
}

// ============================================================
// Plugin Configuration
// ============================================================

const CONFIG = {
  // Target parameter names (e.g., ["comment", "message", "content"])
  targetParams: [{{TARGET_PARAMS}}],
  // XSS detection sensitivity
  sensitivity: "{{SENSITIVITY}}",
  // Dangerous HTML tags and attributes
  dangerousTags: [
    /<script[^>]*>.*?<\/script>/gi,
    /<iframe[^>]*>/gi,
    /<object[^>]*>/gi,
    /<embed[^>]*>/gi,
    /<img[^>]*onerror[^>]*>/gi,
    /<svg[^>]*onload[^>]*>/gi,
  ],
  dangerousPatterns: [
    /javascript:/i,
    /onerror\s*=/i,
    /onclick\s*=/i,
    /onload\s*=/i,
    /onmouseover\s*=/i,
    /<script/i,
    /eval\s*\(/i,
    /alert\s*\(/i,
  ],
};

// ============================================================
// Plugin Metadata
// ============================================================

function get_metadata(): PluginMetadata {
  return {
    id: "{{PLUGIN_ID}}",
    name: "{{PLUGIN_NAME}}",
    version: "1.0.0",
    author: "AI Generated",
    main_category: "passive",
    category: "xss",
    description: "{{DESCRIPTION}}",
    default_severity: "high",
    tags: ["xss", "cross-site-scripting", "injection", "{{TARGET_URL}}"],
  };
}

// ============================================================
// Helper Functions
// ============================================================

function decodeBody(body: number[]): string {
  try {
    return new TextDecoder().decode(new Uint8Array(body));
  } catch {
    return "";
  }
}

function truncate(str: string, maxLen: number): string {
  return str.length > maxLen ? str.slice(0, maxLen) + "..." : str;
}

function isTargetParam(paramName: string): boolean {
  if (CONFIG.targetParams.length === 0) return true;
  return CONFIG.targetParams.some(p => paramName.toLowerCase().includes(p.toLowerCase()));
}

function decodeHtmlEntities(str: string): string {
  return str
    .replace(/&lt;/g, '<')
    .replace(/&gt;/g, '>')
    .replace(/&quot;/g, '"')
    .replace(/&#x27;/g, "'")
    .replace(/&amp;/g, '&');
}

// ============================================================
// Scan Request
// ============================================================

function scan_request(ctx: RequestContext): void {
  // Check URL parameters
  for (const [key, value] of Object.entries(ctx.query_params)) {
    if (!isTargetParam(key)) continue;

    // Check dangerous patterns
    for (const pattern of CONFIG.dangerousPatterns) {
      if (pattern.test(value)) {
        Deno.core.ops.op_emit_finding({
          vuln_type: "xss",
          severity: "high",
          confidence: "medium",
          url: ctx.url,
          method: ctx.method,
          param_name: key,
          param_value: truncate(value, 100),
          evidence: `XSS pattern detected in parameter "${key}"`,
          description: `The parameter "${key}" contains XSS payloads. This could allow attackers to execute malicious JavaScript.`,
          cwe: "CWE-79",
          owasp: "A03:2021",
          remediation: "Encode all user input before rendering in HTML. Use Content Security Policy (CSP) headers.",
        });
      }
    }
  }

  // Check request body
  if (ctx.method === "POST" || ctx.method === "PUT") {
    const bodyStr = decodeBody(ctx.body);
    if (bodyStr) {
      for (const pattern of CONFIG.dangerousPatterns) {
        if (pattern.test(bodyStr)) {
          Deno.core.ops.op_emit_finding({
            vuln_type: "xss",
            severity: "high",
            confidence: "medium",
            url: ctx.url,
            method: ctx.method,
            evidence: truncate(bodyStr, 200),
            description: `XSS patterns detected in request body.`,
            cwe: "CWE-79",
            owasp: "A03:2021",
            remediation: "Encode all user input before rendering. Implement CSP headers.",
          });
        }
      }
    }
  }
}

// ============================================================
// Scan Response
// ============================================================

function scan_response(ctx: CombinedContext): void {
  const responseBody = decodeBody(ctx.response.body);
  const contentType = ctx.response.content_type || '';

  // Only check HTML responses
  if (!contentType.includes('text/html')) return;

  // Check for reflected XSS
  for (const [key, value] of Object.entries(ctx.request.query_params)) {
    if (!isTargetParam(key)) continue;

    // Check if payload is reflected in response
    if (CONFIG.dangerousPatterns.some(p => p.test(value))) {
      const decodedValue = decodeHtmlEntities(value);
      
      // Check both original and decoded values
      if (responseBody.includes(value) || responseBody.includes(decodedValue)) {
        // Check if it's properly encoded
        const isEncoded = responseBody.includes(value.replace(/</g, '&lt;').replace(/>/g, '&gt;'));
        
        if (!isEncoded) {
          Deno.core.ops.op_emit_finding({
            vuln_type: "xss",
            severity: "high",
            confidence: "high",
            url: ctx.request.url,
            method: ctx.request.method,
            param_name: key,
            param_value: truncate(value, 100),
            evidence: `Reflected XSS: payload reflected without proper encoding`,
            description: `Parameter "${key}" with XSS payload is reflected in the response without proper HTML encoding. This is a confirmed reflected XSS vulnerability.`,
            cwe: "CWE-79",
            owasp: "A03:2021",
            remediation: "Encode all user input using context-appropriate encoding (HTML entity encoding for HTML context).",
          });
        }
      }
    }
  }

  // Check for dangerous tags in response
  for (const tagPattern of CONFIG.dangerousTags) {
    const matches = responseBody.match(tagPattern);
    if (matches) {
      Deno.core.ops.op_emit_finding({
        vuln_type: "xss",
        severity: "medium",
        confidence: "low",
        url: ctx.request.url,
        method: ctx.request.method,
        evidence: truncate(matches[0], 200),
        description: `Potentially dangerous HTML tag found in response. May indicate stored XSS vulnerability.`,
        cwe: "CWE-79",
        owasp: "A03:2021",
        remediation: "Review the source of this content and ensure all user-generated content is properly sanitized.",
      });
    }
  }
}

// Required initialization
function init(config?: Record<string, any>): void {
  if (config) {
    Object.assign(CONFIG, config);
  }
}

