//! OODA执行器
//!
//! 实现OODA四阶段执行逻辑和错误回退机制

use super::types::*;
use super::guardrails::GuardrailManager;
use super::threat_intel::ThreatIntelManager;
use super::engine_dispatcher::EngineDispatcher;
use super::memory_integration::TravelMemoryIntegration;
use crate::engines::llm_client::{LlmClient, create_client as create_llm_client};
use crate::utils::message_emitter::{StandardMessageEmitter, TravelPhaseStep, TravelAction};
use crate::utils::ordered_message::ArchitectureType;
use anyhow::{Result, anyhow};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use uuid::Uuid;

/// OODA执行器
pub struct OodaExecutor {
    config: TravelConfig,
    guardrail_manager: GuardrailManager,
    threat_intel_manager: ThreatIntelManager,
    engine_dispatcher: EngineDispatcher,
    memory_integration: Option<TravelMemoryIntegration>,
    // 消息发送相关字段
    app_handle: Option<Arc<tauri::AppHandle>>,
    execution_id: Option<String>,
    message_id: Option<String>,
    conversation_id: Option<String>,
}

impl OodaExecutor {
    pub fn new(config: TravelConfig) -> Self {
        let guardrail_manager = GuardrailManager::new(config.guardrail_config.clone());
        let threat_intel_manager = ThreatIntelManager::new(config.threat_intel_config.clone());
        let engine_dispatcher = EngineDispatcher::new();

        Self {
            config,
            guardrail_manager,
            threat_intel_manager,
            engine_dispatcher,
            memory_integration: None,
            app_handle: None,
            execution_id: None,
            message_id: None,
            conversation_id: None,
        }
    }

    /// 设置Memory集成
    pub fn with_memory_integration(mut self, memory_integration: TravelMemoryIntegration) -> Self {
        self.memory_integration = Some(memory_integration);
        self
    }

    /// 设置Engine Dispatcher
    pub fn with_engine_dispatcher(mut self, dispatcher: EngineDispatcher) -> Self {
        self.engine_dispatcher = dispatcher;
        self
    }

    /// 设置AppHandle用于消息发送
    pub fn with_app_handle(mut self, app_handle: Arc<tauri::AppHandle>) -> Self {
        self.app_handle = Some(app_handle);
        self
    }

    /// 设置执行ID、消息ID和会话ID
    pub fn with_message_ids(mut self, execution_id: String, message_id: String, conversation_id: Option<String>) -> Self {
        self.execution_id = Some(execution_id);
        self.message_id = Some(message_id);
        self.conversation_id = conversation_id;
        self
    }

    /// 创建消息发送器
    fn create_emitter(&self) -> Option<StandardMessageEmitter> {
        if let (Some(app_handle), Some(execution_id), Some(message_id)) = 
            (&self.app_handle, &self.execution_id, &self.message_id) 
        {
            Some(StandardMessageEmitter::new(
                app_handle.clone(),
                execution_id.clone(),
                message_id.clone(),
                self.conversation_id.clone(),
                ArchitectureType::Travel,
            ))
        } else {
            None
        }
    }

    /// 发送阶段思考（流式）
    fn emit_thought(&self, cycle: u32, phase: &str, thought: &str) {
        if let Some(emitter) = self.create_emitter() {
            emitter.emit_travel_thought(cycle, phase, thought);
        }
    }

    /// 发送工具调用开始
    fn emit_tool_start(&self, cycle: u32, phase: &str, tool: &str, args: &serde_json::Value) {
        if let Some(emitter) = self.create_emitter() {
            emitter.emit_travel_tool_start(cycle, phase, tool, args);
        }
    }

    /// 发送工具调用完成
    fn emit_tool_complete(&self, cycle: u32, phase: &str, tool: &str, args: &serde_json::Value, result: &serde_json::Value, success: bool) {
        if let Some(emitter) = self.create_emitter() {
            emitter.emit_travel_tool_complete(cycle, phase, tool, args, result, success);
        }
    }

    /// 发送阶段完成
    fn emit_phase_complete(&self, cycle: u32, phase: &str, output: Option<serde_json::Value>) {
        if let Some(emitter) = self.create_emitter() {
            emitter.emit_travel_phase_complete(cycle, phase, output);
        }
    }

    /// 发送阶段错误
    fn emit_phase_error(&self, cycle: u32, phase: &str, error: &str) {
        if let Some(emitter) = self.create_emitter() {
            emitter.emit_travel_phase_error(cycle, phase, error);
        }
    }

    /// 执行单次OODA循环
    pub async fn execute_cycle(
        &self,
        cycle_number: u32,
        task_description: &str,
        task_complexity: TaskComplexity,
        context: &mut HashMap<String, serde_json::Value>,
    ) -> Result<OodaCycle> {
        let mut cycle = OodaCycle::new(cycle_number);

        log::info!(
            "Starting OODA cycle #{} for task: {}",
            cycle_number,
            task_description
        );

        // 执行四个阶段
        match self.execute_observe_phase(&mut cycle, context).await {
            Ok(_) => {}
            Err(e) => {
                log::error!("Observe phase failed: {}", e);
                cycle.fail(format!("Observe phase error: {}", e));
                return Ok(cycle);
            }
        }

        match self.execute_orient_phase(&mut cycle, context).await {
            Ok(_) => {}
            Err(e) => {
                log::error!("Orient phase failed: {}", e);
                // 尝试回退到Observe
                if let Err(rollback_err) = self.handle_error_rollback(&mut cycle, OodaPhase::Orient, e.to_string(), context).await {
                    log::error!("Rollback failed: {}", rollback_err);
                    cycle.fail(format!("Orient phase error with failed rollback: {}", e));
                    return Ok(cycle);
                }
            }
        }

        match self.execute_decide_phase(&mut cycle, task_complexity.clone(), context).await {
            Ok(_) => {}
            Err(e) => {
                log::error!("Decide phase failed: {}", e);
                // 尝试回退到Orient
                if let Err(rollback_err) = self.handle_error_rollback(&mut cycle, OodaPhase::Decide, e.to_string(), context).await {
                    log::error!("Rollback failed: {}", rollback_err);
                    cycle.fail(format!("Decide phase error with failed rollback: {}", e));
                    return Ok(cycle);
                }
            }
        }

        match self.execute_act_phase(&mut cycle, task_complexity.clone(), context).await {
            Ok(_) => {}
            Err(e) => {
                log::error!("Act phase failed: {}", e);
                // 尝试回退到Orient重新分析
                if let Err(rollback_err) = self.handle_error_rollback(&mut cycle, OodaPhase::Act, e.to_string(), context).await {
                    log::error!("Rollback failed: {}", rollback_err);
                    cycle.fail(format!("Act phase error with failed rollback: {}", e));
                    return Ok(cycle);
                }
            }
        }

        // 构建循环结果
        let result = self.build_cycle_result(&cycle);
        cycle.complete(result);

        // 存储执行经验到Memory
        if let Some(memory_integration) = &self.memory_integration {
            if let Err(e) = memory_integration.store_execution(&cycle).await {
                log::warn!("Failed to store execution experience: {}", e);
            } else {
                log::info!("Execution experience stored to memory");
            }
        }

        log::info!("OODA cycle #{} completed successfully", cycle_number);
        Ok(cycle)
    }

