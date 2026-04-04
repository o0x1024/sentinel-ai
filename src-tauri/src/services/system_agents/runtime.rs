use anyhow::{anyhow, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::sync::RwLock;
use uuid::Uuid;

use sentinel_db::{Database, DatabaseService, SystemAgentProfileRecord, SystemAgentRunRecord};
use sentinel_traffic::HttpRequestRecord;

use crate::agents::executor::{execute_agent, AgentExecuteParams};
use crate::services::system_agents::behavior_signal::TrafficBehaviorSignalSettings;
use crate::services::system_agents::behavior_session::build_behavior_session;
use crate::services::system_agents::clusters::TrafficClusterStore;
use crate::services::system_agents::context::build_traffic_context_snapshot;
use crate::services::system_agents::filters::matches_event_filter;
use crate::services::system_agents::findings::persist_passive_agent_finding;
use crate::services::system_agents::logic_invariants::evaluate_logic_invariants;
use crate::services::system_agents::logic_skill_context::build_logic_skill_context;
use crate::services::system_agents::prompts::resolve_base_prompt;
use crate::services::system_agents::process_graph::build_process_graph;
use crate::services::system_agents::safety::SystemAgentSafetyPolicy;
use crate::services::system_agents::skill_recommendation::recommend_logic_skills;
use crate::services::system_agents::tool_policy::SystemAgentToolPolicy;
use crate::services::system_agents::types::{
    SystemAgentDispatchResult, SystemAgentEvent, SystemAgentRunUpdateEvent,
};
use crate::services::system_agents::verifier::verify_finding_for_runtime;
use crate::services::AiServiceManager;

#[derive(Debug)]
pub struct SystemAgentRuntime {
    db: Arc<DatabaseService>,
    ai_manager: Arc<AiServiceManager>,
    app_handle: AppHandle,
    running_counts: Arc<RwLock<HashMap<String, u32>>>,
    last_started: Arc<RwLock<HashMap<String, chrono::DateTime<Utc>>>>,
    pending_runs: Arc<RwLock<HashMap<String, VecDeque<QueuedSystemAgentRun>>>>,
    recent_sequences: Arc<RwLock<HashMap<String, Vec<String>>>>,
    cluster_store: Arc<RwLock<TrafficClusterStore>>,
}

#[derive(Debug, Clone)]
struct QueuedSystemAgentRun {
    run_id: String,
    profile: SystemAgentProfileRecord,
    payload: Value,
    trigger_event: Option<String>,
    cooldown_key: String,
    priority: i64,
    retry_count: u32,
}

#[derive(Debug, Clone)]
struct MatchedPassiveDispatch {
    profile: SystemAgentProfileRecord,
    priority: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SystemAgentQueueMetadata {
    cooldown_key: String,
    priority: i64,
    retry_count: u32,
    queued_at: String,
    last_error: Option<String>,
    error_category: Option<String>,
}

const MAX_BACKGROUND_RETRIES: u32 = 2;

impl SystemAgentRuntime {
    pub fn new(
        db: Arc<DatabaseService>,
        ai_manager: Arc<AiServiceManager>,
        app_handle: AppHandle,
    ) -> Self {
        Self {
            db,
            ai_manager,
            app_handle,
            running_counts: Arc::new(RwLock::new(HashMap::new())),
            last_started: Arc::new(RwLock::new(HashMap::new())),
            pending_runs: Arc::new(RwLock::new(HashMap::new())),
            recent_sequences: Arc::new(RwLock::new(HashMap::new())),
            cluster_store: Arc::new(RwLock::new(TrafficClusterStore::default())),
        }
    }

    pub fn app_handle(&self) -> AppHandle {
        self.app_handle.clone()
    }

    pub async fn recover_pending_runs(&self) -> Result<usize> {
        let runs = self.db.list_incomplete_system_agent_runs_internal(Some(200)).await?;
        let mut recovered = 0usize;

        for run in runs {
            let Some(profile) = self.db.get_system_agent_profile(&run.profile_id).await? else {
                continue;
            };
            if !profile.enabled || profile.mode != "passive" {
                continue;
            }

            let payload = parse_run_payload(&run.input_summary_json);
            let trigger_event = run.trigger_event.clone();
            let cooldown_key = self.cooldown_key(&profile, &payload, trigger_event.as_deref());
            let meta = parse_queue_metadata(run.output_json.as_deref());
            let retry_count = meta.as_ref().map(|item| item.retry_count).unwrap_or(0);
            let priority = meta.as_ref().map(|item| item.priority).unwrap_or(0);
            let recovered_run = QueuedSystemAgentRun {
                run_id: run.id.clone(),
                profile: profile.clone(),
                payload,
                trigger_event,
                cooldown_key,
                priority,
                retry_count,
            };

            let should_dead_letter =
                run.status == "running" && recovered_run.retry_count >= MAX_BACKGROUND_RETRIES;
            if should_dead_letter {
                self.persist_dead_letter(
                    &recovered_run.run_id,
                    &recovered_run,
                    "Recovered running task exceeded retry limit after restart",
                    "runtime_recovery",
                )
                .await?;
                continue;
            }

            self.persist_queued_run_state(
                &recovered_run.run_id,
                &recovered_run,
                if run.status == "running" {
                    "Recovered interrupted running task after restart"
                } else {
                    "Recovered queued system agent run after restart"
                },
                run.status == "retrying" || run.status == "running",
                "runtime_recovery",
            )
            .await?;
            self.enqueue_existing_run(recovered_run).await;
            recovered += 1;
        }

        let profile_ids = {
            let pending_runs = self.pending_runs.read().await;
            pending_runs
                .iter()
                .filter(|(_, queue)| !queue.is_empty())
                .map(|(profile_id, _)| profile_id.clone())
                .collect::<Vec<_>>()
        };
        for profile_id in profile_ids {
            self.spawn_schedule_next_queued_run(profile_id);
        }

        Ok(recovered)
    }

    pub async fn trigger_profile_now(
        &self,
        profile_id: &str,
        payload: Value,
        trigger_event: Option<String>,
    ) -> Result<SystemAgentRunRecord> {
        let profile = self
            .db
            .get_system_agent_profile(profile_id)
            .await?
            .ok_or_else(|| anyhow!("System agent profile not found: {}", profile_id))?;

        if !profile.enabled {
            return Err(anyhow!("System agent profile is disabled: {}", profile_id));
        }

        self.execute_profile(profile, payload, trigger_event).await
    }

    pub async fn get_profile(&self, profile_id: &str) -> Result<Option<SystemAgentProfileRecord>> {
        self.db.get_system_agent_profile(profile_id).await
    }

    pub async fn start_external_run(
        &self,
        profile_id: &str,
        payload: Value,
        trigger_event: Option<String>,
    ) -> Result<SystemAgentRunRecord> {
        let profile = self
            .db
            .get_system_agent_profile(profile_id)
            .await?
            .ok_or_else(|| anyhow!("System agent profile not found: {}", profile_id))?;
        if !profile.enabled {
            return Err(anyhow!("System agent profile is disabled: {}", profile_id));
        }
        let cooldown_key = self.cooldown_key(&profile, &payload, trigger_event.as_deref());
        self.check_runtime_limits(&profile, &payload, &cooldown_key)
            .await?;

        let run_id = format!("sar-{}", Uuid::new_v4());
        let run = self.build_initial_run(&run_id, &profile, payload, trigger_event, "running", None)?;
        self.db.create_system_agent_run(&run).await?;
        self.mark_run_started(&profile.id, &cooldown_key, run.started_at)
            .await;
        self.emit_run_update(&run);
        Ok(run)
    }

    pub async fn complete_external_run_success(
        &self,
        run_id: &str,
        profile_id: &str,
        output: Value,
    ) -> Result<SystemAgentRunRecord> {
        let output_json = serde_json::to_string(&output)?;
        let finished_at = Utc::now();
        self.db
            .update_system_agent_run(
                run_id,
                "completed",
                Some(&output_json),
                None,
                Some(finished_at),
            )
            .await?;
        self.mark_run_finished(profile_id).await;
        let run = self
            .db
            .get_system_agent_run(run_id)
            .await?
            .ok_or_else(|| anyhow!("System agent run not found: {}", run_id))?;
        self.emit_run_update(&run);
        Ok(run)
    }

    pub async fn complete_external_run_failure(
        &self,
        run_id: &str,
        profile_id: &str,
        error_message: impl ToString,
    ) -> Result<SystemAgentRunRecord> {
        let finished_at = Utc::now();
        let error_message = error_message.to_string();
        self.db
            .update_system_agent_run(
                run_id,
                "failed",
                None,
                Some(&error_message),
                Some(finished_at),
            )
            .await?;
        self.mark_run_finished(profile_id).await;
        let run = self
            .db
            .get_system_agent_run(run_id)
            .await?
            .ok_or_else(|| anyhow!("System agent run not found: {}", run_id))?;
        self.emit_run_update(&run);
        Ok(run)
    }

    pub async fn dispatch_event(
        &self,
        event_name: &str,
        payload: Value,
        source: &str,
    ) -> Result<SystemAgentDispatchResult> {
        let event = SystemAgentEvent {
            event_name: event_name.to_string(),
            payload,
            source: source.to_string(),
            timestamp: Utc::now(),
        };

        let profiles = self.db.list_system_agent_profiles().await?;
        let mut matched_profiles = Vec::new();
        let mut matched = 0usize;
        let mut scheduled_run_ids = Vec::new();

        for profile in profiles {
            if profile.mode != "passive" || !profile.enabled {
                continue;
            }
            let safety_policy = SystemAgentSafetyPolicy::from_profile(&profile);
            if profile.capability == "verifier" && !safety_policy.allows_auto_mode() {
                continue;
            }
            if profile.capability == "triage" {
                let payload_url = event.payload.get("url").and_then(Value::as_str);
                let payload_host = event.payload.get("host").and_then(Value::as_str);
                if let Err(error) = safety_policy.ensure_url_or_host_in_scope(payload_url, payload_host) {
                    tracing::debug!(
                        "Skipping passive system agent {} for event {} due to scope guard: {}",
                        profile.id,
                        event.event_name,
                        error
                    );
                    continue;
                }
            }

            let bindings = self
                .db
                .list_system_agent_bindings(Some(&profile.id))
                .await?;

            let matched_priority = bindings
                .iter()
                .filter(|binding| binding.enabled && binding.event_name == event.event_name)
                .filter_map(|binding| {
                    if binding.filter_json.trim().is_empty() || binding.filter_json.trim() == "{}" {
                        return Some(binding.priority);
                    }

                    match serde_json::from_str::<Value>(&binding.filter_json) {
                        Ok(filter) => matches_event_filter(&filter, &event).then_some(binding.priority),
                        Err(error) => {
                            tracing::warn!(
                                "Invalid system agent binding filter for {}: {}",
                                binding.id,
                                error
                            );
                            None
                        }
                    }
                })
                .max();

            let Some(priority) = matched_priority else {
                continue;
            };

            matched += 1;
            matched_profiles.push(MatchedPassiveDispatch { profile, priority });
        }

        matched_profiles.sort_by(|left, right| {
            right
                .priority
                .cmp(&left.priority)
                .then_with(|| left.profile.name.cmp(&right.profile.name))
        });

        for matched_profile in matched_profiles {
            let profile = matched_profile.profile;
            let priority = matched_profile.priority;

            match self
                .start_background_run(
                    profile.clone(),
                    event.payload.clone(),
                    Some(event.event_name.clone()),
                    priority,
                )
                .await
            {
                Ok(run_id) => scheduled_run_ids.push(run_id),
                Err(error) => {
                    if is_queueable_runtime_error(&error) {
                        if self
                            .enqueue_background_run(
                                profile,
                                event.payload.clone(),
                                Some(event.event_name.clone()),
                                priority,
                            )
                            .await
                        {
                            continue;
                        }
                    }
                    tracing::warn!(
                        "Failed to dispatch system agent for event {}: {}",
                        event.event_name,
                        error
                    );
                }
            }
        }

        let result = SystemAgentDispatchResult {
            event_name: event.event_name.clone(),
            matched_profiles: matched,
            scheduled_run_ids,
        };

        let _ = self
            .app_handle
            .emit("system-agent:event-dispatched", &result);
        Ok(result)
    }

    pub async fn handle_history_record(
        &self,
        record: HttpRequestRecord,
        behavior_signal: TrafficBehaviorSignalSettings,
        browser_extension_behavior: Option<Value>,
    ) -> Result<()> {
        let initial_snapshot = build_traffic_context_snapshot(&record, &[]);
        let recent_sequence = self
            .get_recent_sequence(&initial_snapshot.sequence_key)
            .await;
        let mut snapshot = build_traffic_context_snapshot(&record, &recent_sequence);
        if let Some(payload) = snapshot.payload.as_object_mut() {
            payload.insert("behaviorSignal".to_string(), behavior_signal.to_payload());
            if let Some(browser_extension_behavior) = browser_extension_behavior {
                payload.insert(
                    "browserExtensionBehavior".to_string(),
                    browser_extension_behavior,
                );
            }
        }
        self.push_recent_sequence(&snapshot.sequence_key, snapshot.action_kind.clone())
            .await;
        if let Some(summary) = self.update_cluster_summary(&snapshot.payload).await {
            if let Some(payload) = snapshot.payload.as_object_mut() {
                payload.insert("clusterSummary".to_string(), summary);
            }
        }
        if let Some(behavior_session) = build_behavior_session(&snapshot.payload) {
            if let Some(payload) = snapshot.payload.as_object_mut() {
                payload.insert("behaviorSession".to_string(), behavior_session);
            }
        }
        if let Some(process_graph) = build_process_graph(&snapshot.payload) {
            if let Some(payload) = snapshot.payload.as_object_mut() {
                payload.insert("processGraph".to_string(), process_graph);
            }
        }
        let logic_invariants = evaluate_logic_invariants(&snapshot.payload);
        if let Some(payload) = snapshot.payload.as_object_mut() {
            payload.insert("logicInvariants".to_string(), logic_invariants);
        }
        let skill_recommendations = recommend_logic_skills(&snapshot.payload);
        if let Some(payload) = snapshot.payload.as_object_mut() {
            payload.insert("skillRecommendations".to_string(), skill_recommendations);
        }
        let logic_skill_context = build_logic_skill_context(self.db.as_ref(), &snapshot.payload);
        if let Some(payload) = snapshot.payload.as_object_mut() {
            payload.insert("logicSkillContext".to_string(), logic_skill_context);
        }

        let _ = self
            .dispatch_event("traffic.cluster.ready", snapshot.payload, "traffic_history")
            .await?;
        Ok(())
    }

    fn schedule_event_dispatch(&self, event_name: String, payload: Value, source: String) {
        let runtime = self.clone();
        tauri::async_runtime::spawn(async move {
            if let Err(error) = runtime.dispatch_event(&event_name, payload, &source).await {
                tracing::warn!(
                    "Failed to schedule chained system-agent event {}: {}",
                    event_name,
                    error
                );
            }
        });
    }

    async fn start_background_run(
        &self,
        profile: SystemAgentProfileRecord,
        payload: Value,
        trigger_event: Option<String>,
        priority: i64,
    ) -> Result<String> {
        let cooldown_key = self.cooldown_key(&profile, &payload, trigger_event.as_deref());
        self.check_runtime_limits(&profile, &payload, &cooldown_key)
            .await?;

        let run_id = format!("sar-{}", Uuid::new_v4());
        let run = self.build_initial_run(
            &run_id,
            &profile,
            payload.clone(),
            trigger_event.clone(),
            "running",
            Some(queue_metadata_json(&cooldown_key, priority, 0, None, None)?),
        )?;
        self.db.create_system_agent_run(&run).await?;
        self.emit_run_update(&run);
        self.mark_run_started(&profile.id, &cooldown_key, run.started_at)
            .await;

        let runtime = self.clone();
        let run_id_for_task = run_id.clone();
        let event = SystemAgentEvent {
            event_name: trigger_event
                .clone()
                .unwrap_or_else(|| "manual".to_string()),
            payload: payload.clone(),
            source: "system_agent_runtime".to_string(),
            timestamp: Utc::now(),
        };

        tauri::async_runtime::spawn(async move {
            if let Err(error) = runtime
                .complete_background_run(profile, payload, trigger_event, event, run_id_for_task, 0, priority)
                .await
            {
                tracing::error!("System agent background run failed: {}", error);
            }
        });

        Ok(run_id)
    }

    async fn complete_background_run(
        self,
        profile: SystemAgentProfileRecord,
        payload: Value,
        trigger_event: Option<String>,
        event: SystemAgentEvent,
        run_id: String,
        retry_count: u32,
        priority: i64,
    ) -> Result<()> {
        let finished_at = Utc::now();
        let run_result = if profile.id == "traffic_active_verifier" {
            verify_finding_for_runtime(&self, self.db.as_ref(), &self.app_handle, &payload, &run_id)
                .await
                .and_then(|value| serde_json::to_string(&value).map_err(Into::into))
        } else {
            self.run_profile_llm(&profile, &payload, trigger_event.as_deref(), &run_id)
                .await
        };

        match run_result {
            Ok(output_json) => {
                if profile.mode == "passive" && profile.capability == "triage" {
                    match serde_json::from_str::<Value>(&output_json) {
                        Ok(output_value) => {
                            match persist_passive_agent_finding(
                                self.db.as_ref(),
                                &self.app_handle,
                                &profile.id,
                                &profile.safety_policy_json,
                                &event,
                                &output_value,
                            )
                            .await
                            {
                                Ok(Some(finding_id)) => {
                                    let safety_policy =
                                        SystemAgentSafetyPolicy::from_profile(&profile);
                                    if safety_policy.allows_auto_mode() {
                                        self.schedule_event_dispatch(
                                            "traffic.hypothesis.ready".to_string(),
                                            json!({
                                                "findingId": finding_id,
                                                "sourceProfileId": profile.id,
                                                "riskType": output_value.get("riskType").cloned().unwrap_or(Value::Null),
                                                "confidence": output_value.get("confidence").cloned().unwrap_or(Value::Null),
                                                "verificationPlan": output_value.get("verificationPlan").cloned().unwrap_or(Value::Null),
                                            }),
                                            "system_agent_triage".to_string(),
                                        );
                                    }
                                    let _ = self.app_handle.emit(
                                        "traffic.hypothesis.ready",
                                        json!({
                                            "findingId": finding_id,
                                            "sourceProfileId": profile.id,
                                            "riskType": output_value.get("riskType").cloned().unwrap_or(Value::Null),
                                            "confidence": output_value.get("confidence").cloned().unwrap_or(Value::Null),
                                            "verificationPlan": output_value.get("verificationPlan").cloned().unwrap_or(Value::Null),
                                        }),
                                    );
                                }
                                Ok(None) => {}
                                Err(error) => {
                                    tracing::warn!(
                                        "Failed to persist passive system-agent finding for {}: {}",
                                        profile.id,
                                        error
                                    );
                                }
                            }
                        }
                        Err(error) => {
                            tracing::warn!(
                                "Failed to parse system-agent output as JSON for {}: {}",
                                profile.id,
                                error
                            );
                        }
                    }
                }

                self.db
                    .update_system_agent_run(
                        &run_id,
                        "completed",
                        Some(&output_json),
                        None,
                        Some(finished_at),
                    )
                    .await?;
            }
            Err(error) => {
                if self
                    .schedule_retry_if_needed(
                        &profile,
                        &payload,
                        trigger_event.clone(),
                        &run_id,
                        retry_count,
                        priority,
                        &error,
                    )
                    .await?
                {
                    self.mark_run_finished(&profile.id).await;
                    if let Some(run) = self.db.get_system_agent_run(&run_id).await? {
                        self.emit_run_update(&run);
                    }
                    return Ok(());
                }

                self.db
                    .update_system_agent_run(
                        &run_id,
                        "dead_letter",
                        Some(&queue_metadata_json(
                            &self.cooldown_key(&profile, &payload, trigger_event.as_deref()),
                            priority,
                            retry_count,
                            Some(error.to_string()),
                            Some(categorize_runtime_error(&error).to_string()),
                        )?),
                        Some(&error.to_string()),
                        Some(finished_at),
                    )
                    .await?;
            }
        }

        self.mark_run_finished(&profile.id).await;
        if let Some(run) = self.db.get_system_agent_run(&run_id).await? {
            self.emit_run_update(&run);
        }
        Ok(())
    }

    async fn execute_profile(
        &self,
        profile: SystemAgentProfileRecord,
        payload: Value,
        trigger_event: Option<String>,
    ) -> Result<SystemAgentRunRecord> {
        let cooldown_key = self.cooldown_key(&profile, &payload, trigger_event.as_deref());
        self.check_runtime_limits(&profile, &payload, &cooldown_key)
            .await?;

        let run_id = format!("sar-{}", Uuid::new_v4());
        let run = self.build_initial_run(
            &run_id,
            &profile,
            payload.clone(),
            trigger_event.clone(),
            "running",
            None,
        )?;
        self.db.create_system_agent_run(&run).await?;
        self.emit_run_update(&run);
        self.mark_run_started(&profile.id, &cooldown_key, run.started_at)
            .await;

        let finished_at = Utc::now();
        let result = self
            .run_profile_llm(&profile, &payload, trigger_event.as_deref(), &run_id)
            .await;
        let updated = match result {
            Ok(output_json) => {
                self.db
                    .update_system_agent_run(
                        &run_id,
                        "completed",
                        Some(&output_json),
                        None,
                        Some(finished_at),
                    )
                    .await?;
                self.db
                    .get_system_agent_run(&run_id)
                    .await?
                    .ok_or_else(|| anyhow!("System agent run not found after completion"))?
            }
            Err(error) => {
                self.db
                    .update_system_agent_run(
                        &run_id,
                        "failed",
                        None,
                        Some(&error.to_string()),
                        Some(finished_at),
                    )
                    .await?;
                self.db
                    .get_system_agent_run(&run_id)
                    .await?
                    .ok_or_else(|| anyhow!("System agent run not found after failure"))?
            }
        };

        self.mark_run_finished(&profile.id).await;
        self.emit_run_update(&updated);
        Ok(updated)
    }

    fn build_initial_run(
        &self,
        run_id: &str,
        profile: &SystemAgentProfileRecord,
        payload: Value,
        trigger_event: Option<String>,
        status: &str,
        output_json: Option<String>,
    ) -> Result<SystemAgentRunRecord> {
        let now = Utc::now();
        Ok(SystemAgentRunRecord {
            id: run_id.to_string(),
            profile_id: profile.id.clone(),
            trigger_event,
            status: status.to_string(),
            input_summary_json: serde_json::to_string(&payload)?,
            output_json,
            error_message: None,
            started_at: now,
            finished_at: None,
            created_at: now,
            updated_at: now,
        })
    }

    async fn run_profile_llm(
        &self,
        profile: &SystemAgentProfileRecord,
        payload: &Value,
        trigger_event: Option<&str>,
        run_id: &str,
    ) -> Result<String> {
        let base_prompt = resolve_base_prompt(profile.base_prompt_id.as_deref(), &profile.id);
        let tool_policy = SystemAgentToolPolicy::from_profile(profile);
        tool_policy.validate()?;

        let mut prompt_sections = vec![base_prompt.to_string()];
        if let Some(prompt_patch) = &profile.prompt_patch {
            if !prompt_patch.trim().is_empty() {
                prompt_sections.push(format!("Additional instructions:\n{}", prompt_patch));
            }
        }
        if let Some(tool_policy_note) = tool_policy.prompt_note() {
            prompt_sections.push(tool_policy_note);
        }
        let system_prompt = prompt_sections.join("\n\n");

        let user_input = serde_json::to_string_pretty(&json!({
            "profileId": profile.id,
            "mode": profile.mode,
            "capability": profile.capability,
            "triggerEvent": trigger_event,
            "payload": payload,
        }))?;

        if let Some(tool_config) = tool_policy.build_runtime_tool_config() {
            let raw = self
                .run_profile_with_agent_executor(
                    profile,
                    run_id,
                    system_prompt,
                    user_input,
                    tool_config,
                )
                .await?;
            let json_output = normalize_llm_json_output(&raw);
            return Ok(serde_json::to_string(&json_output)?);
        }

        let service_names = self.ai_manager.list_services();
        let maybe_service = self.ai_manager.get_service("default").or_else(|| {
            service_names
                .first()
                .and_then(|name| self.ai_manager.get_service(name))
        });

        if let Some(service) = maybe_service {
            let llm_config = service.service.to_llm_config();
            let client = sentinel_llm::LlmClient::new(llm_config);
            let raw = client.completion(Some(&system_prompt), &user_input).await?;
            let json_output = match serde_json::from_str::<Value>(raw.trim()) {
                Ok(value) => value,
                Err(_) => {
                    if let (Some(start), Some(end)) = (raw.find('{'), raw.rfind('}')) {
                        serde_json::from_str::<Value>(&raw[start..=end])
                            .unwrap_or_else(|_| json!({ "raw": raw }))
                    } else {
                        json!({ "raw": raw })
                    }
                }
            };
            return Ok(serde_json::to_string(&json_output)?);
        }

        Ok(serde_json::to_string(&json!({
            "summary": "No AI service configured; generated deterministic fallback output.",
            "signals": [
                format!("profile={}", profile.id),
                format!("trigger={}", trigger_event.unwrap_or("manual"))
            ],
            "payloadEcho": payload
        }))?)
    }

    async fn run_profile_with_agent_executor(
        &self,
        profile: &SystemAgentProfileRecord,
        run_id: &str,
        system_prompt: String,
        task: String,
        tool_config: crate::agents::ToolConfig,
    ) -> Result<String> {
        let service = self.resolve_generation_service().await?;
        let config = service.get_config().clone();
        let params = AgentExecuteParams {
            execution_id: run_id.to_string(),
            model: config.model.clone(),
            system_prompt,
            task,
            rig_provider: config
                .rig_provider
                .clone()
                .unwrap_or_else(|| config.provider.clone()),
            api_key: config.api_key.clone(),
            api_base: config.api_base.clone(),
            max_iterations: 6,
            timeout_secs: 120,
            tool_config: Some(tool_config),
            enable_tenth_man_rule: false,
            tenth_man_config: None,
            document_attachments: None,
            image_attachments: None,
            persist_messages: false,
            subagent_run_id: None,
            context_policy: None,
            recursion_depth: 0,
        };

        execute_agent(&self.app_handle, params)
            .await
            .map_err(|error| anyhow!("System agent '{}' execution failed: {}", profile.id, error))
    }

    async fn resolve_generation_service(&self) -> Result<crate::services::ai::AiServiceWrapper> {
        if let Ok(Some((provider, _model_name))) = self.ai_manager.get_default_llm_model().await {
            let provider_lc = provider.to_lowercase();
            let maybe_service =
                self.ai_manager
                    .list_services()
                    .into_iter()
                    .find_map(|service_name| {
                        let service = self.ai_manager.get_service(&service_name)?;
                        let service_provider = service.get_config().provider.to_lowercase();
                        if service_provider == provider_lc
                            || service_name.to_lowercase() == provider_lc
                        {
                            Some(service)
                        } else {
                            None
                        }
                    });

            if let Some(service) = maybe_service {
                return Ok(service);
            }
        }

        self.ai_manager
            .get_service("default")
            .or_else(|| {
                self.ai_manager
                    .list_services()
                    .first()
                    .and_then(|service_name| self.ai_manager.get_service(service_name))
            })
            .ok_or_else(|| anyhow!("No AI service available for system agent runtime"))
    }

    async fn check_runtime_limits(
        &self,
        profile: &SystemAgentProfileRecord,
        payload: &Value,
        cooldown_key: &str,
    ) -> Result<()> {
        {
            let running_counts = self.running_counts.read().await;
            let current = running_counts.get(&profile.id).copied().unwrap_or(0);
            if current >= profile.max_concurrency.max(1) as u32 {
                return Err(anyhow!("System agent concurrency limit reached"));
            }
        }

        if should_bypass_cooldown(profile, payload) {
            return Ok(());
        }

        if profile.cooldown_secs > 0 {
            let last_started = self.last_started.read().await;
            if let Some(last_started_at) = last_started.get(cooldown_key) {
                let elapsed = Utc::now()
                    .signed_duration_since(*last_started_at)
                    .num_seconds();
                if elapsed < profile.cooldown_secs {
                    return Err(anyhow!("System agent cooldown not elapsed"));
                }
            }
        }

        Ok(())
    }

    async fn mark_run_started(
        &self,
        profile_id: &str,
        cooldown_key: &str,
        started_at: chrono::DateTime<Utc>,
    ) {
        let mut running_counts = self.running_counts.write().await;
        *running_counts.entry(profile_id.to_string()).or_insert(0) += 1;
        drop(running_counts);

        let mut last_started = self.last_started.write().await;
        last_started.insert(cooldown_key.to_string(), started_at);
    }

    async fn mark_run_finished(&self, profile_id: &str) {
        let mut running_counts = self.running_counts.write().await;
        if let Some(count) = running_counts.get_mut(profile_id) {
            if *count > 0 {
                *count -= 1;
            }
        }
        drop(running_counts);
        self.spawn_schedule_next_queued_run(profile_id.to_string());
    }

    fn emit_run_update(&self, run: &SystemAgentRunRecord) {
        let payload = SystemAgentRunUpdateEvent {
            run_id: run.id.clone(),
            profile_id: run.profile_id.clone(),
            status: run.status.clone(),
            trigger_event: run.trigger_event.clone(),
            started_at: run.started_at,
            finished_at: run.finished_at,
            error_message: run.error_message.clone(),
        };
        let _ = self.app_handle.emit("system-agent:run-updated", payload);
    }

    async fn get_recent_sequence(&self, sequence_key: &str) -> Vec<String> {
        let sequences = self.recent_sequences.read().await;
        sequences.get(sequence_key).cloned().unwrap_or_default()
    }

    async fn push_recent_sequence(&self, sequence_key: &str, action_kind: String) {
        let mut sequences = self.recent_sequences.write().await;
        let entry = sequences.entry(sequence_key.to_string()).or_default();
        entry.push(action_kind);
        if entry.len() > 6 {
            let overflow = entry.len() - 6;
            entry.drain(0..overflow);
        }
    }

    async fn update_cluster_summary(&self, payload: &Value) -> Option<Value> {
        let mut store = self.cluster_store.write().await;
        store.update_with_payload(payload)
    }

    async fn enqueue_background_run(
        &self,
        profile: SystemAgentProfileRecord,
        payload: Value,
        trigger_event: Option<String>,
        priority: i64,
    ) -> bool {
        let cooldown_key = self.cooldown_key(&profile, &payload, trigger_event.as_deref());
        let mut pending_runs = self.pending_runs.write().await;
        let queue = pending_runs.entry(profile.id.clone()).or_default();
        if queue.iter().any(|item| item.cooldown_key == cooldown_key) {
            return false;
        }
        let run_id = format!("sar-{}", Uuid::new_v4());
        let queued = QueuedSystemAgentRun {
            run_id: run_id.clone(),
            profile,
            payload,
            trigger_event,
            cooldown_key,
            priority,
            retry_count: 0,
        };
        insert_queued_run(queue, queued.clone());
        drop(pending_runs);
        if let Err(error) = self.persist_queued_run(&queued, false, None, None).await {
            tracing::warn!("Failed to persist queued system agent run {}: {}", run_id, error);
        }
        true
    }

    fn spawn_schedule_next_queued_run(&self, profile_id: String) {
        let runtime = self.clone();
        tauri::async_runtime::spawn(async move {
            let queued = {
                let mut pending_runs = runtime.pending_runs.write().await;
                let queue = pending_runs.entry(profile_id.clone()).or_default();
                queue.pop_front()
            };

            let Some(queued) = queued else {
                return;
            };

            match runtime
                .start_background_run_from_queue(queued.clone())
                .await
            {
                Ok(_) => {}
                Err(error) => {
                    if is_queueable_runtime_error(&error) {
                        let wait_secs =
                            runtime.cooldown_wait_seconds(&queued.profile, &queued.payload, &queued.cooldown_key)
                                .await
                                .unwrap_or(1)
                                .max(1);
                        runtime.enqueue_existing_run(queued).await;
                        runtime.spawn_delayed_schedule_next(profile_id.clone(), wait_secs as u64);
                    } else {
                        tracing::warn!(
                            "Failed to schedule queued system agent run for {}: {}",
                            profile_id,
                            error
                        );
                    }
                }
            }
        });
    }

    fn spawn_delayed_schedule_next(&self, profile_id: String, delay_secs: u64) {
        let runtime = self.clone();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(delay_secs)).await;
            runtime.spawn_schedule_next_queued_run(profile_id);
        });
    }

    async fn start_background_run_from_queue(&self, queued: QueuedSystemAgentRun) -> Result<String> {
        self.check_runtime_limits(&queued.profile, &queued.payload, &queued.cooldown_key)
            .await?;

        let queue_meta = queue_metadata_json(
            &queued.cooldown_key,
            queued.priority,
            queued.retry_count,
            None,
            None,
        )?;
        let started_at = Utc::now();
        self.db
            .set_system_agent_run_state_internal(
                &queued.run_id,
                "running",
                Some(&queue_meta),
                None,
                Some(started_at),
                None,
            )
            .await?;
        self.mark_run_started(&queued.profile.id, &queued.cooldown_key, started_at)
            .await;
        if let Some(run) = self.db.get_system_agent_run(&queued.run_id).await? {
            self.emit_run_update(&run);
        }

        let runtime = self.clone();
        let run_id_for_task = queued.run_id.clone();
        let profile = queued.profile.clone();
        let payload = queued.payload.clone();
        let trigger_event = queued.trigger_event.clone();
        let event = SystemAgentEvent {
            event_name: trigger_event
                .clone()
                .unwrap_or_else(|| "manual".to_string()),
            payload: payload.clone(),
            source: "system_agent_runtime".to_string(),
            timestamp: Utc::now(),
        };

        tauri::async_runtime::spawn(async move {
            if let Err(error) = runtime
                .complete_background_run(
                    profile,
                    payload,
                    trigger_event,
                    event,
                    run_id_for_task,
                    queued.retry_count,
                    queued.priority,
                )
                .await
            {
                tracing::error!("System agent queued background run failed: {}", error);
            }
        });

        Ok(queued.run_id)
    }

    fn cooldown_key(
        &self,
        profile: &SystemAgentProfileRecord,
        payload: &Value,
        trigger_event: Option<&str>,
    ) -> String {
        if profile.mode != "passive" {
            return profile.id.clone();
        }

        match trigger_event {
            Some("traffic.cluster.ready") => {
                if let Some(cluster_key) = payload.get("clusterKey").and_then(Value::as_str) {
                    return format!("{}::cluster::{cluster_key}", profile.id);
                }
                if let Some(path_template) = payload.get("pathTemplate").and_then(Value::as_str) {
                    return format!("{}::path::{path_template}", profile.id);
                }
            }
            Some("traffic.hypothesis.ready") => {
                if let Some(finding_id) = payload.get("findingId").and_then(Value::as_str) {
                    return format!("{}::finding::{finding_id}", profile.id);
                }
            }
            _ => {}
        }

        profile.id.clone()
    }

    async fn cooldown_wait_seconds(
        &self,
        profile: &SystemAgentProfileRecord,
        payload: &Value,
        cooldown_key: &str,
    ) -> Option<i64> {
        if should_bypass_cooldown(profile, payload) || profile.cooldown_secs <= 0 {
            return None;
        }
        let last_started = self.last_started.read().await;
        last_started.get(cooldown_key).map(|last_started_at| {
            let elapsed = Utc::now()
                .signed_duration_since(*last_started_at)
                .num_seconds();
            (profile.cooldown_secs - elapsed).max(1)
        })
    }

    async fn schedule_retry_if_needed(
        &self,
        profile: &SystemAgentProfileRecord,
        payload: &Value,
        trigger_event: Option<String>,
        run_id: &str,
        retry_count: u32,
        priority: i64,
        error: &anyhow::Error,
    ) -> Result<bool> {
        if !is_retryable_background_error(error) || retry_count >= MAX_BACKGROUND_RETRIES {
            return Ok(false);
        }

        let queued = QueuedSystemAgentRun {
            run_id: run_id.to_string(),
            profile: profile.clone(),
            payload: payload.clone(),
            trigger_event: trigger_event.clone(),
            cooldown_key: self.cooldown_key(profile, payload, trigger_event.as_deref()),
            priority,
            retry_count: retry_count + 1,
        };

        self.persist_queued_run_state(
            run_id,
            &queued,
            &error.to_string(),
            true,
            categorize_runtime_error(error),
        )
        .await?;
        self.enqueue_existing_run(queued.clone()).await;
        self.spawn_delayed_schedule_next(profile.id.clone(), 1);
        Ok(true)
    }

    async fn persist_queued_run(
        &self,
        queued: &QueuedSystemAgentRun,
        is_retry: bool,
        last_error: Option<String>,
        error_category: Option<&str>,
    ) -> Result<()> {
        let status = if is_retry { "retrying" } else { "queued" };
        let run = self.build_initial_run(
            &queued.run_id,
            &queued.profile,
            queued.payload.clone(),
            queued.trigger_event.clone(),
            status,
            Some(queue_metadata_json(
                &queued.cooldown_key,
                queued.priority,
                queued.retry_count,
                last_error,
                error_category.map(ToString::to_string),
            )?),
        )?;
        self.db.create_system_agent_run(&run).await?;
        self.emit_run_update(&run);
        Ok(())
    }

    async fn persist_queued_run_state(
        &self,
        run_id: &str,
        queued: &QueuedSystemAgentRun,
        last_error: &str,
        is_retry: bool,
        error_category: &str,
    ) -> Result<()> {
        let status = if is_retry { "retrying" } else { "queued" };
        let meta = queue_metadata_json(
            &queued.cooldown_key,
            queued.priority,
            queued.retry_count,
            Some(last_error.to_string()),
            Some(error_category.to_string()),
        )?;
        self.db
            .set_system_agent_run_state_internal(
                run_id,
                status,
                Some(&meta),
                Some(last_error),
                None,
                None,
            )
            .await?;
        Ok(())
    }

    async fn persist_dead_letter(
        &self,
        run_id: &str,
        queued: &QueuedSystemAgentRun,
        error_message: &str,
        error_category: &str,
    ) -> Result<()> {
        let finished_at = Utc::now();
        let meta = queue_metadata_json(
            &queued.cooldown_key,
            queued.priority,
            queued.retry_count,
            Some(error_message.to_string()),
            Some(error_category.to_string()),
        )?;
        self.db
            .set_system_agent_run_state_internal(
                run_id,
                "dead_letter",
                Some(&meta),
                Some(error_message),
                None,
                Some(finished_at),
            )
            .await?;
        if let Some(run) = self.db.get_system_agent_run(run_id).await? {
            self.emit_run_update(&run);
        }
        Ok(())
    }

    async fn enqueue_existing_run(&self, queued: QueuedSystemAgentRun) {
        let mut pending_runs = self.pending_runs.write().await;
        let queue = pending_runs.entry(queued.profile.id.clone()).or_default();
        if queue.iter().any(|item| item.cooldown_key == queued.cooldown_key) {
            return;
        }
        insert_queued_run(queue, queued);
    }
}

