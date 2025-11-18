use crate::agents::orchestrator::sub_agent_interface::*;
use crate::agents::traits::*;
use crate::managers::security_test_manager::SecurityTestManager;
use crate::models::security_testing::*;
use crate::models::ai::AiMessage;
use crate::services::DatabaseService;
use crate::services::ai::AiServiceManager;
use crate::services::prompt_db::PromptRepository;
use crate::models::prompt::{ArchitectureType, StageType};
use crate::engines::orchestrator::planner::{OrchestratorPlanner, OrchestratorPlanningResult, SecurityTask, RiskTolerance};
use anyhow::{Result, anyhow};
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::RwLock;

/// Orchestrator engine adapter
pub struct OrchestratorEngineAdapter {
    session_manager: Arc<SecurityTestManager>,
    sub_agent_registry: Arc<RwLock<SubAgentRegistry>>,
    app_handle: Arc<RwLock<Option<tauri::AppHandle>>>,
    db_service: Arc<DatabaseService>,
    ai_service_manager: Arc<AiServiceManager>,
    prompt_repo: Option<PromptRepository>,
}

impl OrchestratorEngineAdapter {
    pub fn new(
        session_manager: Arc<SecurityTestManager>, 
        db_service: Arc<DatabaseService>,
        ai_service_manager: Arc<AiServiceManager>,
        prompt_repo: Option<PromptRepository>,
    ) -> Self {
        Self {
            session_manager,
            sub_agent_registry: Arc::new(RwLock::new(SubAgentRegistry::new())),
            app_handle: Arc::new(RwLock::new(None)),
            db_service,
            ai_service_manager,
            prompt_repo,
        }
    }
    
    /// Set app handle for message emission
    pub async fn set_app_handle(&self, app_handle: tauri::AppHandle) {
        let mut handle = self.app_handle.write().await;
        *handle = Some(app_handle);
    }
    
    /// Execute sub-agent
    pub async fn execute_sub_agent(&self, request: SubAgentRequest) -> Result<SubAgentResponse> {
        log::info!(
            "Executing sub-agent: kind={:?}, session={}, objective={}",
            request.kind,
            request.session_id,
            request.context.objective
        );
        
        let registry = self.sub_agent_registry.read().await;
        let executor = registry.get_executor(&request.kind)
            .ok_or_else(|| anyhow!("No executor registered for sub-agent kind: {:?}", request.kind))?;
        
        let response = executor.execute(request).await?;
        
        log::info!(
            "Sub-agent execution completed: kind={:?}, success={}",
            response.kind,
            response.success
        );
        
        Ok(response)
    }
    
    /// Register a sub-agent executor
    pub async fn register_sub_agent(&self, kind: SubAgentKind, executor: Arc<dyn SubAgentExecutor>) {
        let mut registry = self.sub_agent_registry.write().await;
        registry.register(kind, executor);
    }
    
    /// Create a new security test session
    pub async fn create_session(
        &self,
        task_kind: SecurityTaskKind,
        primary_target: String,
        summary: String,
    ) -> Result<TestSession> {
        self.session_manager.create_session(task_kind, primary_target, summary).await
    }
    
    /// Get session
    pub async fn get_session(&self, session_id: &str) -> Result<TestSession> {
        self.session_manager.get_session(session_id).await
    }
    
    /// Add step to session
    pub async fn add_step(&self, session_id: &str, step: TestStep) -> Result<()> {
        self.session_manager.add_step(session_id, step).await
    }
    
    /// Update step status
    pub async fn update_step_status(
        &self,
        session_id: &str,
        step_id: &str,
        status: StepStatus,
        output: Option<String>,
    ) -> Result<()> {
        self.session_manager.update_step(session_id, step_id, |step| {
            step.status = status.clone();
            if let Some(out) = output {
                step.output = Some(out);
            }
            if status == StepStatus::Completed || status == StepStatus::Failed {
                step.finished_at = Some(chrono::Utc::now());
            }
        }).await
    }
    
    /// Add finding to session
    pub async fn add_finding(&self, session_id: &str, finding: Finding) -> Result<()> {
        self.session_manager.add_finding(session_id, finding).await
    }
    
    /// Update session stage
    pub async fn update_stage(&self, session_id: &str, stage: TestStage) -> Result<()> {
        self.session_manager.update_stage(session_id, stage).await
    }
    
    /// Update authentication context
    pub async fn update_auth_context(&self, session_id: &str, auth_context: AuthContext) -> Result<()> {
        self.session_manager.update_auth_context(session_id, auth_context).await
    }
    
