//! ReWOO Solver 实现 - DISABLED (ai_adapter removed)

use super::*;
use std::collections::HashMap;
use std::sync::Arc;

/// ReWOO Solver - DISABLED
#[derive(Debug)]
pub struct ReWOOSolver {
    _placeholder: (),
}

impl ReWOOSolver {
    /// 创建新的 ReWOO Solver - DISABLED
    pub fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Ok(Self {
            _placeholder: (),
        })
    }

    /// 执行计划步骤 - DISABLED
    pub async fn solve_step(
        &self,
        _step: &ReWOOStep,
        _context: &HashMap<String, String>,
        _execution_id: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        Err("ReWOO Solver disabled - ai_adapter removed, needs Rig refactor".into())
    }

    /// 生成最终答案 - DISABLED
    pub async fn generate_final_answer(
        &self,
        _query: &str,
        _evidence: &HashMap<String, String>,
        _execution_id: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        Err("ReWOO Solver disabled - ai_adapter removed, needs Rig refactor".into())
    }
}

use super::rewoo_planner::{ReWOOStep};