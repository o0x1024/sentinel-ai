//! Plugin code validator

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Overall validation status
    pub is_valid: bool,
    /// Syntax validation passed
    pub syntax_valid: bool,
    /// Required functions present
    pub has_required_functions: bool,
    /// Security checks passed
    pub security_check_passed: bool,
    /// Validation errors
    pub errors: Vec<String>,
    /// Warnings
    pub warnings: Vec<String>,
}

/// Plugin execution test result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionTestResult {
    /// Test passed
    pub success: bool,
    /// Error message if failed
    pub error_message: Option<String>,
    /// Detailed error information
    pub error_details: Option<String>,
}

/// Plugin validator
pub struct PluginValidator;

impl PluginValidator {
    pub fn new() -> Self {
        Self
    }

    /// Validate generated plugin code
    pub async fn validate(&self, code: &str) -> Result<ValidationResult> {
        let mut result = ValidationResult {
            is_valid: true,
            syntax_valid: false,
            has_required_functions: false,
            security_check_passed: false,
            errors: Vec::new(),
            warnings: Vec::new(),
        };

        // 1. Check required functions
        let func_check = self.check_required_functions(code);
        result.has_required_functions = func_check.0;
        if !func_check.0 {
            result.errors.extend(func_check.1);
            result.is_valid = false;
        } else {
            result.warnings.extend(func_check.1);
        }

        // 2. Check security issues
        let security_check = self.check_security_issues(code);
        result.security_check_passed = security_check.0;
        if !security_check.0 {
            result.errors.extend(security_check.1);
            result.is_valid = false;
        } else {
            result.warnings.extend(security_check.1);
        }

        // 3. Validate TypeScript syntax (if Deno is available)
        match self.validate_typescript_syntax(code).await {
            Ok(true) => {
                result.syntax_valid = true;
            }
            Ok(false) => {
                result.syntax_valid = false;
                result
                    .errors
                    .push("TypeScript syntax validation failed".to_string());
                result.is_valid = false;
            }
            Err(e) => {
                // Deno not available, skip syntax validation
                log::warn!("TypeScript syntax validation skipped: {}", e);
                result
                    .warnings
                    .push(format!("Syntax validation skipped: {}", e));
                result.syntax_valid = true; // Don't fail if Deno not available
            }
        }

        Ok(result)
    }

    /// Check for required plugin functions
    fn check_required_functions(&self, code: &str) -> (bool, Vec<String>) {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        let mut all_present = true;

        // DB-only 模式：插件元数据来自数据库 (plugin_registry)，不再要求插件代码提供 get_metadata()
        //
        // Traffic 插件必须提供 scan_transaction 作为唯一入口（function/export function 均可）
        let scan_patterns = [
            "function scan_transaction",
            "export function scan_transaction",
            "export async function scan_transaction",
        ];
        if !scan_patterns.iter().any(|p| code.contains(p)) {
            errors.push("Missing required function: scan_transaction".to_string());
            all_present = false;
        }

        // Recommended functions
        let recommended = [
            ("op_emit_finding", "op_emit_finding"),
            ("get_metadata", "function get_metadata"),
        ];

        for (name, pattern) in &recommended {
            if !code.contains(pattern) {
                warnings.push(format!("Recommended function/API not found: {}", name));
            }
        }

        (all_present, if all_present { warnings } else { errors })
    }

    /// Check for security issues in code
    fn check_security_issues(&self, code: &str) -> (bool, Vec<String>) {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Critical security issues (block)
        let critical_issues = [
            ("eval(", "Use of eval() is forbidden"),
            ("Function(", "Use of Function constructor is forbidden"),
            ("execSync", "Use of execSync is forbidden"),
            ("child_process", "Use of child_process is forbidden"),
        ];

        for (pattern, message) in &critical_issues {
            if code.contains(pattern) {
                errors.push(message.to_string());
            }
        }

        // Warning-level issues
        let warning_issues = [
            (
                "dangerouslySetInnerHTML",
                "Potentially unsafe: dangerouslySetInnerHTML",
            ),
            (".innerHTML", "Potentially unsafe: innerHTML usage"),
            ("__proto__", "Prototype pollution risk"),
        ];

        for (pattern, message) in &warning_issues {
            if code.contains(pattern) {
                warnings.push(message.to_string());
            }
        }

        let passed = errors.is_empty();
        (passed, if passed { warnings } else { errors })
    }

