//! Advanced AI-powered plugin generator

use anyhow::{Context, Result};
use chrono::Utc;
use sentinel_llm::LlmClient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::auto_approval::{ApprovalDecision, PluginAutoApprovalConfig, PluginAutoApprovalEngine};
use super::few_shot_examples::FewShotRepository;
use super::prompt_templates::PromptTemplateBuilder;
use super::validator::{ExecutionTestResult, PluginValidator, ValidationResult};
use crate::analyzers::WebsiteAnalysis;
use crate::services::ai::AiServiceManager;
use crate::services::DatabaseService;

/// Maximum number of fix attempts for a failed plugin
const MAX_FIX_ATTEMPTS: u32 = 3;

/// Plugin generation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginGenerationRequest {
    /// Website analysis result
    pub analysis: WebsiteAnalysis,
    /// Vulnerability types to detect (e.g., ["sqli", "xss", "idor"])
    pub vuln_types: Vec<String>,
    /// Target endpoints (optional, if empty use all)
    pub target_endpoints: Option<Vec<String>>,
    /// Additional requirements
    pub requirements: Option<String>,
}

/// Generated plugin with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedPlugin {
    /// Plugin ID
    pub plugin_id: String,
    /// Plugin name
    pub plugin_name: String,
    /// Generated TypeScript code
    pub code: String,
    /// Plugin description
    pub description: String,
    /// Vulnerability type
    pub vuln_type: String,
    /// Quality score (0-100)
    pub quality_score: f32,
    /// Quality breakdown
    pub quality_breakdown: QualityBreakdown,
    /// Validation result
    pub validation: ValidationResult,
    /// Execution test result
    pub execution_test: Option<ExecutionTestResult>,
    /// Number of fix attempts made
    pub fix_attempts: u32,
    /// Status
    pub status: PluginStatus,
    /// Generation timestamp
    pub generated_at: chrono::DateTime<Utc>,
    /// LLM model used
    pub model: String,
}

/// Quality score breakdown
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityBreakdown {
    /// Syntax correctness (0-100)
    pub syntax_score: f32,
    /// Logic completeness (0-100)
    pub logic_score: f32,
    /// Security considerations (0-100)
    pub security_score: f32,
    /// Code quality (0-100)
    pub code_quality_score: f32,
}

/// Plugin status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PluginStatus {
    /// Pending review
    PendingReview,
    /// Approved and ready to use
    Approved,
    /// Rejected
    Rejected,
    /// Failed validation
    ValidationFailed,
}

/// Advanced plugin generator
pub struct AdvancedPluginGenerator {
    ai_manager: Arc<AiServiceManager>,
    db: Option<Arc<DatabaseService>>,
    validator: PluginValidator,
    prompt_builder: PromptTemplateBuilder,
    few_shot_repo: FewShotRepository,
    auto_approval_engine: PluginAutoApprovalEngine,
}

impl AdvancedPluginGenerator {
    pub fn new(ai_manager: Arc<AiServiceManager>) -> Self {
        Self {
            ai_manager,
            db: None,
            validator: PluginValidator::new(),
            prompt_builder: PromptTemplateBuilder::new(),
            few_shot_repo: FewShotRepository::new(),
            auto_approval_engine: PluginAutoApprovalEngine::new(PluginAutoApprovalConfig::default()),
        }
    }

    pub fn new_with_config(
        ai_manager: Arc<AiServiceManager>,
        auto_approval_config: PluginAutoApprovalConfig,
    ) -> Self {
        Self {
            ai_manager,
            db: None,
            validator: PluginValidator::new(),
            prompt_builder: PromptTemplateBuilder::new(),
            few_shot_repo: FewShotRepository::new(),
            auto_approval_engine: PluginAutoApprovalEngine::new(auto_approval_config),
        }
    }

    pub fn new_with_db(
        ai_manager: Arc<AiServiceManager>,
        db: Arc<DatabaseService>,
        auto_approval_config: PluginAutoApprovalConfig,
    ) -> Self {
        Self {
            ai_manager,
            db: Some(db.clone()),
            validator: PluginValidator::new(),
            prompt_builder: PromptTemplateBuilder::new(),
            few_shot_repo: FewShotRepository::new(),
            auto_approval_engine: PluginAutoApprovalEngine::new(auto_approval_config),
        }
    }

