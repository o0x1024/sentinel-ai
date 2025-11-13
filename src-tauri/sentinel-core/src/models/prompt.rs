use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ArchitectureType {
    ReWOO,
    LLMCompiler,
    PlanExecute,
    ReAct,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum StageType {
    // ReWOO stages
    Planner,
    Worker,
    Solver,
    // LLMCompiler & Plan&Execute stages
    Planning,
    Execution,
    Evaluation,  // LLMCompiler Joiner/Evaluator stage
    Replan,
}

/// Prompt category defines the scope and level of the template
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PromptCategory {
    System,
    LlmArchitecture,
    Application,
    UserDefined,
}

/// Template type defines the specific role within the architecture
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TemplateType {
    SystemPrompt,
    IntentClassifier,
    Planner,
    Executor,
    Replanner,
    Evaluator,
    ReportGenerator,
    Domain,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptTemplate {
    pub id: Option<i64>,
    pub name: String,
    pub description: Option<String>,
    pub architecture: ArchitectureType,
    pub stage: StageType,
    pub content: String,
    pub is_default: bool,
    pub is_active: bool,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    // Extended fields for unified prompt system
    pub category: Option<PromptCategory>,
    pub template_type: Option<TemplateType>,
    pub target_architecture: Option<ArchitectureType>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPromptConfig {
    pub id: Option<i64>,
    pub architecture: ArchitectureType,
    pub stage: StageType,
    pub template_id: i64,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptGroup {
    pub id: Option<i64>,
    pub architecture: ArchitectureType,
    pub name: String,
    pub description: Option<String>,
    pub is_default: bool,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptGroupItem {
    pub id: Option<i64>,
    pub group_id: i64,
    pub stage: StageType,
    pub template_id: i64,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}