    /// 执行Observe阶段(侦察)
    async fn execute_observe_phase(
        &self,
        cycle: &mut OodaCycle,
        context: &mut HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        let cycle_num = cycle.cycle_number;
        log::info!("Executing Observe phase (cycle #{})", cycle_num);
        
        // 发送阶段开始
        self.emit_thought(cycle_num, "Observe", "[PHASE_START] Starting Observe phase - gathering target information...");

        let started_at = SystemTime::now();
        cycle.current_phase = OodaPhase::Observe;
        let mut tool_calls = Vec::new();

        // 1. 查询Memory:获取相似任务经验
        if let Some(memory_integration) = &self.memory_integration {
            let task_desc = context.get("task_description")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown task");
            let target_info_str = context.get("target_info")
                .map(|v| v.to_string())
                .unwrap_or_default();
            
            match memory_integration
                .query_similar_experiences(task_desc, &target_info_str)
                .await
            {
                Ok(experiences) => {
                    log::info!("Found {} similar experiences from memory", experiences.len());
                    self.emit_thought(cycle_num, "Observe", &format!("[MEMORY] Found {} similar experiences from memory", experiences.len()));
                    context.insert(
                        "memory_experiences".to_string(),
                        serde_json::to_value(&experiences).unwrap_or(serde_json::json!([]))
                    );
                }
                Err(e) => {
                    log::warn!("Failed to query memory experiences: {}", e);
                    self.emit_thought(cycle_num, "Observe", &format!("[WARNING] Memory query failed: {}", e));
                }
            }
        }

        // 2. 护栏检查
        let target_info = context.get("target_info").cloned().unwrap_or(serde_json::json!({}));
        let guardrail_checks = self
            .guardrail_manager
            .check_observe_phase(&target_info)
            .await?;

        self.emit_thought(cycle_num, "Observe", &format!("[GUARDRAIL] Guardrail checks: {} items passed", guardrail_checks.len()));

        // 3. 收集目标信息（带工具调用追踪）
        let observations = self.collect_observations_with_tracking(cycle_num, context, &mut tool_calls).await?;
        
        self.emit_thought(cycle_num, "Observe", "[COMPLETE] Target observations collected");

        // 4. 记录阶段执行
        let phase_execution = OodaPhaseExecution {
            phase: OodaPhase::Observe,
            started_at,
            completed_at: Some(SystemTime::now()),
            status: PhaseExecutionStatus::Completed,
            input: target_info,
            output: Some(serde_json::to_value(&observations)?),
            guardrail_checks,
            tool_calls,
            error: None,
        };

        cycle.add_phase_execution(phase_execution);

        // 5. 更新上下文
        context.insert("observations".to_string(), serde_json::to_value(&observations)?);

        // 发送阶段完成
        self.emit_phase_complete(cycle_num, "Observe", Some(serde_json::to_value(&observations)?));
        Ok(())
    }

    /// 执行Orient阶段(分析定位)
    async fn execute_orient_phase(
        &self,
        cycle: &mut OodaCycle,
        context: &mut HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        let cycle_num = cycle.cycle_number;
        log::info!("Executing Orient phase (cycle #{})", cycle_num);
        
        self.emit_thought(cycle_num, "Orient", "[PHASE_START] Starting Orient phase - analyzing observations...");

        let started_at = SystemTime::now();
        cycle.current_phase = OodaPhase::Orient;

        let observations = context.get("observations").cloned().unwrap_or(serde_json::json!({}));

        // 1. 查询Memory:获取知识图谱信息
        if let Some(memory_integration) = &self.memory_integration {
            let entities = self.extract_entities(&observations);
            
            match memory_integration.query_knowledge_graph(&entities).await {
                Ok(knowledge_entities) => {
                    log::info!("Found {} knowledge entities from memory", knowledge_entities.len());
                    self.emit_thought(cycle_num, "Orient", &format!("🧠 Found {} knowledge entities", knowledge_entities.len()));
                    context.insert(
                        "memory_knowledge".to_string(),
                        serde_json::to_value(&knowledge_entities).unwrap_or(serde_json::json!([]))
                    );
                }
                Err(e) => {
                    log::warn!("Failed to query knowledge graph: {}", e);
                    self.emit_thought(cycle_num, "Orient", &format!("[WARNING] Knowledge graph query failed: {}", e));
                }
            }
        }

        // 2. 查询威胁情报
        let threat_query = self.build_threat_query(&observations);
        self.emit_thought(cycle_num, "Orient", "[SEARCH] Querying threat intelligence...");
        
        let mut threat_context = HashMap::new();
        if let Some(tech) = observations.get("technology").and_then(|v| v.as_str()) {
            threat_context.insert("technology".to_string(), serde_json::Value::String(tech.to_string()));
        }

        let threats = self
            .threat_intel_manager
            .query_threat_intel(&threat_query, &threat_context)
            .await?;

        // 3. 分析威胁
        let vulnerabilities = self.identify_vulnerabilities(&observations, &threats);
        self.emit_thought(cycle_num, "Orient", &format!("[WARNING] Identified {} vulnerabilities", vulnerabilities.len()));

        let analysis = self
            .threat_intel_manager
            .analyze_threats(threats, vulnerabilities)
            .await;

        // 4. 护栏检查
        let guardrail_checks = self
            .guardrail_manager
            .check_orient_phase(&analysis)
            .await?;

        self.emit_thought(cycle_num, "Orient", &format!("[GUARDRAIL] Guardrail checks: {} items passed", guardrail_checks.len()));

        // 5. 记录阶段执行
        let phase_execution = OodaPhaseExecution {
            phase: OodaPhase::Orient,
            started_at,
            completed_at: Some(SystemTime::now()),
            status: PhaseExecutionStatus::Completed,
            input: observations,
            output: Some(serde_json::to_value(&analysis)?),
            guardrail_checks,
            tool_calls: vec![],
            error: None,
        };

        cycle.add_phase_execution(phase_execution);

        // 6. 更新上下文
        context.insert("threat_analysis".to_string(), serde_json::to_value(&analysis)?);

        self.emit_phase_complete(cycle_num, "Orient", Some(serde_json::to_value(&analysis)?));
        Ok(())
    }

