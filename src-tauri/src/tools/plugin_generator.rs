// Plugin Generator
// Auto-generate passive scanning plugins from templates

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Plugin generation parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginGenerationParams {
    /// Template type (sqli, xss, auth_bypass, info_leak, csrf)
    pub template_type: String,
    /// Target URL (e.g., "https://zeus.imgo.tv")
    pub target_url: String,
    /// Target parameter names (e.g., ["id", "search", "q"])
    pub target_params: Vec<String>,
    /// Detection sensitivity (low, medium, high)
    pub sensitivity: Option<String>,
    /// Custom configuration
    pub custom_config: Option<HashMap<String, String>>,
}

/// Generated plugin result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedPlugin {
    /// Plugin ID
    pub plugin_id: String,
    /// Plugin name
    pub plugin_name: String,
    /// Plugin code (TypeScript)
    pub plugin_code: String,
    /// Plugin metadata
    pub metadata: sentinel_passive::PluginMetadata,
}

/// Plugin generator
pub struct PluginGenerator {
    /// Template directory
    template_dir: std::path::PathBuf,
}

impl PluginGenerator {
    /// Create a new plugin generator
    pub fn new() -> Self {
        // Use templates directory in the project
        let template_dir = std::env::current_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("."))
            .join("sentinel-plugins")
            .join("templates");

