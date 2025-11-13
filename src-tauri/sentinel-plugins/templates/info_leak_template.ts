// Information Leakage Detection Template
// Auto-generated plugin template for sensitive information disclosure

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
  // Sensitive keywords to detect
  sensitiveKeywords: [{{SENSITIVE_KEYWORDS}}],
  // Detection sensitivity
  sensitivity: "{{SENSITIVITY}}",
  // Patterns for various sensitive data types
  patterns: {
    // API Keys and Secrets
    apiKey: /['"]?(?:api[_-]?key|apikey|api_token|access_token)['"]?\s*[:=]\s*['"]([a-zA-Z0-9_\-]{20,})['"]/gi,
    awsKey: /AKIA[0-9A-Z]{16}/g,
    privateKey: /-----BEGIN (?:RSA )?PRIVATE KEY-----/gi,
    
    // Credentials
    password: /['"]?password['"]?\s*[:=]\s*['"]([^'"]{6,})['"]/gi,
    jwt: /eyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}/g,
    
    // Personal Information
    email: /[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}/g,
    ssn: /\b\d{3}-\d{2}-\d{4}\b/g,
    creditCard: /\b(?:\d{4}[-\s]?){3}\d{4}\b/g,
    phone: /\b\d{3}[-.]?\d{3}[-.]?\d{4}\b/g,
    
    // Internal Information
    internalIP: /\b(?:10|172\.(?:1[6-9]|2[0-9]|3[0-1])|192\.168)\.\d{1,3}\.\d{1,3}\b/g,
    stackTrace: /at\s+[\w.$]+\s*\([^)]+\)/gi,
    filePath: /(?:[A-Z]:\\|\/(?:home|usr|var|etc))[\w\\/.-]+/gi,
  },
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
    category: "info_disclosure",
    description: "{{DESCRIPTION}}",
    default_severity: "medium",
    tags: ["info-leak", "sensitive-data", "disclosure", "{{TARGET_URL}}"],
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

function maskSensitive(str: string): string {
  // Mask most of the sensitive value
  if (str.length <= 8) return "***";
  return str.substring(0, 4) + "***" + str.substring(str.length - 4);
}

// ============================================================
// Scan Request
// ============================================================

function scan_request(ctx: RequestContext): void {
  // Check for sensitive data in URL parameters
  for (const [key, value] of Object.entries(ctx.query_params)) {
    // Check for potential tokens/keys in URL
    if (/token|key|secret|password/i.test(key)) {
      Deno.core.ops.op_emit_finding({
        vuln_type: "sensitive_data_exposure",
        severity: "medium",
        confidence: "high",
        url: ctx.url,
        method: ctx.method,
        param_name: key,
        evidence: `Sensitive parameter "${key}" in URL`,
        description: `Parameter "${key}" appears to contain sensitive data in the URL. Sensitive data should not be transmitted via URL parameters as they are logged and visible in browser history.`,
        cwe: "CWE-598",
        owasp: "A04:2021",
        remediation: "Use POST method with encrypted body or secure headers for sensitive data transmission.",
      });
    }

    // Check for actual credential patterns
    if (CONFIG.patterns.password.test(value) || CONFIG.patterns.jwt.test(value)) {
      Deno.core.ops.op_emit_finding({
        vuln_type: "sensitive_data_exposure",
        severity: "high",
        confidence: "high",
        url: ctx.url,
        method: ctx.method,
        param_name: key,
        param_value: maskSensitive(value),
        evidence: `Credential detected in URL parameter`,
        description: `Actual credential or token detected in URL parameter "${key}". This is a critical security issue.`,
        cwe: "CWE-598",
        owasp: "A04:2021",
        remediation: "Never transmit credentials via URL. Use secure POST requests with proper encryption.",
      });
    }
  }
}

// ============================================================
// Scan Response
// ============================================================

function scan_response(ctx: CombinedContext): void {
  const responseBody = decodeBody(ctx.response.body);
  const responseHeaders = ctx.response.headers;

  // Check response headers for leaks
  const sensitiveHeaders = ['X-Powered-By', 'Server', 'X-AspNet-Version', 'X-AspNetMvc-Version'];
  for (const header of sensitiveHeaders) {
    if (responseHeaders[header] || responseHeaders[header.toLowerCase()]) {
      Deno.core.ops.op_emit_finding({
        vuln_type: "info_disclosure",
        severity: "low",
        confidence: "high",
        url: ctx.request.url,
        method: ctx.request.method,
        evidence: `Header: ${header}`,
        description: `Response contains "${header}" header which discloses server technology information.`,
        cwe: "CWE-200",
        owasp: "A05:2021",
        remediation: "Remove server identification headers to reduce attack surface.",
      });
    }
  }

  // Check for various sensitive patterns in response body
  for (const [patternName, pattern] of Object.entries(CONFIG.patterns)) {
    const matches = responseBody.match(pattern);
    if (matches && matches.length > 0) {
      let severity = "medium";
      let cwe = "CWE-200";
      
      // Adjust severity based on data type
      if (patternName === "apiKey" || patternName === "awsKey" || patternName === "privateKey" || 
          patternName === "password" || patternName === "jwt") {
        severity = "critical";
        cwe = "CWE-312";
      } else if (patternName === "ssn" || patternName === "creditCard") {
        severity = "critical";
        cwe = "CWE-359";
      } else if (patternName === "stackTrace" || patternName === "filePath") {
        severity = "medium";
        cwe = "CWE-209";
      }

      Deno.core.ops.op_emit_finding({
        vuln_type: "info_disclosure",
        severity,
        confidence: "high",
        url: ctx.request.url,
        method: ctx.request.method,
        evidence: truncate(maskSensitive(matches[0]), 100),
        description: `Sensitive ${patternName} pattern detected in response. This may expose confidential information to unauthorized parties.`,
        cwe,
        owasp: "A04:2021",
        remediation: "Review and sanitize all response data. Ensure sensitive information is never exposed in API responses or HTML.",
      });
    }
  }

  // Check for custom sensitive keywords
  for (const keyword of CONFIG.sensitiveKeywords) {
    const regex = new RegExp(keyword, 'gi');
    if (regex.test(responseBody)) {
      Deno.core.ops.op_emit_finding({
        vuln_type: "info_disclosure",
        severity: "medium",
        confidence: "medium",
        url: ctx.request.url,
        method: ctx.request.method,
        evidence: `Keyword: ${keyword}`,
        description: `Response contains sensitive keyword "${keyword}" which may indicate information leakage.`,
        cwe: "CWE-200",
        owasp: "A04:2021",
        remediation: "Review the response content and remove unnecessary sensitive information.",
      });
    }
  }

  // Check for debug/development information
  const debugPatterns = [
    /debug\s*[:=]\s*true/gi,
    /environment\s*[:=]\s*["'](?:dev|development|staging)["']/gi,
    /console\.log/gi,
    /var_dump/gi,
    /print_r/gi,
  ];

  for (const pattern of debugPatterns) {
    if (pattern.test(responseBody)) {
      Deno.core.ops.op_emit_finding({
        vuln_type: "debug_info",
        severity: "low",
        confidence: "high",
        url: ctx.request.url,
        method: ctx.request.method,
        evidence: truncate(responseBody.match(pattern)?.[0] || "", 100),
        description: `Debug or development information detected in production response.`,
        cwe: "CWE-489",
        owasp: "A05:2021",
        remediation: "Disable debug mode and remove development code in production environment.",
      });
      break;
    }
  }
}

// Required initialization
function init(config?: Record<string, any>): void {
  if (config) {
    Object.assign(CONFIG, config);
  }
}