    /// 执行Decide阶段(决策)
    async fn execute_decide_phase(
        &self,
        cycle: &mut OodaCycle,
        task_complexity: TaskComplexity,
        context: &mut HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        let cycle_num = cycle.cycle_number;
        log::info!("Executing Decide phase (cycle #{})", cycle_num);
        
        self.emit_thought(cycle_num, "Decide", "[PHASE_START] Starting Decide phase - generating action plan...");

        let started_at = SystemTime::now();
        cycle.current_phase = OodaPhase::Decide;

        // 1. 查询Memory:获取计划模板
        if let Some(memory_integration) = &self.memory_integration {
            let task_type = context.get("task_type")
                .and_then(|v| v.as_str())
                .unwrap_or("security_test");
            
            match memory_integration.get_plan_templates(task_type).await {
                Ok(templates) => {
                    log::info!("Found {} plan templates from memory", templates.len());
                    self.emit_thought(cycle_num, "Decide", &format!("[TEMPLATE] Found {} plan templates", templates.len()));
                    context.insert(
                        "memory_plan_templates".to_string(),
                        serde_json::to_value(&templates).unwrap_or(serde_json::json!([]))
                    );
                }
                Err(e) => {
                    log::warn!("Failed to query plan templates: {}", e);
                    self.emit_thought(cycle_num, "Decide", &format!("[WARNING] Plan template query failed: {}", e));
                }
            }
        }

        // 2. 获取威胁分析结果
        let analysis_value = context.get("threat_analysis").cloned().unwrap_or(serde_json::json!({}));
        let analysis: ThreatAnalysis = serde_json::from_value(analysis_value.clone())?;

        // 3. 生成行动计划
        let action_plan = self.generate_action_plan(&analysis, task_complexity, context)?;
        self.emit_thought(cycle_num, "Decide", &format!("[PLAN] Generated action plan: {} ({} steps)", action_plan.name, action_plan.steps.len()));

        // 4. 护栏检查
        let guardrail_checks = self
            .guardrail_manager
            .check_decide_phase(&action_plan)
            .await?;

        self.emit_thought(cycle_num, "Decide", &format!("[GUARDRAIL] Guardrail checks: {} items passed", guardrail_checks.len()));

        // 5. 记录阶段执行
        let phase_execution = OodaPhaseExecution {
            phase: OodaPhase::Decide,
            started_at,
            completed_at: Some(SystemTime::now()),
            status: PhaseExecutionStatus::Completed,
            input: analysis_value,
            output: Some(serde_json::to_value(&action_plan)?),
            guardrail_checks,
            tool_calls: vec![],
            error: None,
        };

        cycle.add_phase_execution(phase_execution);

        // 6. 更新上下文
        context.insert("action_plan".to_string(), serde_json::to_value(&action_plan)?);

        self.emit_phase_complete(cycle_num, "Decide", Some(serde_json::to_value(&action_plan)?));
        Ok(())
    }

    /// 执行Act阶段(执行)
    async fn execute_act_phase(
        &self,
        cycle: &mut OodaCycle,
        task_complexity: TaskComplexity,
        context: &mut HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        let cycle_num = cycle.cycle_number;
        log::info!("Executing Act phase (cycle #{})", cycle_num);
        
        self.emit_thought(cycle_num, "Act", "[PHASE_START] Starting Act phase - executing action plan...");

        let started_at = SystemTime::now();
        cycle.current_phase = OodaPhase::Act;

        // 1. 获取行动计划
        let plan_value = context.get("action_plan").cloned().unwrap_or(serde_json::json!({}));
        let action_plan: ActionPlan = serde_json::from_value(plan_value.clone())?;

        self.emit_thought(cycle_num, "Act", &format!("[EXECUTE] Executing plan: {} ({} steps)", action_plan.name, action_plan.steps.len()));

        // 2. 最终护栏检查
        let execution_context = serde_json::json!({
            "timeout": action_plan.estimated_duration,
        });
        let guardrail_checks = self
            .guardrail_manager
            .check_act_phase(&action_plan, &execution_context)
            .await?;

        self.emit_thought(cycle_num, "Act", &format!("[GUARDRAIL] Final guardrail checks: {} items passed", guardrail_checks.len()));

        // 3. 调度执行
        let mut exec_context = HashMap::new();
        for (k, v) in context.iter() {
            exec_context.insert(k.clone(), v.clone());
        }

        // 传递消息相关ID和cycle_number到dispatcher
        if let Some(_app_handle) = &self.app_handle {
            exec_context.insert("_app_handle".to_string(), serde_json::json!({}));
        }
        if let Some(execution_id) = &self.execution_id {
            exec_context.insert("_execution_id".to_string(), serde_json::Value::String(execution_id.clone()));
        }
        if let Some(message_id) = &self.message_id {
            exec_context.insert("_message_id".to_string(), serde_json::Value::String(message_id.clone()));
        }
        if let Some(conversation_id) = &self.conversation_id {
            exec_context.insert("_conversation_id".to_string(), serde_json::Value::String(conversation_id.clone()));
        }
        exec_context.insert("_cycle_number".to_string(), serde_json::json!(cycle_num));

        self.emit_thought(cycle_num, "Act", "[DISPATCH] Dispatching execution to appropriate engine...");

        let execution_result = self
            .engine_dispatcher
            .dispatch(task_complexity, &action_plan, &exec_context)
            .await?;

        self.emit_thought(cycle_num, "Act", "[SUCCESS] Execution completed");

        // 4. 记录阶段执行
        let phase_execution = OodaPhaseExecution {
            phase: OodaPhase::Act,
            started_at,
            completed_at: Some(SystemTime::now()),
            status: PhaseExecutionStatus::Completed,
            input: plan_value,
            output: Some(execution_result.clone()),
            guardrail_checks,
            tool_calls: vec![],
            error: None,
        };

        cycle.add_phase_execution(phase_execution);

        // 5. 更新上下文
        context.insert("execution_result".to_string(), execution_result.clone());

        // 发送阶段完成
        self.emit_phase_complete(cycle_num, "Act", Some(execution_result));
        Ok(())
    }

    /// 错误回退处理
    async fn handle_error_rollback(
        &self,
        cycle: &mut OodaCycle,
        failed_phase: OodaPhase,
        error: String,
        _context: &mut HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        log::info!("Handling error rollback for phase: {:?}", failed_phase);

        match &self.config.rollback_strategy {
            RollbackStrategy::NoRollback => {
                return Err(anyhow!("Rollback disabled: {}", error));
            }
            RollbackStrategy::PreviousPhase => {
                let target_phase = match failed_phase {
                    OodaPhase::Orient => OodaPhase::Observe,
                    OodaPhase::Decide => OodaPhase::Orient,
                    OodaPhase::Act => OodaPhase::Orient,
                    OodaPhase::Observe => {
                        return Err(anyhow!("Cannot rollback from Observe phase"));
                    }
                };
                log::info!("Rolling back to previous phase: {:?}", target_phase);
                cycle.current_phase = target_phase;
            }
            RollbackStrategy::SpecificPhase(target) => {
                log::info!("Rolling back to specific phase: {:?}", target);
                cycle.current_phase = target.clone();
            }
            RollbackStrategy::Intelligent => {
                // 智能回退:根据错误类型决定
                let target_phase = self.determine_rollback_target(&failed_phase, &error);
                log::info!("Intelligent rollback to phase: {:?}", target_phase);
                cycle.current_phase = target_phase;
            }
        }

        // 标记最后一个阶段为回退状态
        if let Some(last_execution) = cycle.phase_history.last_mut() {
            last_execution.status = PhaseExecutionStatus::RolledBack;
            last_execution.error = Some(error);
        }

        Ok(())
    }

