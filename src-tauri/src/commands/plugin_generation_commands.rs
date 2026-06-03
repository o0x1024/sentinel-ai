//! Plugin generation prompt commands

use crate::generators::agent_contract_generation_instructions;
use crate::services::{IntruderPluginCategory, PluginMainCategory};
use tauri::command;

/// Get combined plugin generation prompt for AI
#[command]
pub fn get_combined_plugin_prompt_api(
    plugin_type: String,
    vuln_type: String,
    _severity: String,
) -> Result<String, String> {
    match PluginMainCategory::parse(&plugin_type)? {
        PluginMainCategory::Agent | PluginMainCategory::Bounty => Ok(get_agent_plugin_prompt()),
        PluginMainCategory::Intruder => {
            let category = IntruderPluginCategory::parse(&vuln_type)?;
            Ok(get_intruder_plugin_prompt(category))
        }
        PluginMainCategory::Traffic => Ok(get_traffic_plugin_prompt()),
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
5. Return an array of findings.

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

You **MUST** export `scan_transaction` to analyze traffic and **return an array of findings**.

```typescript
/**
 * Scans an HTTP transaction (request + response)
 * @param {HttpTransaction} transaction
 * @returns {Array} Array of findings
 */
export async function scan_transaction(transaction) {
    const req = transaction.request;
    const resp = transaction.response; // Note: Can be null/undefined if only request was captured
    const findings = [];
    
    // Detection logic...
    // When vulnerability found:
    findings.push({
        title: "Vulnerability Title",
        severity: "high",
        description: "Detailed description",
        evidence: "Evidence string",
        confidence: "high",
        vuln_type: "xss"
    });
    
    return findings;
}
```

**Data Structures**:

*   **HttpTransaction**: `{ request: RequestContext, response: ResponseContext | null }`
*   **RequestContext**: `{ url, method, headers (Map-like object), body (Uint8Array), ... }`
*   **ResponseContext**: `{ status, headers (Map-like object), body (Uint8Array), ... }`

**Note on Body Handling**:
Bodies are `Uint8Array`. Use `Buffer` or `TextDecoder` to convert to string if needed.

### 2. Finding Object Structure

Each finding object should have:

```typescript
{
    title: string;           // Short vulnerability title
    severity: string;        // "critical", "high", "medium", "low", "info"
    description: string;     // Detailed description
    evidence: string;        // The matched string or header value
    confidence: string;      // "high", "medium", "low"
    vuln_type: string;       // e.g., "xss", "sqli", "ssrf"
    location?: string;       // Optional: "Header: X-Powered-By" or URL, Body line, etc.
}
```

### Available APIs

**Runtime**: Node.js-compatible JavaScript. Standard Node.js APIs are supported (require, Buffer, etc.).

---

## Output Format

Return ONLY the TypeScript plugin code wrapped in a markdown code block:

```typescript
export async function scan_transaction(transaction) {
  const findings = [];
  const resp = transaction.response;
  if (!resp) return findings;

  // Example: Check for a specific header
  const headerValue = resp.headers["x-vulnerable-header"];
  if (headerValue) {
      findings.push({
          title: "Vulnerable Header Detected",
          severity: "medium",
          description: "The server exposes a vulnerable header.",
          evidence: `x-vulnerable-header: ${headerValue}`,
          confidence: "high",
          vuln_type: "security_misconfiguration",
          location: "Response Header"
      });
  }
  
  return findings;
}
  
globalThis.scan_transaction = scan_transaction;
```

**Requirements**:
1. **MUST export `scan_transaction`**.
2. **MUST return an array of findings** (empty array if no issues found).
3. **Handle `transaction.response` being potentially null**.
4. **Bounded active verification is allowed** in traffic plugins. If you use `fetch`, keep requests low-rate, serial by host, and non-destructive by default.
5. **MUST include the `globalThis` export at the end** - Without this, the plugin will fail with "Function not found" error.

Now generate the Traffic Scan Plugin.
"#.to_string()
}

fn get_agent_plugin_prompt() -> String {
    format!(
        r#"# Agent Tool Plugin Generation Task

You are a professional security researcher and TypeScript developer. Your task is to generate high-quality Agent tool plugins for an AI-driven security testing system.

## Task Overview

Agent tool plugins should:
1. Be written in TypeScript.
2. Implement specific security testing or analysis functionality.
3. Follow the Agent tool plugin interface.
4. Include appropriate error handling and validation.
5. Use the `ToolOutput` interface to return structured results.
6. Follow the active Agent Tool Contract exactly.

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

{contract_instructions}

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

## Output Format

Return only the plugin definition JSON described by the active contract. The Rust renderer will generate the final TypeScript file, schema exports, and `globalThis` bindings.

Now generate the Agent Tool Plugin.
"#,
        contract_instructions = agent_contract_generation_instructions()
    )
}

fn get_intruder_plugin_prompt(category: IntruderPluginCategory) -> String {
    let (category_guidance, input_example, implementation_example) = match category {
        IntruderPluginCategory::PayloadProcessor => (
            r#"Generate an Intruder payload processor plugin.

The plugin must:
1. Export `get_input_schema()`.
2. Export `analyze(input)` and assign both to `globalThis`.
3. Accept a payload-oriented input object.
4. Return `ToolOutput` with `data.payload?: string` and optional `data.skip?: boolean`.
5. Never mutate external state. Operate only on the provided payload/context.

Expected output shape:

```typescript
interface ToolOutput {
  success: boolean;
  data?: {
    payload?: string;
    skip?: boolean;
  };
  error?: string;
}
```
"#,
            r#"```json
{
  "payload": "admin",
  "originalPayload": "admin",
  "baseValue": "admin",
  "positionIndex": 0,
  "config": {
    "prefix": "pre-",
    "suffix": "-post",
    "skipIfContains": "forbidden"
  }
}
```"#,
            r#"```typescript
interface ToolInput {
  payload: string;
  originalPayload?: string;
  baseValue?: string;
  positionIndex?: number;
  config?: {
    prefix?: string;
    suffix?: string;
    skipIfContains?: string;
  };
}

interface ToolOutput {
  success: boolean;
  data?: {
    payload?: string;
    skip?: boolean;
  };
  error?: string;
}

export function get_input_schema() {
  return {
    type: 'object',
    properties: {
      config: {
        type: 'object',
        properties: {
          prefix: { type: 'string' },
          suffix: { type: 'string' },
          skipIfContains: { type: 'string' }
        }
      }
    }
  };
}

export async function analyze(input: ToolInput): Promise<ToolOutput> {
  const skipIfContains = input?.config?.skipIfContains || '';
  if (skipIfContains && input.payload.includes(skipIfContains)) {
    return { success: true, data: { skip: true } };
  }

  const prefix = input?.config?.prefix || '';
  const suffix = input?.config?.suffix || '';

  return {
    success: true,
    data: {
      payload: `${prefix}${input.payload}${suffix}`
    }
  };
}

globalThis.get_input_schema = get_input_schema;
globalThis.analyze = analyze;
```"#,
        ),
        IntruderPluginCategory::RequestProcessor => (
            r#"Generate an Intruder request processor plugin.

The plugin must:
1. Export `get_input_schema()`.
2. Export `analyze(input)` and assign both to `globalThis`.
3. Accept a request-oriented input object that includes `rawRequest`.
4. Return `ToolOutput` with `data.rawRequest` containing the transformed raw HTTP request.
5. Keep the request syntactically valid and deterministic when possible.

Expected output shape:

```typescript
interface ToolOutput {
  success: boolean;
  data?: {
    rawRequest: string;
  };
  error?: string;
}
```
"#,
            r#"```json
{
  "rawRequest": "POST /login HTTP/1.1\r\nHost: example.com\r\nContent-Type: application/x-www-form-urlencoded\r\n\r\nuser=alice&ts=1700000000&sign=old",
  "payloadValues": ["alice"],
  "payloadSummary": "position[0]=alice",
  "requestIndex": 0,
  "config": {
    "headerName": "X-Debug",
    "headerValue": "1"
  }
}
```"#,
            r#"```typescript
interface ToolInput {
  rawRequest: string;
  payloadValues?: string[];
  payloadSummary?: string;
  requestIndex?: number;
  config?: {
    headerName?: string;
    headerValue?: string;
  };
}

interface ToolOutput {
  success: boolean;
  data?: {
    rawRequest: string;
  };
  error?: string;
}

export function get_input_schema() {
  return {
    type: 'object',
    properties: {
      config: {
        type: 'object',
        properties: {
          headerName: { type: 'string' },
          headerValue: { type: 'string' }
        }
      }
    }
  };
}

export async function analyze(input: ToolInput): Promise<ToolOutput> {
  const separator = '\r\n\r\n';
  const [head, body = ''] = input.rawRequest.split(separator);
  const lines = head.split('\r\n');
  const requestLine = lines.shift() || 'GET / HTTP/1.1';
  const headerName = input?.config?.headerName || '';
  const headerValue = input?.config?.headerValue || '';

  if (headerName) {
    lines.push(`${headerName}: ${headerValue}`);
  }

  return {
    success: true,
    data: {
      rawRequest: [requestLine, ...lines].join('\r\n') + separator + body
    }
  };
}

globalThis.get_input_schema = get_input_schema;
globalThis.analyze = analyze;
```"#,
        ),
        IntruderPluginCategory::PayloadGenerator => (
            r#"Generate an Intruder payload generator plugin.

The plugin must:
1. Export `get_input_schema()`.
2. Export `analyze(input)` and assign both to `globalThis`.
3. Accept a request/context input object.
4. Return `ToolOutput` with `data.payloads: string[]`.
5. Keep the generator deterministic unless randomness is explicitly required by the prompt.

Expected output shape:

```typescript
interface ToolOutput {
  success: boolean;
  data?: {
    payloads: string[];
  };
  error?: string;
}
```
"#,
            r#"```json
{
  "request": {
    "method": "GET",
    "url": "https://target.test/search?q=§payload§",
    "headers": {}
  },
  "rawRequest": "GET /search?q=test HTTP/1.1\r\nHost: target.test\r\n\r\n",
  "target": {
    "host": "target.test",
    "port": 443,
    "useTls": true
  },
  "positions": [
    { "index": 0, "value": "test" }
  ],
  "config": {
    "values": ["admin", "root", "guest"],
    "limit": 100
  },
  "options": {
    "limit": 100
  }
}
```"#,
            r#"```typescript
interface ToolInput {
  request?: { method: string; url: string; headers: Record<string, string>; body?: string };
  rawRequest?: string;
  target?: { host: string; port: number; useTls: boolean };
  positions?: Array<{ index: number; value: string }>;
  config?: {
    values?: string[];
    limit?: number;
  };
  options?: {
    limit?: number;
  };
}

interface ToolOutput {
  success: boolean;
  data?: {
    payloads: string[];
  };
  error?: string;
}

export function get_input_schema() {
  return {
    type: 'object',
    properties: {
      config: {
        type: 'object',
        properties: {
          values: {
            type: 'array',
            items: { type: 'string' }
          },
          limit: {
            type: 'integer',
            default: 100
          }
        }
      }
    }
  };
}

export async function analyze(input: ToolInput): Promise<ToolOutput> {
  const values = Array.isArray(input?.config?.values) ? input.config.values.filter(Boolean) : [];
  const limit = Math.max(1, Number(input?.config?.limit ?? input?.options?.limit ?? 100));

  return {
    success: true,
    data: {
      payloads: values.slice(0, limit)
    }
  };
}

globalThis.get_input_schema = get_input_schema;
globalThis.analyze = analyze;
```"#,
        ),
    };

    let mut prompt = String::from(
        r#"# Intruder Plugin Generation Task

You are a professional security researcher and TypeScript developer. Your task is to generate a high-quality Intruder plugin for Sentinel.

## Task Overview

Intruder plugins are active request-preparation helpers. They run before requests are sent and must return structured JSON results that Sentinel can consume directly.

## Core Rules

1. The code must be written in TypeScript.
2. The plugin must export `get_input_schema()` and `analyze(input)`.
3. The plugin must assign both exports to `globalThis`.
4. Return a `ToolOutput` object shaped exactly as required by the category.
5. Use defensive input validation and explicit error messages.
6. Do not depend on global mutable state.
7. If the plugin edits a request, return the full updated raw request string.

## Category

Target category: `"#,
    );
    prompt.push_str(&category.to_string());
    prompt.push_str("`\n\n");
    prompt.push_str(category_guidance);
    prompt.push_str(
        r#"

## Input Shape Example
"#,
    );
    prompt.push_str(input_example);
    prompt.push_str(
        r#"

## Minimal Runnable Example
"#,
    );
    prompt.push_str(implementation_example);
    prompt.push_str(
        r#"

## Implementation Guidelines

- Keep logic reusable and generic.
- Use descriptive variable names.
- Prefer pure helper functions.
- Add brief comments only where the logic is non-obvious.
- Handle missing optional fields safely.
- Do not return markdown, explanations, or prose inside `data`.
- Preserve the exact `ToolOutput` shape for the selected category.
- Prefer built-in JavaScript or `require()` imports only when necessary.
- For `request_processor`, preserve CRLF line endings and return the full request.

## Output Format

Return ONLY the TypeScript plugin code wrapped in a markdown code block.

The generated code must end with:

```typescript
globalThis.get_input_schema = get_input_schema;
globalThis.analyze = analyze;
```

Now generate the Intruder plugin.
"#,
    );
    prompt
}