    pub fn set_db(&mut self, db: Arc<DatabaseService>) {
        self.db = Some(db.clone());
        self.prompt_builder = PromptTemplateBuilder::new();
    }

    /// Generate plugin using AI
    pub async fn generate(&self, request: PluginGenerationRequest) -> Result<Vec<GeneratedPlugin>> {
        log::info!(
            "Generating plugins for domain: {}, vuln_types: {:?}",
            request.analysis.domain,
            request.vuln_types
        );

        let mut plugins = Vec::new();

        for vuln_type in &request.vuln_types {
            match self.generate_single_plugin(&request, vuln_type).await {
                Ok(plugin) => {
                    log::info!("Generated plugin: {} for {}", plugin.plugin_name, vuln_type);
                    plugins.push(plugin);
                }
                Err(e) => {
                    log::error!("Failed to generate {} plugin: {}", vuln_type, e);
                    // Continue with other types
                }
            }
        }

        log::info!("Successfully generated {} plugins", plugins.len());
        Ok(plugins)
    }

    /// Generate plugin using AI with callback for each generated plugin
    pub async fn generate_with_callback<F, Fut>(
        &self,
        request: PluginGenerationRequest,
        mut on_plugin_generated: F,
    ) -> Result<Vec<GeneratedPlugin>>
    where
        F: FnMut(GeneratedPlugin) -> Fut,
        Fut: std::future::Future<Output = Result<()>>,
    {
        log::info!(
            "Generating plugins for domain: {}, vuln_types: {:?}",
            request.analysis.domain,
            request.vuln_types
        );

        let mut plugins = Vec::new();

        for vuln_type in &request.vuln_types {
            match self.generate_single_plugin(&request, vuln_type).await {
                Ok(plugin) => {
                    log::info!("Generated plugin: {} for {}", plugin.plugin_name, vuln_type);

                    // 立即调用回调保存插件
                    if let Err(e) = on_plugin_generated(plugin.clone()).await {
                        log::error!(
                            "Failed to save plugin {} to database: {}",
                            plugin.plugin_id,
                            e
                        );
                    } else {
                        log::info!("Plugin {} saved to database", plugin.plugin_id);
                    }

                    plugins.push(plugin);
                }
                Err(e) => {
                    log::error!("Failed to generate {} plugin: {}", vuln_type, e);
                    // Continue with other types
                }
            }
        }

        log::info!("Successfully generated and saved {} plugins", plugins.len());
        Ok(plugins)
    }

