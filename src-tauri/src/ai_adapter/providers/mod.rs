//! AI提供商模块
//! 
//! 包含各种AI服务提供商的实现

pub mod openai;
// pub mod anthropic;
pub mod gemini;
// pub mod ollama;
pub mod deepseek;
pub mod moonshot;
pub mod openrouter;
pub mod modelscope;
// pub mod zhipu;
// pub mod cohere;
// pub mod groq;
// pub mod xai;

pub mod base;

// 重新导出提供商
pub use openai::OpenAiProvider;
// pub use anthropic::AnthropicProvider;
// pub use ollama::OllamaProvider;
// pub use groq::GroqProvider;
// pub use xai::XaiProvider;
pub use gemini::GeminiProvider;
pub use deepseek::DeepSeekProvider;
pub use moonshot::MoonshotProvider;
pub use openrouter::OpenRouterProvider;
pub use modelscope::ModelScopeProvider;
// pub use zhipu::ZhipuProvider;
// pub use cohere::CohereProvider;


use crate::ai_adapter::types::ProviderConfig;
use crate::ai_adapter::error::{AiAdapterError, Result};
use crate::ai_adapter::types::AiProvider;
use std::sync::Arc;

/// 提供商工厂
pub struct ProviderFactory;

impl ProviderFactory {
    /// 根据配置创建提供商实例
    pub fn create(config: ProviderConfig) -> Result<Arc<dyn AiProvider>> {
        // 将提供商名称转换为小写以支持大小写不敏感匹配
        let provider_name = config.name.to_lowercase();
        match provider_name.as_str() {
            // "openai" => {
            //     let provider = OpenAiProvider::new(config)?;
            //     Ok(Arc::new(provider))
            // },
            // "anthropic" => {
            //     let provider = AnthropicProvider::new(config)?;
            //     Ok(Arc::new(provider))
            // },

            // "ollama" => {
            //     let provider = OllamaProvider::new(config)?;
            //     Ok(Arc::new(provider))
            // },

            // "groq" => {
            //     let provider = GroqProvider::new(config)?;
            //     Ok(Arc::new(provider))
            // },
            // "xai" => {
            //     let provider = XaiProvider::new(config)?;
            //     Ok(Arc::new(provider))
            // },
            "gemini" => {
                let provider = GeminiProvider::new(config)?;
                Ok(Arc::new(provider))
            },
            "deepseek" => {
                let provider = DeepSeekProvider::new(config)?;
                Ok(Arc::new(provider))
            },
            "moonshot" => {
                let provider = MoonshotProvider::new(config)?;
                Ok(Arc::new(provider))
            },
            "openrouter" => {
                let provider = OpenRouterProvider::new(config)?;
                Ok(Arc::new(provider))
            },
            "modelscope" => {
                let provider = ModelScopeProvider::new(config)?;
                Ok(Arc::new(provider))
            },
            _ => Err(AiAdapterError::ProviderNotSupportedError(
                format!("Unsupported provider: {}", config.name)
            ))
        }
    }
    
    /// 获取支持的提供商列表
    pub fn supported_providers() -> Vec<&'static str> {
        vec![
            "openai",
            "anthropic", 
            "gemini", 
            "ollama", 
            "deepseek",
            "moonshot",
            "openrouter",
            "modelscope",
            // "zhipu",
            // "cohere",
            "groq",
            "xai"
        ]
    }
    
    /// 检查提供商是否支持
    pub fn is_supported(name: &str) -> bool {
        let name_lower = name.to_lowercase();
        Self::supported_providers().contains(&name_lower.as_str())
    }
}