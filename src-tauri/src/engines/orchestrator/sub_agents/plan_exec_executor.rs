use crate::agents::orchestrator::sub_agent_interface::*;
use crate::models::security_testing::*;
use crate::engines::plan_and_execute::engine_adapter::PlanAndExecuteEngine;
use crate::engines::plan_and_execute::types::PlanAndExecuteConfig;
use crate::services::AiServiceManager;
use crate::services::database::DatabaseService;
use crate::agents::traits::{AgentTask, ExecutionEngine};
use anyhow::Result;
use std::sync::Arc;
use std::collections::HashMap;

/// Plan-and-Execute sub-agent executor
pub struct PlanExecSubAgentExecutor {
    ai_service_manager: Arc<AiServiceManager>,
    db_service: Arc<DatabaseService>,
}

impl PlanExecSubAgentExecutor {
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

impl PlanExecSubAgentExecutor {
    /// Build security-focused prompt for Plan-and-Execute
    /// Checks for Orchestrator's prompt configuration first, falls back to default if not found
    fn build_security_prompt(&self, context: &SubAgentContext) -> String {
        // Try to get executor prompt from Orchestrator's configuration
        if let Some(prompts) = context.task_parameters.get("prompts") {
            if let Some(executor_prompt) = prompts.get("executor").and_then(|v| v.as_str()) {
                if !executor_prompt.trim().is_empty() {
                    log::info!("Plan-and-Execute sub-agent: Using executor prompt from Orchestrator configuration");
                    // Build context variables for prompt
                    let context_vars = self.build_context_variables(context);
                    // Replace variables in prompt
                    return self.replace_prompt_variables(executor_prompt, &context_vars);
                }
            }
        }
        
        // Fallback to default hardcoded prompt
        log::info!("Plan-and-Execute sub-agent: Using default hardcoded prompt");
        self.build_default_security_prompt(context)
    }
    
