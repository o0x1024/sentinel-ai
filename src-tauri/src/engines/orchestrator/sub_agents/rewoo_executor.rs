use crate::agents::orchestrator::sub_agent_interface::*;
use crate::models::security_testing::*;
use crate::engines::rewoo::engine_adapter::ReWooEngine;
use crate::engines::rewoo::rewoo_types::ReWOOConfig;
use crate::services::AiServiceManager;
use crate::services::database::DatabaseService;
use crate::agents::traits::AgentTask;
use anyhow::Result;
use std::sync::Arc;
use std::collections::HashMap;

/// ReWOO sub-agent executor
pub struct ReWOOSubAgentExecutor {
    ai_service_manager: Arc<AiServiceManager>,
    db_service: Arc<DatabaseService>,
}

impl ReWOOSubAgentExecutor {
    pub fn new(
        ai_service_manager: Arc<AiServiceManager>,
        db_service: Arc<DatabaseService>,
    ) -> Self {
        Self {
            ai_service_manager,
            db_service,
        }
    }
}

// Removed Default implementation - requires dependencies

impl ReWOOSubAgentExecutor {
    /// Build security-focused prompt for ReWOO
    /// Checks for Orchestrator's prompt configuration first, falls back to default if not found
    fn build_security_prompt(&self, context: &SubAgentContext) -> String {
        // Try to get planner prompt from Orchestrator's configuration
        log::info!("ReWOO sub-agent: Checking for prompt in task_parameters");
        log::info!("ReWOO sub-agent: task_parameters keys: {:?}", context.task_parameters.keys().collect::<Vec<_>>());
        
        if let Some(prompts) = context.task_parameters.get("prompts") {
            log::info!("ReWOO sub-agent: Found 'prompts' in task_parameters");
            log::info!("ReWOO sub-agent: prompts value type: {:?}", prompts);
            
            if let Some(planner_prompt) = prompts.get("planner").and_then(|v| v.as_str()) {
                log::info!("ReWOO sub-agent: Found 'planner' prompt, length: {}", planner_prompt.len());
                if !planner_prompt.trim().is_empty() {
                    log::info!("ReWOO sub-agent: Using planner prompt from database (via Orchestrator)");
                    // Build context variables for prompt
                    let context_vars = self.build_context_variables(context);
                    // Replace variables in prompt
                    return self.replace_prompt_variables(planner_prompt, &context_vars);
                } else {
                    log::warn!("ReWOO sub-agent: Planner prompt is empty");
                }
            } else {
                log::warn!("ReWOO sub-agent: 'planner' key not found in prompts or not a string");
            }
        } else {
            log::warn!("ReWOO sub-agent: 'prompts' key not found in task_parameters");
        }
        
        // Fallback to default hardcoded prompt
        log::info!("ReWOO sub-agent: Using default hardcoded prompt as fallback");
        self.build_default_security_prompt(context)
    }
    
    /// Build default security prompt (original hardcoded logic)
    fn build_default_security_prompt(&self, context: &SubAgentContext) -> String {
        let task_context = match context.task_kind {
            SecurityTaskKind::WebPentest | SecurityTaskKind::APIPentest => {
                format!(
                    "You are planning a security penetration test for: {}\n\
                    Current stage: {:?}\n\
                    Authentication available: {}\n\
                    Previous findings: {} vulnerabilities discovered",
                    context.primary_target,
                    context.current_stage,
                    context.auth_context.is_authenticated(),
                    context.findings.len()
                )
            }
            SecurityTaskKind::Forensics => {
                format!(
                    "You are planning a forensics analysis for: {}\n\
                    Current stage: {:?}\n\
                    Previous findings: {} indicators extracted",
                    context.primary_target,
                    context.current_stage,
                    context.findings.len()
                )
            }
            SecurityTaskKind::CTF => {
                format!(
                    "You are planning a CTF challenge solution for: {}\n\
                    Current stage: {:?}\n\
                    Previous findings: {} clues discovered",
                    context.primary_target,
                    context.current_stage,
                    context.findings.len()
                )
            }
            SecurityTaskKind::ReverseEngineering => {
                format!(
                    "You are planning reverse engineering analysis for: {}\n\
                    Current stage: {:?}\n\
                    Previous findings: {} behaviors identified",
                    context.primary_target,
                    context.current_stage,
                    context.findings.len()
                )
            }
            SecurityTaskKind::OtherSecurity => {
                format!(
                    "You are planning security analysis for: {}\n\
                    Current stage: {:?}",
                    context.primary_target,
                    context.current_stage
                )
            }
        };
        
        let mut prompt = format!(
            "{}\n\n\
            Objective: {}\n\n",
            task_context,
            context.objective
        );
        
        if !context.previous_steps.is_empty() {
            prompt.push_str("Previous steps completed:\n");
            for step in &context.previous_steps {
                prompt.push_str(&format!("- {:?}: {}\n", step.step_type, step.summary));
            }
            prompt.push_str("\n");
        }
        
        if !context.findings.is_empty() {
            prompt.push_str("Key findings so far:\n");
            for finding in &context.findings {
                prompt.push_str(&format!(
                    "- [{:?}] {}: {}\n",
                    finding.risk_level, finding.location, finding.title
                ));
            }
            prompt.push_str("\n");
        }
        
        if !context.constraints.is_empty() {
            prompt.push_str("Constraints:\n");
            for constraint in &context.constraints {
                prompt.push_str(&format!("- {}\n", constraint));
            }
            prompt.push_str("\n");
        }
        
        prompt.push_str(
            "Create a comprehensive multi-branch plan with:\n\
            1. Clear task decomposition\n\
            2. Dependencies between tasks\n\
            3. Risk assessment for each branch\n\
            4. Parallel execution opportunities where safe\n\n\
            Focus on actionable, specific steps."
        );
        
        prompt
    }
    
