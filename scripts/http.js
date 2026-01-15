/**
 * @plugin _
 * @name 生成一个获取指定网站响应的插件
 * @version 1.0.0
 * @author AI Generated
 * @category 
 * @default_severity medium
 * @tags ai-generated, agent
 * @description 生成一个获取指定网站响应的插件
 */

/**
 * Tool Plugin
 * @plugin web_fetcher
 * @name Website Response Fetcher
 * @version 1.0.0
 * @author Sentinel AI
 * @category Utility
 * @default_severity info
 * @tags network, fetch, response, http
 * @description Fetches the full HTTP response from a specified website, including status code, headers, and body.
 */

const http = require('http');
const https = require('https');
const { URL } = require('url');

// Tool input interface
interface ToolInput {
    target: string;
    method?: string;
    headers?: Record<string, string>;
    body?: string;
    timeout?: number;
    followRedirects?: boolean;
}

// Tool output interface
interface ToolOutput {
    success: boolean;
    data?: {
        status: number;
        statusText: string;
        headers: Record<string, string>;
        body: string;
        url: string;
        requestConfig: any;
    };
    error?: string;
}

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
                default: "www.baidu.com",
                description: "The full URL of the website to fetch (e.g., https://example.com)"
            },
            method: {
                type: "string",
                enum: ["GET", "POST", "PUT", "DELETE", "HEAD", "OPTIONS"],
                default: "GET",
                description: "HTTP request method"
            },
            headers: {
                type: "object",
                description: "Custom HTTP headers as key-value pairs",
                additionalProperties: { type: "string" }
            },
            body: {
                type: "string",
                description: "Request body for POST/PUT requests"
            },
            timeout: {
                type: "integer",
                default: 10000,
                description: "Request timeout in milliseconds"
            },
            followRedirects: {
                type: "boolean",
                default: true,
                description: "Whether to automatically follow HTTP redirects"
            }
        }
    };
}

/**
 * Main tool function to fetch website response
 */
export async function analyze(input: ToolInput): Promise<ToolOutput> {
    try {
         input.target = "www.baidu.com"
        // Validate input
        if (!input || !input.target) {
            return {
                success: false,
                error: "Invalid input: target URL is required"
            };
        }

        let currentUrl = input.target;
        if (!currentUrl.startsWith('http://') && !currentUrl.startsWith('https://')) {
            currentUrl = 'http://' + currentUrl;
        }

        const method = (input.method || 'GET').toUpperCase();
        const timeout = input.timeout || 10000;
        const headers = input.headers || {
            'User-Agent': 'Mozilla/5.0 (Sentinel-AI Security Scanner)',
            'Accept': '*/*'
        };

        // Execution of the request logic wrapped in a promise
        const performRequest = (urlStr: string): Promise<any> => {
            return new Promise((resolve, reject) => {
                const parsedUrl = new URL(urlStr);
                const protocol = parsedUrl.protocol === 'https:' ? https : http;
                
                const options = {
                    hostname: parsedUrl.hostname,
                    port: parsedUrl.port || (parsedUrl.protocol === 'https:' ? 443 : 80),
                    path: parsedUrl.pathname + parsedUrl.search,
                    method: method,
                    headers: headers,
                    timeout: timeout
                };

                const req = protocol.request(options, (res) => {
                    let data = '';
                    
                    // Handle redirects manually if needed (limited to 5 hops)
                    if (input.followRedirects !== false && [301, 302, 303, 307, 308].includes(res.statusCode) && res.headers.location) {
                        resolve({ isRedirect: true, location: new URL(res.headers.location, urlStr).href });
                        return;
                    }

                    res.setEncoding('utf8');
                    res.on('data', (chunk) => { data += chunk; });
                    res.on('end', () => {
                        resolve({
                            status: res.statusCode,
                            statusText: res.statusMessage,
                            headers: res.headers,
                            body: data,
                            url: urlStr
                        });
                    });
                });

                req.on('error', (e) => reject(e));
                req.on('timeout', () => {
                    req.destroy();
                    reject(new Error(`Request timed out after ${timeout}ms`));
                });

                if (input.body && (method === 'POST' || method === 'PUT')) {
                    req.write(input.body);
                }
                req.end();
            });
        };

        let response = await performRequest(currentUrl);
        let redirectCount = 0;

        // Follow redirects logic (up to 5 levels)
        while (response.isRedirect && redirectCount < 5) {
            redirectCount++;
            response = await performRequest(response.location);
        }

        if (response.isRedirect) {
            throw new Error("Too many redirects");
        }

        // Optional: Emit finding if the status code indicates a potential issue (e.g., 500 server error)
        if (response.status >= 500) {
            // @ts-ignore
            // eslint-disable-next-line no-undef
            if (typeof Sentinel !== 'undefined') {
                // eslint-disable-next-line no-undef
                Sentinel.emitFinding({
                    title: 'Server Error Detected',
                    description: `Target ${currentUrl} returned a ${response.status} status code.`,
                    severity: 'low',
                    confidence: 'high',
                    vuln_type: 'information_exposure',
                    evidence: `Status: ${response.status}\nResponse: ${response.body.substring(0, 200)}`,
                    url: currentUrl,
                    method: method
                });
            }
        }

        return {
            success: true,
            data: {
                status: response.status,
                statusText: response.statusText,
                headers: response.headers,
                body: response.body,
                url: response.url,
                requestConfig: {
                    method,
                    timeout,
                    redirects: redirectCount
                }
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