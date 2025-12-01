//! Advanced plugin generator MCP tools

use async_trait::async_trait;
use sentinel_tools::unified_types::*;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use chrono::Utc;

use crate::generators::{AdvancedPluginGenerator, PluginGenerationRequest, PluginStatus};
use crate::services::ai::AiServiceManager;
use crate::analyzers::WebsiteAnalysis;
use crate::commands::passive_scan_commands::PassiveScanState;

/// Generate advanced plugin tool
pub struct GenerateAdvancedPluginTool {
    generator: Arc<AdvancedPluginGenerator>,
    passive_state: Arc<PassiveScanState>,
    parameters: ToolParameters,
    metadata: ToolMetadata,
}

impl std::fmt::Debug for GenerateAdvancedPluginTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GenerateAdvancedPluginTool")
            .field("parameters", &self.parameters)
            .field("metadata", &self.metadata)
            .finish()
    }
}

impl GenerateAdvancedPluginTool {
    pub fn new(ai_manager: Arc<AiServiceManager>, passive_state: Arc<PassiveScanState>) -> Self {
        let parameters = ToolParameters {
            parameters: vec![
                ParameterDefinition {
                    name: "analysis".to_string(),
                    param_type: ParameterType::Object,
                    description: "Website analysis result from analyze_website tool".to_string(),
                    required: true,
                    default_value: None,
                },
                ParameterDefinition {
                    name: "vuln_types".to_string(),
                    param_type: ParameterType::Array,
                    description: "List of vulnerability types to generate plugins for (e.g., ['sqli', 'xss', 'idor'])".to_string(),
                    required: true,
                    default_value: None,
                },
                ParameterDefinition {
                    name: "target_endpoints".to_string(),
                    param_type: ParameterType::Array,
                    description: "Optional: Specific endpoints to focus on".to_string(),
                    required: false,
                    default_value: None,
                },
                ParameterDefinition {
                    name: "requirements".to_string(),
                    param_type: ParameterType::String,
                    description: "Optional: Additional requirements for plugin generation".to_string(),
                    required: false,
                    default_value: None,
                },
            ],
            schema: json!({
                "type": "object",
                "properties": {
                    "analysis": {
                        "type": "object",
                        "description": "Website analysis result from analyze_website tool"
                    },
                    "vuln_types": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "List of vulnerability types (sqli, xss, idor, info_leak, csrf)"
                    },
                    "target_endpoints": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Optional: Specific endpoints to focus on"
                    },
                    "requirements": {
                        "type": "string",
                        "description": "Optional: Additional requirements"
                    }
                },
                "required": ["analysis", "vuln_types"]
            }),
            required: vec!["analysis".to_string(), "vuln_types".to_string()],
            optional: vec!["target_endpoints".to_string(), "requirements".to_string()],
        };

        let metadata = ToolMetadata {
            author: "Sentinel AI".to_string(),
            version: "1.0.0".to_string(),
            license: "MIT".to_string(),
            homepage: None,
            repository: None,
            tags: vec!["generator".to_string(), "plan-b".to_string(), "ai".to_string(), "security".to_string()],
            install_command: None,
            requirements: vec![],
        };

        let generator = Arc::new(AdvancedPluginGenerator::new(ai_manager));

        Self {
            generator,
            passive_state,
            parameters,
            metadata,
        }
    }
}

#[async_trait]
impl UnifiedTool for GenerateAdvancedPluginTool {
    fn name(&self) -> &str {
        "generate_advanced_plugin"
    }

    fn description(&self) -> &str {
        "Generate security testing plugins using AI based on website analysis. This is Plan B advanced feature that uses LLM to generate intelligent, context-aware detection plugins."
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Analysis
    }

    fn parameters(&self) -> &ToolParameters {
        &self.parameters
    }

    fn metadata(&self) -> &ToolMetadata {
        &self.metadata
    }