    /// 确定智能回退目标
    fn determine_rollback_target(&self, failed_phase: &OodaPhase, error: &str) -> OodaPhase {
        let error_lower = error.to_lowercase();

        // 如果是数据不足错误,回退到Observe
        if error_lower.contains("insufficient data")
            || error_lower.contains("missing information")
        {
            return OodaPhase::Observe;
        }

        // 如果是分析错误,回退到Orient
        if error_lower.contains("analysis failed") || error_lower.contains("threat intel") {
            return OodaPhase::Orient;
        }

        // 默认回退到上一个阶段
        match failed_phase {
            OodaPhase::Orient => OodaPhase::Observe,
            OodaPhase::Decide | OodaPhase::Act => OodaPhase::Orient,
            OodaPhase::Observe => OodaPhase::Observe,
        }
    }

    /// 收集观察信息（带工具调用追踪）
    async fn collect_observations_with_tracking(
        &self,
        cycle_num: u32,
        context: &HashMap<String, serde_json::Value>,
        tool_calls: &mut Vec<ToolCallRecord>,
    ) -> Result<HashMap<String, serde_json::Value>> {
        let mut observations = HashMap::new();

        let target = context.get("target")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let task_type = context.get("task_type")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let query = context.get("query")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        observations.insert("target".to_string(), serde_json::json!(target));
        observations.insert("task_type".to_string(), serde_json::json!(task_type));

        log::info!("Collecting observations for task_type: {}, target: {} (cycle #{})", task_type, target, cycle_num);

        // 使用 LLM 动态规划 Observe 流程（使用内置 LlmClient）
        let llm_client = match &self.engine_dispatcher.ai_service {
            Some(ai_service) => Some(create_llm_client(ai_service)),
            None => {
                log::error!("Travel OODA: AI service not available, cannot create LLM client");
                None
            }
        };

        if let Some(client) = llm_client {
            match self.plan_observation_with_llm_tracked(&client, cycle_num, task_type, target, query, context, tool_calls).await {
                Ok(planned_observations) => {
                    for (key, value) in planned_observations {
                        observations.insert(key, value);
                    }
                }
                Err(e) => {
                    log::error!("Travel OODA: LLM observation planning failed: {}", e);
                    return Err(anyhow!("LLM observation planning failed: {}", e));
                }
            }
        } else {
            return Err(anyhow!("Travel LLM client not available"));
        }

        Ok(observations)
    }

    /// 收集观察信息
    /// 使用 LLM 动态规划 Observe 流程
    async fn collect_observations(
        &self,
        context: &HashMap<String, serde_json::Value>,
    ) -> Result<HashMap<String, serde_json::Value>> {
        let mut observations = HashMap::new();

        // 从上下文中提取任务信息
        let target = context.get("target")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let task_type = context.get("task_type")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let query = context.get("query")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        observations.insert("target".to_string(), serde_json::json!(target));
        observations.insert("task_type".to_string(), serde_json::json!(task_type));

        log::info!("Collecting observations for task_type: {}, target: {}", task_type, target);

        // 使用 LLM 动态规划 Observe 流程（使用内置 LlmClient）
        let llm_client = match &self.engine_dispatcher.ai_service {
            Some(ai_service) => Some(create_llm_client(ai_service)),
            None => {
                log::error!("Travel OODA: AI service not available, cannot create LLM client");
                None
            }
        };

        if let Some(client) = llm_client {
            match self.plan_observation_with_llm(&client, task_type, target, query, context).await {
                Ok(planned_observations) => {
                    for (key, value) in planned_observations {
                        observations.insert(key, value);
                    }
                }
                Err(e) => {
                    log::error!("Travel OODA: LLM observation planning failed: {}", e);
                    return Err(anyhow!("LLM observation planning failed: {}", e));
                }
            }
        } else {
            return Err(anyhow!("Travel LLM client not available"));
        }

        Ok(observations)
    }

    /// 使用 LLM 规划观察流程（使用内置 LlmClient）
    async fn plan_observation_with_llm(
        &self,
        llm_client: &LlmClient,
        task_type: &str,
        target: &str,
        query: &str,
        context: &HashMap<String, serde_json::Value>,
    ) -> Result<HashMap<String, serde_json::Value>> {
        // 构建可用工具列表（从 context 中获取允许的工具）
        let available_tools = self.get_available_tools_for_observation(context).await;
        
        // 构建 system prompt 和 user prompt
        let (system_prompt, user_prompt) = self.build_observation_planning_prompt(
            task_type,
            target,
            query,
            &available_tools,
        ).await?;

        // 调用 LLM（使用内置客户端）
        let response = llm_client
            .completion(Some(&system_prompt), &user_prompt)
            .await?;

        // 解析 LLM 响应
        let plan: serde_json::Value = self.parse_llm_observation_plan(&response)?;

        // 执行规划的步骤
        let mut observations = HashMap::new();
        
        if let Some(steps) = plan.get("steps").and_then(|s| s.as_array()) {
            log::info!("LLM planned {} observation steps", steps.len());
            
            for (idx, step) in steps.iter().enumerate() {
                let tool_name = step.get("tool")
                    .and_then(|t| t.as_str())
                    .unwrap_or("");
                let args = step.get("args")
                    .and_then(|a| a.as_object())
                    .map(|obj| {
                        obj.iter()
                            .map(|(k, v)| (k.clone(), v.clone()))
                            .collect::<HashMap<String, serde_json::Value>>()
                    })
                    .unwrap_or_default();
                
                log::info!("Executing observation step {}: {} with args: {:?}", idx + 1, tool_name, args);
                
                // 执行工具
                match self.engine_dispatcher.execute_tool(tool_name, &args, context).await {
                    Ok(result) => {
                        observations.insert(format!("{}_result", tool_name), result);
                    }
                    Err(e) => {
                        log::warn!("Observation step {} ({}) failed: {}", idx + 1, tool_name, e);
                    }
                }
            }
        }

        // 添加规划理由
        if let Some(reasoning) = plan.get("reasoning").and_then(|r| r.as_str()) {
            observations.insert("observation_reasoning".to_string(), serde_json::json!(reasoning));
        }

        Ok(observations)
    }