    /// Generate a single plugin for specific vulnerability type
    async fn generate_single_plugin(
        &self,
        request: &PluginGenerationRequest,
        vuln_type: &str,
    ) -> Result<GeneratedPlugin> {
        log::info!(
            "Generating {} plugin for {} (with Few-shot learning)",
            vuln_type,
            request.analysis.domain
        );

        // 1. Get Few-shot examples for this vulnerability type
        let examples = self.few_shot_repo.get_examples(vuln_type);
        log::debug!(
            "Using {} Few-shot examples for {}",
            examples.len(),
            vuln_type
        );

        // 2. Build generation prompt with examples
        let prompt = self.prompt_builder.build_generation_prompt_with_examples_async(
            &request.analysis,
            vuln_type,
            request.target_endpoints.as_deref(),
            request.requirements.as_deref(),
            &examples,
        ).await?;

        // 3. Call LLM to generate code
        let (code, model) = self.call_llm_for_generation(&prompt).await?;

        // 4. Extract and clean code
        let mut cleaned_code = self.extract_and_clean_code(&code)?;

        // 5. Validate generated code
        let mut validation = self.validator.validate(&cleaned_code).await?;

        // 6. Test plugin execution and fix if needed
        let mut execution_test = self.validator.test_plugin_execution(&cleaned_code).await;
        let mut fix_attempts = 0;

        while !execution_test.success && fix_attempts < MAX_FIX_ATTEMPTS {
            fix_attempts += 1;
            log::warn!(
                "Plugin execution test failed (attempt {}/{}): {}",
                fix_attempts,
                MAX_FIX_ATTEMPTS,
                execution_test
                    .error_message
                    .as_deref()
                    .unwrap_or("Unknown error")
            );

            // Try to fix the plugin using LLM
            match self
                .fix_plugin_code(&cleaned_code, &execution_test, vuln_type, fix_attempts)
                .await
            {
                Ok((fixed_code, _)) => {
                    log::info!("Plugin code fixed, re-validating...");
                    cleaned_code = fixed_code;

                    // Re-validate
                    validation = self.validator.validate(&cleaned_code).await?;

                    // Re-test execution
                    execution_test = self.validator.test_plugin_execution(&cleaned_code).await;

                    if execution_test.success {
                        log::info!(
                            "Plugin execution test passed after fix attempt {}",
                            fix_attempts
                        );
                        break;
                    }
                }
                Err(e) => {
                    log::error!("Failed to fix plugin code: {}", e);
                    break;
                }
            }
        }

        // 7. Calculate quality score
        let quality_breakdown = self.calculate_quality(&cleaned_code, &validation);
        let quality_score = (quality_breakdown.syntax_score
            + quality_breakdown.logic_score
            + quality_breakdown.security_score
            + quality_breakdown.code_quality_score)
            / 4.0;

        // 8. Generate metadata (通用插件，不包含特定网站信息，便于跨站复用)
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let plugin_id = format!("ai_gen_{}_{}", vuln_type, timestamp);
        let plugin_name = format!("{} Detector", self.get_vuln_display_name(vuln_type));

        // 9. Apply auto-approval logic
        let validation_status = if validation.is_valid && execution_test.success {
            "Passed"
        } else {
            "Failed"
        };

        let approval_decision = self.auto_approval_engine.evaluate_plugin(
            quality_score,
            validation_status,
            &cleaned_code,
            fix_attempts,
        );

        // 10. Determine final status based on approval decision and execution test
        let status = if !execution_test.success {
            log::warn!(
                "Plugin {} failed execution test after {} attempts",
                plugin_id,
                fix_attempts
            );
            PluginStatus::ValidationFailed
        } else {
            match approval_decision {
                ApprovalDecision::AutoApprove { reason } => {
                    log::info!("Plugin {} auto-approved: {}", plugin_id, reason);
                    PluginStatus::Approved
                }
                ApprovalDecision::RequireHumanReview { reason } => {
                    log::info!("Plugin {} requires review: {}", plugin_id, reason);
                    PluginStatus::PendingReview
                }
                ApprovalDecision::AutoReject { reason } => {
                    log::info!("Plugin {} auto-rejected: {}", plugin_id, reason);
                    PluginStatus::Rejected
                }
                ApprovalDecision::Regenerate { reason, .. } => {
                    log::info!("Plugin {} should be regenerated: {}", plugin_id, reason);
                    PluginStatus::ValidationFailed
                }
            }
        };

        Ok(GeneratedPlugin {
            plugin_id,
            plugin_name,
            code: cleaned_code,
            description: format!(
                "AI-generated plugin for detecting {} vulnerabilities{}",
                vuln_type,
                if fix_attempts > 0 {
                    format!(" (fixed after {} attempts)", fix_attempts)
                } else {
                    String::new()
                }
            ),
            vuln_type: vuln_type.to_string(),
            quality_score,
            quality_breakdown,
            validation,
            execution_test: Some(execution_test),
            fix_attempts,
            status,
            generated_at: Utc::now(),
            model,
        })
    }