    /// Build context variables for prompt replacement
    fn build_context_variables(&self, context: &SubAgentContext) -> HashMap<String, String> {
        let mut vars = HashMap::new();
        vars.insert("primary_target".to_string(), context.primary_target.clone());
        vars.insert("objective".to_string(), context.objective.clone());
        vars.insert("task_kind".to_string(), format!("{:?}", context.task_kind));
        vars.insert("current_stage".to_string(), format!("{:?}", context.current_stage));
        vars.insert("is_authenticated".to_string(), context.auth_context.is_authenticated().to_string());
        vars.insert("findings_count".to_string(), context.findings.len().to_string());
        vars.insert("steps_count".to_string(), context.previous_steps.len().to_string());
        
        // Build previous steps summary
        if !context.previous_steps.is_empty() {
            let steps_summary = context.previous_steps.iter()
                .map(|s| format!("- {:?}: {}", s.step_type, s.summary))
                .collect::<Vec<_>>()
                .join("\n");
            vars.insert("previous_steps".to_string(), steps_summary);
        } else {
            vars.insert("previous_steps".to_string(), "None".to_string());
        }
        
        // Build findings summary
        if !context.findings.is_empty() {
            let findings_summary = context.findings.iter()
                .map(|f| format!("- [{:?}] {}: {}", f.risk_level, f.location, f.title))
                .collect::<Vec<_>>()
                .join("\n");
            vars.insert("findings".to_string(), findings_summary);
        } else {
            vars.insert("findings".to_string(), "None".to_string());
        }
        
        vars
    }
    
    /// Replace variables in prompt template
    fn replace_prompt_variables(&self, template: &str, vars: &HashMap<String, String>) -> String {
        let mut result = template.to_string();
        for (key, value) in vars {
            let placeholder_curly = format!("{{{}}}", key);
            let placeholder_double = format!("{{{{{}}}}}", key.to_uppercase());
            result = result.replace(&placeholder_curly, value);
            result = result.replace(&placeholder_double, value);
        }
        result
    }
    
    /// Parse ReWOO output into plan nodes
    fn parse_rewoo_output(&self, output: &str) -> Vec<PlanNode> {
        // This is a simplified parser - in production, you'd use the actual ReWOO output format
        let mut nodes = Vec::new();
        
        // For now, create a basic node structure
        // In real implementation, parse the ReWOO plan format
        nodes.push(PlanNode {
            id: uuid::Uuid::new_v4().to_string(),
            step_type: TestStepType::PlanSecurityTest,
            description: "Plan created by ReWOO".to_string(),
            dependencies: Vec::new(),
            estimated_risk: RiskImpact::Low,
        });
        
        nodes
    }
}