    /// 使用 LLM 规划观察流程（带工具调用追踪，使用内置 LlmClient）
    async fn plan_observation_with_llm_tracked(
        &self,
        llm_client: &LlmClient,
        cycle_num: u32,
        task_type: &str,
        target: &str,
        query: &str,
        context: &HashMap<String, serde_json::Value>,
        tool_calls: &mut Vec<ToolCallRecord>,
    ) -> Result<HashMap<String, serde_json::Value>> {
        let available_tools = self.get_available_tools_for_observation(context).await;
        
        let (system_prompt, user_prompt) = self.build_observation_planning_prompt(
            task_type,
            target,
            query,
            &available_tools,
        ).await?;

        // 使用内置 LLM 客户端
        let response = llm_client
            .completion(Some(&system_prompt), &user_prompt)
            .await?;

        if let Ok(plan_preview) = serde_json::from_str::<serde_json::Value>(&response) {
            if let Some(reasoning) = plan_preview.get("reasoning").and_then(|v| v.as_str()) {
                self.emit_thought(cycle_num, "Observe", &format!("Thought: {}", reasoning));
            }
        }

        let plan: serde_json::Value = self.parse_llm_observation_plan(&response)?;
        let mut observations = HashMap::new();
        
        if let Some(steps) = plan.get("steps").and_then(|s| s.as_array()) {
            log::info!("LLM planned {} observation steps", steps.len());
            
            for (idx, step) in steps.iter().enumerate() {
                let tool_name = step.get("tool")
                    .and_then(|t| t.as_str())
                    .unwrap_or("");
                let args = step.get("args")
                    .and_then(|a| a.as_object())
                    .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect::<HashMap<String, serde_json::Value>>())
                    .unwrap_or_default();
                
                let args_value = serde_json::to_value(&args).unwrap_or(serde_json::json!({}));
                log::info!("Executing observation step {}: {} with args: {:?}", idx + 1, tool_name, args);
                
                // 发送工具调用开始
                self.emit_tool_start(cycle_num, "Observe", tool_name, &args_value);
                let call_time = SystemTime::now();
                
                // 执行工具
                match self.engine_dispatcher.execute_tool(tool_name, &args, context).await {
                    Ok(result) => {
                        // 发送工具调用完成
                        self.emit_tool_complete(cycle_num, "Observe", tool_name, &args_value, &result, true);
                        
                        // 记录工具调用
                        tool_calls.push(ToolCallRecord {
                            call_id: Uuid::new_v4().to_string(),
                            tool_name: tool_name.to_string(),
                            args: args.clone(),
                            called_at: call_time,
                            completed_at: Some(SystemTime::now()),
                            status: ToolCallStatus::Completed,
                            result: Some(result.clone()),
                            error: None,
                        });
                        
                        observations.insert(format!("{}_result", tool_name), result);
                    }
                    Err(e) => {
                        let error_msg = e.to_string();
                        self.emit_tool_complete(cycle_num, "Observe", tool_name, &args_value, &serde_json::json!({"error": error_msg}), false);
                        
                        tool_calls.push(ToolCallRecord {
                            call_id: Uuid::new_v4().to_string(),
                            tool_name: tool_name.to_string(),
                            args: args.clone(),
                            called_at: call_time,
                            completed_at: Some(SystemTime::now()),
                            status: ToolCallStatus::Failed,
                            result: None,
                            error: Some(error_msg.clone()),
                        });
                        
                        log::warn!("Observation step {} ({}) failed: {}", idx + 1, tool_name, error_msg);
                    }
                }
            }
        }

        if let Some(reasoning) = plan.get("reasoning").and_then(|r| r.as_str()) {
            observations.insert("observation_reasoning".to_string(), serde_json::json!(reasoning));
        }

