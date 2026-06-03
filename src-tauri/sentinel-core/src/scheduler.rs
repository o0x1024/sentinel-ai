//! Scheduler stage configuration

use serde::{Deserialize, Serialize};

/// AI 调度器阶段配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SchedulerStage {
    System,
    Planning,
    Execution,
    Replan,
    IntentClassifier,
}

/// Scheduler configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerConfig {
    pub stages: Vec<SchedulerStage>,
    pub default_stage: Option<SchedulerStage>,
}

