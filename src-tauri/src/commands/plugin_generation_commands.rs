//! Plugin generation prompt commands

use tauri::command;

/// Get combined plugin generation prompt for AI
#[command]
pub fn get_combined_plugin_prompt_api(
    plugin_type: String,
    _vuln_type: String,
    _severity: String,
) -> Result<String, String> {
    if plugin_type == "agent" {
        Ok(get_agent_plugin_prompt())
    } else {
        Ok(get_traffic_plugin_prompt())
    }
}

fn get_traffic_plugin_prompt() -> String {
    r#"# Traffic Scan Plugin Generation Task

You are a professional security researcher and TypeScript developer. Your task is to generate high-quality Traffic Scan plugins for an AI-driven security testing system.

## Task Overview

Traffic scan plugins should:
1. Be written in TypeScript.
2. Analyze HTTP traffic (requests and responses) without modifying it.
3. Detect specific vulnerabilities or collect information.
4. Export required scanning functions.
5. Use `Sentinel.emitFinding()` to report findings.

## Key Principles

**Traffic Analysis**:
- NEVER send new requests or block traffic.
- Analyze existing `HttpTransaction` data strictly.
- Be stateless and performant (process thousands of requests/sec).

**Detection Strategy**:
- Check response headers for missing/insecure configurations.
- Inspect response bodies for sensitive information leaks (regex/pattern matching).
- Analyze request parameters for obvious security flaws.

**Code Quality**:
- Write concise, performant TypeScript.
- Handle missing optional fields safely (e.g., `transaction.response` might be null).
- Use descriptive variable names.

---

## Traffic Plugin Interface (Required Structure)

The Traffic Scan plugin you generate **MUST** include the following structure:

### 1. Main Scan Function

You **MUST** export `scan_transaction` to analyze traffic.

```typescript
/**
 * Scans an HTTP transaction (request + response)
 * @param {HttpTransaction} transaction
 */
export async function scan_transaction(transaction) {
    const req = transaction.request;
    const resp = transaction.response; // Note: Can be null/undefined if only request was captured
    
    // Logic goes here...
}
```

**Data Structures**:

*   **HttpTransaction**: `{ request: RequestContext, response: ResponseContext | null }`
*   **RequestContext**: `{ url, method, headers (Map-like object), body (Uint8Array), ... }`
*   **ResponseContext**: `{ status, headers (Map-like object), body (Uint8Array), ... }`

**Note on Body Handling**:
Bodies are `Uint8Array`. Use `Buffer` or `TextDecoder` to convert to string if needed.

### 2. Reporting Findings

Use the global `Sentinel` object to report results.

```typescript
Sentinel.emitFinding({
    title: "Vulnerability Title",
    severity: "high",
    description: "Detailed description...",
    evidence: "The matched string or header value",
    confidence: "high", // "high", "medium", "low"
    location: "Header: X-Powered-By" // or URL, Body line, etc.
});
```

### Available APIs

**Runtime**: Node.js-compatible JavaScript. Standard Node.js APIs are supported (require, Buffer, etc.).

**Custom API**:
*   `Sentinel.emitFinding(finding)` - Report vulnerabilities

---

## Output Format

Return ONLY the TypeScript plugin code wrapped in a markdown code block:

```typescript
export async function scan_transaction(transaction) {
  const resp = transaction.response;
  if (!resp) return;

  // Example: Check for a specific header
  const headerValue = resp.headers["x-vulnerable-header"];
  if (headerValue) {
      Sentinel.emitFinding({
          title: "Vulnerable Header Detected",
          severity: "medium",
          description: "The server exposes a vulnerable header.",
          evidence: `x-vulnerable-header: ${headerValue}`,
          confidence: "high",
          location: "Response Header"
      });
  }
}
  
globalThis.scan_transaction = scan_transaction;
```

**Requirements**:
1. **MUST export `scan_transaction`**.
2. **Handle `transaction.response` being potentially null**.
3. **Use `Sentinel.emitFinding`** for reporting.
4. **No network calls** (fetch, etc.) are allowed in traffic plugins.
5. **MUST include the `globalThis` export at the end** - Without this, the plugin will fail with "Function not found" error.

Now generate the Traffic Scan Plugin.
"#.to_string()
}

