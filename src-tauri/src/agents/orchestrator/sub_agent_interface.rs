use crate::models::security_testing::*;
use serde::{Deserialize, Serialize};
use anyhow::Result;

/// Sub-agent request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubAgentRequest {
    pub kind: SubAgentKind,
    pub session_id: String,
    pub context: SubAgentContext,
}

/// Sub-agent context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubAgentContext {
    pub task_kind: SecurityTaskKind,
    pub primary_target: String,
    pub current_stage: TestStage,
    pub auth_context: AuthContext,
    pub previous_steps: Vec<StepSummary>,
    pub findings: Vec<FindingSummary>,
    pub objective: String,
    pub constraints: Vec<String>,
    /// Task parameters from original AgentTask (includes tools_allow, etc.)
    #[serde(default)]
    pub task_parameters: std::collections::HashMap<String, serde_json::Value>,
}

/// Step summary for context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepSummary {
    pub step_type: TestStepType,
    pub summary: String,
    pub output: Option<String>,
}

/// Finding summary for context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindingSummary {
    pub location: String,
    pub risk_level: RiskImpact,
    pub title: String,
}

/// Sub-agent response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubAgentResponse {
    pub kind: SubAgentKind,
    pub success: bool,
    pub output: SubAgentOutput,
    pub error: Option<String>,
}

/// Sub-agent output
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SubAgentOutput {
    /// ReWOO output: plan with multiple branches
    Plan {
        nodes: Vec<PlanNode>,
        dependencies: Vec<(String, String)>,
        summary: String,
        /// Raw JSON plan from ReWOO Planner (for Execution phase)
        raw_plan: Option<serde_json::Value>,
    },
    /// Plan-and-Execute output: execution results
    Execution {
        steps: Vec<ExecutionStep>,
        final_result: String,
        auth_context_updated: Option<AuthContext>,
    },
    /// LLM-Compiler output: generated code/scripts
    Code {
        language: String,
        code: String,
        explanation: String,
        usage: String,
    },
    /// Generic output
    Generic {
        content: String,
    },
}

/// Plan node from ReWOO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanNode {
    pub id: String,
    pub step_type: TestStepType,
    pub description: String,
    pub dependencies: Vec<String>,
    pub estimated_risk: RiskImpact,
}

/// Execution step from Plan-and-Execute
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStep {
    pub index: usize,
    pub action: String,
    pub result: String,
    pub success: bool,
}

impl SubAgentRequest {
    pub fn new(
        kind: SubAgentKind,
        session_id: String,
        context: SubAgentContext,
    ) -> Self {
        Self {
            kind,
            session_id,
            context,
        }
    }
}

impl SubAgentContext {
    pub fn new(
        task_kind: SecurityTaskKind,
        primary_target: String,
        objective: String,
    ) -> Self {
        Self {
            task_kind,
            primary_target,
            current_stage: TestStage::Recon,
            auth_context: AuthContext::new(),
            previous_steps: Vec::new(),
            findings: Vec::new(),
            objective,
            constraints: Vec::new(),
            task_parameters: std::collections::HashMap::new(),
        }
    }
    
    pub fn with_stage(mut self, stage: TestStage) -> Self {
        self.current_stage = stage;
        self
    }
    
    pub fn with_auth_context(mut self, auth_context: AuthContext) -> Self {
        self.auth_context = auth_context;
        self
    }
    
    pub fn with_previous_steps(mut self, steps: Vec<StepSummary>) -> Self {
        self.previous_steps = steps;
        self
    }
    
    pub fn with_findings(mut self, findings: Vec<FindingSummary>) -> Self {
        self.findings = findings;
        self
    }
    
    pub fn with_constraints(mut self, constraints: Vec<String>) -> Self {
        self.constraints = constraints;
        self
    }
    
    pub fn with_task_parameters(mut self, task_parameters: std::collections::HashMap<String, serde_json::Value>) -> Self {
        self.task_parameters = task_parameters;
        self
    }
}

impl SubAgentResponse {
    pub fn success(kind: SubAgentKind, output: SubAgentOutput) -> Self {
        Self {
            kind,
            success: true,
            output,
            error: None,
        }
    }
    
    pub fn error(kind: SubAgentKind, error: String) -> Self {
        Self {
            kind,
            success: false,
            output: SubAgentOutput::Generic {
                content: String::new(),
            },
            error: Some(error),
        }
    }
}

/// Sub-agent executor trait
#[async_trait::async_trait]
pub trait SubAgentExecutor: Send + Sync {
    async fn execute(&self, request: SubAgentRequest) -> Result<SubAgentResponse>;
}

