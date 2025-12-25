use serde::{Deserialize, Serialize};

/// Prompt category defines the scope and level of the template
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PromptCategory {
    System,        // 系统级（包含所有系统提示）
    Application,   // 应用级（插件生成/修复等）
    UserDefined,   // 用户自定义
}

/// Template type defines the specific role within the architecture
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TemplateType {
    SystemPrompt,
    IntentClassifier,
    Domain,
    Custom,
    // Plugin generation templates (合并后的完整模板)
    PluginGeneration,        // 流量分析插件生成（合并了 interface 和 output format）
    AgentPluginGeneration,   // Agent 工具插件生成（合并了 interface 和 output format）
    PluginFix,               // 流量分析插件修复
    AgentPluginFix,          // Agent 插件修复
    PluginVulnSpecific,      // 漏洞特定插件模板
    // Vision Explorer templates
    VisionExplorerVision,    // VisionExplorer 多模态模型提示
    VisionExplorerText,      // VisionExplorer 文本模型提示
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptTemplate {
    pub id: Option<i64>,
    pub name: String,
    pub description: Option<String>,
    pub content: String,
    pub is_default: bool,
    pub is_active: bool,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    // Extended fields for unified prompt system
    pub category: Option<PromptCategory>,
    pub template_type: Option<TemplateType>,
    #[serde(default)]
    pub is_system: bool,
    #[serde(default = "default_priority")]
    pub priority: i32,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub variables: Vec<String>,
    #[serde(default = "default_version")]
    pub version: String,
}

fn default_priority() -> i32 {
    50
}

fn default_version() -> String {
    "1.0.0".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_type_serde() {
        // Test all plugin-related template types
        let test_cases = vec![
            ("PluginGeneration", TemplateType::PluginGeneration),
            ("AgentPluginGeneration", TemplateType::AgentPluginGeneration),
            ("PluginFix", TemplateType::PluginFix),
            ("AgentPluginFix", TemplateType::AgentPluginFix),
            ("PluginVulnSpecific", TemplateType::PluginVulnSpecific),
            ("VisionExplorerVision", TemplateType::VisionExplorerVision),
            ("VisionExplorerText", TemplateType::VisionExplorerText),
        ];

        for (json_str, expected) in test_cases {
            let json = format!("\"{}\"", json_str);
            let deserialized: TemplateType = serde_json::from_str(&json)
                .expect(&format!("Failed to deserialize {}", json_str));
            assert_eq!(deserialized, expected, "Mismatch for {}", json_str);
            
            let serialized = serde_json::to_string(&expected)
                .expect(&format!("Failed to serialize {:?}", expected));
            assert_eq!(serialized, json, "Serialization mismatch for {:?}", expected);
        }
    }
}