    async fn execute(&self, params: ToolExecutionParams) -> anyhow::Result<ToolExecutionResult> {
        let start_time = Utc::now();

        // Parse analysis
        let analysis_value = params.inputs.get("analysis")
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: analysis"))?;
        
        let analysis: WebsiteAnalysis = serde_json::from_value(analysis_value.clone())
            .map_err(|e| anyhow::anyhow!("Failed to parse analysis: {}", e))?;

        // Parse vuln_types
        let vuln_types_value = params.inputs.get("vuln_types")
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: vuln_types"))?;
        
        let raw_vuln_types: Vec<String> = serde_json::from_value(vuln_types_value.clone())
            .map_err(|e| anyhow::anyhow!("Failed to parse vuln_types: {}", e))?;
        
        // 标准化漏洞类型名称
        let vuln_types: Vec<String> = raw_vuln_types.iter()
            .map(|t| Self::normalize_vuln_type(t))
            .collect();

        // Parse optional parameters
        let target_endpoints: Option<Vec<String>> = params.inputs.get("target_endpoints")
            .and_then(|v| serde_json::from_value(v.clone()).ok());

        let requirements: Option<String> = params.inputs.get("requirements")
            .and_then(|v| v.as_str().map(|s| s.to_string()));

        log::debug!(
            "Generating advanced plugins for domain: {}, vuln_types: {:?}",
            analysis.domain,
            vuln_types
        );

        // 检查数据库中是否已有可复用的高质量插件
        let db_service = self.passive_state.get_db_service().await
            .map_err(|e| anyhow::anyhow!("Failed to get database service: {}", e))?;
        
        let mut types_to_generate = Vec::new();
        let mut reused_plugins = Vec::new();
        let min_quality_score = 70.0; // 最低质量分数阈值
        
        for vuln_type in &vuln_types {
            log::debug!("Checking for reusable plugins for type: {}", vuln_type);
            
            match db_service.find_reusable_plugins_by_category(vuln_type, min_quality_score).await {
                Ok(existing_plugins) if !existing_plugins.is_empty() => {
                    // 找到可复用的插件
                    let best_plugin = &existing_plugins[0];
                    let plugin_id = best_plugin.get("id").and_then(|v| v.as_str()).unwrap_or("unknown");
                    let plugin_name = best_plugin.get("name").and_then(|v| v.as_str()).unwrap_or("unknown");
                    let quality_score = best_plugin.get("quality_score").and_then(|v| v.as_f64()).unwrap_or(0.0);
                    
                    log::info!(
                        "Found reusable plugin for {}: {} (ID: {}, Quality: {:.1})",
                        vuln_type, plugin_name, plugin_id, quality_score
                    );
                    
                    reused_plugins.push(best_plugin.clone());
                }
                _ => {
                    // 没有找到合适的插件，需要生成
                    log::info!("No reusable plugin found for {}, will generate new one", vuln_type);
                    types_to_generate.push(vuln_type.clone());
                }
            }
        }
        
        // 如果所有类型都有可复用的插件，直接返回
        if types_to_generate.is_empty() {
            log::info!("All requested plugin types have reusable plugins, skipping generation");
            
            let output_text = format!(
                "✅ All {} plugin types already have high-quality plugins in database.\nReused existing plugins instead of generating new ones.",
                reused_plugins.len()
            );
            
            let end_time = Utc::now();
            let duration = (end_time - start_time).to_std().unwrap_or_default();
            
            return Ok(ToolExecutionResult {
                execution_id: params.execution_id.unwrap_or_else(|| uuid::Uuid::new_v4()),
                tool_name: "generate_advanced_plugin".to_string(),
                tool_id: "generator.generate_advanced_plugin".to_string(),
                success: true,
                output: json!({
                    "plugins": reused_plugins,
                    "summary": output_text,
                    "statistics": {
                        "total": reused_plugins.len(),
                        "reused": reused_plugins.len(),
                        "generated": 0,
                    }
                }),
                error: None,
                execution_time_ms: duration.as_millis() as u64,
                metadata: HashMap::new(),
                started_at: start_time,
                completed_at: Some(end_time),
                status: ExecutionStatus::Completed,
            });
        }
        
        log::info!("Will generate {} new plugins: {:?}", types_to_generate.len(), types_to_generate);

        // Create generation request (只生成需要的类型)
        let request = PluginGenerationRequest {
            analysis,
            vuln_types: types_to_generate,
            target_endpoints,
            requirements,
        };

        // 使用计数器追踪保存和加载状态
        let saved_count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let loaded_count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let auto_approved_count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let reused_count = reused_plugins.len();
        
        let saved_count_clone = saved_count.clone();
        let loaded_count_clone = loaded_count.clone();
        let auto_approved_count_clone = auto_approved_count.clone();
        
        // 克隆 self 的必要字段用于闭包
        let passive_state = self.passive_state.clone();
        
        // Generate plugins with callback - 每生成一个插件就立即保存
        let plugins = self.generator.generate_with_callback(request, move |plugin| {
            let passive_state = passive_state.clone();
            let saved_count = saved_count_clone.clone();
            let loaded_count = loaded_count_clone.clone();
            let auto_approved_count = auto_approved_count_clone.clone();
            
            async move {
                // Save to database
                let save_result = Self::save_plugin_to_db_static(&passive_state, &plugin).await;
                
                match save_result {
                    Ok(_) => {
                        saved_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                        log::info!("Saved plugin {} to database immediately after generation", plugin.plugin_id);
                        
                        // Auto-approve and load if status is Approved
                        if plugin.status == PluginStatus::Approved {
                            match Self::enable_and_load_plugin_static(&passive_state, &plugin.plugin_id).await {
                                Ok(_) => {
                                    loaded_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                                    auto_approved_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                                    log::info!("Auto-approved and loaded plugin {}", plugin.plugin_id);
                                }
                                Err(e) => {
                                    log::error!("Failed to load plugin {}: {}", plugin.plugin_id, e);
                                }
                            }
                        }
                        Ok(())
                    }
                    Err(e) => {
                        log::error!("Failed to save plugin {}: {}", plugin.plugin_id, e);
                        Err(e)
                    }
                }
            }
        }).await?;
        
        // 获取最终计数
        let saved_count = saved_count.load(std::sync::atomic::Ordering::SeqCst);
        let loaded_count = loaded_count.load(std::sync::atomic::Ordering::SeqCst);
        let auto_approved_count = auto_approved_count.load(std::sync::atomic::Ordering::SeqCst);

        // 合并生成的插件和复用的插件
        let total_plugins = plugins.len() + reused_count;
        
        // Build output summary
        let mut output_parts = Vec::new();
        output_parts.push(format!("🤖 AI Plugin Generation Complete"));
        output_parts.push(format!("Total: {} plugins ({} generated, {} reused)", 
            total_plugins, plugins.len(), reused_count));
        output_parts.push(String::new());
        
        // 显示复用的插件
        if !reused_plugins.is_empty() {
            output_parts.push(format!("♻️  Reused Existing High-Quality Plugins:"));
            for (idx, plugin) in reused_plugins.iter().enumerate() {
                let plugin_id = plugin.get("id").and_then(|v| v.as_str()).unwrap_or("unknown");
                let plugin_name = plugin.get("name").and_then(|v| v.as_str()).unwrap_or("unknown");
                let category = plugin.get("category").and_then(|v| v.as_str()).unwrap_or("unknown");
                let quality_score = plugin.get("quality_score").and_then(|v| v.as_f64()).unwrap_or(0.0);
                
                output_parts.push(format!("{}. {} (ID: {})", idx + 1, plugin_name, plugin_id));
                output_parts.push(format!("   Type: {}", category));
                output_parts.push(format!("   Quality Score: {:.1}/100", quality_score));
                output_parts.push(format!("   Status: Already in database"));
            }
            output_parts.push(String::new());
        }
        
        // 显示新生成的插件
        if !plugins.is_empty() {
            output_parts.push(format!("🆕 Newly Generated Plugins:"));
        }

        for (idx, plugin) in plugins.iter().enumerate() {
            output_parts.push(format!("{}. {} (ID: {})", idx + 1, plugin.plugin_name, plugin.plugin_id));
            output_parts.push(format!("   Type: {}", plugin.vuln_type));
            output_parts.push(format!("   Quality Score: {:.1}/100", plugin.quality_score));
            output_parts.push(format!("   Status: {:?}", plugin.status));
            output_parts.push(format!("   Model: {}", plugin.model));
            
            // Quality breakdown
            output_parts.push(format!("   Quality Breakdown:"));
            output_parts.push(format!("     - Syntax: {:.0}%", plugin.quality_breakdown.syntax_score));
            output_parts.push(format!("     - Logic: {:.0}%", plugin.quality_breakdown.logic_score));
            output_parts.push(format!("     - Security: {:.0}%", plugin.quality_breakdown.security_score));
            output_parts.push(format!("     - Code Quality: {:.0}%", plugin.quality_breakdown.code_quality_score));

            // Validation status
            if plugin.validation.is_valid {
                output_parts.push(format!("   ✅ Validation: PASSED"));
            } else {
                output_parts.push(format!("   ❌ Validation: FAILED"));
                for error in &plugin.validation.errors {
                    output_parts.push(format!("      - {}", error));
                }
            }

            if !plugin.validation.warnings.is_empty() {
                output_parts.push(format!("   ⚠️  Warnings:"));
                for warning in &plugin.validation.warnings {
                    output_parts.push(format!("      - {}", warning));
                }
            }

            output_parts.push(String::new());
        }

        // Statistics
        let pending_review = plugins.iter().filter(|p| p.status == PluginStatus::PendingReview).count();
        let validation_failed = plugins.iter().filter(|p| p.status == PluginStatus::ValidationFailed).count();
        let avg_quality: f32 = if !plugins.is_empty() {
            plugins.iter().map(|p| p.quality_score).sum::<f32>() / plugins.len() as f32
        } else {
            0.0
        };

        output_parts.push(format!("📊 Summary:"));
        output_parts.push(format!("   - Total Plugins: {}", total_plugins));
        output_parts.push(format!("   - Newly Generated: {}", plugins.len()));
        output_parts.push(format!("   - Reused Existing: {}", reused_count));
        output_parts.push(format!("   - Saved to Database: {}", saved_count));
        output_parts.push(format!("   - Auto-Approved & Loaded: {}", auto_approved_count));
        output_parts.push(format!("   - Pending Review: {}", pending_review));
        output_parts.push(format!("   - Validation Failed: {}", validation_failed));
        output_parts.push(format!("   - Average Quality: {:.1}/100", avg_quality));
        output_parts.push(String::new());
        output_parts.push(format!("✅ {} plugins are now actively scanning for vulnerabilities!", loaded_count));

        let output_text = output_parts.join("\n");

        let end_time = Utc::now();
        let duration = (end_time - start_time).to_std().unwrap_or_default();

        // 合并所有插件信息用于输出
        let mut all_plugins_output = Vec::new();
        for plugin in reused_plugins {
            all_plugins_output.push(plugin);
        }
        for plugin in &plugins {
            all_plugins_output.push(serde_json::to_value(plugin).unwrap_or_default());
        }

        Ok(ToolExecutionResult {
            execution_id: params.execution_id.unwrap_or_else(|| uuid::Uuid::new_v4()),
            tool_name: "generate_advanced_plugin".to_string(),
            tool_id: "generator.generate_advanced_plugin".to_string(),
            success: true,
            output: json!({
                "plugins": all_plugins_output,
                "summary": output_text,
                "statistics": {
                    "total": total_plugins,
                    "generated": plugins.len(),
                    "reused": reused_count,
                    "pending_review": pending_review,
                    "validation_failed": validation_failed,
                    "average_quality": avg_quality,
                }
            }),
            error: None,
            execution_time_ms: duration.as_millis() as u64,
            metadata: HashMap::new(),
            started_at: start_time,
            completed_at: Some(end_time),
            status: ExecutionStatus::Completed,
        })
    }
}