fn normalize_llm_json_output(raw: &str) -> Value {
    match serde_json::from_str::<Value>(raw.trim()) {
        Ok(value) => value,
        Err(_) => {
            if let (Some(start), Some(end)) = (raw.find('{'), raw.rfind('}')) {
                serde_json::from_str::<Value>(&raw[start..=end])
                    .unwrap_or_else(|_| json!({ "raw": raw }))
            } else {
                json!({ "raw": raw })
            }
        }
    }
}

fn should_bypass_cooldown(profile: &SystemAgentProfileRecord, payload: &Value) -> bool {
    if profile.capability != "triage" {
        return false;
    }

    payload
        .get("clusterSummary")
        .and_then(|value| value.get("totalRequests"))
        .and_then(Value::as_u64)
        .map(|count| count <= 2)
        .unwrap_or(false)
}

impl Clone for SystemAgentRuntime {
    fn clone(&self) -> Self {
        Self {
            db: self.db.clone(),
            ai_manager: self.ai_manager.clone(),
            app_handle: self.app_handle.clone(),
            running_counts: self.running_counts.clone(),
            last_started: self.last_started.clone(),
            pending_runs: self.pending_runs.clone(),
            recent_sequences: self.recent_sequences.clone(),
            cluster_store: self.cluster_store.clone(),
        }
    }
}

