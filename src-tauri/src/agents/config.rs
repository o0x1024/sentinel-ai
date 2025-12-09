//! Agent 配置
//!
//! 定义 Agent 系统的配置选项

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Agent 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// 最大迭代次数
    pub max_iterations: usize,
    /// 执行超时（秒）
    pub timeout_secs: u64,
    /// 是否启用反思
    pub enable_reflection: bool,
    /// 是否自动创建 Todos
    pub auto_create_todos: bool,
    /// 强制创建 Todos（忽略复杂度判断）
    pub force_todos: bool,
    /// 是否启用流式输出
    pub enable_streaming: bool,
    /// LLM 配置
    pub llm: LlmConfig,
    /// 工具配置
    pub tools: ToolConfig,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            max_iterations: 10,
            timeout_secs: 300, // 5 minutes
            enable_reflection: true,
            auto_create_todos: true,
            force_todos: false,
            enable_streaming: true,
            llm: LlmConfig::default(),
            tools: ToolConfig::default(),
        }
    }
}

impl AgentConfig {
    pub fn timeout(&self) -> Duration {
        Duration::from_secs(self.timeout_secs)
    }

    /// 创建简单任务配置（禁用 Todos）
    pub fn simple() -> Self {
        Self {
            max_iterations: 5,
            auto_create_todos: false,
            enable_reflection: false,
            ..Default::default()
        }
    }

    /// 创建复杂任务配置
    pub fn complex() -> Self {
        Self {
            max_iterations: 20,
            timeout_secs: 600, // 10 minutes
            force_todos: true,
            ..Default::default()
        }
    }
}

/// LLM 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    /// 模型名称
    pub model: String,
    /// 温度
    pub temperature: f32,
    /// 最大输出 tokens
    pub max_tokens: usize,
    /// 是否启用思考
    pub enable_thinking: bool,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            model: "gpt-4".to_string(),
            temperature: 0.7,
            max_tokens: 4096,
            enable_thinking: true,
        }
    }
}

/// 工具配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolConfig {
    /// 单个工具执行超时（秒）
    pub tool_timeout_secs: u64,
    /// 是否需要确认危险操作
    pub require_confirmation: bool,
    /// 启用的工具类别
    pub enabled_categories: Vec<String>,
    /// 禁用的工具列表
    pub disabled_tools: Vec<String>,
}

impl Default for ToolConfig {
    fn default() -> Self {
        Self {
            tool_timeout_secs: 60,
            require_confirmation: true,
            enabled_categories: vec![
                "reconnaissance".to_string(),
                "scanning".to_string(),
                "utility".to_string(),
            ],
            disabled_tools: vec![],
        }
    }
}

impl ToolConfig {
    pub fn tool_timeout(&self) -> Duration {
        Duration::from_secs(self.tool_timeout_secs)
    }

    pub fn is_tool_enabled(&self, tool_name: &str) -> bool {
        !self.disabled_tools.contains(&tool_name.to_string())
    }

    /// 允许所有工具
    pub fn allow_all() -> Self {
        Self {
            require_confirmation: false,
            enabled_categories: vec![
                "reconnaissance".to_string(),
                "scanning".to_string(),
                "exploitation".to_string(),
                "post_exploitation".to_string(),
                "code_analysis".to_string(),
                "baseline_check".to_string(),
                "remediation".to_string(),
                "utility".to_string(),
            ],
            disabled_tools: vec![],
            ..Default::default()
        }
    }
}

// Note: PlannerConfig is defined in planner.rs

/// Reflector 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflectorConfig {
    /// 最小反思迭代间隔
    pub min_iterations_between_reflections: usize,
    /// 是否在错误后强制反思
    pub reflect_on_error: bool,
    /// 重规划阈值（失败步骤比例）
    pub replan_threshold: f32,
}

impl Default for ReflectorConfig {
    fn default() -> Self {
        Self {
            min_iterations_between_reflections: 1,
            reflect_on_error: true,
            replan_threshold: 0.3, // 30% 失败率触发重规划
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AgentConfig::default();
        assert_eq!(config.max_iterations, 10);
        assert!(config.enable_streaming);
    }

    #[test]
    fn test_simple_config() {
        let config = AgentConfig::simple();
        assert!(!config.auto_create_todos);
        assert_eq!(config.max_iterations, 5);
    }

    #[test]
    fn test_tool_enabled() {
        let config = ToolConfig::default();
        assert!(config.is_tool_enabled("port_scan"));
        
        let config_with_disabled = ToolConfig {
            disabled_tools: vec!["dangerous_tool".to_string()],
            ..Default::default()
        };
        assert!(!config_with_disabled.is_tool_enabled("dangerous_tool"));
    }
}

