## Output Format

Return ONLY the TypeScript plugin code wrapped in a markdown code block:

```typescript
// Your complete plugin code here
function get_metadata(): PluginMetadata {
    // ...
}

export function scan_request(ctx: RequestContext): void {
    // ...
}

export function scan_response(ctx: CombinedContext): void {
    // ...
}

// **CRITICAL**: MUST export all functions to globalThis
// The plugin engine calls functions from globalThis, not from module exports
// Use direct assignment without type casting to ensure proper execution
globalThis.get_metadata = get_metadata;
globalThis.scan_request = scan_request;
globalThis.scan_response = scan_response;
```

**Requirements**:
1. Include comprehensive comments explaining detection logic
2. Use proper TypeScript typing
3. Handle edge cases and errors gracefully
4. Emit findings only when confidence is reasonable
5. Include CWE and OWASP references when applicable
6. Make detection patterns specific to the analyzed website
7. Avoid false positives by validating patterns thoroughly
8. **MUST include globalThis exports at the end** - Without these, the plugin will fail with "Function not found" error

Generate the plugin now.