        Ok(observations)
    }

    /// 降级观察流程（带追踪）
    async fn collect_observations_fallback_tracked(
        &self,
        cycle_num: u32,
        target: &str,
        task_type: &str,
        context: &HashMap<String, serde_json::Value>,
        observations: &mut HashMap<String, serde_json::Value>,
        tool_calls: &mut Vec<ToolCallRecord>,
    ) {
        if target.is_empty() {
            log::warn!("No target specified, skipping observation collection");
            return;
        }

        match task_type {
            "web_pentest" | "api_pentest" => {
                if let Some(result) = self.try_tool_with_tracking(cycle_num, "analyze_website", target, context, tool_calls).await {
                    observations.insert("website_analysis".to_string(), result);
                }
                if let Some(result) = self.try_tool_with_tracking(cycle_num, "http_request", target, context, tool_calls).await {
                    observations.insert("http_response".to_string(), result);
                }
            }
            "code_audit" => {
                observations.insert("code_target".to_string(), serde_json::json!(target));
                observations.insert("audit_type".to_string(), serde_json::json!("static_analysis"));
            }
            "ctf" => {
                if target.starts_with("http://") || target.starts_with("https://") {
                    if let Some(result) = self.try_tool_with_tracking(cycle_num, "http_request", target, context, tool_calls).await {
                        observations.insert("http_response".to_string(), result);
                    }
                } else {
                    observations.insert("ctf_target".to_string(), serde_json::json!(target));
                }
            }
            _ => {
                log::warn!("Unknown task type: {}, using basic HTTP observation", task_type);
                if target.starts_with("http://") || target.starts_with("https://") {
                    if let Some(result) = self.try_tool_with_tracking(cycle_num, "http_request", target, context, tool_calls).await {
                        observations.insert("http_response".to_string(), result);
                    }
                }
            }
        }
    }

    /// 尝试执行工具（带追踪）
    async fn try_tool_with_tracking(
        &self,
        cycle_num: u32,
        tool_name: &str,
        target: &str,
        context: &HashMap<String, serde_json::Value>,
        tool_calls: &mut Vec<ToolCallRecord>,
    ) -> Option<serde_json::Value> {
        let mut args = HashMap::new();
        
        // 根据工具名准备参数
        match tool_name {
            "analyze_website" => {
                let domain = target
                    .trim_start_matches("http://")
                    .trim_start_matches("https://")
                    .split('/')
                    .next()
                    .unwrap_or(target)
                    .split(':')
                    .next()
                    .unwrap_or(target);
                args.insert("domain".to_string(), serde_json::json!(domain));
            }
            "http_request" => {
                args.insert("url".to_string(), serde_json::json!(target));
                args.insert("method".to_string(), serde_json::json!("GET"));
            }
            _ => {
                args.insert("target".to_string(), serde_json::json!(target));
            }
        }

        let args_value = serde_json::to_value(&args).unwrap_or(serde_json::json!({}));
        
        self.emit_tool_start(cycle_num, "Observe", tool_name, &args_value);
        let call_time = SystemTime::now();

        match self.engine_dispatcher.execute_tool(tool_name, &args, context).await {
            Ok(result) => {
                self.emit_tool_complete(cycle_num, "Observe", tool_name, &args_value, &result, true);
                
                tool_calls.push(ToolCallRecord {
                    call_id: Uuid::new_v4().to_string(),
                    tool_name: tool_name.to_string(),
                    args,
                    called_at: call_time,
                    completed_at: Some(SystemTime::now()),
                    status: ToolCallStatus::Completed,
                    result: Some(result.clone()),
                    error: None,
                });
                
                log::info!("Tool {} completed successfully", tool_name);
                Some(result)
            }
            Err(e) => {
                let error_msg = e.to_string();
                self.emit_tool_complete(cycle_num, "Observe", tool_name, &args_value, &serde_json::json!({"error": error_msg}), false);
                
                tool_calls.push(ToolCallRecord {
                    call_id: Uuid::new_v4().to_string(),
                    tool_name: tool_name.to_string(),
                    args,
                    called_at: call_time,
                    completed_at: Some(SystemTime::now()),
                    status: ToolCallStatus::Failed,
                    result: None,
                    error: Some(error_msg.clone()),
                });
                
                log::warn!("Tool {} failed: {}", tool_name, error_msg);
                None
            }
        }
    }

    /// 构建观察规划的 prompt（返回 system prompt 和 user prompt）
    async fn build_observation_planning_prompt(
        &self,
        task_type: &str,
        target: &str,
        query: &str,
        available_tools: &str,
    ) -> Result<(String, String)> {
        use crate::models::prompt::{ArchitectureType, StageType};
        
        // 从数据库获取 Travel Observe 阶段的 prompt 模板
        let system_template = if let Some(prompt_repo) = &self.engine_dispatcher.prompt_repo {
            if let Ok(Some(template)) = prompt_repo
                .get_template_by_arch_stage(ArchitectureType::Travel, StageType::Observe)
                .await
            {
                log::info!("Travel Observe Planner: Using prompt from database");
                template.content
            } else {
                log::warn!("Travel Observe template not found in database, using default template");
                self.get_default_observe_planning_prompt()
            }
        } else {
            log::warn!("No prompt repository available, using default template");
            self.get_default_observe_planning_prompt()
        };
        
        // 填充 system prompt 中的占位符
        let system_prompt = system_template
            .replace("{tools}", available_tools)
            .replace("{task_type}", task_type)
            .replace("{target}", target);
        
        // user prompt 是用户的查询
        let user_prompt = format!(
            "任务类型: {}\n目标: {}\n用户查询: {}",
            task_type,
            target,
            query
        );
        
        Ok((system_prompt, user_prompt))
    }
    
    /// 获取默认的观察规划 prompt
    fn get_default_observe_planning_prompt(&self) -> String {
        r#"你是一个安全测试专家，负责规划 Observe (侦察) 阶段的信息收集流程。

**可用工具**:
{tools}

**任务类型说明**:
- web_pentest: Web 渗透测试 → 使用 analyze_website, http_request, port_scan
- api_pentest: API 安全测试 → 使用 http_request, analyze_website
- code_audit: 代码审计 → 不需要网络工具，直接分析代码
- ctf: CTF 夺旗 → 根据题目类型选择工具
- mobile_security: 移动应用安全 → 分析 APK/IPA 文件
- cloud_security: 云安全评估 → 使用云服务 API
- network_security: 网络安全 → 使用 port_scan, rsubdomain

**请规划需要执行的观察步骤**，以 JSON 格式返回：

```json
{
  "steps": [
    {
      "tool": "工具名称",
      "args": {"参数名": "参数值"},
      "description": "步骤描述"
    }
  ],
  "reasoning": "规划理由"
}
```

**注意事项**:
1. 根据任务类型选择合适的工具
2. 代码审计、CTF 等任务可能不需要网络扫描
3. 工具参数必须正确（如 analyze_website 需要 domain，不是 url）
4. 端口扫描需要 IP 地址，不是域名
5. 只规划侦察阶段，不要包含攻击步骤

只返回 JSON，不要其他文字。"#.to_string()
    }

    /// 解析 LLM 的观察规划
    fn parse_llm_observation_plan(&self, response: &str) -> Result<serde_json::Value> {
        // 尝试提取 JSON（可能包含在 markdown 代码块中）
        let json_str = if response.contains("```json") {
            response
                .split("```json")
                .nth(1)
                .and_then(|s| s.split("```").next())
                .unwrap_or(response)
                .trim()
        } else if response.contains("```") {
            response
                .split("```")
                .nth(1)
                .and_then(|s| s.split("```").next())
                .unwrap_or(response)
                .trim()
        } else {
            response.trim()
        };

        serde_json::from_str(json_str)
            .map_err(|e| anyhow!("Failed to parse LLM observation plan: {}", e))
    }

    /// 获取可用于观察的工具列表（从 Agent 设置的工具白名单中获取）
    async fn get_available_tools_for_observation(&self, context: &HashMap<String, serde_json::Value>) -> String {
        // 从 context 中提取工具白名单
        let allowed_tools: Vec<String> = context
            .get("tools_allow")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|x| x.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_else(Vec::new);
        
        if allowed_tools.is_empty() {
            log::warn!("No allowed tools found in context, using empty tool list");
            return "No tools available".to_string();
        }
        
        log::info!("Building observation tool list from {} allowed tools", allowed_tools.len());
        
        // 尝试从 FrameworkToolAdapter 或 EngineToolAdapter 获取工具详细信息
        let mut tool_descriptions = Vec::new();
        
        if let Some(adapter) = &self.engine_dispatcher.framework_adapter {
            // 使用 FrameworkToolAdapter
            for tool_name in &allowed_tools {
                if let Some(tool_info) = adapter.get_tool_info(tool_name).await {
                    // 构建工具参数签名
                    let mut params = Vec::new();
                    for param in &tool_info.parameters.parameters {
                        let param_type = match param.param_type {
                            crate::tools::ParameterType::String => "string",
                            crate::tools::ParameterType::Number => "number",
                            crate::tools::ParameterType::Boolean => "boolean",
                            crate::tools::ParameterType::Array => "array",
                            crate::tools::ParameterType::Object => "object",
                        };
                        let param_str = if param.required {
                            format!("{}: {}", param.name, param_type)
                        } else {
                            format!("{}?: {}", param.name, param_type)
                        };
                        params.push(param_str);
                    }
                    
                    let signature = format!("{}({})", tool_info.name, params.join(", "));
                    let description = format!("- {} - {}", signature, tool_info.description);
                    tool_descriptions.push(description);
                }
            }
        } else {
            // 降级：使用全局 EngineToolAdapter
            log::info!("No framework adapter, trying global engine adapter for tool info");
            match crate::tools::get_global_engine_adapter() {
                Ok(engine_adapter) => {
                    for tool_name in &allowed_tools {
                        if let Some(tool_info) = engine_adapter.get_tool_info(tool_name).await {
                            // 构建工具参数签名
                            let mut params = Vec::new();
                            for param in &tool_info.parameters.parameters {
                                let param_type = match param.param_type {
                                    crate::tools::ParameterType::String => "string",
                                    crate::tools::ParameterType::Number => "number",
                                    crate::tools::ParameterType::Boolean => "boolean",
                                    crate::tools::ParameterType::Array => "array",
                                    crate::tools::ParameterType::Object => "object",
                                };
                                let param_str = if param.required {
                                    format!("{}: {}", param.name, param_type)
                                } else {
                                    format!("{}?: {}", param.name, param_type)
                                };
                                params.push(param_str);
                            }
                            
                            let signature = format!("{}({})", tool_info.name, params.join(", "));
                            let description = format!("- {} - {}", signature, tool_info.description);
                            tool_descriptions.push(description);
                        }
                    }
                }
                Err(e) => {
                    log::error!("Failed to get global engine adapter: {}", e);
                }
            }
        }
        
        if tool_descriptions.is_empty() {
            log::warn!("No tool descriptions generated, using tool names only");
            allowed_tools.iter().map(|name| format!("- {}", name)).collect::<Vec<_>>().join("\n")
        } else {
            log::info!("Generated {} tool descriptions for observation", tool_descriptions.len());
            tool_descriptions.join("\n")
        }
    }

    /// 降级到默认的观察流程（按任务类型）
    async fn collect_observations_fallback(
        &self,
        target: &str,
        task_type: &str,
        context: &HashMap<String, serde_json::Value>,
        observations: &mut HashMap<String, serde_json::Value>,
    ) {
        if target.is_empty() {
            log::warn!("No target specified, skipping observation collection");
            return;
        }

        match task_type {
            "web_pentest" | "api_pentest" => {
                // Web/API 渗透测试：网站分析 + HTTP 请求 + 端口扫描
                if let Some(result) = self.try_analyze_website(target, context).await {
                    observations.insert("website_analysis".to_string(), result);
                }
                
                if let Some(result) = self.try_http_request(target, context).await {
                    observations.insert("http_response".to_string(), result);
                }
                
                if let Some(result) = self.try_port_scan(target, context).await {
                    observations.insert("port_scan".to_string(), result);
                }
            }
            "code_audit" => {
                // 代码审计：不需要网络扫描，直接记录目标
                observations.insert("code_target".to_string(), serde_json::json!(target));
                observations.insert("audit_type".to_string(), serde_json::json!("static_analysis"));
            }
            "ctf" => {
                // CTF：根据目标类型决定
                if target.starts_with("http://") || target.starts_with("https://") {
                    // Web CTF
                    if let Some(result) = self.try_http_request(target, context).await {
                        observations.insert("http_response".to_string(), result);
                    }
                } else {
                    // 其他类型 CTF
                    observations.insert("ctf_target".to_string(), serde_json::json!(target));
                }
            }
            _ => {
                // 未知任务类型：尝试基本的 HTTP 请求
                log::warn!("Unknown task type: {}, using basic HTTP observation", task_type);
                if target.starts_with("http://") || target.starts_with("https://") {
                    if let Some(result) = self.try_http_request(target, context).await {
                        observations.insert("http_response".to_string(), result);
                    }
                }
            }
        }
    }
    
    /// 尝试分析网站
    async fn try_analyze_website(
        &self,
        target: &str,
        context: &HashMap<String, serde_json::Value>,
    ) -> Option<serde_json::Value> {
        // 从 URL 中提取域名
        let domain = target
            .trim_start_matches("http://")
            .trim_start_matches("https://")
            .split('/')
            .next()
            .unwrap_or(target)
            .split(':')
            .next()
            .unwrap_or(target);
        
        let mut args = HashMap::new();
        args.insert("domain".to_string(), serde_json::json!(domain));  // 使用 domain 而不是 url
        
        match self.engine_dispatcher.execute_tool("analyze_website", &args, context).await {
            Ok(result) => {
                log::info!("Website analysis completed for {}", domain);
                Some(result)
            }
            Err(e) => {
                log::warn!("Failed to analyze website: {}", e);
                None
            }
        }
    }
    
    /// 尝试 HTTP 请求
    async fn try_http_request(
        &self,
        target: &str,
        context: &HashMap<String, serde_json::Value>,
    ) -> Option<serde_json::Value> {
        let mut args = HashMap::new();
        args.insert("url".to_string(), serde_json::json!(target));
        args.insert("method".to_string(), serde_json::json!("GET"));
        
        match self.engine_dispatcher.execute_tool("http_request", &args, context).await {
            Ok(result) => {
                log::info!("HTTP request completed for {}", target);
                Some(result)
            }
            Err(e) => {
                log::warn!("Failed to perform HTTP request: {}", e);
                None
            }
        }
    }
    
    /// 尝试端口扫描
    async fn try_port_scan(
        &self,
        target: &str,
        context: &HashMap<String, serde_json::Value>,
    ) -> Option<serde_json::Value> {
        // 从 URL 中提取主机名
        let host = target
            .trim_start_matches("http://")
            .trim_start_matches("https://")
            .split('/')
            .next()
            .unwrap_or(target)
            .split(':')
            .next()
            .unwrap_or(target);
        
        // 尝试解析为 IP 地址
        use std::net::ToSocketAddrs;
        let ip_address = if host.parse::<std::net::IpAddr>().is_ok() {
            // 已经是 IP 地址
            host.to_string()
        } else {
            // 是域名，尝试解析
            match format!("{}:80", host).to_socket_addrs() {
                Ok(mut addrs) => {
                    if let Some(addr) = addrs.next() {
                        addr.ip().to_string()
                    } else {
                        log::warn!("Failed to resolve domain: {}", host);
                        return None;
                    }
                }
                Err(e) => {
                    log::warn!("Failed to resolve domain {}: {}", host, e);
                    return None;
                }
            }
        };
        
        log::info!("Resolved {} to IP: {}", host, ip_address);
        
        let mut args = HashMap::new();
        args.insert("target".to_string(), serde_json::json!(ip_address));
        args.insert("ports".to_string(), serde_json::json!("80,443,8080,8443"));
        
        match self.engine_dispatcher.execute_tool("port_scan", &args, context).await {
            Ok(result) => {
                log::info!("Port scan completed for {} ({})", host, ip_address);
                Some(result)
            }
            Err(e) => {
                log::warn!("Failed to perform port scan: {}", e);
                None
            }
        }
    }
    
    /// 从 HTTP 响应中提取技术信息
    fn extract_technology_from_response(&self, response: &serde_json::Value) -> Option<String> {
        // 尝试从响应头中提取服务器信息
        if let Some(headers) = response.get("headers").and_then(|h| h.as_object()) {
            if let Some(server) = headers.get("server").and_then(|s| s.as_str()) {
                return Some(server.to_string());
            }
            if let Some(powered_by) = headers.get("x-powered-by").and_then(|s| s.as_str()) {
                return Some(powered_by.to_string());
            }
        }
        None
    }

    /// 构建威胁查询
    fn build_threat_query(&self, observations: &serde_json::Value) -> String {
        let tech = observations
            .get("technology")
            .and_then(|v| v.as_str())
            .unwrap_or("web application");

        format!("security vulnerabilities in {}", tech)
    }

    /// 识别漏洞
    fn identify_vulnerabilities(
        &self,
        _observations: &serde_json::Value,
        _threats: &[ThreatInfo],
    ) -> Vec<VulnerabilityInfo> {
        // 占位实现
        vec![]
    }

    /// 提取实体(用于知识图谱查询)
    fn extract_entities(&self, observations: &serde_json::Value) -> Vec<String> {
        let mut entities = Vec::new();

        // 提取技术栈
        if let Some(tech) = observations.get("technology").and_then(|v| v.as_str()) {
            entities.push(tech.to_string());
        }

        // 提取服务
        if let Some(services) = observations.get("services").and_then(|v| v.as_array()) {
            for service in services {
                if let Some(s) = service.as_str() {
                    entities.push(s.to_string());
                }
            }
        }

        // 提取端口
        if let Some(ports) = observations.get("ports").and_then(|v| v.as_array()) {
            for port in ports {
                if let Some(p) = port.as_u64() {
                    entities.push(format!("port_{}", p));
                }
            }
        }

        entities
    }

    /// 生成行动计划
    fn generate_action_plan(
        &self,
        analysis: &ThreatAnalysis,
        task_complexity: TaskComplexity,
        context: &HashMap<String, serde_json::Value>,
    ) -> Result<ActionPlan> {
        let mut steps = Vec::new();
        
        // 从上下文中获取目标
        let target = context.get("target")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        // 根据任务复杂度生成不同的执行步骤
        match task_complexity {
            TaskComplexity::Simple => {
                // 简单任务：直接工具调用
                // 从 URL 中提取域名
                let domain = target
                    .trim_start_matches("http://")
                    .trim_start_matches("https://")
                    .split('/')
                    .next()
                    .unwrap_or(target)
                    .split(':')
                    .next()
                    .unwrap_or(target);
                
                steps.push(ActionStep {
                    id: "step-1".to_string(),
                    name: "Quick Security Scan".to_string(),
                    description: format!("Perform quick security scan on {}", target),
                    step_type: ActionStepType::DirectToolCall,
                    tool_name: Some("analyze_website".to_string()),
                    tool_args: {
                        let mut args = HashMap::new();
                        args.insert("domain".to_string(), serde_json::json!(domain));
                        args
                    },
                    estimated_duration: 30,
                });
            }
            TaskComplexity::Medium => {
                // 中等任务：多个工具顺序调用
                // 从 URL 中提取域名
                let domain = target
                    .trim_start_matches("http://")
                    .trim_start_matches("https://")
                    .split('/')
                    .next()
                    .unwrap_or(target)
                    .split(':')
                    .next()
                    .unwrap_or(target);
                
                // 1. 网站分析
                steps.push(ActionStep {
                    id: "step-1".to_string(),
                    name: "Website Analysis".to_string(),
                    description: format!("Analyze website structure of {}", target),
                    step_type: ActionStepType::DirectToolCall,
                    tool_name: Some("analyze_website".to_string()),
                    tool_args: {
                        let mut args = HashMap::new();
                        args.insert("domain".to_string(), serde_json::json!(domain));
                        args
                    },
                    estimated_duration: 60,
                });
                
                // 2. 被动扫描
                if !target.is_empty() {
                    steps.push(ActionStep {
                        id: "step-2".to_string(),
                        name: "Passive Scan".to_string(),
                        description: format!("Start passive security scan on {}", target),
                        step_type: ActionStepType::DirectToolCall,
                        tool_name: Some("start_passive_scan".to_string()),
                        tool_args: {
                            let mut args = HashMap::new();
                            args.insert("target_url".to_string(), serde_json::json!(target));
                            args
                        },
                        estimated_duration: 120,
                    });
                }
            }
            TaskComplexity::Complex => {
                // 复杂任务：使用 ReAct 引擎进行推理
                // 但我们需要提供具体的任务描述而不是空的工具调用
                steps.push(ActionStep {
                    id: "step-1".to_string(),
                    name: "综合安全评估".to_string(),
                    description: format!( //使用中文
                        "对 {} 全面安全评估 . 分析安全风险, 识别漏洞, 测试常见安全风险包括: {}",
                        target,
                        analysis.threats.iter()
                            .map(|t| t.name.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                    step_type: ActionStepType::ReactEngine,
                    tool_name: None, // ReAct 引擎会自己选择工具
                    tool_args: {
                        let mut args = HashMap::new();
                        args.insert("target".to_string(), serde_json::json!(target));
                        args.insert("task_description".to_string(), serde_json::json!(
                            format!("对 {} 全面安全评估, 识别漏洞, 测试常见安全风险包括: {}", target, analysis.threats.iter()
                            .map(|t| t.name.as_str())
                            .collect::<Vec<_>>()
                            .join(", "))
                        ));
                        args
                    },
                    estimated_duration: 300,
                });
            }
        }

        // 如果没有生成任何步骤（不应该发生），添加默认步骤
        if steps.is_empty() {
            log::warn!("No steps generated, adding default step");
            steps.push(ActionStep {
                id: "step-1".to_string(),
                name: "Basic Assessment".to_string(),
                description: format!("Perform basic assessment on {}", target),
                step_type: ActionStepType::DirectToolCall,
                tool_name: Some("http_request".to_string()),
                tool_args: {
                    let mut args = HashMap::new();
                    args.insert("url".to_string(), serde_json::json!(target));
                    args.insert("method".to_string(), serde_json::json!("GET"));
                    args
                },
                estimated_duration: 30,
            });
        }

        let total_duration: u64 = steps.iter().map(|s| s.estimated_duration).sum();

        Ok(ActionPlan {
            id: Uuid::new_v4().to_string(),
            name: "Security Assessment Plan".to_string(),
            description: format!(
                "Plan to assess {} with {} steps (complexity: {:?})",
                target,
                steps.len(),
                task_complexity
            ),
            steps,
            estimated_duration: total_duration,
            risk_assessment: RiskAssessment {
                risk_level: match analysis.threat_level {
                    ThreatLevel::Critical => RiskLevel::Critical,
                    ThreatLevel::High => RiskLevel::High,
                    ThreatLevel::Medium => RiskLevel::Medium,
                    ThreatLevel::Low | ThreatLevel::Info => RiskLevel::Low,
                },
                risk_factors: vec![],
                mitigations: vec![],
                requires_manual_approval: analysis.threat_level >= ThreatLevel::High,
            },
        })
    }

    /// 构建循环结果
    fn build_cycle_result(&self, cycle: &OodaCycle) -> OodaCycleResult {
        let mut observations = HashMap::new();
        let mut analysis = None;
        let mut decision = None;
        let mut execution_result = None;

        for phase_exec in &cycle.phase_history {
            match phase_exec.phase {
                OodaPhase::Observe => {
                    if let Some(output) = &phase_exec.output {
                        if let Some(obs) = output.as_object() {
                            for (k, v) in obs {
                                observations.insert(k.clone(), v.clone());
                            }
                        }
                    }
                }
                OodaPhase::Orient => {
                    if let Some(output) = &phase_exec.output {
                        analysis = serde_json::from_value(output.clone()).ok();
                    }
                }
                OodaPhase::Decide => {
                    if let Some(output) = &phase_exec.output {
                        decision = serde_json::from_value(output.clone()).ok();
                    }
                }
                OodaPhase::Act => {
                    execution_result = phase_exec.output.clone();
                }
            }
        }

        OodaCycleResult {
            success: cycle.status == OodaCycleStatus::Completed,
            observations,
            analysis,
            decision,
            execution_result,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_execute_cycle() {
        let config = TravelConfig::default();
        let executor = OodaExecutor::new(config);

        let mut context = HashMap::new();
        context.insert(
            "target".to_string(),
            serde_json::Value::String("localhost".to_string()),
        );

        let cycle = executor
            .execute_cycle(1, "Test security", TaskComplexity::Simple, &mut context)
            .await
            .unwrap();

        assert_eq!(cycle.cycle_number, 1);
        assert_eq!(cycle.status, OodaCycleStatus::Completed);
    }
}

