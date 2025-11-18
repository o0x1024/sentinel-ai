//! Orchestrator built-in planning module
//!
//! This module implements the Planning phase for Orchestrator,
//! generating structured security test plans without relying on ReWOO.

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{AppHandle, Manager};
use crate::models::security_testing::*;
use crate::services::prompt_db::PromptRepository;
use crate::services::ai::{AiService, AiServiceManager};
use crate::utils::ordered_message::ChunkType;
use crate::utils::prompt_resolver::{PromptResolver, CanonicalStage, AgentPromptConfig};
use crate::models::prompt::ArchitectureType;
use tracing::info;

/// Orchestrator plan - structured security test plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorPlan {
    /// Plan ID
    pub id: String,
    /// Task kind
    pub task_kind: SecurityTaskKind,
    /// Primary target (URL/domain/file path)
    pub primary_target: String,
    /// Plan summary
    pub plan_summary: String,
    /// Execution steps
    pub steps: Vec<PlanStep>,
    /// Estimated total duration (minutes)
    pub estimated_duration_min: Option<u32>,
}

/// Single step in the orchestrator plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    /// Step ID (e.g., "step_1", "step_2")
    pub id: String,
    /// Step index (1-based)
    pub index: usize,
    /// Step type (e.g., Recon, VulnScan, etc.)
    pub step_type: TestStepType,
    /// Which sub-agent to use
    pub sub_agent_kind: SubAgentKind,
    /// Step objective/description
    pub objective: String,
    /// Actions to perform (tool names or commands)
    pub actions: Vec<String>,
    /// Expected outputs
    pub expected_outputs: Vec<String>,
    /// Dependencies (step IDs this step depends on)
    pub depends_on: Vec<String>,
    /// Estimated risk level
    pub risk_level: RiskImpact,
    /// Additional parameters
    #[serde(default)]
    pub parameters: HashMap<String, serde_json::Value>,
}

/// Planning result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorPlanningResult {
    /// Execution plan
    pub plan: OrchestratorPlan,
    /// Planning confidence (0.0-1.0)
    pub confidence: f32,
    /// Planning reasoning process
    pub reasoning: String,
}

/// Built-in planner for Orchestrator
pub struct OrchestratorPlanner {
    /// Prompt repository for dynamic prompt loading
    prompt_repo: Option<PromptRepository>,
    /// AI service manager for calling LLMs
    ai_service_manager: Arc<AiServiceManager>,
    /// App handle for emitting messages to frontend
    app_handle: Option<Arc<AppHandle>>,
}

impl OrchestratorPlanner {
    pub fn new(
        prompt_repo: Option<PromptRepository>,
        ai_service_manager: Arc<AiServiceManager>,
        app_handle: Option<Arc<AppHandle>>,
    ) -> Self {
        Self {
            prompt_repo,
            ai_service_manager,
            app_handle,
        }
    }
    