impl GenerateAdvancedPluginTool {
    /// Save generated plugin to database (static version for use in closures)
    async fn save_plugin_to_db_static(
        passive_state: &Arc<PassiveScanState>,
        plugin: &crate::generators::GeneratedPlugin
    ) -> anyhow::Result<()> {
        use sentinel_passive::{PluginMetadata, Severity};
        
        // Get database service
        let db_service = passive_state.get_db_service().await
            .map_err(|e| anyhow::anyhow!("Failed to get database service: {}", e))?;
        
        // Create plugin metadata
        let metadata = PluginMetadata {
            id: plugin.plugin_id.clone(),
            name: plugin.plugin_name.clone(),
            version: "1.0.0".to_string(),
            author: Some(format!("AI Generator ({})", plugin.model)),
            main_category: "passive".to_string(),
            category: plugin.vuln_type.clone(),
            description: Some(plugin.description.clone()),
            default_severity: Severity::Medium, // Default, will be determined by plugin logic
            tags: vec!["ai-generated".to_string(), plugin.vuln_type.clone()],
        };
        
        // Determine validation status string
        let status_str = match plugin.status {
            PluginStatus::Approved => "Approved",
            PluginStatus::PendingReview => "PendingReview",
            PluginStatus::Rejected => "Rejected",
            PluginStatus::ValidationFailed => "ValidationFailed",
        };
        
        // Save to database with quality score and validation status in one operation
        db_service.register_plugin_with_code_and_quality(
            &metadata, 
            &plugin.code,
            Some(plugin.quality_score as f64),
            Some(status_str)
        ).await
            .map_err(|e| anyhow::anyhow!("Failed to register plugin: {}", e))?;
        
        Ok(())
    }

