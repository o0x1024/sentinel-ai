# Node.js Compatibility Layer

## Overview

The plugin runtime now includes a **complete Node.js compatibility layer**, allowing plugins to be written using standard Node.js APIs instead of Deno-specific APIs. This significantly reduces the learning curve and allows LLMs to generate plugins without needing extensive API documentation in prompts.

## Benefits

1. **Reduced Token Usage**: No need to include extensive API documentation in LLM prompts
2. **Lower LLM Hallucination**: LLMs are already trained on Node.js APIs, reducing incorrect code generation
3. **Familiar APIs**: Developers can use standard Node.js patterns they already know
4. **Better Compatibility**: Easier to port existing Node.js code to plugins

## Supported Node.js APIs

### Core Modules (via `require()`)

#### `fs` - File System
```javascript
const fs = require('fs');

// Callback-based API
fs.readFile('/path/to/file.txt', 'utf8', (err, data) => {
    if (err) throw err;
    console.log(data);
});

fs.writeFile('/path/to/file.txt', 'content', (err) => {
    if (err) throw err;
});

// Promise-based API (recommended)
const fs = require('fs').promises;
const content = await fs.readFile('/path/to/file.txt', 'utf8');
await fs.writeFile('/path/to/file.txt', 'content');
await fs.mkdir('/path/to/dir', { recursive: true });
await fs.readdir('/path/to/dir');
await fs.stat('/path/to/file.txt');
await fs.unlink('/path/to/file.txt');
await fs.rmdir('/path/to/dir', { recursive: true });
await fs.copyFile('/source.txt', '/dest.txt');
```

#### `path` - Path Manipulation
```javascript
const path = require('path');

path.join('/foo', 'bar', 'baz.txt');  // '/foo/bar/baz.txt'
path.resolve('foo', 'bar');           // '/foo/bar'
path.dirname('/foo/bar/baz.txt');     // '/foo/bar'
path.basename('/foo/bar/baz.txt');    // 'baz.txt'
path.extname('/foo/bar/baz.txt');     // '.txt'
```

#### `crypto` - Cryptographic Functions
```javascript
const crypto = require('crypto');

// Random bytes
const random = crypto.randomBytes(16);

// Hashing (SHA-256, SHA-512, SHA-1)
const hash = crypto.createHash('sha256');
hash.update('hello world');
const digest = await hash.digest('hex');

// UUID generation
const uuid = crypto.randomUUID();
```

**Note**: MD5 is not supported (use SHA-256 instead for security).

#### `http` / `https` - HTTP Client
```javascript
const http = require('http');

http.get('http://example.com', (res) => {
    res.on('data', (chunk) => {
        console.log(chunk.toString());
    });
    res.on('end', () => {
        console.log('Done');
    });
});

// Or use fetch() instead (recommended)
const response = await fetch('https://example.com');
const data = await response.json();
```

#### `url` - URL Parsing
```javascript
const url = require('url');

const parsed = url.parse('https://example.com/path?query=value');
// { protocol: 'https:', hostname: 'example.com', pathname: '/path', ... }

// Or use the URL class (recommended)
const myUrl = new URL('https://example.com/path?query=value');
```

#### `querystring` - Query String Parsing
```javascript
const qs = require('querystring');

const parsed = qs.parse('foo=bar&baz=qux');  // { foo: 'bar', baz: 'qux' }
const str = qs.stringify({ foo: 'bar' });    // 'foo=bar'
```

#### `util` - Utilities
```javascript
const util = require('util');

util.promisify(callback_fn);  // Convert callback to promise
util.inspect(obj);            // Pretty-print objects
```

#### `os` - Operating System
```javascript
const os = require('os');

os.platform();  // 'darwin', 'linux', 'win32'
os.arch();      // 'x64', 'arm64'
os.hostname();  // System hostname
os.tmpdir();    // Temp directory path
```

#### `buffer` - Buffer Class
```javascript
const { Buffer } = require('buffer');

// Or use global Buffer
const buf = Buffer.from('hello');
const hex = buf.toString('hex');
const base64 = buf.toString('base64');
```

### Global Objects

#### `process` - Process Information
```javascript
process.platform;  // 'darwin', 'linux', 'win32'
process.arch;      // 'x64', 'arm64'
process.pid;       // Process ID
process.version;   // Node.js version
process.env;       // Environment variables (empty in plugins)

process.nextTick(callback);  // Schedule callback for next tick
```

#### `Buffer` - Binary Data
```javascript
// Create buffers
const buf1 = Buffer.from('hello');
const buf2 = Buffer.from([0x68, 0x65, 0x6c, 0x6c, 0x6f]);
const buf3 = Buffer.from('aGVsbG8=', 'base64');
const buf4 = Buffer.alloc(10);

// Convert to string
buf1.toString();        // 'hello'
buf1.toString('hex');   // '68656c6c6f'
buf1.toString('base64'); // 'aGVsbG8='

// Concatenate
Buffer.concat([buf1, buf2]);
```

#### `__dirname` and `__filename`
```javascript
console.log(__dirname);   // '/plugin'
console.log(__filename);  // '/plugin/index.js'
```

### Web Standard APIs

These are available globally (no `require()` needed):

- `fetch()` - HTTP requests (recommended over http/https modules)
- `console.log()` - Logging
- `setTimeout()` / `setInterval()` - Timers
- `URL` / `URLSearchParams` - URL parsing
- `TextEncoder` / `TextDecoder` - Text encoding
- `crypto.getRandomValues()` - Cryptographic random
- `crypto.randomUUID()` - UUID generation