    /// Generate plan using LLM with retry logic
    pub async fn generate_plan(
        &self,
        task: &SecurityTask,
    ) -> Result<OrchestratorPlanningResult> {
        log::info!(
            "Generating Orchestrator plan for task_kind={:?}, target={}",
            task.task_kind,
            task.primary_target
        );
        
        // Extract IDs from task parameters
        let conversation_id = task.parameters.get("conversation_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let message_id = task.parameters.get("message_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let execution_id = task.parameters.get("execution_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or_else(|| message_id.clone());
        
        // Build planning prompts (system + user)
        let (system_prompt, user_prompt) = self.build_planning_prompts(task).await?;
        
        // Retry logic: up to 3 attempts
        let mut last_err: Option<anyhow::Error> = None;
        for attempt in 1..=3 {
            match self.call_ai_for_planning(
                &system_prompt,
                &user_prompt,
                conversation_id.clone(),
                message_id.clone(),
                task,
            ).await {
                Ok(response) => {
                    match self.parse_plan_response(&response, task) {
                        Ok(plan) => {
                            let confidence = self.calculate_confidence(&plan);
                            
                            log::info!(
                                "Generated Orchestrator plan: {} steps, confidence={:.2}",
                                plan.steps.len(),
                                confidence
                            );
                            
                            return Ok(OrchestratorPlanningResult {
                                plan,
                                confidence,
                                reasoning: response,
                            });
                        }
                        Err(e) => {
                            log::warn!("Plan parsing failed (attempt {}/3): {}", attempt, e);
                            last_err = Some(e);
                            self.emit_planning_content(
                                &execution_id,
                                &message_id,
                                conversation_id.as_deref(),
                                &format!("Plan parsing failed (attempt {}): {}", attempt, last_err.as_ref().unwrap())
                            );
                        }
                    }
                }
                Err(e) => {
                    log::warn!("AI call failed (attempt {}/3): {}", attempt, e);
                    last_err = Some(e);
                    self.emit_planning_content(
                        &execution_id,
                        &message_id,
                        conversation_id.as_deref(),
                        &format!("Plan generation failed (attempt {}): {}", attempt, last_err.as_ref().unwrap())
                    );
                }
            }
            
            if attempt < 3 {
                tokio::time::sleep(std::time::Duration::from_millis(500 * attempt as u64)).await;
            }
        }
        
        // All retries failed
        let error_message = if let Some(err) = last_err {
            format!("Failed to generate Orchestrator plan after 3 attempts: {}", err)
        } else {
            "Failed to generate Orchestrator plan: unknown error".to_string()
        };
        
        self.emit_planning_error(&execution_id, &message_id, conversation_id.as_deref(), &error_message);
        
        Err(anyhow!(error_message))
    }
    
    /// Build planning prompts (system + user) using dynamic prompt configuration
    async fn build_planning_prompts(&self, task: &SecurityTask) -> Result<(String, String)> {
        use crate::models::prompt::StageType;
        
        // 优先使用 task.parameters 中的自定义 system prompt
        let system_template = if let Some(custom_prompt) = task.parameters.get("custom_system_prompt")
            .and_then(|v| v.as_str()) {
            info!("Orchestrator Planner: Using custom system prompt from parameters");
            custom_prompt.to_string()
        } else if let Some(repo) = &self.prompt_repo {
            // 从数据库获取 Orchestrator Planning 阶段的激活模板
            match repo.get_template_by_arch_stage(ArchitectureType::Orchestrator, StageType::Planning).await {
                Ok(Some(template)) => {
                    info!("Orchestrator Planner: Using prompt from database (template_id: {})", template.id.unwrap_or(0));
                    template.content
                }
                Ok(None) => {
                    log::warn!("Orchestrator Planning template not found in database, using default template");
                    self.get_default_planning_prompt(task)
                }
                Err(e) => {
                    log::warn!("Failed to load Orchestrator Planning template: {}, using default", e);
                    self.get_default_planning_prompt(task)
                }
            }
        } else {
            log::warn!("Prompt repository not available, using default template");
            self.get_default_planning_prompt(task)
        };
        
        // Render variables in system template
        let mut system_ctx = HashMap::new();
        system_ctx.insert("task_kind".to_string(), serde_json::Value::String(format!("{:?}", task.task_kind)));
        system_ctx.insert("primary_target".to_string(), serde_json::Value::String(task.primary_target.clone()));
        system_ctx.insert("description".to_string(), serde_json::Value::String(task.description.clone().unwrap_or_default()));
        system_ctx.insert("step_types".to_string(), serde_json::Value::String(self.get_step_types_hint(&task.task_kind)));
        
        let system_prompt = if let Some(repo) = &self.prompt_repo {
            let resolver = PromptResolver::new(repo.clone());
            resolver.render_variables(&system_template, &system_ctx)
                .unwrap_or(system_template)
        } else {
            let mut rendered = system_template;
            for (key, value) in &system_ctx {
                let placeholder = format!("{{{}}}", key);
                let replacement = match value {
                    serde_json::Value::String(s) => s.clone(),
                    _ => value.to_string(),
                };
                rendered = rendered.replace(&placeholder, &replacement);
            }
            rendered
        };
        
        // User prompt: task description or name
        let user_prompt = task.description.clone().unwrap_or_else(|| task.primary_target.clone());
        
        Ok((system_prompt, user_prompt))
    }
    
    /// Get default planning prompt (fallback)
    fn get_default_planning_prompt(&self, _task: &SecurityTask) -> String {
        r#"You are a Security Testing Orchestration Planner. Generate a structured security test plan.

**Task Information:**
- Task Kind: {task_kind}
- Primary Target: {primary_target}
- Description: {description}

**Requirements:**
1. Generate a step-by-step security testing plan
2. Each step should specify which sub-agent to use:
   - **PlanAndExecute**: For linear execution chains (login, scanning, testing)
   - **ReWOO**: For complex multi-branch planning
   - **LLMCompiler**: For generating scripts, payloads, or tools
3. Include dependencies between steps
4. Estimate risk level for each step

**Output Format (JSON):**
```json
{{
  "plan_summary": "Brief summary of the overall plan",
  "steps": [
    {{
      "id": "step_1",
      "index": 1,
      "step_type": "Recon",
      "sub_agent_kind": "PlanAndExecute",
      "objective": "Information gathering and reconnaissance",
      "actions": ["start_passive_scan", "rsubdomain <domain>"],
      "expected_outputs": ["Subdomain list", "Passive scan data"],
      "depends_on": [],
      "risk_level": "None"
    }},
    {{
      "id": "step_2",
      "index": 2,
      "step_type": "VulnScan",
      "sub_agent_kind": "PlanAndExecute",
      "objective": "Active vulnerability scanning",
      "actions": ["nuclei -u <target>"],
      "expected_outputs": ["Vulnerability report"],
      "depends_on": ["step_1"],
      "risk_level": "Low"
    }}
  ],
  "estimated_duration_min": 30
}}
```

**Important:**
- Use "PlanAndExecute" for most steps (linear execution)
- Use "ReWOO" only when you need complex multi-branch planning
- Use "LLMCompiler" for script/payload generation
- Risk levels: None, Info, Low, Medium, High, Critical
- Step types: {step_types}

Generate the plan now (JSON only, no explanation):
"#.to_string()
    }
    
    /// Call AI for planning
    async fn call_ai_for_planning(
        &self,
        system_prompt: &str,
        user_prompt: &str,
        conversation_id: Option<String>,
        message_id: Option<String>,
        task: &SecurityTask,
    ) -> Result<String> {
        // Get AI configuration from parameters or use default
        let (provider_name, model_name) = self.get_ai_config(task).await?;
        
        // Create AI service
        let ai_service = if let Ok(Some(cfg)) = self.ai_service_manager.get_provider_config(&provider_name).await {
            let mut dc = cfg;
            dc.model = model_name.clone();
            let app_handle = self.app_handle.as_ref().map(|a| a.as_ref().clone());
            AiService::new(dc, self.ai_service_manager.get_db_arc(), app_handle, None)
        } else {
            return Err(anyhow!("Provider {} not found", provider_name));
        };
        
        // Send request using stream API
        let result = ai_service.send_message_stream(
            Some(user_prompt),
            Some(system_prompt),
            conversation_id,
            message_id,
            false,
            false,
            Some(ChunkType::PlanInfo),
        ).await?;
        
        log::info!("AI planning response received");
        Ok(result)
    }
    
    /// Get AI configuration
    async fn get_ai_config(&self, task: &SecurityTask) -> Result<(String, String)> {
        use crate::services::ai::SchedulerStage;
        
        // 1. 尝试从调度器配置获取规划阶段的 AI 配置
        match self.ai_service_manager.get_ai_config_for_stage(SchedulerStage::Planning).await {
            Ok(Some(config)) => {
                log::info!("使用调度器配置的规划模型: {} ({})", config.model, config.provider);
                return Ok((config.provider, config.model));
            }
            Ok(None) => {
                log::info!("调度器未配置规划模型，尝试其他配置源");
            }
            Err(e) => {
                log::warn!("获取调度器规划配置失败: {}, 尝试其他配置源", e);
            }
        }
        
        // 2. 检查任务参数中的 LLM 覆盖配置
        if let Some(llm_params) = task.parameters.get("llm").and_then(|v| v.get("default")) {
            let provider_str = llm_params.get("provider").and_then(|v| v.as_str()).unwrap_or("");
            let model_str = llm_params.get("model").and_then(|v| v.as_str()).unwrap_or("");
            
            // 跳过 "auto" 配置
            if model_str != "auto" && !model_str.trim().is_empty() {
                let provider = if provider_str != "auto" && !provider_str.trim().is_empty() {
                    provider_str.to_string()
                } else {
                    // 默认使用 deepseek provider
                    "deepseek".to_string()
                };
                log::info!("使用任务参数覆盖的规划模型: {} ({})", model_str, provider);
                return Ok((provider, model_str.to_string()));
            }
        }
        
        // 3. 使用默认配置 (deepseek-chat)
        log::info!("使用默认 Orchestrator planner AI 配置: deepseek-chat (deepseek)");
        Ok(("deepseek".to_string(), "deepseek-chat".to_string()))
    }
    
    /// Emit planning content to frontend
    fn emit_planning_content(
        &self,
        execution_id: &Option<String>,
        message_id: &Option<String>,
        conversation_id: Option<&str>,
        msg: &str
    ) {
        if let (Some(app), Some(exec_id), Some(msg_id)) = (
            &self.app_handle,
            execution_id.as_ref(),
            message_id.as_ref()
        ) {
            crate::utils::ordered_message::emit_message_chunk_arc(
                app,
                exec_id,
                msg_id,
                conversation_id,
                ChunkType::Content,
                msg,
                false,
                Some("orchestrator_planner"),
                None,
            );
        }
    }
    
    /// Emit planning error to frontend
    fn emit_planning_error(
        &self,
        execution_id: &Option<String>,
        message_id: &Option<String>,
        conversation_id: Option<&str>,
        msg: &str
    ) {
        if let (Some(app), Some(exec_id), Some(msg_id)) = (
            &self.app_handle,
            execution_id.as_ref(),
            message_id.as_ref()
        ) {
            crate::utils::ordered_message::emit_message_chunk_arc(
                app,
                exec_id,
                msg_id,
                conversation_id,
                ChunkType::Error,
                msg,
                false,
                Some("orchestrator_planner"),
                None,
            );
        }
    }
    
    /// Calculate plan confidence
    fn calculate_confidence(&self, plan: &OrchestratorPlan) -> f32 {
        if plan.steps.is_empty() {
            return 0.0;
        }
       
       // Basic confidence based on plan completeness
       let mut confidence: f32 = 0.8;
       
       // Bonus for clear dependencies
       let has_dependencies = plan.steps.iter().any(|s| !s.depends_on.is_empty());
       if has_dependencies {
           confidence += 0.1;
       }
       
       // Bonus for expected outputs
       let has_outputs = plan.steps.iter().any(|s| !s.expected_outputs.is_empty());
       if has_outputs {
           confidence += 0.1;
       }
       
       confidence.min(1.0)
    }
    
    /// Get step types hint based on task kind
    fn get_step_types_hint(&self, task_kind: &SecurityTaskKind) -> String {
        match task_kind {
            SecurityTaskKind::WebPentest | SecurityTaskKind::APIPentest => {
                "Recon, Login, APIMapping, VulnScan, Exploit, Report"
            }
            SecurityTaskKind::Forensics => {
                "LogCollection, TimelineReconstruction, IOCExtraction, BehaviorAnalysis, Report"
            }
            SecurityTaskKind::CTF => {
                "ChallengeAnalysis, VulnIdentification, PayloadCrafting, FlagExtraction, Writeup"
            }
            SecurityTaskKind::ReverseEngineering => {
                "BinaryLoading, StaticAnalysis, DynamicAnalysis, Deobfuscation, BehaviorSummary"
            }
            _ => "PlanSecurityTest, ExecuteSecurityTest, AnalyzeResults, GenerateReport"
        }.to_string()
    }
    
    /// Parse LLM response into OrchestratorPlan
    fn parse_plan_response(&self, response: &str, task: &SecurityTask) -> Result<OrchestratorPlan> {
        info!("start parse_plan_response");
        // Extract JSON from response (might be wrapped in markdown code blocks)
        let json_str = self.extract_json(response)?;
        
        // Parse JSON
        let mut parsed: serde_json::Value = serde_json::from_str(&json_str)
            .map_err(|e| anyhow!("Failed to parse plan JSON: {}", e))?;
        
        // Extract fields
        let plan_summary = parsed.get("plan_summary")
            .and_then(|v| v.as_str())
            .unwrap_or("Security test plan")
            .to_string();
        
        let steps_json = parsed.get("steps")
            .and_then(|v| v.as_array())
            .ok_or_else(|| anyhow!("No steps array in plan"))?;
        
        let estimated_duration = parsed.get("estimated_duration_min")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32);
        
        // Parse steps
        let mut steps = Vec::new();
        for (idx, step_json) in steps_json.iter().enumerate() {
            let step = self.parse_step(step_json, idx + 1)?;
            steps.push(step);
        }
        
        Ok(OrchestratorPlan {
            id: uuid::Uuid::new_v4().to_string(),
            task_kind: task.task_kind.clone(),
            primary_target: task.primary_target.clone(),
            plan_summary,
            steps,
            estimated_duration_min: estimated_duration,
        })
    }
    
    /// Parse single step from JSON
    fn parse_step(&self, json: &serde_json::Value, default_index: usize) -> Result<PlanStep> {
        let id = json.get("id")
            .and_then(|v| v.as_str())
            .unwrap_or(&format!("step_{}", default_index))
            .to_string();
        
        let index = json.get("index")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(default_index);
        
       let _step_type_str = json.get("step_type")
           .and_then(|v| v.as_str())
           .unwrap_or("PlanSecurityTest");
       // TODO: Parse step_type from string, for now use default
       let step_type = TestStepType::PlanSecurityTest;
        
        let sub_agent_str = json.get("sub_agent_kind")
            .and_then(|v| v.as_str())
            .unwrap_or("PlanAndExecute");
        let sub_agent_kind = match sub_agent_str {
            "ReWOO" => SubAgentKind::ReWOO,
            "LLMCompiler" => SubAgentKind::LLMCompiler,
            _ => SubAgentKind::PlanAndExecute,
        };
        
        let objective = json.get("objective")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        
        let actions: Vec<String> = json.get("actions")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();
        
        let expected_outputs: Vec<String> = json.get("expected_outputs")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();
        
        let depends_on: Vec<String> = json.get("depends_on")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();
        
        let risk_level_str = json.get("risk_level")
            .and_then(|v| v.as_str())
            .unwrap_or("None");
        let risk_level = match risk_level_str.to_lowercase().as_str() {
            "critical" => RiskImpact::Critical,
            "high" => RiskImpact::High,
            "medium" => RiskImpact::Medium,
            "low" => RiskImpact::Low,
            "info" => RiskImpact::Info,
            _ => RiskImpact::None,
        };
        
        let parameters: HashMap<String, serde_json::Value> = json.get("parameters")
            .and_then(|v| v.as_object())
            .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
            .unwrap_or_default();
        
        Ok(PlanStep {
            id,
            index,
            step_type,
            sub_agent_kind,
            objective,
            actions,
            expected_outputs,
            depends_on,
            risk_level,
            parameters,
        })
    }
    
    /// Extract JSON from LLM response (handle markdown code blocks)
    fn extract_json(&self, response: &str) -> Result<String> {
        let trimmed = response.trim();
        
        // Try to find JSON in markdown code block
        if let Some(start_idx) = trimmed.find("```json") {
            // Skip the ```json 标记本身，直接在其后查找结束的 ```
            let content_start = start_idx + "```json".len();
            let after = &trimmed[content_start..];

            if let Some(end_rel) = after.find("```") {
                let json_str = &after[..end_rel];
                return Ok(json_str.trim().to_string());
            }
        }
        
        // Try to find JSON without markdown
        if let Some(start_idx) = trimmed.find('{') {
            if let Some(end_idx) = trimmed.rfind('}') {
                return Ok(trimmed[start_idx..=end_idx].to_string());
            }
        }
        
        Err(anyhow!("Could not extract JSON from response"))
    }
}

/// SecurityTask definition (input to planner)
#[derive(Debug, Clone)]
pub struct SecurityTask {
    pub task_kind: SecurityTaskKind,
    pub primary_target: String,
    pub description: Option<String>,
    pub parameters: HashMap<String, serde_json::Value>,
    pub additional_targets: Vec<String>,
    pub scope: Vec<String>,
    pub constraints: Vec<String>,
    pub risk_tolerance: RiskTolerance,
    pub session_id: String,
    pub summary: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Risk tolerance level
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskTolerance {
    Low,
    Medium,
    High,
}

