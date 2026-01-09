//! Prompt templates for AI plugin generation

use super::few_shot_examples::FewShotExample;
use crate::analyzers::WebsiteAnalysis;
use anyhow::Result;

/// Template type enum
#[derive(Debug, Clone, PartialEq)]
pub enum TemplateType {
    PluginGeneration,
    AgentPluginGeneration,
    PluginFix,
    AgentPluginFix,
}

/// Prompt template builder
pub struct PromptTemplateBuilder;

impl PromptTemplateBuilder {
    pub fn new() -> Self {
        Self
    }

    /// Get built-in template content
    async fn get_template_content(&self, template_type: TemplateType) -> Result<String> {
        // Return built-in templates
        match template_type {
            TemplateType::PluginGeneration => {
                // 流量分析插件生成 - 使用合并后的完整模板
                Ok("".to_string())
            }
            TemplateType::AgentPluginGeneration => {
                // Agent 工具插件生成 - 使用合并后的完整模板
                Ok("".to_string())
            }
            TemplateType::PluginFix => Ok("".to_string()),
            TemplateType::AgentPluginFix => {
                Ok("".to_string())
            }
        }
    }

    /// Build generation prompt for LLM with Few-shot examples (async version with DB support)
    pub async fn build_generation_prompt_async(
        &self,
        analysis: &WebsiteAnalysis,
        vuln_type: &str,
        target_endpoints: Option<&[String]>,
        requirements: Option<&str>,
    ) -> Result<String> {
        self.build_generation_prompt_with_examples_async(
            analysis,
            vuln_type,
            target_endpoints,
            requirements,
            &[],
        )
        .await
    }

    /// Build generation prompt for LLM with Few-shot examples (sync version, uses built-in templates)
    pub fn build_generation_prompt(
        &self,
        analysis: &WebsiteAnalysis,
        vuln_type: &str,
        target_endpoints: Option<&[String]>,
        requirements: Option<&str>,
    ) -> Result<String> {
        self.build_generation_prompt_with_examples(
            analysis,
            vuln_type,
            target_endpoints,
            requirements,
            &[],
        )
    }

    /// Build fix prompt for LLM to repair broken plugin code (async version with DB support)
    pub async fn build_fix_prompt_async(
        &self,
        original_code: &str,
        error_message: &str,
        error_details: Option<&str>,
        vuln_type: &str,
        attempt: u32,
    ) -> Result<String> {
        // Try to get template from database first
        let base_template = self.get_template_content(TemplateType::PluginFix).await?;

        // Build context for variable replacement
        let mut context = serde_json::json!({
            "original_code": original_code,
            "error_message": error_message,
            "vuln_type": vuln_type,
            "attempt": attempt
        });

        if let Some(details) = error_details {
            context["error_details"] = serde_json::Value::String(details.to_string());
        }

        // Simple variable replacement
        let mut prompt = base_template;
        if let Some(context_obj) = context.as_object() {
            for (key, value) in context_obj {
                let placeholder_curly = format!("{{{}}}", key);
                let placeholder_double = format!("{{{{{}}}}}", key.to_uppercase());
                let replacement = match value {
                    serde_json::Value::String(s) => s.clone(),
                    _ => value.to_string(),
                };
                prompt = prompt.replace(&placeholder_curly, &replacement);
                prompt = prompt.replace(&placeholder_double, &replacement);
            }
        }

        Ok(prompt)
    }

