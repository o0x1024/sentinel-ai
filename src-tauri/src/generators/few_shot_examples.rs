//! Few-shot learning examples for plugin generation

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Few-shot example for plugin generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FewShotExample {
    pub vuln_type: String,
    pub context: String,
    pub code: String,
    pub quality_score: f32,
}

/// Few-shot example repository
pub struct FewShotRepository {
    examples: HashMap<String, Vec<FewShotExample>>,
}

impl FewShotRepository {
    pub fn new() -> Self {
        let mut repo = Self {
            examples: HashMap::new(),
        };
        repo.load_builtin_examples();
        repo
    }
    
    /// Get examples for a specific vulnerability type
    pub fn get_examples(&self, vuln_type: &str) -> Vec<&FewShotExample> {
        self.examples
            .get(vuln_type)
            .map(|examples| examples.iter().collect())
            .unwrap_or_default()
    }
    
    /// Add a new example to the repository
    pub fn add_example(&mut self, example: FewShotExample) {
        self.examples
            .entry(example.vuln_type.clone())
            .or_insert_with(Vec::new)
            .push(example);
    }
    
    /// Load built-in high-quality examples
    fn load_builtin_examples(&mut self) {
        // SQL Injection Example
        self.add_example(FewShotExample {
            vuln_type: "sqli".to_string(),
            context: "MySQL database, REST API with numeric user_id parameter".to_string(),
            quality_score: 90.0,
            code: r#"// High-quality SQL Injection detector for MySQL
export const plugin = {
    get_metadata: () => ({
        id: "sqli_mysql_numeric",
        name: "SQL Injection - MySQL Numeric Parameters",
        version: "1.0.0",
        author: "Sentinel AI",
        description: "Detects SQL injection vulnerabilities in MySQL numeric parameters",
        severity: "high",
        category: "injection",
        tags: ["sqli", "mysql", "numeric"]
    }),

    scan_request: async (ctx) => {
        const { url, method } = ctx.request;
        const urlObj = new URL(url);
        const params = urlObj.searchParams;
        
        // SQL injection payloads for MySQL numeric parameters
        const payloads = [
            "1 OR 1=1",
            "1' OR '1'='1",
            "1 UNION SELECT NULL",
            "1 AND 1=1",
            "1' AND '1'='1",
        ];
        
        for (const [key, value] of params.entries()) {
            // Check if parameter looks numeric
            if (/^\d+$/.test(value)) {
                for (const payload of payloads) {
                    await Deno.core.ops.op_emit_finding({
                        vuln_type: "sqli",
                        severity: "high",
                        title: `Potential SQL Injection in numeric parameter: ${key}`,
                        description: `Parameter '${key}' may be vulnerable to SQL injection attacks`,
                        endpoint: url,
                        method: method,
                        evidence: {
                            parameter: key,
                            original_value: value,
                            test_payload: payload,
                            type: "numeric"
                        }
                    });
                }
            }
        }
    },

    scan_response: async (ctx) => {
        const { response, request } = ctx;
        
        // MySQL error patterns
        const mysqlErrors = [
            /You have an error in your SQL syntax/i,
            /mysql_fetch_array\(\)/i,
            /mysqli_error/i,
            /SQL syntax.*MySQL/i,
            /Warning.*mysql_/i
        ];
        
        const body = response.body || "";
        
        for (const pattern of mysqlErrors) {
            if (pattern.test(body)) {
                await Deno.core.ops.op_emit_finding({
                    vuln_type: "sqli",
                    severity: "critical",
                    title: "SQL Error Message Detected - Confirmed SQLi",
                    description: "Response contains MySQL error message, confirming SQL injection vulnerability",
                    endpoint: request.url,
                    method: request.method,
                    evidence: {
                        error_pattern: pattern.toString(),
                        response_snippet: body.substring(0, 200),
                        confirmed: true
                    }
                });
                break; // Only emit once per response
            }
        }
    }
};"#.to_string(),
        });

        // XSS Example
        self.add_example(FewShotExample {
            vuln_type: "xss".to_string(),
            context: "Express.js application with user comments feature".to_string(),
            quality_score: 88.0,
            code: r#"// High-quality XSS detector
export const plugin = {
    get_metadata: () => ({
        id: "xss_reflected",
        name: "Reflected XSS Detector",
        version: "1.0.0",
        author: "Sentinel AI",
        description: "Detects reflected XSS vulnerabilities in user input",
        severity: "high",
        category: "injection",
        tags: ["xss", "injection", "reflected"]
    }),

    scan_request: async (ctx) => {
        const { url, method, body } = ctx.request;
        const urlObj = new URL(url);
        const params = urlObj.searchParams;
        
        // XSS payloads
        const xssPayloads = [
            "<script>alert(1)</script>",
            "<img src=x onerror=alert(1)>",
            "javascript:alert(1)",
            "<svg onload=alert(1)>",
        ];
        
        // Check URL parameters
        for (const [key, value] of params.entries()) {
            if (typeof value === 'string' && value.length > 0) {
                for (const payload of xssPayloads) {
                    await Deno.core.ops.op_emit_finding({
                        vuln_type: "xss",
                        severity: "high",
                        title: `Potential Reflected XSS in parameter: ${key}`,
                        description: `Parameter '${key}' may be vulnerable to XSS attacks`,
                        endpoint: url,
                        method: method,
                        evidence: {
                            parameter: key,
                            location: "url",
                            test_payload: payload
                        }
                    });
                }
            }
        }
    },

    scan_response: async (ctx) => {
        const { response, request } = ctx;
        const body = response.body || "";
        
        // Check if response reflects input without encoding
        const urlObj = new URL(request.url);
        const params = urlObj.searchParams;
        
        for (const [key, value] of params.entries()) {
            if (typeof value === 'string' && value.length > 0) {
                // Check if value appears unencoded in response
                if (body.includes(value) && /<[^>]+>/.test(value)) {
                    await Deno.core.ops.op_emit_finding({
                        vuln_type: "xss",
                        severity: "critical",
                        title: "Confirmed Reflected XSS",
                        description: `User input is reflected without proper encoding`,
                        endpoint: request.url,
                        method: request.method,
                        evidence: {
                            parameter: key,
                            reflected_value: value,
                            confirmed: true
                        }
                    });
                }
            }
        }
    }
};"#.to_string(),
        });

        // IDOR Example
        self.add_example(FewShotExample {
            vuln_type: "idor".to_string(),
            context: "REST API with user profile access by ID".to_string(),
            quality_score: 85.0,
            code: r#"// High-quality IDOR detector
export const plugin = {
    get_metadata: () => ({
        id: "idor_sequential_ids",
        name: "IDOR - Sequential ID Access",
        version: "1.0.0",
        author: "Sentinel AI",
        description: "Detects Insecure Direct Object Reference vulnerabilities",
        severity: "medium",
        category: "access_control",
        tags: ["idor", "access_control", "authorization"]
    }),

    scan_request: async (ctx) => {
        const { url, method, headers } = ctx.request;
        
        // Look for ID parameters in URL path and query
        const idPatterns = [
            /\/users?\/(\d+)/i,
            /\/profiles?\/(\d+)/i,
            /\/orders?\/(\d+)/i,
            /[?&]id=(\d+)/i,
            /[?&]user_id=(\d+)/i,
        ];
        
        for (const pattern of idPatterns) {
            const match = url.match(pattern);
            if (match) {
                const originalId = match[1];
                const testIds = [
                    parseInt(originalId) + 1,  // Next ID
                    parseInt(originalId) - 1,  // Previous ID
                    "1",                        // First ID
                ];
                
                for (const testId of testIds) {
                    await Deno.core.ops.op_emit_finding({
                        vuln_type: "idor",
                        severity: "medium",
                        title: "Potential IDOR - Unvalidated Object Access",
                        description: `Endpoint accesses object by ID without proper authorization check`,
                        endpoint: url,
                        method: method,
                        evidence: {
                            original_id: originalId,
                            test_id: testId.toString(),
                            pattern: pattern.toString(),
                            auth_header_present: !!headers["authorization"]
                        }
                    });
                }
                break; // Only test first match
            }
        }
    },

    scan_response: async (ctx) => {
        const { response, request } = ctx;
        
        // Check for successful access without proper authorization
        if (response.status >= 200 && response.status < 300) {
            const hasAuth = !!request.headers["authorization"];
            
            // If no auth header but successful response, potential IDOR
            if (!hasAuth) {
                await Deno.core.ops.op_emit_finding({
                    vuln_type: "idor",
                    severity: "high",
                    title: "Possible IDOR - Unauthorized Access Granted",
                    description: "Resource accessed successfully without authentication",
                    endpoint: request.url,
                    method: request.method,
                    evidence: {
                        status: response.status,
                        no_auth_header: true,
                        response_has_data: !!response.body
                    }
                });
            }
        }
    }
};"#.to_string(),
        });
        
        // Add more examples for other vulnerability types...
    }
}

impl Default for FewShotRepository {
    fn default() -> Self {
        Self::new()
    }
}

