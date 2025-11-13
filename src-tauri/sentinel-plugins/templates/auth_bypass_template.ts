// Authorization Bypass / IDOR Detection Template
// Auto-generated plugin template for access control vulnerabilities

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
  // ID parameter names (e.g., ["id", "user_id", "account_id"])
  idParams: [{{ID_PARAMS}}],
  // Sensitive paths that require authorization
  sensitivePaths: [{{SENSITIVE_PATHS}}],
  // Detection sensitivity
  sensitivity: "{{SENSITIVITY}}",
  // Track accessed resources
  accessedResources: new Map<string, Set<string>>(),
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
    category: "access_control",
    description: "{{DESCRIPTION}}",
    default_severity: "high",
    tags: ["idor", "auth", "access-control", "{{TARGET_URL}}"],
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

function isIdParam(paramName: string): boolean {
  if (CONFIG.idParams.length === 0) {
    // Default ID parameter patterns
    return /^(id|user_id|account_id|order_id|profile_id|uid)$/i.test(paramName);
  }
  return CONFIG.idParams.some(p => paramName.toLowerCase() === p.toLowerCase());
}

function isSensitivePath(url: string): boolean {
  if (CONFIG.sensitivePaths.length === 0) {
    // Default sensitive path patterns
    return /\/(admin|user|profile|account|order|private|dashboard)/i.test(url);
  }
  return CONFIG.sensitivePaths.some(path => url.includes(path));
}

function extractUserId(ctx: RequestContext): string | null {
  // Try to extract user ID from headers
  const authHeader = ctx.headers['Authorization'] || ctx.headers['authorization'];
  if (authHeader) {
    // Extract from Bearer token (simplified)
    return authHeader.substring(0, 20);
  }
  
  // Try cookies
  const cookie = ctx.headers['Cookie'] || ctx.headers['cookie'];
  if (cookie) {
    return cookie.substring(0, 20);
  }
  
  return null;
}

function trackAccess(userId: string | null, resourceId: string): void {
  if (!userId) return;
  
  if (!CONFIG.accessedResources.has(userId)) {
    CONFIG.accessedResources.set(userId, new Set());
  }
  
  CONFIG.accessedResources.get(userId)!.add(resourceId);
}

function checkIDOR(userId: string | null, resourceId: string): boolean {
  // If no user ID, can't determine IDOR
  if (!userId) return false;
  
  // Check if this user has accessed other resources
  const userResources = CONFIG.accessedResources.get(userId);
  if (!userResources) return false;
  
  // If accessing a different resource, potential IDOR
  return !userResources.has(resourceId);
}

// ============================================================
// Scan Request
// ============================================================

function scan_request(ctx: RequestContext): void {
  // Check for ID parameters in URL
  for (const [key, value] of Object.entries(ctx.query_params)) {
    if (isIdParam(key)) {
      // Check if accessing sensitive path with ID parameter
      if (isSensitivePath(ctx.url)) {
        const userId = extractUserId(ctx);
        
        // Track this access
        trackAccess(userId, value);
        
        Deno.core.ops.op_emit_finding({
          vuln_type: "idor",
          severity: "high",
          confidence: "medium",
          url: ctx.url,
          method: ctx.method,
          param_name: key,
          param_value: value,
          evidence: `ID parameter "${key}" in sensitive endpoint`,
          description: `The endpoint contains an ID parameter "${key}" which may be vulnerable to IDOR (Insecure Direct Object Reference) attacks. Verify that proper access control is enforced.`,
          cwe: "CWE-639",
          owasp: "A01:2021",
          remediation: "Implement proper authorization checks. Verify that the authenticated user has permission to access the requested resource.",
        });
      }
    }
  }

  // Check for missing authorization headers on sensitive paths
  if (isSensitivePath(ctx.url)) {
    const hasAuth = ctx.headers['Authorization'] || ctx.headers['authorization'];
    if (!hasAuth) {
      Deno.core.ops.op_emit_finding({
        vuln_type: "missing_auth",
        severity: "medium",
        confidence: "low",
        url: ctx.url,
        method: ctx.method,
        evidence: "No Authorization header present",
        description: `Accessing sensitive endpoint "${ctx.url}" without Authorization header. Verify that authentication is properly enforced.`,
        cwe: "CWE-306",
        owasp: "A01:2021",
        remediation: "Enforce authentication on all sensitive endpoints.",
      });
    }
  }
}

// ============================================================
// Scan Response
// ============================================================

function scan_response(ctx: CombinedContext): void {
  const responseBody = decodeBody(ctx.response.body);
  
  // Check if sensitive path returned successful response
  if (isSensitivePath(ctx.request.url) && ctx.response.status === 200) {
    const hasAuth = ctx.request.headers['Authorization'] || ctx.request.headers['authorization'];
    
    if (!hasAuth) {
      Deno.core.ops.op_emit_finding({
        vuln_type: "auth_bypass",
        severity: "critical",
        confidence: "high",
        url: ctx.request.url,
        method: ctx.request.method,
        evidence: `Status 200 returned for sensitive endpoint without authentication`,
        description: `Sensitive endpoint "${ctx.request.url}" returned successful response (200) without authentication. This indicates a critical authorization bypass vulnerability.`,
        cwe: "CWE-306",
        owasp: "A01:2021",
        remediation: "Implement proper authentication checks. Reject unauthorized requests with 401 or 403 status codes.",
      });
    }
  }

  // Check for exposed sensitive data
  const sensitivePatterns = [
    /password["']?\s*:\s*["'][^"']+["']/gi,
    /api_key["']?\s*:\s*["'][^"']+["']/gi,
    /secret["']?\s*:\s*["'][^"']+["']/gi,
    /token["']?\s*:\s*["'][^"']+["']/gi,
    /ssn["']?\s*:\s*["'][0-9-]+["']/gi,
    /credit_card["']?\s*:\s*["'][0-9-]+["']/gi,
  ];

  for (const pattern of sensitivePatterns) {
    const matches = responseBody.match(pattern);
    if (matches) {
      Deno.core.ops.op_emit_finding({
        vuln_type: "data_exposure",
        severity: "high",
        confidence: "medium",
        url: ctx.request.url,
        method: ctx.request.method,
        evidence: truncate(matches[0], 100),
        description: `Sensitive data pattern found in response. May indicate information disclosure or insufficient access control.`,
        cwe: "CWE-200",
        owasp: "A01:2021",
        remediation: "Review access control logic. Ensure sensitive data is only returned to authorized users.",
      });
    }
  }

  // Check for potential IDOR success
  for (const [key, value] of Object.entries(ctx.request.query_params)) {
    if (isIdParam(key)) {
      const userId = extractUserId(ctx.request);
      
      // If we've seen this user access different IDs successfully, likely IDOR
      if (checkIDOR(userId, value) && ctx.response.status === 200) {
        Deno.core.ops.op_emit_finding({
          vuln_type: "idor",
          severity: "critical",
          confidence: "high",
          url: ctx.request.url,
          method: ctx.request.method,
          param_name: key,
          param_value: value,
          evidence: `Successfully accessed resource with ID "${value}"`,
          description: `Confirmed IDOR vulnerability. User successfully accessed a resource ID that differs from previously accessed IDs. The application does not properly validate resource ownership.`,
          cwe: "CWE-639",
          owasp: "A01:2021",
          remediation: "Implement authorization checks to verify the user owns or has permission to access the requested resource.",
        });
      }
    }
  }
}

// Required initialization
function init(config?: Record<string, any>): void {
  if (config) {
    Object.assign(CONFIG, config);
  }
}