    /// Save generated plugin to database
    async fn save_plugin_to_db(&self, plugin: &crate::generators::GeneratedPlugin) -> anyhow::Result<()> {
        Self::save_plugin_to_db_static(&self.passive_state, plugin).await
    }

    /// Enable and load plugin into scan engine (static version for use in closures)
    async fn enable_and_load_plugin_static(
        passive_state: &Arc<PassiveScanState>,
        plugin_id: &str
    ) -> anyhow::Result<()> {
        // Get database service
        let db_service = passive_state.get_db_service().await
            .map_err(|e| anyhow::anyhow!("Failed to get database service: {}", e))?;
        
        // Enable plugin in database
        db_service.update_plugin_enabled(plugin_id, true).await
            .map_err(|e| anyhow::anyhow!("Failed to enable plugin: {}", e))?;
        
        // Reload plugins into scan pipeline
        // Note: The scan pipeline will automatically pick up enabled plugins on next scan
        // or we can trigger a reload here if needed
        
        log::info!("Plugin {} enabled and will be loaded on next scan", plugin_id);
        Ok(())
    }

    /// Enable and load plugin into scan engine
    async fn enable_and_load_plugin(&self, plugin_id: &str) -> anyhow::Result<()> {
        Self::enable_and_load_plugin_static(&self.passive_state, plugin_id).await
    }
    