    /// Fix plugin code using LLM
    async fn fix_plugin_code(
        &self,
        original_code: &str,
        execution_test: &ExecutionTestResult,
        vuln_type: &str,
        attempt: u32,
    ) -> Result<(String, String)> {
        log::info!("Attempting to fix plugin code (attempt {})", attempt);

        // Build fix prompt - try async version first (uses DB templates), fallback to sync
        let prompt = match self
            .prompt_builder
            .build_fix_prompt_async(
                original_code,
                execution_test
                    .error_message
                    .as_deref()
                    .unwrap_or("Unknown error"),
                execution_test.error_details.as_deref(),
                vuln_type,
                attempt,
            )
            .await
        {
            Ok(p) => {
                log::info!("Using dynamic fix prompt template from database");
                p
            }
            Err(e) => {
                log::warn!(
                    "Failed to load dynamic fix prompt template: {}, using fallback",
                    e
                );
                self.prompt_builder.build_fix_prompt(
                    original_code,
                    execution_test
                        .error_message
                        .as_deref()
                        .unwrap_or("Unknown error"),
                    execution_test.error_details.as_deref(),
                    vuln_type,
                    attempt,
                )?
            }
        };

        // Call LLM to fix the code
        let (fixed_code, model) = self.call_llm_for_generation(&prompt).await?;

        // Extract and clean fixed code
        let cleaned_code = self.extract_and_clean_code(&fixed_code)?;

        Ok((cleaned_code, model))
    }

    /// Call LLM service for code generation
    async fn call_llm_for_generation(&self, prompt: &str) -> Result<(String, String)> {
        log::debug!("Calling LLM for code generation");

        // 优先使用 UI 配置的默认 LLM 模型
        let service = if let Ok(Some((provider, model_name))) =
            self.ai_manager.get_default_llm_model().await
        {
            log::info!(
                "AdvancedPluginGenerator using default chat model: {}/{}",
                provider,
                model_name
            );

            // 尝试通过 provider 字段查找服务（不区分大小写）
            let provider_lc = provider.to_lowercase();
            let found_service = {
                let services_list = self.ai_manager.list_services();
                let mut found = None;

                for service_name in services_list {
                    if let Some(svc) = self.ai_manager.get_service(&service_name) {
                        let svc_provider = svc.get_config().provider.to_lowercase();
                        // 匹配 provider 字段或服务名称
                        if svc_provider == provider_lc || service_name.to_lowercase() == provider_lc
                        {
                            found = Some(svc);
                            break;
                        }
                    }
                }
                found
            };

            if let Some(service) = found_service {
                log::info!("Found service for provider '{}'", provider);
                service
            } else {
                log::warn!(
                    "Default chat provider '{}' not found in registered services, falling back to 'default' service",
                    provider
                );
                self.ai_manager
                    .get_service("default")
                    .ok_or_else(|| anyhow::anyhow!("No default AI service available"))?
            }
        } else {
            // 如果没有配置默认 Chat 模型，使用 default 服务
            log::info!("No default chat model configured, using 'default' service");
            if let Some(default_service) = self.ai_manager.get_service("default") {
                default_service
            } else {
                // 最后退化到第一个已注册服务
                let service_names = self.ai_manager.list_services();
                let service_name = service_names
                    .first()
                    .ok_or_else(|| anyhow::anyhow!("No AI service available"))?;

                self.ai_manager
                    .get_service(service_name)
                    .ok_or_else(|| anyhow::anyhow!("Failed to get AI service"))?
            }
        };

        // Get model info
        let config = service.get_config();
        let model = config.model.clone();
        log::info!(
            "AdvancedPluginGenerator finally using AI provider '{}' with model '{}'",
            config.provider,
            config.model
        );

        // Build complete prompt with system message
        // Use built-in template
        let full_prompt = {
            log::debug!("No database configured for prompts, using fallback prompt");
            format!(
                "You are an expert security researcher and TypeScript developer. Generate high-quality security testing plugins based on website analysis.\n\n{}",
                prompt
            )
        };

        // 使用 LlmClient 进行非流式调用
        let llm_config = service.service.to_llm_config();
        let llm_client = LlmClient::new(llm_config);

        let content = llm_client
            .completion(None, &full_prompt)
            .await
            .context("Failed to call LLM service")?;

        log::debug!("LLM response received, length: {} chars", content.len());

        Ok((content, model))
    }