#[async_trait::async_trait]
impl SubAgentExecutor for ReWOOSubAgentExecutor {
    async fn execute(&self, request: SubAgentRequest) -> Result<SubAgentResponse> {
        log::info!(
            "ReWOO sub-agent executing (PLANNING ONLY): session={}, objective={}",
            request.session_id,
            request.context.objective
        );
        
        // Create ReWOO Planner directly (not the full engine)
        use crate::engines::rewoo::rewoo_planner::ReWOOPlanner;
        use crate::engines::rewoo::rewoo_types::PlannerConfig;
        use crate::services::prompt_db::PromptRepository;
        
        let db = self.db_service.get_db()?;
        let prompt_repo = Arc::new(PromptRepository::new(db.pool().clone()));
        
        let planner_config = PlannerConfig {
            model_name: "deepseek-chat".to_string(),
            temperature: 0.0,
            max_tokens: 8000,
            max_steps: 20,
        };
        let framework_adapter = crate::tools::get_framework_adapter(crate::tools::FrameworkType::ReWOO).await?;
        
        let mut planner = ReWOOPlanner::new(
            self.ai_service_manager.clone(),
            prompt_repo,
            planner_config,
            framework_adapter.clone(),
        )?;
        
        // Set runtime params with tool permissions from context
        let mut runtime_params = HashMap::new();
        
        // Read tools_allow from context.task_parameters
        if let Some(tools_allow) = request.context.task_parameters.get("tools_allow") {
            runtime_params.insert("tools_allow".to_string(), tools_allow.clone());
        } else {
            log::warn!("ReWOO sub-agent: No tools_allow in task parameters, using empty list");
            runtime_params.insert("tools_allow".to_string(), serde_json::json!([]));
        }
        
        // Also pass tools_deny if present
        if let Some(tools_deny) = request.context.task_parameters.get("tools_deny") {
            runtime_params.insert("tools_deny".to_string(), tools_deny.clone());
        }
        
        // Pass custom system prompt from Orchestrator if available
        if let Some(prompts) = request.context.task_parameters.get("prompts") {
            if let Some(planner_prompt) = prompts.get("planner").and_then(|v| v.as_str()) {
                log::info!("ReWOO sub-agent: Using custom system prompt from Orchestrator");
                let context_vars = self.build_context_variables(&request.context);
                let rendered_prompt = self.replace_prompt_variables(planner_prompt, &context_vars);
                runtime_params.insert("custom_system_prompt".to_string(), serde_json::json!(rendered_prompt));
            }
        }
        
        planner.set_runtime_params(runtime_params);
        
        // Get available tools
        let available_tools = framework_adapter.list_available_tools().await;
        
        // Call planner to generate plan (PLANNING ONLY - NO EXECUTION)
        let execution_id = uuid::Uuid::new_v4().to_string();
        log::info!("ReWOO sub-agent: Calling planner for plan generation only");
        let plan = planner.plan(
            &request.context.objective,
            &available_tools,
            None,
            &execution_id,
        ).await?;
        
        log::info!("ReWOO sub-agent: Generated plan with {} steps (no execution)", plan.steps.len());
        
        // Parse the plan.reasoning (which contains the JSON) to extract plan_summary
        let raw_plan_json: serde_json::Value = if let Ok(v) = serde_json::from_str(&plan.reasoning) {
            v
        } else {
            // Fallback: construct JSON from parsed steps
            serde_json::json!({
                "plan_summary": format!("Generated security test plan with {} steps", plan.steps.len()),
                "steps": plan.steps.iter().map(|s| {
                    serde_json::json!({
                        "id": s.step_id.trim_start_matches('#'),
                        "tool": s.tool_name,
                        "args": s.tool_args,
                        "depends_on": s.dependencies.iter().map(|d| d.trim_start_matches('#')).collect::<Vec<_>>(),
                        "description": s.description
                    })
                }).collect::<Vec<_>>()
            })
        };
        
        let plan_summary = raw_plan_json.get("plan_summary")
            .and_then(|v| v.as_str())
            .unwrap_or("Generated security test plan")
            .to_string();
        
        // Convert ReWOO steps to PlanNodes for compatibility
        let mut nodes = Vec::new();
        let mut dependencies = Vec::new();
        
        for step in &plan.steps {
            nodes.push(PlanNode {
                id: step.step_id.clone(),
                step_type: TestStepType::PlanSecurityTest, // Will be refined based on tool
                description: format!("[{}] {}", step.tool_name, step.description),
                dependencies: step.dependencies.clone(),
                estimated_risk: RiskImpact::Medium, // Default, can be refined
            });
            
            // Record dependencies
            for dep in &step.dependencies {
                dependencies.push((step.step_id.clone(), dep.clone()));
            }
        }
        
        log::info!("ReWOO sub-agent: Planning completed with {} nodes", nodes.len());
        
        Ok(SubAgentResponse::success(
            SubAgentKind::ReWOO,
            SubAgentOutput::Plan {
                nodes,
                dependencies,
                summary: plan_summary,
                raw_plan: Some(raw_plan_json),
            },
        ))
    }
}

