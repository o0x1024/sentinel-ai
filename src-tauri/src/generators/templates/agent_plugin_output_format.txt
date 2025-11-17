## Agent Tool Plugin Output Format

Return ONLY the TypeScript plugin code wrapped in a markdown code block:

```typescript
// Your complete Agent tool plugin code here
interface ToolInput {
    [key: string]: any;
}

interface ToolOutput {
    success: boolean;
    data?: any;
    error?: string;
}

export async function analyze(input: ToolInput): Promise<ToolOutput> {
    try {
        // Your tool logic here
        return {
            success: true,
            data: {
                // Your results
            }
        };
    } catch (error) {
        return {
            success: false,
            error: error instanceof Error ? error.message : String(error)
        };
    }
}

// **CRITICAL**: MUST export function to globalThis
// The plugin engine calls functions from globalThis, not from module exports
globalThis.analyze = analyze;
```

**Requirements**:
1. Include comprehensive comments explaining tool logic
2. Use proper TypeScript typing
3. Handle edge cases and errors gracefully
4. Return structured ToolOutput with success/error status
5. Include validation for input parameters
6. **MUST include globalThis export at the end** - Without this, the plugin will fail with "Function not found" error

Generate the Agent tool plugin now.