    /// Build sub-agent context from session
    pub async fn build_sub_agent_context(
        &self,
        session_id: &str,
        objective: String,
    ) -> Result<SubAgentContext> {
        let session = self.get_session(session_id).await?;
        
        let previous_steps = session.steps.iter()
            .filter(|s| s.status == StepStatus::Completed)
            .map(|s| StepSummary {
                step_type: s.step_type.clone(),
                summary: s.short_summary.clone(),
                output: s.output.clone(),
            })
            .collect();
        
        let findings = session.findings.iter()
            .map(|f| FindingSummary {
                location: f.location.clone(),
                risk_level: f.risk_level.clone(),
                title: f.title.clone(),
            })
            .collect();
        
        // Load prompt templates from database for Orchestrator sub-agents
        let mut task_parameters = session.task_parameters.clone();
        if let Err(e) = self.load_orchestrator_prompts(&mut task_parameters).await {
            log::warn!("Failed to load Orchestrator prompts from database: {}", e);
        }
        
        // Log tool permissions for debugging
        if let Some(tools_allow) = task_parameters.get("tools_allow") {
            log::info!("Orchestrator: Passing tools_allow to sub-agent: {:?}", tools_allow);
        } else {
            log::warn!("Orchestrator: No tools_allow found in task_parameters!");
        }
        
        Ok(SubAgentContext {
            task_kind: session.task_kind,
            primary_target: session.primary_target,
            current_stage: session.stage,
            auth_context: session.auth_context,
            previous_steps,
            findings,
            objective,
            constraints: Vec::new(),
            task_parameters,
        })
    }
    
    /// Load prompt templates for Orchestrator sub-agents
    ///
    /// Prompt 来源统一走 Orchestrator 架构 + CanonicalStage：
    /// - Orchestrator + Planner  → ReWOO 规划子代理使用（key: "planner")
    /// - Orchestrator + Execution → Plan-and-Execute 执行子代理使用（key: "executor")
    async fn load_orchestrator_prompts(
        &self,
        _task_parameters: &mut HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        // NOTE: 不再注入 Orchestrator prompts 到 sub-agents
        // 每个 sub-agent (ReWOO, Plan-and-Execute, LLMCompiler) 应该使用自己架构的 prompt
        // 而不是 Orchestrator 的 prompt，否则会导致任务解析失败
        // 因为不同架构的 prompt 格式、输出要求、工具调用方式都不同
        log::debug!("Sub-agents will use their own architecture-specific prompts, not Orchestrator prompts");
        Ok(())
    }
    
    /// Convert session to messages for display
    pub fn session_to_messages(&self, session: &TestSession) -> Vec<AiMessage> {
        let mut messages = Vec::new();
        
        // Add session summary message
        let summary_content = serde_json::json!({
            "type": "orchestrator_session",
            "session_id": session.id,
            "task_kind": session.task_kind,
            "primary_target": session.primary_target,
            "stage": session.stage,
            "summary": session.summary,
            "total_steps": session.steps.len(),
            "total_findings": session.findings.len(),
            "high_risk_findings": session.get_high_risk_findings().len(),
        });
        
        messages.push(AiMessage {
            id: uuid::Uuid::new_v4().to_string(),
            role: crate::models::ai::MessageRole::Assistant,
            content: serde_json::to_string(&summary_content).unwrap_or_default(),
            timestamp: chrono::Utc::now(),
            model: None,
            provider: None,
            tokens_used: None,
            tools_used: None,
            metadata: None,
        });
        
        // Add step messages
        for step in &session.steps {
            let step_content = serde_json::json!({
                "type": "orchestrator_step",
                "step_id": step.id,
                "index": step.index,
                "sub_agent_kind": step.sub_agent_kind,
                "step_type": step.step_type,
                "short_summary": step.short_summary,
                "risk_impact": step.risk_impact,
                "status": step.status,
                "output": step.output,
            });
            
            messages.push(AiMessage {
                id: uuid::Uuid::new_v4().to_string(),
                role: crate::models::ai::MessageRole::Assistant,
                content: serde_json::to_string(&step_content).unwrap_or_default(),
                timestamp: chrono::Utc::now(),
                model: None,
                provider: None,
                tokens_used: None,
                tools_used: None,
                metadata: None,
            });
        }
        
        messages
    }
    
