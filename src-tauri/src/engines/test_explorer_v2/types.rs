
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageState {
    pub url: String,
    pub title: String,
    pub content: String, // Visible text or simplified HTML
    pub interactive_elements: Option<serde_json::Value>, // From annotate
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub action_type: ActionType,
    pub selector: Option<String>,
    pub index: Option<usize>,
    pub value: Option<String>,
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionType {
    Navigate,
    Click,
    Type,
    Scroll,
    Wait,
    Back,
    Finish,
    Extract, // Extract data
}
