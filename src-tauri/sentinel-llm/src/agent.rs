//! Agent 辅助模块
//!
//! 提供 LLM 配置验证和提供商映射功能。

use anyhow::{anyhow, Result};

use crate::config::LlmConfig;

/// 验证 LLM 配置
pub fn validate_config(config: &LlmConfig) -> Result<()> {
    let provider_lc = config.provider.to_lowercase();

    // 检查提供商是否已配置
    if provider_lc == "unconfigured" || provider_lc == "mock" {
        return Err(anyhow!(
            "AI provider not configured. Please go to Settings > AI Configuration"
        ));
    }

    // 检查 API Key（某些提供商不需要）
    let api_key_required = match provider_lc.as_str() {
        "ollama" | "lm studio" | "lmstudio" | "lm_studio" => false,
        _ => true,
    };

    if api_key_required && config.api_key.as_ref().is_none_or(|k| k.is_empty()) {
        return Err(anyhow!(
            "API key not configured for provider '{}'. Please check your AI configuration settings.",
            config.provider
        ));
    }

    Ok(())
}

/// 获取 rig 使用的提供商名称（映射兼容提供商）
pub fn get_rig_provider(provider: &str) -> String {
    match provider.to_lowercase().as_str() {
        "lm studio" | "lmstudio" | "lm_studio" => "openai".to_string(),
        other => other.to_string(),
    }
}

/// 检查提供商是否需要 Gemini 特殊配置
pub fn needs_gemini_config(provider: &str) -> bool {
    matches!(provider.to_lowercase().as_str(), "gemini" | "google")
}