    /// Create session message for display
    fn create_session_message(&self, session: &TestSession) -> AiMessage {
        let summary_content = serde_json::json!({
            "type": "orchestrator_session",
            "session_id": session.id,
            "task_kind": session.task_kind,
            "primary_target": session.primary_target,
            "stage": session.stage,
            "summary": session.summary,
            "total_steps": session.steps.len(),
            "total_findings": session.findings.len(),
            "high_risk_findings": session.get_high_risk_findings().len(),
        });
        
        AiMessage {
            id: uuid::Uuid::new_v4().to_string(),
            role: crate::models::ai::MessageRole::Assistant,
            content: serde_json::to_string(&summary_content).unwrap_or_default(),
            timestamp: chrono::Utc::now(),
            model: None,
            provider: None,
            tokens_used: None,
            tools_used: None,
            metadata: None,
        }
    }
    
    /// Emit step message to frontend
    fn emit_step_message(&self, step: &TestStep, session: &TestSession) {
        let step_content = serde_json::json!({
            "type": "orchestrator_step",
            "step_id": step.id,
            "index": step.index,
            "sub_agent_kind": step.sub_agent_kind,
            "step_type": step.step_type,
            "short_summary": step.short_summary,
            "risk_impact": step.risk_impact,
            "status": step.status,
            "output": step.output,
        });
        
        let content_str = serde_json::to_string(&step_content).unwrap_or_default();
        log::info!("Orchestrator step: {}", content_str);
        
        // Send to frontend if app_handle is available
        let app_handle = self.app_handle.clone();
        let content_for_emit = content_str.clone();
        let task_params = session.task_parameters.clone();
        
        tokio::spawn(async move {
            let handle_guard = app_handle.read().await;
            if let Some(app) = handle_guard.as_ref() {
                // Extract execution_id, message_id, conversation_id from task_parameters
                let execution_id = task_params.get("execution_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();
                let message_id = task_params.get("message_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&execution_id)
                    .to_string();
                let conversation_id = task_params.get("conversation_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                
                crate::utils::ordered_message::emit_message_chunk(
                    app,
                    &execution_id,
                    &message_id,
                    conversation_id.as_deref(),
                    crate::utils::ordered_message::ChunkType::Meta,
                    &content_for_emit,
                    false,
                    Some("orchestrator"),
                    None,
                );
            }
        });
    }
}

/// Sub-agent registry
struct SubAgentRegistry {
    executors: HashMap<SubAgentKind, Arc<dyn SubAgentExecutor>>,
}

impl SubAgentRegistry {
    fn new() -> Self {
        Self {
            executors: HashMap::new(),
        }
    }
    
    fn register(&mut self, kind: SubAgentKind, executor: Arc<dyn SubAgentExecutor>) {
        self.executors.insert(kind, executor);
        log::info!("Registered sub-agent executor: {:?}", kind);
    }
    
    fn get_executor(&self, kind: &SubAgentKind) -> Option<&Arc<dyn SubAgentExecutor>> {
        self.executors.get(kind)
    }
}

/// Implementation of ExecutionEngine trait for Orchestrator
#[async_trait::async_trait]
impl ExecutionEngine for OrchestratorEngineAdapter {
    fn get_engine_info(&self) -> &EngineInfo {
        static ENGINE_INFO: std::sync::OnceLock<EngineInfo> = std::sync::OnceLock::new();
        ENGINE_INFO.get_or_init(|| EngineInfo {
            name: "Orchestrator".to_string(),
            version: "1.0.0".to_string(),
            description: "Security Testing Orchestrator - coordinates ReWOO, Plan-and-Execute, and LLM-Compiler agents".to_string(),
            supported_scenarios: vec![
                "web_pentest".to_string(),
                "api_pentest".to_string(),
                "forensics".to_string(),
                "ctf".to_string(),
                "reverse_engineering".to_string(),
            ],
            performance_characteristics: PerformanceCharacteristics {
                token_efficiency: 75,
                execution_speed: 70,
                resource_usage: 60,
                concurrency_capability: 80,
                complexity_handling: 90,
            },
        })
    }
    
    fn supports_task(&self, task: &AgentTask) -> bool {
        // Orchestrator supports all security-related tasks
        let task_desc = task.description.to_lowercase();
        task_desc.contains("安全") 
            || task_desc.contains("测试")
            || task_desc.contains("渗透")
            || task_desc.contains("漏洞")
            || task_desc.contains("取证")
            || task_desc.contains("ctf")
            || task_desc.contains("逆向")
            || task_desc.contains("security")
            || task_desc.contains("pentest")
            || task_desc.contains("vulnerability")
    }
    
    /// Create execution plan (ExecutionEngine trait implementation)
    /// 
    /// Note: This returns a coarse-grained, hardcoded plan for the external ExecutionEngine interface.
    /// The actual detailed orchestration (Planning + Execution phases) happens internally in
    /// execute_orchestration_workflow, not exposed through this trait method.
    /// 
    /// This is a reasonable abstraction for the current architecture where Orchestrator is an
    /// independent engine. If we need fine-grained plan visibility at the ExecutionEngine level,
    /// we would need to parse ReWOO plan JSON and convert it to ExecutionPlan steps.
    async fn create_plan(&self, task: &AgentTask) -> Result<ExecutionPlan> {
        log::info!("Creating Orchestrator execution plan for task: {}", task.description);
        
        // Determine task kind from description
        let task_kind = self.determine_task_kind(&task.description);
        
        // Create initial test session with task parameters (includes tools_allow)
        let session = self.session_manager.create_session_with_params(
            task_kind.clone(),
            task.description.clone(),
            format!("Security testing task: {}", task.description),
            task.parameters.clone()
        ).await?;
        
        log::info!("Created security test session: {} with {} parameters", session.id, session.task_parameters.len());
        
        // Log tool permissions for debugging
        if let Some(tools_allow) = session.task_parameters.get("tools_allow") {
            log::info!("Orchestrator: Session created with tools_allow: {:?}", tools_allow);
        } else {
            log::warn!("Orchestrator: Session created WITHOUT tools_allow in task_parameters!");
        }
        
        // Create coarse-grained execution plan (detailed steps happen internally)
        Ok(ExecutionPlan {
            id: session.id.clone(),
            name: format!("Orchestrator: {:?}", task_kind),
            steps: vec![
                crate::agents::traits::ExecutionStep {
                    id: uuid::Uuid::new_v4().to_string(),
                    name: "Initialize Orchestrator".to_string(),
                    description: "Set up security testing environment".to_string(),
                    step_type: StepType::ToolCall,
                    dependencies: vec![],
                    parameters: {
                        let mut map = HashMap::new();
                        map.insert("session_id".to_string(), serde_json::json!(session.id));
                        map.insert("task_kind".to_string(), serde_json::json!(task_kind));
                        map
                    },
                },
                crate::agents::traits::ExecutionStep {
                    id: uuid::Uuid::new_v4().to_string(),
                    name: "Execute Security Testing".to_string(),
                    description: "Orchestrate sub-agents for comprehensive testing".to_string(),
                    step_type: StepType::ToolCall,
                    dependencies: vec![],
                    parameters: HashMap::new(),
                },
            ],
            estimated_duration: 305,
            resource_requirements: ResourceRequirements {
                cpu_cores: Some(2),
                memory_mb: Some(1024),
                network_concurrency: Some(10),
                disk_space_mb: Some(100),
            },
        })
    }
    
    async fn execute_plan(&self, plan: &ExecutionPlan) -> Result<AgentExecutionResult> {
        let start_time = std::time::Instant::now();
        log::info!("Executing Orchestrator plan: {}", plan.name);
        
        // Extract session_id from first step parameters
        let session_id = plan.steps.first()
            .and_then(|step| step.parameters.get("session_id"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("No session_id in plan"))?
            .to_string();
        
        // Get session
        let session = self.get_session(&session_id).await?;
        
        log::info!(
            "Starting Orchestrator execution for session: {}, task_kind: {:?}",
            session_id,
            session.task_kind
        );
        
        // Register default sub-agents if not already registered
        // Note: We pass dummy values here since the actual sub-agent executors
        // are simple placeholders that don't use these dependencies yet
        let db_service = Arc::new(crate::services::database::DatabaseService::new());
        self.ensure_sub_agents_registered(
            Arc::new(crate::services::AiServiceManager::new(db_service.clone())),
            db_service,
            None,
        ).await?;
        
        // Execute orchestration workflow
        let result = self.execute_orchestration_workflow(&session_id, &session).await;
        
        let execution_time = start_time.elapsed().as_millis() as u64;
        
        match result {
            Ok(output) => {
                log::info!("Orchestrator execution completed successfully for session: {}", session_id);
                
                Ok(AgentExecutionResult {
                    id: uuid::Uuid::new_v4().to_string(),
                    success: true,
                    data: Some(serde_json::json!({
                        "session_id": session_id,
                        "message": output,
                        "task_kind": session.task_kind,
                    })),
                    error: None,
                    execution_time_ms: execution_time,
                    resources_used: HashMap::new(),
                    artifacts: vec![],
                })
            }
            Err(e) => {
                log::error!("Orchestrator execution failed for session {}: {}", session_id, e);
                
                Ok(AgentExecutionResult {
                    id: uuid::Uuid::new_v4().to_string(),
                    success: false,
                    data: None,
                    error: Some(format!("Orchestrator execution failed: {}", e)),
                    execution_time_ms: execution_time,
                    resources_used: HashMap::new(),
                    artifacts: vec![],
                })
            }
        }
    }
    
    async fn get_progress(&self, session_id: &str) -> Result<ExecutionProgress> {
        let session = self.get_session(session_id).await?;
        let stats = self.session_manager.get_session_stats(session_id).await?;
        
        let total_steps = stats.total_steps as u32;
        let completed_steps = stats.completed_steps as u32;
        let progress_percentage = if total_steps > 0 {
            (completed_steps as f32 / total_steps as f32) * 100.0
        } else {
            0.0
        };
        
        let current_step = if stats.running_steps > 0 {
            Some(format!("Running: {} active steps", stats.running_steps))
        } else if completed_steps < total_steps {
            Some(format!("Stage: {:?}", session.stage))
        } else {
            Some("Completed".to_string())
        };
        
        Ok(ExecutionProgress {
            total_steps,
            completed_steps,
            current_step,
            progress_percentage,
            estimated_remaining_seconds: None,
        })
    }
    
    async fn cancel_execution(&self, session_id: &str) -> Result<()> {
        log::info!("Cancelling Orchestrator execution for session: {}", session_id);
        
        // Update session stage to indicate cancellation
        self.update_stage(session_id, TestStage::Completed).await?;
        
        // Mark all pending/running steps as failed
        let session = self.get_session(session_id).await?;
        for step in &session.steps {
            if step.status == StepStatus::Pending || step.status == StepStatus::Running {
                self.update_step_status(
                    session_id,
                    &step.id,
                    StepStatus::Failed,
                    Some("Cancelled by user".to_string())
                ).await?;
            }
        }
        
        Ok(())
    }
}

impl OrchestratorEngineAdapter {
    /// Determine task kind from description
    fn determine_task_kind(&self, description: &str) -> SecurityTaskKind {
        let desc_lower = description.to_lowercase();
        
        if desc_lower.contains("api") || desc_lower.contains("接口") {
            SecurityTaskKind::APIPentest
        } else if desc_lower.contains("web") || desc_lower.contains("网站") {
            SecurityTaskKind::WebPentest
        } else if desc_lower.contains("取证") || desc_lower.contains("forensic") {
            SecurityTaskKind::Forensics
        } else if desc_lower.contains("ctf") {
            SecurityTaskKind::CTF
        } else if desc_lower.contains("逆向") || desc_lower.contains("reverse") {
            SecurityTaskKind::ReverseEngineering
        } else {
            SecurityTaskKind::OtherSecurity
        }
    }
    
    /// Ensure sub-agents are registered
    pub async fn ensure_sub_agents_registered(
        &self,
        ai_service_manager: Arc<crate::services::AiServiceManager>,
        db_service: Arc<crate::services::database::DatabaseService>,
        _app_handle: Option<tauri::AppHandle>,
    ) -> Result<()> {
        use crate::engines::orchestrator::sub_agents::*;
        
        let registry = self.sub_agent_registry.read().await;
        let has_rewoo = registry.get_executor(&SubAgentKind::ReWOO).is_some();
        let has_plan_exec = registry.get_executor(&SubAgentKind::PlanAndExecute).is_some();
        let has_compiler = registry.get_executor(&SubAgentKind::LLMCompiler).is_some();
        drop(registry);
        
        if !has_rewoo {
            self.register_sub_agent(
                SubAgentKind::ReWOO,
                Arc::new(ReWOOSubAgentExecutor::new(
                    ai_service_manager.clone(),
                    db_service.clone(),
                ))
            ).await;
        }
        
        if !has_plan_exec {
            self.register_sub_agent(
                SubAgentKind::PlanAndExecute,
                Arc::new(PlanExecSubAgentExecutor::new(
                    ai_service_manager.clone(),
                    db_service.clone(),
                ))
            ).await;
        }
        
        if !has_compiler {
            self.register_sub_agent(
                SubAgentKind::LLMCompiler,
                Arc::new(CompilerSubAgentExecutor::new(
                    ai_service_manager.clone(),
                    db_service.clone(),
                ))
            ).await;
        }
        
        Ok(())
    }
    
    /// Execute orchestration workflow
    /// 
    /// Orchestrator uses a two-phase execution model:
    /// 1. Planning Phase: Use ReWOO to generate comprehensive security test plan
    /// 2. Execution Phase: Use Plan-and-Execute to execute the plan
    /// 
    /// Note: This is currently a simplified implementation with hardcoded two steps.
    /// Future enhancement: Parse ReWOO plan JSON and dynamically dispatch steps.
    async fn execute_orchestration_workflow(
        &self,
        session_id: &str,
        session: &TestSession,
    ) -> Result<String> {
        log::info!("Starting Orchestrator workflow for session: {} (task_kind: {:?})", 
            session_id, session.task_kind);
        
        // === Session Initialization ===
        // Emit session summary message to frontend
        let session_message = self.create_session_message(session);
        log::info!("Orchestrator session created: {:?}", session_message.content);
        
        // Send session message to frontend
        let app_handle = self.app_handle.clone();
        let session_content = session_message.content.clone();
        let task_params = session.task_parameters.clone();
        
        tokio::spawn(async move {
            let handle_guard = app_handle.read().await;
            if let Some(app) = handle_guard.as_ref() {
                // Extract execution_id, message_id, conversation_id from task_parameters
                let execution_id = task_params.get("execution_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();
                let message_id = task_params.get("message_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&execution_id)
                    .to_string();
                let conversation_id = task_params.get("conversation_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                
                crate::utils::ordered_message::emit_message_chunk(
                    app,
                    &execution_id,
                    &message_id,
                    conversation_id.as_deref(),
                    crate::utils::ordered_message::ChunkType::Meta,
                    &session_content,
                    false,
                    Some("orchestrator"),
                    None,
                );
            }
        });
        
        // === Phase 1: Planning ===
        // Use Orchestrator's built-in planner to generate comprehensive security test plan
        log::info!("Phase 1: Planning - Using Orchestrator built-in planner");
        let plan_step = TestStep::new(
            1,
            SubAgentKind::Orchestrator,
            TestStepType::PlanSecurityTest,
            "Creating comprehensive security test plan with Orchestrator Planner".to_string()
        );
        self.add_step(session_id, plan_step.clone()).await?;
        
        // Emit step message
        self.emit_step_message(&plan_step, session);
        
        // Create Orchestrator planner
        let app_handle_clone = {
            let handle_guard = self.app_handle.read().await;
            handle_guard.clone().map(Arc::new)
        };

        // Create PromptRepository from DatabaseService pool so that Orchestrator
        // can use the same prompt configuration as PromptManagement.vue
        let planner_prompt_repo = {
            let pool = self.db_service.get_pool()?;
            Some(PromptRepository::new(pool.clone()))
        };

        let planner = OrchestratorPlanner::new(
            planner_prompt_repo,
            self.ai_service_manager.clone(),
            app_handle_clone,
        );
        
        // Build SecurityTask from session
        let security_task = SecurityTask {
            task_kind: session.task_kind.clone(),
            primary_target: session.primary_target.clone(),
            description: Some(session.summary.clone()),
            parameters: session.task_parameters.clone(),
            additional_targets: Vec::new(),
            scope: vec![session.primary_target.clone()],
            constraints: Vec::new(),
            risk_tolerance: RiskTolerance::Medium,
            session_id: session_id.to_string(),
            summary: session.summary.clone(),
            created_at: session.created_at,
        };
        
        // Generate plan using built-in planner
        log::info!("Calling Orchestrator planner to generate plan for task: {}", session.primary_target);
        let planning_result = planner.generate_plan(&security_task).await?;
        
        log::info!(
            "Orchestrator plan generated: {} steps, confidence: {:.2}",
            planning_result.plan.steps.len(),
            planning_result.confidence
        );
        
        // Convert OrchestratorPlan to JSON for storage
        let plan_json = serde_json::to_value(&planning_result.plan)?;
        
        // Store plan in session's task_parameters for Execution phase to use
        self.session_manager.update_session_with(session_id, |session| {
            session.task_parameters.insert("orchestrator_plan".to_string(), plan_json.clone());
            session.task_parameters.insert("plan_confidence".to_string(), 
                serde_json::json!(planning_result.confidence));
            session.task_parameters.insert("plan_reasoning".to_string(), 
                serde_json::json!(planning_result.reasoning));
        }).await?;
        
        log::info!("Orchestrator plan stored in session: {} steps", planning_result.plan.steps.len());
        
        self.update_step_status(
            session_id,
            &plan_step.id,
            StepStatus::Completed,
            Some(format!("Plan created with {} steps (confidence: {:.0}%)", 
                planning_result.plan.steps.len(),
                planning_result.confidence * 100.0))
        ).await?;
        
        log::info!("Phase 1 completed: Orchestrator plan generated and stored");
        
        // Emit updated step message
        let updated_session = self.get_session(session_id).await?;
        let updated_plan_step = updated_session.steps.iter()
            .find(|s| s.id == plan_step.id)
            .cloned()
            .unwrap_or(plan_step.clone());
        self.emit_step_message(&updated_plan_step, &updated_session);
        
        // === Phase 2: Execution ===
        // Execute the Orchestrator plan step by step
        log::info!("Phase 2: Execution - Executing Orchestrator plan steps");
        
        // Retrieve the stored plan from session
        let session_with_plan = self.get_session(session_id).await?;
        let stored_plan_json = session_with_plan.task_parameters.get("orchestrator_plan").cloned();
        
        if let Some(plan_json) = stored_plan_json {
            let steps_array = plan_json.get("steps").and_then(|s| s.as_array());
            if let Some(steps) = steps_array {
                log::info!("Executing {} steps from Orchestrator plan", steps.len());
                
                // Execute each step in the plan
                let mut step_results: HashMap<String, serde_json::Value> = HashMap::new();
                
                for (idx, step_json) in steps.iter().enumerate() {
                    // Parse Orchestrator plan step format
                    let default_step_id = format!("step_{}", idx + 1);
                    let step_id = step_json.get("id").and_then(|v| v.as_str())
                        .unwrap_or(&default_step_id);
                    let objective = step_json.get("objective").and_then(|v| v.as_str()).unwrap_or("");
                    let sub_agent_str = step_json.get("sub_agent_kind").and_then(|v| v.as_str())
                        .unwrap_or("PlanAndExecute");
                    let actions = step_json.get("actions").and_then(|v| v.as_array())
                        .map(|arr| arr.iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect::<Vec<_>>())
                        .unwrap_or_default();
                    
                    // Map sub_agent_kind string to enum
                    let sub_agent_kind = match sub_agent_str {
                        "ReWOO" => SubAgentKind::ReWOO,
                        "LLMCompiler" => SubAgentKind::LLMCompiler,
                        _ => SubAgentKind::PlanAndExecute,
                    };
                    
                    log::info!("Executing step {}/{}: {} (sub-agent: {:?})", 
                        idx + 1, steps.len(), objective, sub_agent_kind);
                    
                    // Create TestStep for this execution
                    let exec_step = TestStep::new(
                        idx + 2, // Start from 2 (after planning step)
                        sub_agent_kind.clone(),
                        TestStepType::PlanSecurityTest,
                        objective.to_string()
                    );
                    self.add_step(session_id, exec_step.clone()).await?;
                    
                    // Emit step message
                    let current_session = self.get_session(session_id).await?;
                    self.emit_step_message(&exec_step, &current_session);
                    
                    // Build sub-agent context
                    let mut context = self.build_sub_agent_context(
                        session_id,
                        objective.to_string()
                    ).await?;
                    
                    // Attach step-level parameters (e.g. target_url, scan_depth) so sub-agents can use them
                    if let Some(step_params) = step_json.get("parameters").and_then(|v| v.as_object()) {
                        context.task_parameters.insert(
                            "step_parameters".to_string(),
                            serde_json::Value::Object(step_params.clone())
                        );
                    }
                    
                    // Add actions as task parameters
                    let mut context_with_actions = context;
                    if !actions.is_empty() {
                        context_with_actions.task_parameters.insert(
                            "suggested_actions".to_string(),
                            serde_json::to_value(&actions)?
                        );
                    }
                    
                    // Execute via sub-agent
                    let sub_agent_request = SubAgentRequest::new(
                        sub_agent_kind,
                        session_id.to_string(),
                        context_with_actions
                    );
                    
                    match self.execute_sub_agent(sub_agent_request).await {
                        Ok(response) => {
                            // Store result
                            let result_json = serde_json::to_value(&response.output)?;
                            step_results.insert(step_id.to_string(), result_json);
                            
                            self.update_step_status(
                                session_id,
                                &exec_step.id,
                                StepStatus::Completed,
                                Some(format!("Sub-agent {:?} completed successfully", sub_agent_kind))
                            ).await?;
                            
                            log::info!("Step {} completed successfully", step_id);
                        }
                        Err(e) => {
                            log::error!("Step {} failed: {}", step_id, e);
                            self.update_step_status(
                                session_id,
                                &exec_step.id,
                                StepStatus::Failed,
                                Some(format!("Sub-agent {:?} failed: {}", sub_agent_kind, e))
                            ).await?;
                            // Continue with other steps even if one fails
                        }
                    }
                    
                    // Emit updated step message
                    let updated_session = self.get_session(session_id).await?;
                    let updated_exec_step = updated_session.steps.iter()
                        .find(|s| s.id == exec_step.id)
                        .cloned()
                        .unwrap_or(exec_step.clone());
                    self.emit_step_message(&updated_exec_step, &updated_session);
                }
                
                log::info!("Phase 2 completed: Executed {} plan steps", steps.len());
            } else {
                log::warn!("No steps found in Orchestrator plan, falling back to single execution");
                // Fallback: execute as a single step
                self.execute_single_fallback_step(session_id, session).await?;
            }
        } else {
            log::warn!("No Orchestrator plan available, falling back to single execution");
            // Fallback: execute as a single step
            self.execute_single_fallback_step(session_id, session).await?;
        }
        
        log::info!("Phase 2 completed: Security testing workflow executed");
        
        // === Phase 3: Report Generation ===
        self.update_stage(session_id, TestStage::Report).await?;
        
        let final_session = self.get_session(session_id).await?;
        let summary = format!(
            "Orchestrator completed security testing for {}. \
            Task kind: {:?}, Steps completed: {}, Findings: {} (High-risk: {})",
            session.primary_target,
            session.task_kind,
            final_session.steps.len(),
            final_session.findings.len(),
            final_session.get_high_risk_findings().len()
        );
        
        log::info!("Orchestrator workflow completed: {}", summary);
        
        Ok(summary)
    }
    
    /// Substitute variables in JSON (like #E1, #E2) with previous step results
    fn substitute_variables_in_json(
        &self,
        json: &serde_json::Value,
        step_results: &HashMap<String, serde_json::Value>,
    ) -> serde_json::Value {
        match json {
            serde_json::Value::String(s) => {
                // Check if this is a variable reference like "#E1"
                if s.starts_with('#') && step_results.contains_key(s) {
                    step_results[s].clone()
                } else {
                    json.clone()
                }
            }
            serde_json::Value::Array(arr) => {
                serde_json::Value::Array(
                    arr.iter()
                        .map(|v| self.substitute_variables_in_json(v, step_results))
                        .collect()
                )
            }
            serde_json::Value::Object(obj) => {
                let mut new_obj = serde_json::Map::new();
                for (k, v) in obj {
                    new_obj.insert(
                        k.clone(),
                        self.substitute_variables_in_json(v, step_results)
                    );
                }
                serde_json::Value::Object(new_obj)
            }
            _ => json.clone(),
        }
    }
    
    /// Execute a single tool from the plan
    async fn execute_plan_tool(
        &self,
        tool_name: &str,
        args: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        use crate::tools::{FrameworkType, UnifiedToolCall};
        
        let framework_adapter = crate::tools::get_framework_adapter(FrameworkType::ReWOO).await?;
        
        // Convert args to HashMap
        let params = if let Some(obj) = args.as_object() {
            obj.iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect()
        } else {
            HashMap::new()
        };
        
        let tool_call = UnifiedToolCall {
            id: uuid::Uuid::new_v4().to_string(),
            tool_name: tool_name.to_string(),
            parameters: params,
            timeout: Some(std::time::Duration::from_secs(300)),
            context: HashMap::new(),
            retry_count: 0,
        };
        
        let result = framework_adapter.execute_tool(tool_call).await?;
        
        if result.success {
            Ok(result.output)
        } else {
            Err(anyhow::anyhow!("{}", result.error.unwrap_or_else(|| "Unknown error".to_string())))
        }
    }
    
    /// Fallback: execute as a single step using Plan-and-Execute
    async fn execute_single_fallback_step(
        &self,
        session_id: &str,
        session: &TestSession,
    ) -> Result<()> {
        log::info!("Executing fallback single execution step");
        
        let exec_step = TestStep::new(
            2,
            SubAgentKind::PlanAndExecute,
            TestStepType::ExecuteLoginFlow,
            "Executing security testing workflow (fallback)".to_string()
        );
        self.add_step(session_id, exec_step.clone()).await?;
        
        let current_session = self.get_session(session_id).await?;
        self.emit_step_message(&exec_step, &current_session);
        
        let exec_context = self.build_sub_agent_context(
            session_id,
            format!("Execute security testing for: {}", session.primary_target)
        ).await?;
        
        let exec_request = SubAgentRequest::new(
            SubAgentKind::PlanAndExecute,
            session_id.to_string(),
            exec_context
        );
        
        let exec_response = self.execute_sub_agent(exec_request).await?;
        self.update_step_status(
            session_id,
            &exec_step.id,
            StepStatus::Completed,
            Some(format!("Execution completed: {:?}", exec_response.kind))
        ).await?;
        
        Ok(())
    }
}