    /// Build default security prompt (original hardcoded logic)
    fn build_default_security_prompt(&self, context: &SubAgentContext) -> String {
        let mut prompt = format!(
            "Execute the following security testing objective: {}\n\n\
            Target: {}\n\
            Current stage: {:?}\n",
            context.objective,
            context.primary_target,
            context.current_stage
        );
        
        // Add authentication context if available
        if context.auth_context.is_authenticated() {
            prompt.push_str("\nAuthentication context available:\n");
            if !context.auth_context.cookies.is_empty() {
                prompt.push_str(&format!("- Cookies: {} items\n", context.auth_context.cookies.len()));
            }
            if !context.auth_context.tokens.is_empty() {
                prompt.push_str(&format!("- Tokens: {} items\n", context.auth_context.tokens.len()));
            }
            if !context.auth_context.headers.is_empty() {
                prompt.push_str(&format!("- Headers: {} items\n", context.auth_context.headers.len()));
            }
            prompt.push_str("Use this authentication context in all requests.\n");
        } else {
            prompt.push_str("\nNo authentication context available. If login is required, perform authentication first.\n");
        }
        
        // Add previous context
        if !context.previous_steps.is_empty() {
            prompt.push_str("\nPrevious steps completed:\n");
            for step in context.previous_steps.iter().rev().take(5) {
                prompt.push_str(&format!("- {:?}: {}\n", step.step_type, step.summary));
            }
        }
        
        // Add findings context
        if !context.findings.is_empty() {
            prompt.push_str("\nPrevious findings:\n");
            for finding in context.findings.iter().rev().take(3) {
                prompt.push_str(&format!(
                    "- [{:?}] {}\n",
                    finding.risk_level, finding.title
                ));
            }
        }
        
        // Add constraints
        if !context.constraints.is_empty() {
            prompt.push_str("\nConstraints:\n");
            for constraint in &context.constraints {
                prompt.push_str(&format!("- {}\n", constraint));
            }
        }
        
        prompt.push_str(
            "\nExecute this objective step by step:\n\
            1. Break down into specific actions\n\
            2. Execute each action in sequence\n\
            3. Maintain state across steps\n\
            4. Report results clearly\n\
            5. Update authentication context if it changes\n\n\
            Focus on concrete, executable actions."
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
            let steps_summary = context.previous_steps.iter().rev().take(5)
                .map(|s| format!("- {:?}: {}", s.step_type, s.summary))
                .collect::<Vec<_>>()
                .join("\n");
            vars.insert("previous_steps".to_string(), steps_summary);
        } else {
            vars.insert("previous_steps".to_string(), "None".to_string());
        }
        
        // Build findings summary
        if !context.findings.is_empty() {
            let findings_summary = context.findings.iter().rev().take(3)
                .map(|f| format!("- [{:?}] {}", f.risk_level, f.title))
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
}

#[async_trait::async_trait]
impl SubAgentExecutor for PlanExecSubAgentExecutor {
    async fn execute(&self, request: SubAgentRequest) -> Result<SubAgentResponse> {
        log::info!(
            "Plan-and-Execute sub-agent executing: session={}, objective={}",
            request.session_id,
            request.context.objective
        );
        
        // Create Plan-and-Execute engine instance
        let config = PlanAndExecuteConfig::default();
        let mut engine = PlanAndExecuteEngine::new_with_dependencies(
            self.ai_service_manager.clone(),
            config,
            self.db_service.clone(),
            None, // No app_handle needed for sub-agent execution
        ).await?;
        
        // Create agent task
        let mut task_params = HashMap::new();
        task_params.insert("session_id".to_string(), serde_json::json!(request.session_id));
        
        // Read tools_allow from context.task_parameters (passed from AgentManager)
        if let Some(tools_allow) = request.context.task_parameters.get("tools_allow") {
            // log::info!("Plan-and-Execute sub-agent: Using tools_allow from task parameters: {:?}", tools_allow);
            task_params.insert("tools_allow".to_string(), tools_allow.clone());
        } else {
            log::warn!("Plan-and-Execute sub-agent: No tools_allow in task parameters, using empty list");
            task_params.insert("tools_allow".to_string(), serde_json::json!([]));
        }
        
        // Also pass tools_deny if present
        if let Some(tools_deny) = request.context.task_parameters.get("tools_deny") {
            log::info!("Plan-and-Execute sub-agent: Using tools_deny from task parameters: {:?}", tools_deny);
            task_params.insert("tools_deny".to_string(), tools_deny.clone());
        }
        
        // Merge step-level parameters from Orchestrator plan (e.g. target_url, scan_depth)
        if let Some(step_params) = request.context.task_parameters.get("step_parameters") {
            if let Some(obj) = step_params.as_object() {
                for (k, v) in obj {
                    // 避免覆盖已有的核心控制字段
                    if k != "tools_allow" && k != "tools_deny" && k != "custom_system_prompt" {
                        task_params.insert(k.clone(), v.clone());
                    }
                }
            }
        }
        
        // Pass custom system prompt from Orchestrator if available
        if let Some(prompts) = request.context.task_parameters.get("prompts") {
            if let Some(executor_prompt) = prompts.get("executor").and_then(|v| v.as_str()) {
                log::info!("Plan-and-Execute sub-agent: Using custom system prompt from Orchestrator");
                // Build context variables and replace them in the prompt
                let context_vars = self.build_context_variables(&request.context);
                let rendered_prompt = self.replace_prompt_variables(executor_prompt, &context_vars);
                task_params.insert("custom_system_prompt".to_string(), serde_json::json!(rendered_prompt));
            }
        }
        
        // Ensure runtime params are available for both planner & executor
        // Planner 使用 AgentTask.parameters，但 Executor 使用 runtime_params 构造内部 TaskRequest
        // 如果不设置 runtime_params，执行阶段将看不到 tools_allow / tools_deny，从而报“未配置工具权限”
        engine.set_runtime_params(task_params.clone());
        
        // Build a richer task description including original target information when available
        let mut task_description = request.context.objective.clone();
        // 优先使用步骤参数中的 target_url 作为明确目标
        if let Some(step_params) = request.context.task_parameters.get("step_parameters") {
            if let Some(url) = step_params.get("target_url").and_then(|v| v.as_str()) {
                task_description = format!("目标: {}，子任务: {}", url, request.context.objective);
            }
        } else if !request.context.primary_target.is_empty() {
            // 回退使用会话的 primary_target 作为目标描述
            task_description = format!("{} —— {}", request.context.primary_target, request.context.objective);
        }
        
        let task = AgentTask {
            id: uuid::Uuid::new_v4().to_string(),
            user_id: "orchestrator".to_string(),
            description: task_description.clone(),
            priority: crate::agents::traits::TaskPriority::Normal,
            target: Some(request.context.primary_target.clone()),
            parameters: task_params,
            timeout: Some(600), // 10 minutes for execution
        };
        
        // Execute Plan-and-Execute engine
        log::info!("Calling Plan-and-Execute engine for execution");
        
        // Create execution plan first
        let plan = engine.create_plan(&task).await?;
        
        // Then execute the plan
        let result = engine.execute_plan(&plan).await?;
        
        if !result.success {
            let error_msg = result.error.unwrap_or_else(|| "Unknown error".to_string());
            log::error!("Plan-and-Execute execution failed: {}", error_msg);
            return Ok(SubAgentResponse::error(
                SubAgentKind::PlanAndExecute,
                error_msg,
            ));
        }
        
        // Parse result data and extract execution steps
        let mut exec_steps = Vec::new();
        let mut final_result = "Plan-and-Execute execution completed successfully".to_string();
        
        if let Some(data) = &result.data {
            // Try to extract steps from artifacts or data
            if let Some(steps_data) = data.get("steps") {
                if let Some(steps_arr) = steps_data.as_array() {
                    for (idx, step_val) in steps_arr.iter().enumerate() {
                        if let Some(step_obj) = step_val.as_object() {
                            exec_steps.push(ExecutionStep {
                                index: idx + 1,
                                action: step_obj.get("name")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("Unknown action")
                                    .to_string(),
                                result: step_obj.get("result")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("Completed")
                                    .to_string(),
                                success: step_obj.get("success")
                                    .and_then(|v| v.as_bool())
                                    .unwrap_or(true),
                            });
                        }
                    }
                }
            }
            
            if let Some(result_str) = data.get("result").and_then(|v| v.as_str()) {
                final_result = result_str.to_string();
            }
        }
        
        // If no steps were extracted, create a summary step
        if exec_steps.is_empty() {
            exec_steps.push(ExecutionStep {
                index: 1,
                action: "Execute security testing workflow".to_string(),
                result: final_result.clone(),
                success: true,
            });
        }
        
        log::info!("Plan-and-Execute sub-agent completed with {} steps", exec_steps.len());
        
        Ok(SubAgentResponse::success(
            SubAgentKind::PlanAndExecute,
            SubAgentOutput::Execution {
                steps: exec_steps,
                final_result,
                auth_context_updated: None, // TODO: Extract auth context from execution if available
            },
        ))
    }
}