fn is_queueable_runtime_error(error: &anyhow::Error) -> bool {
    let message = error.to_string();
    message.contains("System agent concurrency limit reached")
        || message.contains("System agent cooldown not elapsed")
}

fn is_retryable_background_error(error: &anyhow::Error) -> bool {
    let category = categorize_runtime_error(error);
    matches!(category, "llm_service" | "verification_execution" | "runtime_recovery" | "unknown")
}

fn categorize_runtime_error(error: &anyhow::Error) -> &'static str {
    let message = error.to_string().to_lowercase();
    if message.contains("concurrency limit") {
        return "concurrency_limit";
    }
    if message.contains("cooldown") {
        return "cooldown";
    }
    if message.contains("scope") || message.contains("policy") {
        return "policy_blocked";
    }
    if message.contains("verification") || message.contains("request failed") {
        return "verification_execution";
    }
    if message.contains("no ai service")
        || message.contains("llm")
        || message.contains("model")
        || message.contains("provider")
    {
        return "llm_service";
    }
    if message.contains("json") || message.contains("parse") {
        return "invalid_output";
    }
    "unknown"
}

fn queue_metadata_json(
    cooldown_key: &str,
    priority: i64,
    retry_count: u32,
    last_error: Option<String>,
    error_category: Option<String>,
) -> Result<String> {
    serde_json::to_string(&json!({
        "queueMeta": {
            "cooldownKey": cooldown_key,
            "priority": priority,
            "retryCount": retry_count,
            "queuedAt": Utc::now().to_rfc3339(),
            "lastError": last_error,
            "errorCategory": error_category,
        }
    }))
    .map_err(Into::into)
}

fn parse_queue_metadata(raw: Option<&str>) -> Option<SystemAgentQueueMetadata> {
    let value = raw.and_then(|item| serde_json::from_str::<Value>(item).ok())?;
    serde_json::from_value::<SystemAgentQueueMetadata>(value.get("queueMeta")?.clone()).ok()
}

fn parse_run_payload(raw: &str) -> Value {
    serde_json::from_str::<Value>(raw).unwrap_or(Value::Null)
}

fn insert_queued_run(queue: &mut VecDeque<QueuedSystemAgentRun>, queued: QueuedSystemAgentRun) {
    let position = queue
        .iter()
        .position(|item| item.priority < queued.priority)
        .unwrap_or(queue.len());
    queue.insert(position, queued);
}
