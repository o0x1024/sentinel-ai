//! Token usage tracking and statistics
//!
//! 提供 token 使用统计和成本估算功能

use serde::{Deserialize, Serialize};

/// Token 使用统计
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    /// 输入 token 数量
    pub input_tokens: u32,
    /// 输出 token 数量
    pub output_tokens: u32,
    /// 总 token 数量
    pub total_tokens: u32,
    /// 估算成本（美元）
    pub estimated_cost: f64,
}

impl TokenUsage {
    /// 创建新的 token 使用统计
    pub fn new(input_tokens: u32, output_tokens: u32) -> Self {
        Self {
            input_tokens,
            output_tokens,
            total_tokens: input_tokens + output_tokens,
            estimated_cost: 0.0,
        }
    }

    /// 设置成本
    pub fn with_cost(mut self, cost: f64) -> Self {
        self.estimated_cost = cost;
        self
    }

    /// 估算成本（基于模型定价）
    pub fn estimate_cost(&mut self, provider: &str, model: &str) {
        self.estimated_cost = calculate_cost(provider, model, self.input_tokens, self.output_tokens);
    }

    /// 合并另一个使用统计
    pub fn merge(&mut self, other: &TokenUsage) {
        self.input_tokens += other.input_tokens;
        self.output_tokens += other.output_tokens;
        self.total_tokens += other.total_tokens;
        self.estimated_cost += other.estimated_cost;
    }
}

/// 计算成本（美元）
///
/// 基于各提供商的公开定价
pub fn calculate_cost(provider: &str, model: &str, input_tokens: u32, output_tokens: u32) -> f64 {
    let provider_lower = provider.to_lowercase();
    let model_lower = model.to_lowercase();

    // 价格单位：美元 / 1M tokens
    let (input_price, output_price) = match provider_lower.as_str() {
        "openai" => match model_lower.as_str() {
            m if m.contains("gpt-4o-mini") => (0.15, 0.6),
            m if m.contains("gpt-4o") => (2.5, 10.0),
            m if m.contains("gpt-4-turbo") => (10.0, 30.0),
            m if m.contains("gpt-4") => (30.0, 60.0),
            m if m.contains("gpt-3.5-turbo") => (0.5, 1.5),
            m if m.contains("o1-preview") => (15.0, 60.0),
            m if m.contains("o1-mini") => (3.0, 12.0),
            m if m.contains("o3-mini") => (1.1, 4.4),
            _ => (0.0, 0.0),
        },
        "anthropic" => match model_lower.as_str() {
            m if m.contains("claude-3-5-sonnet") => (3.0, 15.0),
            m if m.contains("claude-3-5-haiku") => (0.8, 4.0),
            m if m.contains("claude-3-opus") => (15.0, 75.0),
            m if m.contains("claude-3-sonnet") => (3.0, 15.0),
            m if m.contains("claude-3-haiku") => (0.25, 1.25),
            _ => (0.0, 0.0),
        },
        "gemini" | "google" => match model_lower.as_str() {
            m if m.contains("gemini-2.0-flash-exp") => (0.0, 0.0), // Free tier
            m if m.contains("gemini-1.5-pro") => (1.25, 5.0),
            m if m.contains("gemini-1.5-flash") => (0.075, 0.3),
            m if m.contains("gemini-1.0-pro") => (0.5, 1.5),
            _ => (0.0, 0.0),
        },
        "deepseek" => match model_lower.as_str() {
            m if m.contains("deepseek-chat") => (0.14, 0.28),
            m if m.contains("deepseek-reasoner") => (0.55, 2.19),
            _ => (0.0, 0.0),
        },
        "groq" => {
            // Groq 提供免费额度，但有速率限制
            (0.0, 0.0)
        },
        "ollama" => {
            // 本地模型无成本
            (0.0, 0.0)
        },
        "openrouter" => {
            // OpenRouter 价格因模型而异，这里提供一些常见模型的估算
            match model_lower.as_str() {
                m if m.contains("claude") => (3.0, 15.0),
                m if m.contains("gpt-4") => (10.0, 30.0),
                m if m.contains("llama-3.1-405b") => (2.7, 2.7),
                m if m.contains("llama-3.1-70b") => (0.59, 0.79),
                _ => (0.0, 0.0),
            }
        },
        "moonshot" => match model_lower.as_str() {
            m if m.contains("moonshot-v1-8k") => (12.0, 12.0),
            m if m.contains("moonshot-v1-32k") => (24.0, 24.0),
            m if m.contains("moonshot-v1-128k") => (60.0, 60.0),
            _ => (0.0, 0.0),
        },
        "xai" => match model_lower.as_str() {
            m if m.contains("grok-beta") => (5.0, 15.0),
            _ => (0.0, 0.0),
        },
        "perplexity" => match model_lower.as_str() {
            m if m.contains("sonar-pro") => (3.0, 15.0),
            m if m.contains("sonar") => (1.0, 1.0),
            _ => (0.0, 0.0),
        },
        "togetherai" => {
            // TogetherAI 价格因模型而异
            (0.2, 0.2) // 平均估算
        },
        "cohere" => match model_lower.as_str() {
            m if m.contains("command-r-plus") => (3.0, 15.0),
            m if m.contains("command-r") => (0.5, 1.5),
            _ => (0.0, 0.0),
        },
        _ => (0.0, 0.0),
    };

    // 计算成本：(tokens / 1,000,000) * price_per_million
    let input_cost = (input_tokens as f64 / 1_000_000.0) * input_price;
    let output_cost = (output_tokens as f64 / 1_000_000.0) * output_price;

    input_cost + output_cost
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_usage_creation() {
        let usage = TokenUsage::new(100, 200);
        assert_eq!(usage.input_tokens, 100);
        assert_eq!(usage.output_tokens, 200);
        assert_eq!(usage.total_tokens, 300);
    }

    #[test]
    fn test_cost_calculation() {
        // GPT-4o-mini: $0.15 / 1M input, $0.6 / 1M output
        let cost = calculate_cost("openai", "gpt-4o-mini", 1_000_000, 1_000_000);
        assert!((cost - 0.75).abs() < 0.01);

        // Claude 3.5 Sonnet: $3 / 1M input, $15 / 1M output
        let cost = calculate_cost("anthropic", "claude-3-5-sonnet", 1_000_000, 1_000_000);
        assert!((cost - 18.0).abs() < 0.01);

        // Ollama (free)
        let cost = calculate_cost("ollama", "llama3.2", 1_000_000, 1_000_000);
        assert_eq!(cost, 0.0);
    }

    #[test]
    fn test_merge_usage() {
        let mut usage1 = TokenUsage::new(100, 200).with_cost(0.01);
        let usage2 = TokenUsage::new(50, 100).with_cost(0.005);
        
        usage1.merge(&usage2);
        
        assert_eq!(usage1.input_tokens, 150);
        assert_eq!(usage1.output_tokens, 300);
        assert_eq!(usage1.total_tokens, 450);
        assert!((usage1.estimated_cost - 0.015).abs() < 0.0001);
    }
}