    /// Build fix prompt for LLM to repair broken plugin code (sync version, uses built-in templates)
    pub fn build_fix_prompt(
        &self,
        original_code: &str,
        error_message: &str,
        error_details: Option<&str>,
        vuln_type: &str,
        attempt: u32,
    ) -> Result<String> {
        let mut prompt = String::new();

        // Header
        prompt.push_str("# Plugin Code Fix Task\n\n");
        prompt.push_str("You are an expert TypeScript developer and security researcher. ");
        prompt.push_str("A security plugin was generated but failed execution testing. ");
        prompt.push_str("Your task is to fix the code so it executes correctly.\n\n");

        // Attempt info
        if attempt > 1 {
            prompt.push_str(&format!("**Fix Attempt**: {}\n\n", attempt));
        }

        // Error information
        prompt.push_str("## Error Information\n\n");
        prompt.push_str(&format!("**Error**: {}\n\n", error_message));

        if let Some(details) = error_details {
            prompt.push_str("**Detailed Error**:\n```\n");
            prompt.push_str(details);
            prompt.push_str("\n```\n\n");
        }

        // Original code
        prompt.push_str("## Original Plugin Code\n\n");
        prompt.push_str("```typescript\n");
        prompt.push_str(original_code);
        prompt.push_str("\n```\n\n");

        // Fix instructions
        prompt.push_str("## Fix Instructions\n\n");
        prompt.push_str("Please fix the code to resolve the error. The fixed plugin must:\n\n");
        prompt.push_str("1. **Fix the specific error** mentioned above\n");
        prompt.push_str("2. **Maintain the plugin interface**:\n");
        prompt.push_str("   - `function scan_transaction(transaction)` - scans HTTP transaction for vulnerabilities\n");
        prompt.push_str(&format!(
            "3. **Detect {} vulnerabilities** correctly\n",
            vuln_type
        ));
        prompt.push_str("4. **Use proper TypeScript syntax** - no syntax errors\n");
        prompt.push_str("5. **Return findings array** from scan_transaction function\n");
        prompt.push_str("6. **Include error handling** - use try-catch blocks\n");
        prompt.push_str("7. **Be executable** - the code must run without errors\n\n");

        // Common issues
        prompt.push_str("## Common Issues to Check\n\n");
        prompt.push_str("- Missing or incorrect function signatures\n");
        prompt.push_str("- Undefined variables or functions\n");
        prompt.push_str(
            "- Incorrect API usage (return findings array, not using Deno APIs)\n",
        );
        prompt.push_str("- Syntax errors (missing brackets, semicolons, etc.)\n");
        prompt.push_str("- Type errors in TypeScript\n");
        prompt.push_str("- Accessing undefined properties on context objects\n\n");

        // Output format
        prompt.push_str("## Output Format\n\n");
        prompt.push_str("Return ONLY the fixed TypeScript code, wrapped in a code block:\n\n");
        prompt.push_str("```typescript\n");
        prompt.push_str("// Fixed plugin code here\n");
        prompt.push_str("```\n\n");
        prompt.push_str("Do NOT include explanations, comments about the fix, or any other text outside the code block.\n");

        Ok(prompt)
    }

    /// Build generation prompt with Few-shot examples (async version with DB support)
    ///
    /// 使用合并后的完整模板，模板中已包含接口定义和输出格式要求
    pub async fn build_generation_prompt_with_examples_async(
        &self,
        analysis: &WebsiteAnalysis,
        vuln_type: &str,
        target_endpoints: Option<&[String]>,
        requirements: Option<&str>,
        examples: &[&FewShotExample],
    ) -> Result<String> {
        // 获取合并后的完整模板（已包含接口和输出格式）
        let base_template = self
            .get_template_content(TemplateType::PluginGeneration)
            .await?;

        let mut prompt = base_template;

        // Add dynamic content
        prompt.push_str("\n\n---\n\n## 动态上下文\n\n");

        // Few-shot examples (if provided)
        if !examples.is_empty() {
            prompt.push_str(&self.build_few_shot_examples(examples));
            prompt.push_str("\n\n");
        }

        // Website analysis context
        prompt.push_str(&self.build_analysis_context(analysis));
        prompt.push_str("\n\n");

        // Vulnerability-specific instructions
        prompt.push_str(&self.build_vuln_specific_instructions(vuln_type, analysis));
        prompt.push_str("\n\n");

        // Target endpoints (if specified)
        if let Some(endpoints) = target_endpoints {
            prompt.push_str(&self.build_target_endpoints(endpoints));
            prompt.push_str("\n\n");
        }

        // Additional requirements
        if let Some(reqs) = requirements {
            prompt.push_str("**附加要求**:\n");
            prompt.push_str(reqs);
            prompt.push_str("\n\n");
        }

        Ok(prompt)
    }