        Self { template_dir }
    }

    /// Generate a plugin from template
    pub fn generate_from_template(&self, params: PluginGenerationParams) -> Result<GeneratedPlugin> {
        // Select template file
        let template_filename = match params.template_type.as_str() {
            "sqli" => "sqli_template.ts",
            "xss" => "xss_template.ts",
            "auth_bypass" | "idor" => "auth_bypass_template.ts",
            "info_leak" | "info_disclosure" => "info_leak_template.ts",
            "csrf" => "csrf_template.ts",
            _ => return Err(anyhow!("Unknown template type: {}", params.template_type)),
        };

        let template_path = self.template_dir.join(template_filename);
        
        if !template_path.exists() {
            return Err(anyhow!("Template file not found: {:?}", template_path));
        }

        // Read template
        let template_content = std::fs::read_to_string(&template_path)
            .map_err(|e| anyhow!("Failed to read template file: {}", e))?;

        // Generate plugin ID and name
        let domain = self.extract_domain(&params.target_url);
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let plugin_id = format!("auto_gen_{}_{}_{}", params.template_type, domain, timestamp);
        let plugin_name = format!("{} Scanner for {}", 
            self.get_template_display_name(&params.template_type), 
            domain
        );

        // Fill template placeholders
        let plugin_code = self.fill_template(
            template_content,
            &plugin_id,
            &plugin_name,
            &params,
        )?;

        // Create metadata
        let metadata = self.create_metadata(&plugin_id, &plugin_name, &params);

        Ok(GeneratedPlugin {
            plugin_id: plugin_id.clone(),
            plugin_name: plugin_name.clone(),
            plugin_code,
            metadata,
        })
    }

    /// Fill template with actual values
    fn fill_template(
        &self,
        template: String,
        plugin_id: &str,
        plugin_name: &str,
        params: &PluginGenerationParams,
    ) -> Result<String> {
        let mut result = template;

        // Replace basic placeholders
        result = result.replace("{{PLUGIN_ID}}", plugin_id);
        result = result.replace("{{PLUGIN_NAME}}", plugin_name);
        result = result.replace("{{TARGET_URL}}", &params.target_url);
        result = result.replace(
            "{{DESCRIPTION}}",
            &format!(
                "Auto-generated {} detection plugin for {}",
                self.get_template_display_name(&params.template_type),
                params.target_url
            ),
        );

        // Replace sensitivity
        let sensitivity = params.sensitivity.as_deref().unwrap_or("medium");
        result = result.replace("{{SENSITIVITY}}", sensitivity);

        // Replace target parameters
        let target_params_str = params
            .target_params
            .iter()
            .map(|p| format!("\"{}\"", p))
            .collect::<Vec<_>>()
            .join(", ");
        result = result.replace("{{TARGET_PARAMS}}", &target_params_str);

        // Template-specific replacements
        match params.template_type.as_str() {
            "sqli" => {
                let db_type = params
                    .custom_config
                    .as_ref()
                    .and_then(|c| c.get("db_type"))
                    .map(|s| s.as_str())
                    .unwrap_or("mysql");
                result = result.replace("{{DB_TYPE}}", db_type);
            }
            "auth_bypass" | "idor" => {
                // ID parameters
                let id_params = if params.target_params.is_empty() {
                    vec!["id".to_string(), "user_id".to_string()]
                } else {
                    params.target_params.clone()
                };
                let id_params_str = id_params
                    .iter()
                    .map(|p| format!("\"{}\"", p))
                    .collect::<Vec<_>>()
                    .join(", ");
                result = result.replace("{{ID_PARAMS}}", &id_params_str);

                // Sensitive paths
                let sensitive_paths = params
                    .custom_config
                    .as_ref()
                    .and_then(|c| c.get("sensitive_paths"))
                    .map(|s| s.as_str())
                    .unwrap_or("/api/user, /api/profile, /api/admin");
                let sensitive_paths_str = sensitive_paths
                    .split(',')
                    .map(|p| format!("\"{}\"", p.trim()))
                    .collect::<Vec<_>>()
                    .join(", ");
                result = result.replace("{{SENSITIVE_PATHS}}", &sensitive_paths_str);
            }
            "info_leak" => {
                // Sensitive keywords
                let sensitive_keywords = params
                    .custom_config
                    .as_ref()
                    .and_then(|c| c.get("sensitive_keywords"))
                    .map(|s| s.as_str())
                    .unwrap_or("password, secret, token, api_key");
                let keywords_str = sensitive_keywords
                    .split(',')
                    .map(|k| format!("\"{}\"", k.trim()))
                    .collect::<Vec<_>>()
                    .join(", ");
                result = result.replace("{{SENSITIVE_KEYWORDS}}", &keywords_str);
            }
            "csrf" => {
                // State-changing endpoints
                let state_endpoints = params
                    .custom_config
                    .as_ref()
                    .and_then(|c| c.get("state_changing_endpoints"))
                    .map(|s| s.as_str())
                    .unwrap_or("/api/update, /api/delete, /api/create");
                let endpoints_str = state_endpoints
                    .split(',')
                    .map(|e| format!("\"{}\"", e.trim()))
                    .collect::<Vec<_>>()
                    .join(", ");
                result = result.replace("{{STATE_CHANGING_ENDPOINTS}}", &endpoints_str);
            }
            _ => {}
        }

        Ok(result)
    }

    /// Create plugin metadata
    fn create_metadata(
        &self,
        plugin_id: &str,
        plugin_name: &str,
        params: &PluginGenerationParams,
    ) -> sentinel_passive::PluginMetadata {
        sentinel_passive::PluginMetadata {
            id: plugin_id.to_string(),
            name: plugin_name.to_string(),
            version: "1.0.0".to_string(),
            author: Some("AI Generated".to_string()),
            main_category: "passive".to_string(),
            category: params.template_type.clone(),
            description: Some(format!(
                "Auto-generated {} detection plugin for {}",
                self.get_template_display_name(&params.template_type),
                params.target_url
            )),
            default_severity: self.get_default_severity(&params.template_type),
            tags: vec![
                params.template_type.clone(),
                "auto-generated".to_string(),
                self.extract_domain(&params.target_url),
            ],
        }
    }

    /// Extract domain from URL
    fn extract_domain(&self, url: &str) -> String {
        url::Url::parse(url)
            .ok()
            .and_then(|u| u.host_str().map(|h| h.to_string()))
            .unwrap_or_else(|| {
                // Fallback: simple extraction
                url.split('/')
                    .nth(2)
                    .unwrap_or("unknown")
                    .replace(":", "_")
                    .replace(".", "_")
            })
    }

    /// Get template display name
    fn get_template_display_name(&self, template_type: &str) -> &str {
        match template_type {
            "sqli" => "SQL Injection",
            "xss" => "Cross-Site Scripting (XSS)",
            "auth_bypass" | "idor" => "Authorization Bypass / IDOR",
            "info_leak" | "info_disclosure" => "Information Disclosure",
            "csrf" => "Cross-Site Request Forgery (CSRF)",
            _ => "Unknown",
        }
    }

    /// Get default severity for template type
    fn get_default_severity(&self, template_type: &str) -> sentinel_passive::Severity {
        match template_type {
            "sqli" => sentinel_passive::Severity::Critical,
            "xss" => sentinel_passive::Severity::High,
            "auth_bypass" | "idor" => sentinel_passive::Severity::High,
            "info_leak" | "info_disclosure" => sentinel_passive::Severity::Medium,
            "csrf" => sentinel_passive::Severity::Medium,
            _ => sentinel_passive::Severity::Medium,
        }
    }

    /// List available templates
    pub fn list_templates(&self) -> Vec<String> {
        vec![
            "sqli".to_string(),
            "xss".to_string(),
            "auth_bypass".to_string(),
            "info_leak".to_string(),
            "csrf".to_string(),
        ]
    }
}

impl Default for PluginGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_domain() {
        let gen = PluginGenerator::new();
        assert_eq!(gen.extract_domain("https://example.com/path"), "example.com");
        assert_eq!(gen.extract_domain("http://test.example.com:8080/api"), "test.example.com");
    }

    #[test]
    fn test_template_display_names() {
        let gen = PluginGenerator::new();
        assert_eq!(gen.get_template_display_name("sqli"), "SQL Injection");
        assert_eq!(gen.get_template_display_name("xss"), "Cross-Site Scripting (XSS)");
    }
}

