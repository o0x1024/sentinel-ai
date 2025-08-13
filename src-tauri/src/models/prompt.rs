use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ArchitectureType {
    ReWOO,
    LLMCompiler,
    PlanExecute,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum StageType {
    // ReWOO stages
    Planner,
    Worker,
    Solver,
    // LLMCompiler stages
    Planning,
    Execution,
    Replan,
    // Plan&Execute stages
    Reflection,
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


