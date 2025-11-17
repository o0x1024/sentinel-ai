## Plugin Interface (Required Structure)

Your generated plugin MUST implement the following TypeScript interface:

```typescript
// Plugin metadata
function get_metadata(): PluginMetadata {
    return {
        id: "unique_plugin_id",
        name: "Plugin Name",
        version: "1.0.0",
        author: "AI Generated",
        main_category: "passive",
        category: "vuln_type", // e.g., "sqli", "xss", "file_upload", "command_injection", "path_traversal", "xxe", "ssrf", "idor", "auth_bypass", "info_leak", "csrf"
        description: "Detailed description",
        default_severity: "critical" | "high" | "medium" | "low",
        tags: ["tag1", "tag2"],
    };
}

// Analyze HTTP request (optional but recommended)
export function scan_request(ctx: RequestContext): void {
    // Analyze request parameters, headers, body
    // Emit findings if vulnerabilities detected
}

// Analyze HTTP response (required)
export function scan_response(ctx: CombinedContext): void {
    // Analyze request + response together
    // Check for vulnerability indicators in response
    // Emit findings if vulnerabilities detected
}

// **CRITICAL**: Export functions to globalThis for plugin engine to call
// Without these exports, the plugin will fail with "Function not found" error
// Use direct assignment without type casting to ensure proper execution
globalThis.get_metadata = get_metadata;
globalThis.scan_request = scan_request;
globalThis.scan_response = scan_response;

// Emit finding when vulnerability is detected
Deno.core.ops.op_emit_finding({
    vuln_type: "sqli",
    severity: "critical",
    confidence: "high",
    url: ctx.url,
    method: ctx.method,
    param_name: "paramName",
    param_value: "paramValue",
    evidence: "Evidence text",
    description: "Vulnerability description",
    cwe: "CWE-89",
    owasp: "A03:2021",
    remediation: "Fix suggestion",
});
```

**Context Objects**:
```typescript
interface RequestContext {
    request_id: string;
    url: string;
    method: string;
    headers: Record<string, string>;
    query_params: Record<string, string>;
    body: number[]; // UTF-8 bytes
    timestamp: string;
}

interface ResponseContext {
    request_id: string;
    status: number;
    headers: Record<string, string>;
    body: number[]; // UTF-8 bytes
    timestamp: string;
}

interface CombinedContext {
    request: RequestContext;
    response: ResponseContext;
}
```

**Available APIs**:

Plugins have access to the following built-in APIs:

1. **Fetch API** - Make HTTP requests:
```typescript
// Make HTTP requests to external services
const response = await fetch('https://example.com/api', {
    method: 'POST',
    headers: {
        'Content-Type': 'application/json',
    },
    body: JSON.stringify({ key: 'value' }),
    timeout: 5000, // optional, default 30000ms
});

const data = await response.json();
// or: const text = await response.text();
```

2. **TextDecoder/TextEncoder** - Decode/encode bytes:
```typescript
const decoder = new TextDecoder();
const bodyText = decoder.decode(new Uint8Array(ctx.request.body));
```

3. **URL/URLSearchParams** - Parse URLs:
```typescript
const url = new URL(ctx.request.url);
const params = new URLSearchParams(url.search);
const userId = params.get('user_id');
```

4. **Logging** - Debug output:
```typescript
Deno.core.ops.op_plugin_log('info', 'Processing request...');
```