    /// 标准化漏洞类型名称
    fn normalize_vuln_type(vuln_type: &str) -> String {
        let normalized = vuln_type.to_lowercase();
        match normalized.as_str() {
            // SQL 注入变体
            "sql injection" | "sqli" | "sql_injection" | "sqlinjection" => "sqli".to_string(),
            // XSS 变体
            "cross-site scripting" | "cross site scripting" | "xss" | "crosssitescripting" => "xss".to_string(),
            // 路径遍历变体
            "path traversal" | "path_traversal" | "directory traversal" | "lfi" | "local file inclusion" => "path_traversal".to_string(),
            // 命令注入变体
            "command injection" | "command_injection" | "cmd injection" | "os command injection" | "rce" => "command_injection".to_string(),
            // 文件上传变体
            "file upload" | "file_upload" | "fileupload" | "unrestricted file upload" => "file_upload".to_string(),
            // SSRF 变体
            "ssrf" | "server-side request forgery" | "server side request forgery" => "ssrf".to_string(),
            // XXE 变体
            "xxe" | "xml external entity" | "xml_external_entity" => "xxe".to_string(),
            // CSRF 变体
            "csrf" | "cross-site request forgery" | "cross site request forgery" | "xsrf" => "csrf".to_string(),
            // IDOR 变体
            "idor" | "insecure direct object reference" | "broken access control" | "authorization bypass" => "idor".to_string(),
            // 认证绕过变体
            "auth bypass" | "auth_bypass" | "authentication bypass" | "authbypass" => "auth_bypass".to_string(),
            // 信息泄露变体
            "info leak" | "info_leak" | "information disclosure" | "information leak" | "infoleak" => "info_leak".to_string(),
            // 默认保持原样（已经是标准格式或未知类型）
            _ => normalized.replace(" ", "_").replace("-", "_"),
        }
    }
}

/// Generator tool provider
#[derive(Debug)]
pub struct GeneratorToolProvider {
    ai_manager: Arc<AiServiceManager>,
    passive_state: Arc<PassiveScanState>,
}

impl GeneratorToolProvider {
    pub fn new(ai_manager: Arc<AiServiceManager>, passive_state: Arc<PassiveScanState>) -> Self {
        Self { ai_manager, passive_state }
    }
}

#[async_trait]
impl ToolProvider for GeneratorToolProvider {
    fn name(&self) -> &str {
        "generator"
    }

    fn description(&self) -> &str {
        "AI-powered plugin generation tools for Plan B"
    }

    async fn get_tools(&self) -> anyhow::Result<Vec<Arc<dyn UnifiedTool>>> {
        Ok(vec![
            Arc::new(GenerateAdvancedPluginTool::new(self.ai_manager.clone(), self.passive_state.clone())),
        ])
    }

    async fn get_tool(&self, name: &str) -> anyhow::Result<Option<Arc<dyn UnifiedTool>>> {
        match name {
            "generate_advanced_plugin" => Ok(Some(Arc::new(GenerateAdvancedPluginTool::new(self.ai_manager.clone(), self.passive_state.clone())))),
            _ => Ok(None),
        }
    }

    async fn refresh(&self) -> anyhow::Result<()> {
        Ok(())
    }
}