    /// Extract and clean TypeScript code from LLM response
    fn extract_and_clean_code(&self, response: &str) -> Result<String> {
        // Try to extract code from markdown code blocks
        if let Some(code) = self.extract_from_markdown(response) {
            return Ok(code);
        }

        // Try to extract from JSON
        if let Some(code) = self.extract_from_json(response) {
            return Ok(code);
        }

        // If no code block found, treat entire response as code
        Ok(response.trim().to_string())
    }

    /// Extract code from markdown code blocks
    fn extract_from_markdown(&self, text: &str) -> Option<String> {
        // Look for ```typescript or ```ts code blocks
        let patterns = ["```typescript\n", "```ts\n", "```\n"];

        for pattern in &patterns {
            if let Some(start_pos) = text.find(pattern) {
                let code_start = start_pos + pattern.len();
                if let Some(end_pos) = text[code_start..].find("\n```") {
                    return Some(text[code_start..code_start + end_pos].trim().to_string());
                }
            }
        }

        None
    }

    /// Extract code from JSON response
    fn extract_from_json(&self, text: &str) -> Option<String> {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(text) {
            // Try common field names
            for field in &["code", "plugin_code", "typescript", "content"] {
                if let Some(code) = json.get(field).and_then(|v| v.as_str()) {
                    return Some(code.to_string());
                }
            }
        }
        None
    }

    /// Calculate quality score
    fn calculate_quality(&self, code: &str, validation: &ValidationResult) -> QualityBreakdown {
        // Syntax score (from validation)
        let syntax_score = if validation.syntax_valid { 100.0 } else { 0.0 };

        // Logic score (check for required functions and patterns)
        let logic_score = self.calculate_logic_score(code);

        // Security score (check for security best practices)
        let security_score = self.calculate_security_score(code);

        // Code quality score (readability, comments, structure)
        let code_quality_score = self.calculate_code_quality_score(code);

        QualityBreakdown {
            syntax_score,
            logic_score,
            security_score,
            code_quality_score,
        }
    }

    /// Calculate logic completeness score
    fn calculate_logic_score(&self, code: &str) -> f32 {
        let mut score: f32 = 0.0;
        let checks = [
            ("get_metadata", 20.0),
            ("scan_transaction", 50.0),
            ("return", 20.0),
            ("vuln_type:", 10.0),
        ];

        for (pattern, points) in &checks {
            if code.contains(pattern) {
                score += points;
            }
        }

        score.min(100.0)
    }

    /// Calculate security score
    fn calculate_security_score(&self, code: &str) -> f32 {
        let mut score: f32 = 100.0;

        // Deduct points for security issues
        let issues = [
            ("eval(", 30.0),
            ("Function(", 30.0),
            ("dangerouslySetInnerHTML", 20.0),
            (".innerHTML", 15.0),
        ];

        for (pattern, penalty) in &issues {
            if code.contains(pattern) {
                score -= penalty;
                log::warn!("Security issue detected: {}", pattern);
            }
        }

        score.max(0.0)
    }

    /// Calculate code quality score
    fn calculate_code_quality_score(&self, code: &str) -> f32 {
        let mut score: f32 = 50.0; // Base score

        // Check for comments
        if code.contains("//") || code.contains("/*") {
            score += 15.0;
        }

        // Check for type annotations
        if code.matches(": string").count() > 2 {
            score += 10.0;
        }

        // Check for error handling
        if code.contains("try") && code.contains("catch") {
            score += 15.0;
        }

        // Check for constants/config
        if code.contains("const CONFIG") || code.contains("const PATTERNS") {
            score += 10.0;
        }

        score.min(100.0)
    }

    /// Get display name for vulnerability type
    fn get_vuln_display_name<'a>(&self, vuln_type: &'a str) -> &'a str {
        match vuln_type {
            "sqli" => "SQL Injection",
            "xss" => "Cross-Site Scripting (XSS)",
            "idor" => "Insecure Direct Object Reference",
            "auth_bypass" => "Authorization Bypass",
            "info_leak" => "Information Disclosure",
            "csrf" => "Cross-Site Request Forgery",
            "ssrf" => "Server-Side Request Forgery",
            "xxe" => "XML External Entity",
            _ => vuln_type,
        }
    }
}