## Custom Sentinel APIs

### `Sentinel.emitFinding()`

Report vulnerabilities or findings:

```javascript
Sentinel.emitFinding({
    title: 'SQL Injection Detected',
    description: 'The application is vulnerable to SQL injection',
    severity: 'high',        // 'critical', 'high', 'medium', 'low', 'info'
    confidence: 'high',      // 'high', 'medium', 'low'
    vuln_type: 'sql_injection',
    evidence: 'Payload: \' OR 1=1--',
    url: 'https://target.com/api',
    method: 'POST',
});
```

## Example Plugins

### Traffic Scan Plugin (Node.js Style)
```javascript
export async function scan_transaction(transaction) {
    const resp = transaction.response;
    if (!resp) return;

    // Convert body to string
    const body = Buffer.from(resp.body).toString('utf8');
    
    // Check for SQL errors
    if (body.includes('SQL syntax error')) {
        Sentinel.emitFinding({
            title: 'SQL Error Detected',
            severity: 'high',
            description: 'Database error message exposed',
            evidence: body.substring(0, 200),
            confidence: 'high',
        });
    }
}

globalThis.scan_transaction = scan_transaction;
```

### Agent Tool Plugin (Node.js Style)
```javascript
const fs = require('fs').promises;
const path = require('path');
const crypto = require('crypto');

interface ToolInput {
    target_url: string;
    wordlist_path?: string;
}

interface ToolOutput {
    success: boolean;
    found_paths: string[];
}

export async function analyze(input: ToolInput): Promise<ToolOutput> {
    const { target_url, wordlist_path } = input;
    
    // Read wordlist
    const wordlist = wordlist_path 
        ? (await fs.readFile(wordlist_path, 'utf8')).split('\n')
        : ['admin', 'api', 'config'];
    
    const found_paths = [];
    
    // Test each path
    for (const word of wordlist) {
        const url = new URL(word, target_url).href;
        
        try {
            const response = await fetch(url);
            if (response.status === 200) {
                found_paths.push(url);
                
                Sentinel.emitFinding({
                    title: 'Exposed Path Found',
                    severity: 'medium',
                    description: `Found accessible path: ${url}`,
                    evidence: `Status: ${response.status}`,
                    confidence: 'high',
                    url,
                });
            }
        } catch (err) {
            // Ignore errors
        }
    }
    
    return {
        success: true,
        found_paths,
    };
}

export function get_input_schema() {
    return {
        type: 'object',
        properties: {
            target_url: {
                type: 'string',
                description: 'Target URL to scan',
            },
            wordlist_path: {
                type: 'string',
                description: 'Path to wordlist file (optional)',
            },
        },
        required: ['target_url'],
    };
}

globalThis.analyze = analyze;
globalThis.get_input_schema = get_input_schema;
```

## Migration Guide

### From Deno to Node.js Style

| Deno API | Node.js API |
|----------|-------------|
| `Deno.readTextFile(path)` | `require('fs').promises.readFile(path, 'utf8')` |
| `Deno.writeTextFile(path, data)` | `require('fs').promises.writeFile(path, data)` |
| `Deno.readFile(path)` | `require('fs').promises.readFile(path)` |
| `Deno.writeFile(path, data)` | `require('fs').promises.writeFile(path, data)` |
| `Deno.mkdir(path, opts)` | `require('fs').promises.mkdir(path, opts)` |
| `Deno.readDir(path)` | `require('fs').promises.readdir(path)` |
| `Deno.stat(path)` | `require('fs').promises.stat(path)` |
| `Deno.copyFile(src, dst)` | `require('fs').promises.copyFile(src, dst)` |
| `Deno.remove(path)` | `require('fs').promises.unlink(path)` |
| `new TextEncoder().encode(str)` | `Buffer.from(str)` |
| `new TextDecoder().decode(buf)` | `buf.toString()` |

## Implementation Details

The Node.js compatibility layer is implemented in `plugin_bootstrap.js` and provides:

1. **`require()` function**: Implements a simplified CommonJS module loader
2. **`process` object**: Provides process information and utilities
3. **`Buffer` class**: Full Buffer API implementation
4. **Module polyfills**: fs, path, crypto, http, https, util, os, url, querystring, buffer

All APIs are mapped to the underlying Deno Core operations, ensuring security and sandboxing while providing a familiar Node.js interface.

## Testing

Run the compatibility tests:

```bash
cd src-tauri/sentinel-plugins
cargo test --test nodejs_compatibility_test
```

## Limitations

1. **Synchronous APIs**: `fs.readFileSync()` and similar sync methods are not supported. Use async versions instead.
2. **MD5 Hashing**: Not supported by Web Crypto API. Use SHA-256 or SHA-512 instead.
3. **Child Processes**: `child_process` module is not available for security reasons.
4. **Native Modules**: Cannot load native Node.js addons.
5. **npm Packages**: Cannot install or use npm packages (except by bundling the code).

## Best Practices

1. **Use `fs.promises`** instead of callback-based fs APIs
2. **Use `fetch()`** for HTTP requests instead of http/https modules
3. **Use `Buffer`** for binary data manipulation
4. **Use `crypto.createHash('sha256')`** instead of MD5
5. **Handle errors properly** with try-catch blocks
6. **Validate inputs** before processing
7. **Use TypeScript types** for better code quality

## Conclusion

With the Node.js compatibility layer, plugin development is now much easier and more intuitive. LLMs can generate correct code without extensive API documentation, and developers can use familiar Node.js patterns they already know.
