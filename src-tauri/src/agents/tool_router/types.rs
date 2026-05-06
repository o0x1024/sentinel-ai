pub use sentinel_tools::ToolExposure;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 工具元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMetadata {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: ToolCategory,
    pub tags: Vec<String>,
    #[serde(default)]
    pub search_hint: Option<String>,
    pub cost_estimate: ToolCost,
    #[serde(default = "default_exposure")]
    pub exposure: ToolExposure,
}

fn default_exposure() -> ToolExposure {
    ToolExposure::Standard
}

/// 工具分类
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ToolCategory {
    Network,
    Security,
    Data,
    AI,
    System,
    MCP,
    Plugin,
    Workflow,
    Browser,
    Utility,
    Recon,
    Scanning,
    Exploitation,
    Monitoring,
    Traffic,
    Other,
}

impl std::fmt::Display for ToolCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ToolCategory::Network => write!(f, "network"),
            ToolCategory::Security => write!(f, "security"),
            ToolCategory::Data => write!(f, "data"),
            ToolCategory::AI => write!(f, "ai"),
            ToolCategory::System => write!(f, "system"),
            ToolCategory::MCP => write!(f, "mcp"),
            ToolCategory::Plugin => write!(f, "plugin"),
            ToolCategory::Workflow => write!(f, "workflow"),
            ToolCategory::Browser => write!(f, "browser"),
            ToolCategory::Utility => write!(f, "utility"),
            ToolCategory::Recon => write!(f, "recon"),
            ToolCategory::Scanning => write!(f, "scanning"),
            ToolCategory::Exploitation => write!(f, "exploitation"),
            ToolCategory::Monitoring => write!(f, "monitoring"),
            ToolCategory::Traffic => write!(f, "traffic"),
            ToolCategory::Other => write!(f, "other"),
        }
    }
}

/// 工具成本估算
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolCost {
    Low,
    Medium,
    High,
}

/// 工具选择策略
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ToolSelectionStrategy {
    All,
    #[default]
    Keyword,
    LLM,
    Hybrid,
    Manual(Vec<String>),
    Skills(Vec<String>),
    Deferred,
    None,
}

/// 选中的 Skill 摘要
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectedSkill {
    pub id: String,
    pub name: String,
}

/// 工具选择计划
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSelectionPlan {
    pub tool_ids: Vec<String>,
    pub injected_system_prompt: Option<String>,
    pub selected_skill: Option<SelectedSkill>,
}

/// 工具配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolConfig {
    pub selection_strategy: ToolSelectionStrategy,
    pub max_tools: usize,
    pub preselected_tools: Vec<String>,
    pub disabled_tools: Vec<String>,
    #[serde(default)]
    pub allowed_tools: Vec<String>,
    pub enabled: bool,
}

impl Default for ToolConfig {
    fn default() -> Self {
        Self {
            selection_strategy: ToolSelectionStrategy::Keyword,
            max_tools: 5,
            preselected_tools: vec![],
            disabled_tools: vec![],
            allowed_tools: vec![],
            enabled: true,
        }
    }
}

impl ToolConfig {
    pub fn from_json_str(raw: &str) -> serde_json::Result<Self> {
        let value = serde_json::from_str::<serde_json::Value>(raw)?;
        Self::from_json_value(value)
    }

    pub fn from_json_value(value: serde_json::Value) -> serde_json::Result<Self> {
        serde_json::from_value(migrate_tool_config_value(value))
    }
}

fn migrate_tool_config_value(value: serde_json::Value) -> serde_json::Value {
    let Some(mut object) = value.as_object().cloned() else {
        return value;
    };

    if !object.contains_key("preselected_tools") {
        if let Some(legacy) = object.remove("fixed_tools") {
            object.insert("preselected_tools".to_string(), legacy);
        }
    } else {
        object.remove("fixed_tools");
    }

    serde_json::Value::Object(object)
}

/// 工具统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolStatistics {
    pub total_tools: usize,
    pub builtin_tools: usize,
    pub workflow_tools: usize,
    pub mcp_tools: usize,
    pub plugin_tools: usize,
    pub by_category: HashMap<String, usize>,
    pub by_cost: HashMap<String, usize>,
}

/// 工具使用记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUsageRecord {
    pub tool_id: String,
    pub tool_name: String,
    pub execution_id: String,
    pub timestamp: i64,
    pub success: bool,
    pub execution_time_ms: u64,
    pub error_message: Option<String>,
}

/// 工具使用统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUsageStatistics {
    pub total_executions: usize,
    pub successful_executions: usize,
    pub failed_executions: usize,
    pub by_tool: HashMap<String, ToolUsageStats>,
    pub recent_executions: Vec<ToolUsageRecord>,
}

/// 单个工具的使用统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUsageStats {
    pub tool_id: String,
    pub tool_name: String,
    pub execution_count: usize,
    pub success_count: usize,
    pub failure_count: usize,
    pub avg_execution_time_ms: f64,
    pub last_used: i64,
}
