## Agent Tool Plugin Interface (Required Structure)

Your generated Agent tool plugin MUST implement the following TypeScript interface:

```typescript
// Tool input interface - customize based on your tool's needs
interface ToolInput {
    [key: string]: any;  // Flexible input structure
    // Common fields (optional):
    // target?: string;
    // options?: Record<string, any>;
    // context?: Record<string, any>;
}

// Tool output interface - standardized result structure
interface ToolOutput {
    success: boolean;           // Whether the tool execution succeeded
    data?: any;                 // Tool-specific result data
    error?: string;             // Error message if failed
    // Optional fields:
    // confidence?: "high" | "medium" | "low";
    // evidence?: string[];
    // metadata?: Record<string, any>;
}

// Main tool function - implements the tool logic
export async function analyze(input: ToolInput): Promise<ToolOutput> {
    try {
        // Validate input
        if (!input) {
            return {
                success: false,
                error: "Invalid input: input is required"
            };
        }
        
        // Implement your tool logic here
        // Example:
        // const result = await performAnalysis(input);
        
        return {
            success: true,
            data: {
                // Your tool's results
            }
        };
    } catch (error) {
        return {
            success: false,
            error: error instanceof Error ? error.message : String(error)
        };
    }
}

// **CRITICAL**: Export function to globalThis for the plugin engine
// Without this export, the plugin will fail with "Function not found" error
globalThis.analyze = analyze;
```

**Context Objects**:

Agent tool plugins receive flexible input and should return structured output:

```typescript
// Example input for a port scanner tool
interface PortScanInput extends ToolInput {
    target: string;
    ports?: string;
    timeout?: number;
}

// Example output for a port scanner tool
interface PortScanOutput extends ToolOutput {
    data?: {
        open_ports: number[];
        closed_ports: number[];
        scan_duration: number;
    };
}
```

**Available APIs**:

Agent tool plugins have access to the following built-in APIs:

1. **Fetch API** - Make HTTP requests:
```typescript
const response = await fetch('https://example.com/api', {
    method: 'POST',
    headers: {
        'Content-Type': 'application/json',
    },
    body: JSON.stringify({ key: 'value' }),
    timeout: 5000,
});

const data = await response.json();
```

2. **TextDecoder/TextEncoder** - Decode/encode text:
```typescript
const encoder = new TextEncoder();
const bytes = encoder.encode('Hello, World!');

const decoder = new TextDecoder();
const text = decoder.decode(bytes);
```

3. **URL/URLSearchParams** - Parse URLs:
```typescript
const url = new URL('https://example.com/api?key=value');
const params = new URLSearchParams(url.search);
const key = params.get('key');
```

4. **Logging** - Debug output:
```typescript
Deno.core.ops.op_plugin_log('info', 'Tool execution started...');
Deno.core.ops.op_plugin_log('error', 'An error occurred');
```

