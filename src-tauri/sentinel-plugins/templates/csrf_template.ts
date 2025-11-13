// CSRF (Cross-Site Request Forgery) Detection Template
// Auto-generated plugin template for CSRF vulnerability detection

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
  // State-changing endpoints (e.g., ["/api/transfer", "/api/delete"])
  stateChangingEndpoints: [{{STATE_CHANGING_ENDPOINTS}}],
  // Detection sensitivity
  sensitivity: "{{SENSITIVITY}}",
  // CSRF token parameter names
  csrfTokenNames: [
    'csrf_token',
    'csrftoken',
    '_csrf',
    'csrf',
    'token',
    'authenticity_token',
    '_token',
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
    category: "csrf",
    description: "{{DESCRIPTION}}",
    default_severity: "medium",
    tags: ["csrf", "cross-site-request-forgery", "{{TARGET_URL}}"],
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

function isStateChangingMethod(method: string): boolean {
  return ['POST', 'PUT', 'DELETE', 'PATCH'].includes(method.toUpperCase());
}

function isStateChangingEndpoint(url: string): boolean {
  if (CONFIG.stateChangingEndpoints.length === 0) {
    // Default patterns for state-changing endpoints
    const patterns = [
      /\/(?:api|v\d+)\/(?:create|update|delete|transfer|withdraw|deposit|change|modify)/i,
      /\/(?:user|account|profile)\/(?:delete|update)/i,
      /\/(?:order|payment|transaction)/i,
    ];
    return patterns.some(p => p.test(url));
  }
  return CONFIG.stateChangingEndpoints.some(endpoint => url.includes(endpoint));
}

function hasCsrfToken(params: Record<string, string>, body: string): boolean {
  // Check in parameters
  for (const tokenName of CONFIG.csrfTokenNames) {
    if (params[tokenName]) return true;
  }
  
  // Check in body
  for (const tokenName of CONFIG.csrfTokenNames) {
    const regex = new RegExp(`["']?${tokenName}["']?\\s*[:=]\\s*["']?[^"'\\s]+["']?`, 'i');
    if (regex.test(body)) return true;
  }
  
  return false;
}

function hasCsrfHeader(headers: Record<string, string>): boolean {
  const csrfHeaders = [
    'X-CSRF-Token',
    'X-XSRF-Token',
    'X-CSRFToken',
  ];
  
  for (const header of csrfHeaders) {
    if (headers[header] || headers[header.toLowerCase()]) {
      return true;
    }
  }
  
  return false;
}

function hasRefererCheck(headers: Record<string, string>): boolean {
  return !!(headers['Referer'] || headers['referer']);
}

function hasSameSiteCookie(cookies: string): boolean {
  return /SameSite\s*=\s*(Strict|Lax)/i.test(cookies);
}

// ============================================================
// Scan Request
// ============================================================

function scan_request(ctx: RequestContext): void {
  // Only check state-changing methods
  if (!isStateChangingMethod(ctx.method)) return;

  // Only check if it's a state-changing endpoint
  if (!isStateChangingEndpoint(ctx.url)) return;

  const body = decodeBody(ctx.body);
  const hasCsrf = hasCsrfToken(ctx.query_params, body) || hasCsrfHeader(ctx.headers);
  const hasReferer = hasRefererCheck(ctx.headers);

  // Check for missing CSRF protection
  if (!hasCsrf) {
    let confidence = "medium";
    let description = `State-changing endpoint "${ctx.url}" does not contain a CSRF token. This may allow attackers to perform unauthorized actions on behalf of authenticated users.`;

    // If also missing Referer, higher confidence
    if (!hasReferer) {
      confidence = "high";
      description += " Additionally, no Referer header is being checked, making CSRF attacks easier.";
    }

    Deno.core.ops.op_emit_finding({
      vuln_type: "csrf",
      severity: "medium",
      confidence,
      url: ctx.url,
      method: ctx.method,
      evidence: "No CSRF token detected",
      description,
      cwe: "CWE-352",
      owasp: "A01:2021",
      remediation: "Implement CSRF tokens (synchronizer token pattern) or use double-submit cookie pattern. Consider using SameSite cookie attribute.",
    });
  }

  // Check for weak CSRF protection (GET method for state changes)
  if (ctx.method === "GET" && isStateChangingEndpoint(ctx.url)) {
    Deno.core.ops.op_emit_finding({
      vuln_type: "csrf",
      severity: "high",
      confidence: "high",
      url: ctx.url,
      method: ctx.method,
      evidence: "State-changing action via GET method",
      description: `Endpoint "${ctx.url}" performs state-changing operations via GET method. This is highly vulnerable to CSRF attacks as GET requests can be triggered by simply visiting a malicious link.`,
      cwe: "CWE-352",
      owasp: "A01:2021",
      remediation: "Use POST, PUT, or DELETE methods for all state-changing operations. Never allow state changes via GET requests.",
    });
  }
}

// ============================================================
// Scan Response
// ============================================================

function scan_response(ctx: CombinedContext): void {
  const responseBody = decodeBody(ctx.response.body);
  const contentType = ctx.response.content_type || '';

  // Only check state-changing operations
  if (!isStateChangingMethod(ctx.request.method)) return;
  if (!isStateChangingEndpoint(ctx.request.url)) return;

  // Check if request was successful without CSRF token
  if (ctx.response.status >= 200 && ctx.response.status < 300) {
    const body = decodeBody(ctx.request.body);
    const hasCsrf = hasCsrfToken(ctx.request.query_params, body) || hasCsrfHeader(ctx.request.headers);

    if (!hasCsrf) {
      Deno.core.ops.op_emit_finding({
        vuln_type: "csrf",
        severity: "high",
        confidence: "high",
        url: ctx.request.url,
        method: ctx.request.method,
        evidence: `Request succeeded (${ctx.response.status}) without CSRF token`,
        description: `State-changing request to "${ctx.request.url}" completed successfully without CSRF protection. This confirms the CSRF vulnerability.`,
        cwe: "CWE-352",
        owasp: "A01:2021",
        remediation: "Implement CSRF tokens for all state-changing operations. Reject requests without valid CSRF tokens.",
      });
    }
  }

  // Check Set-Cookie headers for SameSite attribute
  const setCookie = ctx.response.headers['Set-Cookie'] || ctx.response.headers['set-cookie'];
  if (setCookie && !hasSameSiteCookie(setCookie)) {
    // Check if it's a session cookie
    if (/session|sess|token|auth/i.test(setCookie)) {
      Deno.core.ops.op_emit_finding({
        vuln_type: "csrf",
        severity: "medium",
        confidence: "high",
        url: ctx.request.url,
        method: ctx.request.method,
        evidence: "Session cookie without SameSite attribute",
        description: `Session cookie is set without SameSite attribute. Modern browsers' default SameSite=Lax provides some protection, but explicit SameSite=Strict or Lax is recommended for better CSRF protection.`,
        cwe: "CWE-352",
        owasp: "A01:2021",
        remediation: "Add SameSite=Strict or SameSite=Lax attribute to all session cookies.",
      });
    }
  }

  // Check for CSRF token in HTML forms (if HTML response)
  if (contentType.includes('text/html')) {
    const hasForm = /<form[^>]*>/i.test(responseBody);
    if (hasForm) {
      // Check if forms have CSRF tokens
      const formWithToken = CONFIG.csrfTokenNames.some(tokenName => {
        const regex = new RegExp(`<input[^>]*name=["']${tokenName}["'][^>]*>`, 'i');
        return regex.test(responseBody);
      });

      if (!formWithToken) {
        Deno.core.ops.op_emit_finding({
          vuln_type: "csrf",
          severity: "medium",
          confidence: "medium",
          url: ctx.request.url,
          method: ctx.request.method,
          evidence: "HTML form without CSRF token",
          description: `Response contains HTML forms without CSRF tokens. Forms that perform state-changing operations should include CSRF protection.`,
          cwe: "CWE-352",
          owasp: "A01:2021",
          remediation: "Add hidden CSRF token fields to all forms. Generate unique tokens per session and validate them on the server.",
        });
      }
    }
  }

  // Check for CORS misconfigurations that could aid CSRF
  const corsOrigin = ctx.response.headers['Access-Control-Allow-Origin'] || ctx.response.headers['access-control-allow-origin'];
  const corsCredentials = ctx.response.headers['Access-Control-Allow-Credentials'] || ctx.response.headers['access-control-allow-credentials'];
  
  if (corsOrigin === '*' && corsCredentials === 'true') {
    Deno.core.ops.op_emit_finding({
      vuln_type: "csrf",
      severity: "high",
      confidence: "high",
      url: ctx.request.url,
      method: ctx.request.method,
      evidence: "CORS: Access-Control-Allow-Origin: * with credentials",
      description: `Dangerous CORS configuration detected. Allowing all origins with credentials enabled can facilitate CSRF attacks.`,
      cwe: "CWE-352",
      owasp: "A01:2021",
      remediation: "Do not use wildcard (*) for Access-Control-Allow-Origin when Access-Control-Allow-Credentials is true. Specify exact origins or implement proper CORS validation.",
    });
  }
}

// Required initialization
function init(config?: Record<string, any>): void {
  if (config) {
    Object.assign(CONFIG, config);
  }
}

