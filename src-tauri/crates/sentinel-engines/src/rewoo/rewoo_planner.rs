//! ReWOO Planner 实现 - DISABLED (ai_adapter removed)
//! 
//! 基于 LangGraph ReWOO 标准实现的 Planner 模块
//! 负责生成标准格式的执行计划：Plan: <reasoning> #E1 = Tool[args]

use super::*;
use crate::utils::ordered_message::ChunkType;
use regex::Regex;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use crate::services::prompt_db::PromptRepository;

/// ReWOO Planner - DISABLED
#[derive(Debug)]
pub struct ReWOOPlanner {
    /// Disabled - needs Rig refactor
    _placeholder: (),
}

impl ReWOOPlanner {
    /// 创建新的 ReWOO Planner - DISABLED
    pub fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Ok(Self {
            _placeholder: (),
        })
    }

    /// 生成执行计划 - DISABLED
    pub async fn plan(
        &self,
        _query: &str,
        _available_tools: &[String],
        _context: Option<&str>,
        _execution_id: &str,
    ) -> Result<ReWOOPlan, Box<dyn std::error::Error + Send + Sync>> {
        Err("ReWOO Planner disabled - ai_adapter removed, needs Rig refactor".into())
    }
}

/// ReWOO 执行计划 - DISABLED
#[derive(Debug, Clone)]
pub struct ReWOOPlan {
    pub steps: Vec<ReWOOStep>,
    pub reasoning: String,
    pub execution_id: String,
    pub created_at: SystemTime,
}

/// ReWOO 执行步骤 - DISABLED
#[derive(Debug, Clone)]
pub struct ReWOOStep {
    pub step_id: String,
    pub tool_name: String,
    pub tool_args: HashMap<String, serde_json::Value>,
    pub dependencies: Vec<String>,
    pub description: String,
}