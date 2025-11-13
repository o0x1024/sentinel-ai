// SQL Injection Detection Template
// Auto-generated plugin template for SQL injection detection

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

interface Finding {
  vuln_type: string;
  severity: string;
  confidence: string;
  url: string;
  method: string;
  param_name?: string;
  param_value?: string;
  evidence: string;
  description: string;
  cwe?: string;
  owasp?: string;
  remediation?: string;
}

// ============================================================
// Plugin Configuration (can be customized)
// ============================================================

const CONFIG = {
  // Target parameter names (e.g., ["id", "user", "search"])
  targetParams: [{{TARGET_PARAMS}}],
  // Database type hints (mysql, postgresql, mssql, oracle)
  dbType: "{{DB_TYPE}}",
  // Detection sensitivity (low, medium, high)
  sensitivity: "{{SENSITIVITY}}",
  // Common SQL injection patterns
  sqlPatterns: [
    /['";]|--|\bOR\b|\bAND\b|\bUNION\b|\bSELECT\b|\bINSERT\b|\bDELETE\b|\bUPDATE\b|\bDROP\b/i,
    /\/\*.*?\*\//,
    /\bEXEC\b|\bEXECUTE\b|\bxp_cmdshell\b/i,
    /\bWAIT\s+FOR\s+DELAY\b|\bSLEEP\b|\bBENCHMARK\b/i,
  ],
  // Database error patterns
  errorPatterns: [
    /SQL syntax.*?MySQL/i,
    /Warning.*?\Wmysqli?/i,
    /PostgreSQL.*?ERROR/i,
    /Warning.*?\Wpg_/i,
    /Microsoft SQL Native Client error/i,
    /ODBC SQL Server Driver/i,
    /Oracle error/i,
    /ORA-[0-9]{5}/i,
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
    category: "sqli",
    description: "{{DESCRIPTION}}",
    default_severity: "critical",
    tags: ["sql", "injection", "database", "{{TARGET_URL}}"],
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

// ============================================================
// Scan Request
// ============================================================

function scan_request(ctx: RequestContext): void {
  // Check URL parameters
  for (const [key, value] of Object.entries(ctx.query_params)) {
    if (!isTargetParam(key)) continue;

    // Check SQL injection patterns
    for (const pattern of CONFIG.sqlPatterns) {
      if (pattern.test(value)) {
        Deno.core.ops.op_emit_finding({
          vuln_type: "sqli",
          severity: "critical",
          confidence: "medium",
          url: ctx.url,
          method: ctx.method,
          param_name: key,
          param_value: truncate(value, 100),
          evidence: `SQL injection pattern detected in parameter "${key}"`,
          description: `The parameter "${key}" contains SQL injection patterns. This could allow attackers to manipulate database queries.`,
          cwe: "CWE-89",
          owasp: "A03:2021",
          remediation: "Use parameterized queries or prepared statements. Never concatenate user input directly into SQL queries.",
        });
      }
    }
  }

  // Check request body (if POST/PUT)
  if (ctx.method === "POST" || ctx.method === "PUT") {
    const bodyStr = decodeBody(ctx.body);
    if (bodyStr) {
      for (const pattern of CONFIG.sqlPatterns) {
        if (pattern.test(bodyStr)) {
          Deno.core.ops.op_emit_finding({
            vuln_type: "sqli",
            severity: "critical",
            confidence: "medium",
            url: ctx.url,
            method: ctx.method,
            evidence: truncate(bodyStr, 200),
            description: `SQL injection patterns detected in request body.`,
            cwe: "CWE-89",
            owasp: "A03:2021",
            remediation: "Use parameterized queries or prepared statements.",
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

  // Check for database error messages
  for (const pattern of CONFIG.errorPatterns) {
    if (pattern.test(responseBody)) {
      Deno.core.ops.op_emit_finding({
        vuln_type: "sqli",
        severity: "high",
        confidence: "high",
        url: ctx.request.url,
        method: ctx.request.method,
        evidence: truncate(responseBody, 200),
        description: `Database error message detected in response. This confirms SQL injection vulnerability.`,
        cwe: "CWE-89",
        owasp: "A03:2021",
        remediation: "Fix the SQL injection vulnerability and disable detailed error messages in production.",
      });
      break;
    }
  }

  // Check for unusual query parameters in reflected values
  for (const [key, value] of Object.entries(ctx.request.query_params)) {
    if (!isTargetParam(key)) continue;

    if (CONFIG.sqlPatterns.some(p => p.test(value)) && responseBody.includes(value)) {
      Deno.core.ops.op_emit_finding({
        vuln_type: "sqli",
        severity: "high",
        confidence: "medium",
        url: ctx.request.url,
        method: ctx.request.method,
        param_name: key,
        param_value: truncate(value, 100),
        evidence: `SQL injection payload reflected in response`,
        description: `Parameter "${key}" with SQL injection patterns is reflected in the response, indicating potential blind SQL injection.`,
        cwe: "CWE-89",
        owasp: "A03:2021",
        remediation: "Use parameterized queries and validate/sanitize all user inputs.",
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