    /// Build generation prompt with Few-shot examples (sync version, uses built-in templates)
    pub fn build_generation_prompt_with_examples(
        &self,
        analysis: &WebsiteAnalysis,
        vuln_type: &str,
        target_endpoints: Option<&[String]>,
        requirements: Option<&str>,
        examples: &[&FewShotExample],
    ) -> Result<String> {
        let mut prompt = String::new();

        // Header
        prompt.push_str(&self.build_header());
        prompt.push_str("\n\n");

        // Few-shot examples (if provided)
        if !examples.is_empty() {
            prompt.push_str(&self.build_few_shot_examples(examples));
            prompt.push_str("\n\n");
        }

        // Website analysis context
        prompt.push_str(&self.build_analysis_context(analysis));
        prompt.push_str("\n\n");

        // Vulnerability-specific instructions
        prompt.push_str(&self.build_vuln_specific_instructions(vuln_type, analysis));
        prompt.push_str("\n\n");

        // Target endpoints (if specified)
        if let Some(endpoints) = target_endpoints {
            prompt.push_str(&self.build_target_endpoints(endpoints));
            prompt.push_str("\n\n");
        }

        // Additional requirements
        if let Some(reqs) = requirements {
            prompt.push_str("**Additional Requirements**:\n");
            prompt.push_str(reqs);
            prompt.push_str("\n\n");
        }

        // Plugin template structure
        prompt.push_str(&self.build_plugin_template());
        prompt.push_str("\n\n");

        // Output format instructions
        prompt.push_str(&self.build_output_format());

        Ok(prompt)
    }

    fn build_header(&self) -> String {
        r#"# Security Plugin Generation Task

You are an expert security researcher and TypeScript developer. Your task is to generate a high-quality security testing plugin for a traffic scanning system.

The plugin should:
1. Be written in TypeScript
2. Detect specific vulnerability types based on HTTP traffic analysis
3. Follow the provided plugin interface
4. Include proper error handling and validation
5. Return findings as an array from the scan_transaction function

**IMPORTANT**: Generate GENERIC detection logic that can work across different websites, not just the analyzed target. Use the website analysis as reference for common patterns, but make the detection rules broadly applicable."#.to_string()
    }

    fn build_few_shot_examples(&self, examples: &[&FewShotExample]) -> String {
        let mut section = String::from("## Few-Shot Examples\n\n");
        section.push_str(
            "Here are high-quality examples of similar plugins to guide your implementation:\n\n",
        );

        for (idx, example) in examples.iter().enumerate() {
            section.push_str(&format!(
                "### Example {} - {} Plugin\n\n",
                idx + 1,
                example.vuln_type.to_uppercase()
            ));
            section.push_str(&format!("**Context**: {}\n\n", example.context));
            section.push_str(&format!(
                "**Quality Score**: {:.1}/100\n\n",
                example.quality_score
            ));
            section.push_str("**Implementation**:\n\n");
            section.push_str("```typescript\n");
            section.push_str(&example.code);
            section.push_str("\n```\n\n");

            if idx < examples.len() - 1 {
                section.push_str("---\n\n");
            }
        }

        section.push_str("**Important**: Use these examples as inspiration. Generate GENERIC detection patterns that work across different websites, not just the current target.\n");

        section
    }

    fn build_analysis_context(&self, analysis: &WebsiteAnalysis) -> String {
        let mut context = String::from("## Website Analysis Context\n\n");
        context.push_str("The following analysis is from a sample website. Use it as REFERENCE for common patterns, but generate GENERIC detection logic that works across different websites.\n\n");

        context.push_str(&format!("**Sample Domain**: {}\n", analysis.domain));
        context.push_str(&format!(
            "**Total Requests Analyzed**: {}\n",
            analysis.total_requests
        ));
        context.push_str(&format!(
            "**API Endpoints**: {}\n",
            analysis.api_endpoints_count
        ));
        context.push_str(&format!(
            "**Unique Parameters**: {}\n\n",
            analysis.all_parameters.len()
        ));

        // Technology stack
        context.push_str("**Technology Stack**:\n");
        if let Some(ref server) = analysis.tech_stack.server {
            context.push_str(&format!("- Server: {}\n", server));
        }
        if let Some(ref framework) = analysis.tech_stack.framework {
            context.push_str(&format!("- Framework: {}\n", framework));
        }
        if let Some(ref database) = analysis.tech_stack.database {
            context.push_str(&format!("- Database: {}\n", database));
        }
        if let Some(ref language) = analysis.tech_stack.language {
            context.push_str(&format!("- Language: {}\n", language));
        }

        // Key endpoints (top 10)
        if !analysis.endpoints.is_empty() {
            context.push_str("\n**Key API Endpoints**:\n");
            for (idx, endpoint) in analysis.endpoints.iter().take(10).enumerate() {
                context.push_str(&format!(
                    "{}. {} {} (hits: {})\n",
                    idx + 1,
                    endpoint.method,
                    endpoint.pattern,
                    endpoint.hit_count
                ));

                if !endpoint.query_params.is_empty() {
                    let params: Vec<String> = endpoint
                        .query_params
                        .iter()
                        .take(5)
                        .map(|p| p.name.to_string())
                        .collect();
                    context.push_str(&format!("   Query params: {}\n", params.join(", ")));
                }

                if !endpoint.body_params.is_empty() {
                    let params: Vec<String> = endpoint
                        .body_params
                        .iter()
                        .take(5)
                        .map(|p| p.name.to_string())
                        .collect();
                    context.push_str(&format!("   Body params: {}\n", params.join(", ")));
                }
            }
        }

        // Common parameters
        if !analysis.all_parameters.is_empty() {
            context.push_str("\n**Common Parameters** (for detection patterns):\n");
            let param_names: Vec<String> = analysis
                .all_parameters
                .iter()
                .take(30)
                .map(|p| p.name.clone())
                .collect();
            context.push_str(&format!("{}\n", param_names.join(", ")));
        }

        context
    }