fn get_agent_plugin_prompt() -> String {
    r#"# Agent Tool Plugin Generation Task

You are a professional security researcher and TypeScript developer. Your task is to generate high-quality Agent tool plugins for an AI-driven security testing system.

## Task Overview

Agent tool plugins should:
1. Be written in TypeScript.
2. Implement specific security testing or analysis functionality.
3. Follow the Agent tool plugin interface.
4. Include appropriate error handling and validation.
5. Use the `ToolOutput` interface to return structured results.
6. **MUST export a `get_input_schema()` function to declare the input parameters' JSON Schema.**

## Key Principles

**Important**: Generate generic tool logic that works in different scenarios, not just for specific targets. Use requirements as a reference for common patterns, but make the tool widely applicable.

**Implementation Strategy**:
- Focus on reusable tool functionality (scanning, analysis, reporting, etc.).
- Use correct TypeScript types and interfaces.
- Validate inputs and handle edge cases.
- Return detailed, actionable results.
- Include confidence levels and evidence where applicable.

**Code Quality**:
- Write concise, well-commented TypeScript code.
- Use try-catch blocks for appropriate error handling.
- Include descriptive variable names.
- Add inline comments explaining tool logic.

**Security Best Practices**:
- Validate all inputs before processing.
- Handle sensitive data appropriately.
- Provide detailed error messages for debugging.
- Include appropriate logging to improve observability.

---

## Agent Tool Plugin Interface (Required Structure)

The Agent tool plugin you generate **MUST** include the following structure:

### 1. get_input_schema() Function (Required)

Export a `get_input_schema()` function that returns a JSON Schema object describing the input parameters:

```typescript
/**
 * Export parameter schema function (Required)
 */
export function get_input_schema() {
    return {
        type: "object",
        required: ["target"],
        properties: {
            target: {
                type: "string",
                description: "Target URL or host address"
            },
            timeout: {
                type: "integer",
                default: 5000,
                description: "Timeout in milliseconds"
            }
        }
    };
}

globalThis.get_input_schema = get_input_schema;
```

### 2. TypeScript Interface and Implementation

```typescript
// Tool input interface
interface ToolInput {
    target: string;
    timeout?: number;
}

// Tool output interface
interface ToolOutput {
    success: boolean;
    data?: any;
    error?: string;
}

// Main tool function
export async function analyze(input: ToolInput): Promise<ToolOutput> {
    try {
        // Validate input
        if (!input || !input.target) {
            return {
                success: false,
                error: "Invalid input: target is required"
            };
        }
        
        // Implement your tool logic here
        
        return {
            success: true,
            data: {
                // Your tool results
            }
        };
    } catch (error) {
        return {
            success: false,
            error: error instanceof Error ? error.message : String(error)
        };
    }
}

// **CRITICAL**: Export functions to globalThis
globalThis.get_input_schema = get_input_schema;
globalThis.analyze = analyze;
```

### Available APIs

**Runtime Environment**: The plugin runs in a **Node.js-compatible** JavaScript runtime. You can use standard Node.js APIs.

**IMPORTANT**: Use `require()` for importing modules, NOT ES6 `import` statements:
```typescript
// ✅ CORRECT - Use require()
const fs = require('fs').promises;
const crypto = require('crypto');

// ❌ WRONG - Do NOT use import
import * as fs from 'fs/promises';  // This will fail!
```

**Custom Sentinel API** - Report findings:
```typescript
// Report a vulnerability or finding
Sentinel.emitFinding({
    title: 'Finding title',
    description: 'Detailed description',
    severity: 'high', // 'critical', 'high', 'medium', 'low', 'info'
    confidence: 'high', // 'high', 'medium', 'low'
    vuln_type: 'xss',
    evidence: 'Proof of vulnerability',
    url: 'https://target.com/path',
    method: 'GET',
});
```


---

## Output Format

Return ONLY the TypeScript plugin code wrapped in a markdown code block:

```typescript
/**
 * Tool Plugin
 * @plugin tool_id
 * @name Tool Name
 * @version 1.0.0
 * @author Sentinel AI
 * @category category
 * @default_severity medium
 * @tags tag1, tag2
 * @description Tool description
 */

interface ToolInput {
    target: string;
}

interface ToolOutput {
    success: boolean;
    data?: any;
    error?: string;
}

export function get_input_schema() {
    return {
        type: "object",
        required: ["target"],
        properties: {
            target: {
                type: "string",
                description: "Target address"
            }
        }
    };
}

export async function analyze(input: ToolInput): Promise<ToolOutput> {
    try {
        if (!input || !input.target) {
            return { success: false, error: "target is required" };
        }
        
        // Your tool logic
        return {
            success: true,
            data: {}
        };
    } catch (error) {
        return {
            success: false,
            error: error instanceof Error ? error.message : String(error)
        };
    }
}

// **CRITICAL**: Must export functions to globalThis
globalThis.get_input_schema = get_input_schema;
globalThis.analyze = analyze;
```

**Requirements**:
1. **MUST export a `get_input_schema()` function**.
2. Include detailed comments explaining the tool logic.
3. Use correct TypeScript types.
4. Handle edge cases and errors gracefully.
5. Return a structured `ToolOutput` with success/error status.
6. **MUST bind both `get_input_schema` and `analyze` to globalThis**.

Now generate the Agent Tool Plugin.
"#.to_string()
}