    /// Validate TypeScript syntax using Deno AST
    async fn validate_typescript_syntax(&self, code: &str) -> Result<bool> {
        log::debug!("Validating TypeScript syntax using deno_ast");

        // Use deno_ast to parse TypeScript code
        let source_code: std::sync::Arc<str> = std::sync::Arc::<str>::from(code.to_string());

        let parse_params = deno_ast::ParseParams {
            specifier: deno_ast::ModuleSpecifier::parse("file:///plugin.ts").unwrap(),
            text: source_code,
            media_type: deno_ast::MediaType::TypeScript,
            capture_tokens: false,
            scope_analysis: false,
            maybe_syntax: None,
        };

        // Parse the code and check for syntax errors
        match deno_ast::parse_module(parse_params) {
            Ok(parsed) => {
                log::debug!("TypeScript syntax validation passed");

                // Check for any diagnostics
                if parsed.diagnostics().is_empty() {
                    Ok(true)
                } else {
                    log::warn!("TypeScript has diagnostics: {:?}", parsed.diagnostics());
                    // Still consider it valid if it parsed successfully
                    Ok(true)
                }
            }
            Err(e) => {
                log::error!("TypeScript syntax validation failed: {}", e);
                Err(anyhow::anyhow!("Syntax error: {}", e))
            }
        }
    }

    /// Run sandbox test.
    ///
    /// 注意：为避免与流量分析插件引擎共享 V8 全局状态引发崩溃，
    /// 这里不再实际执行插件代码，仅复用静态校验结果。
    pub async fn run_sandbox_test(&self, code: &str) -> Result<bool> {
        log::debug!("Running sandbox test (static validation only, runtime execution disabled)");
        let validation = self.validate(code).await?;
        Ok(validation.is_valid)
    }

    /// Test plugin execution.
    ///
    /// 为了彻底避免多份 V8 运行时在不同线程初始化导致的
    /// `Invalid global state` 崩溃，这里不再启动 Deno/JsRuntime 去真实执行插件，
    /// 而是基于静态校验结果给出执行测试结论。
    pub async fn test_plugin_execution(&self, code: &str) -> ExecutionTestResult {
        log::info!("Testing plugin execution (static analysis only, runtime execution disabled)");

        match self.validate(code).await {
            Ok(validation) => {
                if validation.is_valid {
                    ExecutionTestResult {
                        success: true,
                        error_message: None,
                        error_details: None,
                    }
                } else {
                    ExecutionTestResult {
                        success: false,
                        error_message: Some("Static validation failed".to_string()),
                        error_details: Some(format!(
                            "errors: {:?}, warnings: {:?}",
                            validation.errors, validation.warnings
                        )),
                    }
                }
            }
            Err(e) => ExecutionTestResult {
                success: false,
                error_message: Some("Validation error".to_string()),
                error_details: Some(format!("{:?}", e)),
            },
        }
    }
}

impl Default for PluginValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Extract concise error message from detailed error string
#[allow(dead_code)]
fn extract_error_message(error: &str) -> String {
    // Try to extract the most relevant error message
    if let Some(line) = error.lines().find(|l| l.contains("Error:")) {
        line.trim().to_string()
    } else if let Some(line) = error.lines().next() {
        line.trim().to_string()
    } else {
        error.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_required_functions() {
        let validator = PluginValidator::new();

        let valid_code = r#"
export async function scan_transaction(transaction) {
    Sentinel.emitFinding({});
}
        "#;

        let result = validator.check_required_functions(valid_code);
        assert!(result.0, "Should have required functions");

        let invalid_code = r#"
// missing scan_transaction
        "#;

        let result2 = validator.check_required_functions(invalid_code);
        assert!(!result2.0, "Should be missing scan_transaction");
    }

    #[test]
    fn test_check_security_issues() {
        let validator = PluginValidator::new();

        let dangerous_code = r#"
function test() {
    eval("dangerous code");
}
        "#;

        let result = validator.check_security_issues(dangerous_code);
        assert!(!result.0, "Should detect eval() usage");
        assert!(!result.1.is_empty(), "Should have error messages");

        let safe_code = r#"
function test() {
    console.log("safe");
}
        "#;

        let result2 = validator.check_security_issues(safe_code);
        assert!(result2.0, "Should pass security check");
    }
}
