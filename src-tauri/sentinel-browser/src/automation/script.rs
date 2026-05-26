use serde::{Deserialize, Serialize};

/// A reusable automation script that can be executed by the engine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationScript {
    pub name: String,
    pub description: String,
    pub steps: Vec<ScriptStep>,
}

/// Individual step in an automation script
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum ScriptStep {
    Navigate { url: String },
    Click { selector: String },
    Type { selector: String, text: String, clear_first: bool },
    Scroll { direction: ScrollDirection, amount: f64 },
    Wait { condition: WaitType, timeout_ms: u64 },
    Screenshot { name: String },
    EvalJs { expression: String, store_as: Option<String> },
    Conditional { condition: String, then_steps: Vec<ScriptStep>, else_steps: Vec<ScriptStep> },
    Loop { times: u32, steps: Vec<ScriptStep> },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ScrollDirection {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WaitType {
    Element(String),
    Navigation,
    NetworkIdle,
    Time,
}