    fn build_vuln_specific_instructions(
        &self,
        vuln_type: &str,
        analysis: &WebsiteAnalysis,
    ) -> String {
        match vuln_type {
            "sqli" => self.build_sqli_instructions(analysis),
            "xss" => self.build_xss_instructions(analysis),
            "idor" | "auth_bypass" => self.build_idor_instructions(analysis),
            "info_leak" => self.build_info_leak_instructions(analysis),
            "csrf" => self.build_csrf_instructions(analysis),
            "file_upload" => self.build_file_upload_instructions(analysis),
            "file_inclusion" => self.build_file_inclusion_instructions(analysis),
            "command_injection" => self.build_command_injection_instructions(analysis),
            "path_traversal" => self.build_path_traversal_instructions(analysis),
            "xxe" => self.build_xxe_instructions(analysis),
            "ssrf" => self.build_ssrf_instructions(analysis),
            _ => format!(
                "## {}\n\nGenerate a plugin to detect {} vulnerabilities.",
                vuln_type, vuln_type
            ),
        }
    }

    fn build_sqli_instructions(&self, analysis: &WebsiteAnalysis) -> String {
        let db_hint = analysis.tech_stack.database.as_deref().unwrap_or("MySQL");

        format!(
            r#"## SQL Injection Detection Requirements

**Vulnerability Type**: SQL Injection (SQLi)
**Target Database**: {} (detected)

**Detection Strategy**:
1. Monitor query parameters and request bodies for SQL injection patterns
2. Check for database error messages in responses
3. Detect SQL keywords in parameters (SELECT, UNION, OR, AND, etc.)
4. Identify dangerous characters (', ", --, ;, /*, */)

**Specific Patterns to Detect**:
- Classic SQL injection: `' OR '1'='1`
- Union-based: `UNION SELECT`
- Time-based blind: `SLEEP()`, `WAITFOR DELAY`, `BENCHMARK()`
- Error-based: Database error messages in response
- Boolean-based: `AND 1=1`, `OR 1=2`

**Database-Specific Errors** ({}):
{}

**Priority Parameters**: Focus on parameters commonly used in database queries: id, user_id, search, query, filter, sort, order"#,
            db_hint,
            db_hint,
            self.get_db_error_patterns(db_hint)
        )
    }

    fn build_xss_instructions(&self, _analysis: &WebsiteAnalysis) -> String {
        r#"## Cross-Site Scripting (XSS) Detection Requirements

**Vulnerability Type**: XSS

**Detection Strategy**:
1. Monitor parameters that might be reflected in responses
2. Check for dangerous HTML tags and JavaScript in parameters
3. Detect unencoded user input in responses
4. Identify JavaScript event handlers (onclick, onerror, etc.)

**Patterns to Detect**:
- Script tags: `<script>`, `</script>`
- Event handlers: `onerror=`, `onclick=`, `onload=`
- JavaScript URLs: `javascript:`
- Data URLs: `data:text/html`
- SVG/XML vectors: `<svg onload=...>`

**Response Analysis**:
- Check if input is reflected without encoding
- Look for Content-Type that allows script execution
- Detect missing XSS protection headers (X-XSS-Protection, CSP)

**Priority Parameters**: Focus on parameters that display user content: comment, message, content, name, title, description"#.to_string()
    }

    fn build_idor_instructions(&self, _analysis: &WebsiteAnalysis) -> String {
        r#"## Authorization Bypass / IDOR Detection Requirements

**Vulnerability Type**: Insecure Direct Object Reference (IDOR) / Authorization Bypass

**Detection Strategy**:
1. Identify endpoints with ID parameters (user_id, account_id, order_id, etc.)
2. Monitor for sequential or predictable IDs
3. Check for missing authorization checks
4. Detect access to resources without proper authentication

**Detection Approach**:
- Track ID parameters in URLs and bodies
- Monitor HTTP status codes (200 when should be 403/401)
- Check for exposure of other users' data
- Detect missing authentication tokens/cookies

**Common IDOR Patterns**:
- Sequential IDs: `/user/1`, `/user/2`, `/user/3`
- UUID/GUID parameters
- Object references in POST/PUT bodies

**Priority Parameters**: id, user_id, account_id, order_id, document_id, file_id"#
            .to_string()
    }

    fn build_info_leak_instructions(&self, analysis: &WebsiteAnalysis) -> String {
        let server = analysis.tech_stack.server.as_deref().unwrap_or("unknown");

        format!(
            r#"## Information Disclosure Detection Requirements

**Vulnerability Type**: Information Leakage
**Detected Server**: {}

**Detection Strategy**:
1. Check response headers for sensitive information
2. Detect error messages with stack traces
3. Identify exposed configuration files
4. Monitor for debug/development endpoints

**Information to Detect**:
- Stack traces and error messages
- Server version disclosure
- Database connection strings
- API keys and secrets
- Internal IP addresses and hostnames
- Directory listings
- Backup files (.bak, .old, .backup)
- Development/debug endpoints (/debug, /test, /_debug)

**Response Header Checks**:
- X-Powered-By (framework disclosure)
- Server (version info)
- X-AspNet-Version
- X-Debug-Token

**Content Patterns**:
- Stack traces (at line, Traceback, Exception)
- SQL errors
- PHP/Python/Java error messages
- Configuration dumps (JSON/XML config)"#,
            server
        )
    }

    fn build_csrf_instructions(&self, _analysis: &WebsiteAnalysis) -> String {
        r#"## CSRF Detection Requirements

**Vulnerability Type**: Cross-Site Request Forgery (CSRF)

**Detection Strategy**:
1. Identify state-changing operations (POST, PUT, DELETE, PATCH)
2. Check for missing CSRF tokens
3. Detect weak CSRF protection (predictable tokens, no SameSite cookies)
4. Monitor for missing Origin/Referer validation

**Detection Approach**:
- Track POST/PUT/DELETE requests
- Check for CSRF token in forms and headers
- Validate token presence and randomness
- Check SameSite cookie attribute
- Verify Origin/Referer headers

**Priority Endpoints**:
- Form submissions
- State-changing API endpoints
- Login/logout operations
- Account management endpoints

**Token Patterns to Check**:
- csrf_token, _csrf, authenticity_token
- X-CSRF-Token header
- Cookie: csrf_token
"#
        .to_string()
    }

    fn build_file_upload_instructions(&self, _analysis: &WebsiteAnalysis) -> String {
        r#"## File Upload Vulnerability Detection Requirements

**Vulnerability Type**: Insecure File Upload

**Detection Strategy**:
1. Identify endpoints that handle file uploads (multipart/form-data, file input fields)
2. Check content-type, filename, and file extension of uploaded files
3. Detect missing or weak server-side validation of file type and size
4. Monitor responses for evidence of uploaded files being executed or served directly

**Patterns to Detect**:
- Dangerous extensions: .php, .jsp, .asp, .aspx, .exe, .sh
- Double extensions: image.jpg.php, file.png.asp
- Uploaded files accessible under /uploads, /files, /images, /static

**Detection Approach**:
- Track requests with multipart/form-data
- Analyze filename and Content-Type headers
- Look for responses that reflect uploaded file URLs without sanitization"#
            .to_string()
    }

    fn build_file_inclusion_instructions(&self, _analysis: &WebsiteAnalysis) -> String {
        r#"## File Inclusion Vulnerability Detection Requirements

**Vulnerability Type**: Local/Remote File Inclusion (LFI/RFI)

**Detection Strategy**:
1. Identify parameters that look like file paths or template names
2. Detect directory traversal sequences (`../`, `..\\`) in parameters
3. Monitor responses for inclusion of unexpected files (e.g., /etc/passwd)

**Patterns to Detect**:
- `../`, `..\\`, `%2e%2e/`, `%2e%2e\\`
- Common sensitive files: `/etc/passwd`, `web.config`, `.htaccess`

**Detection Approach**:
- Analyze query/body parameters for path-like values
- Check responses for known file content signatures"#
            .to_string()
    }

    fn build_command_injection_instructions(&self, _analysis: &WebsiteAnalysis) -> String {
        r#"## Command Injection Detection Requirements

**Vulnerability Type**: OS Command Injection

**Detection Strategy**:
1. Identify endpoints that execute system commands (ping, nslookup, traceroute, backup, etc.)
2. Detect shell metacharacters in parameters: `;`, `&&`, `||`, `|`, backticks
3. Monitor responses for command output or error messages

**Patterns to Detect**:
- Input containing `;`, `&&`, `||`, `|`, `$()`, backticks
- Error messages from shell or OS commands

**Detection Approach**:
- Focus on parameters like host, ip, command, cmd, target
- Analyze responses for echoed command output"#
            .to_string()
    }

    fn build_path_traversal_instructions(&self, _analysis: &WebsiteAnalysis) -> String {
        r#"## Path Traversal Detection Requirements

**Vulnerability Type**: Directory Traversal

**Detection Strategy**:
1. Detect directory traversal sequences in parameters: `../`, `..\\`, encoded variants
2. Monitor file download or view endpoints for access to unexpected directories

**Patterns to Detect**:
- `../`, `..\\`, `%2e%2e/`, `%2e%2e\\`
- Access to /etc/, /var/, C:\Windows\ and other system paths

**Detection Approach**:
- Focus on parameters like path, file, filename, dir, template
- Check for responses that contain file content outside expected directories"#
            .to_string()
    }

    fn build_xxe_instructions(&self, _analysis: &WebsiteAnalysis) -> String {
        r#"## XXE (XML External Entity) Detection Requirements

**Vulnerability Type**: XML External Entity (XXE)

**Detection Strategy**:
1. Identify endpoints that consume XML (Content-Type: application/xml or text/xml)
2. Detect presence of DOCTYPE declarations and external entity definitions

**Patterns to Detect**:
- `<!DOCTYPE`, `<!ENTITY`, `SYSTEM`, `PUBLIC`
- External entity URLs (http://, file://)

**Detection Approach**:
- Analyze request bodies for XML with DOCTYPE
- Check responses for error messages indicating entity resolution"#
            .to_string()
    }

    fn build_ssrf_instructions(&self, _analysis: &WebsiteAnalysis) -> String {
        r#"## SSRF (Server-Side Request Forgery) Detection Requirements

**Vulnerability Type**: Server-Side Request Forgery (SSRF)

**Detection Strategy**:
1. Identify parameters that accept URLs or hostnames
2. Detect access to internal IP ranges (127.0.0.1, 169.254.169.254, 10.x.x.x, 192.168.x.x, etc.)
3. Use fetch API to verify if the target URL is accessible and returns sensitive data

**Patterns to Detect**:
- Parameters named url, target, callback, webhook, feed, endpoint
- URLs pointing to private IPs or cloud metadata services

**Detection Approach**:
- Track requests where server fetches remote resources based on user input
- Analyze responses for content from unexpected internal services
- **Optional**: Use fetch() to verify suspected SSRF by checking if internal URLs are accessible

**Example using fetch API**:
```typescript
// Verify if a suspected SSRF URL returns sensitive data
try {
    const testUrl = extractedUrl; // from request parameter
    if (isInternalIP(testUrl)) {
        const response = await fetch(testUrl, { timeout: 3000 });
        if (response.ok) {
            // Confirmed SSRF - internal resource is accessible
        }
    }
} catch (e) {
    // Network error - might still be SSRF attempt
}
```"#
            .to_string()
    }

    fn get_db_error_patterns(&self, db_type: &str) -> String {
        match db_type.to_lowercase().as_str() {
            "mysql" | "mariadb" => {
                "- MySQL syntax error\n- You have an error in your SQL syntax\n- mysql_fetch"
            }
            "postgresql" | "postgres" => "- PostgreSQL ERROR\n- pg_query()\n- PSQLException",
            "mssql" | "sqlserver" => {
                "- Microsoft SQL Server error\n- ODBC SQL Server Driver\n- SqlException"
            }
            "oracle" => "- ORA-[0-9]{5}\n- Oracle error",
            "mongodb" => "- MongoError\n- MongoDB Error",
            _ => "- SQL syntax error\n- Database error\n- Query failed",
        }
        .to_string()
    }

    fn build_target_endpoints(&self, endpoints: &[String]) -> String {
        let mut text = String::from("**Target Endpoints** (focus detection on these):\n");
        for endpoint in endpoints {
            text.push_str(&format!("- {}\n", endpoint));
        }
        text
    }

    fn build_plugin_template(&self) -> String {
        r#"## Plugin Interface (Required Structure)

Your generated plugin MUST implement the following TypeScript interface:

```typescript
// Analyze HTTP transaction (required)
export function scan_transaction(ctx: HttpTransaction): void {
    // Analyze request + response together
    // Check for vulnerability indicators in response
    // Emit findings if vulnerabilities detected
}

// **CRITICAL**: Export functions to globalThis for plugin engine to call
// Without these exports, the plugin will fail with "Function not found" error
// Use direct assignment without type casting to ensure proper execution
globalThis.scan_transaction = scan_transaction;

// Return findings array example
export async function scan_transaction(transaction) {
    const findings = [];
    
    // When vulnerability is detected, add to findings array
    findings.push({
        title: "SQL Injection Detected",
        vuln_type: "sqli",
        severity: "critical",
        confidence: "high",
        url: transaction.request.url,
        method: transaction.request.method,
        param_name: "paramName",
        param_value: "paramValue",
        evidence: "Evidence text",
        description: "Vulnerability description",
        cwe: "CWE-89",
        owasp: "A03:2021",
        remediation: "Fix suggestion",
    });
    
    return findings;
}
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

**Runtime Environment**: Node.js-compatible JavaScript runtime. You can use standard Node.js APIs.

**IMPORTANT**: Use `require()` for modules, NOT ES6 `import`:
```typescript
// ✅ CORRECT
const fs = require('fs').promises;

// ❌ WRONG
import * as fs from 'fs/promises';  // Will fail!
```

1. **Reporting Findings** - Return findings in output:
```typescript
// For agent tools, include findings in return value
return {
    success: true,
    data: { /* tool results */ },
    findings: [
        {
            title: 'Finding title',
            severity: 'high', // 'critical', 'high', 'medium', 'low', 'info'
            confidence: 'high', // 'high', 'medium', 'low'
            vuln_type: 'sqli',
            evidence: 'Proof of vulnerability',
        }
    ]
};
```

2. **Standard Node.js APIs**:
```typescript
// File operations
const fs = require('fs').promises;
const content = await fs.readFile('/path/to/file.txt', 'utf8');

// HTTP requests (or use fetch())
const response = await fetch('https://example.com/api');
const data = await response.json();

// Buffer for binary data
const bodyText = Buffer.from(ctx.request.body).toString('utf8');

// Crypto
const crypto = require('crypto');
const hash = crypto.createHash('sha256').update('data').digest('hex');
```

3. **Web Standard APIs**:
```typescript
// URL parsing
const url = new URL(ctx.request.url);
const params = new URLSearchParams(url.search);

// Text encoding
const decoder = new TextDecoder();
const bodyText = decoder.decode(new Uint8Array(ctx.request.body));

// Logging
console.log('Processing request...');
```"#.to_string()
    }

    fn build_output_format(&self) -> String {
        r#"## Output Format

Return ONLY the TypeScript plugin code wrapped in a markdown code block:

```typescript
// Your complete plugin code here
export function scan_transaction(ctx: HttpTransaction): void {
    // ...
}

// **CRITICAL**: MUST export all functions to globalThis
// The plugin engine calls functions from globalThis, not from module exports
// Use direct assignment without type casting to ensure proper execution
globalThis.scan_transaction = scan_transaction;
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

Generate the plugin now."#.to_string()
    }
}

impl Default for PromptTemplateBuilder {
    fn default() -> Self {
        Self::new()
    }
}
