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

use crate::agents::executor::{
    execute_agent, take_execution_tool_trace, AgentExecuteParams, ToolCallRecord,
};
use crate::services::system_agents::behavior_signal::TrafficBehaviorSignalSettings;
use crate::services::system_agents::clusters::TrafficClusterStore;
use crate::services::system_agents::context_agent::build_raw_context_request_payload;
use crate::services::system_agents::context_settings::TrafficContextExtractionSettings;
use crate::services::system_agents::filters::matches_event_filter;
use crate::services::system_agents::language::{output_language_instruction, resolve_ui_language};
use crate::services::system_agents::logic_sop_context::{
    build_logic_sop_context, render_logic_sop_prompt,
};
use crate::services::system_agents::pipeline::{
    build_hypothesis_ready_payload, extract_scope_host, extract_scope_url,
    EVENT_TRAFFIC_CONTEXT_READY, EVENT_TRAFFIC_HYPOTHESIS_READY, EVENT_TRAFFIC_RAW_READY,
    EVENT_TRAFFIC_VERIFICATION_COMPLETED, TRAFFIC_CONTEXT_AGENT_PROFILE_ID,
    TRAFFIC_DECISION_AGENT_PROFILE_ID, TRAFFIC_VERIFICATION_AGENT_PROFILE_ID,
};
use crate::services::system_agents::prompts::{
    resolve_base_prompt, triage_verification_bootstrap_prompt,
};
use crate::services::system_agents::safety::SystemAgentSafetyPolicy;
use crate::services::system_agents::semantic_mapper::{
    build_semantic_feature_payload, build_semantic_signature, deterministic_semantic_abstraction,
    normalize_semantic_abstraction_output, semantic_mapper_prompt,
    should_attempt_ai_semantic_mapping,
};
use crate::services::system_agents::tool_policy::SystemAgentToolPolicy;
use crate::services::system_agents::triage_enrichment::{
    merge_triage_bootstrap_decision, should_attempt_triage_bootstrap, TriageBootstrapDecision,
};
use crate::services::system_agents::types::{
    SystemAgentDispatchResult, SystemAgentEvent, SystemAgentRunUpdateEvent,
};
use crate::services::system_agents::verification_hypothesis_memory::{
    derive_triage_hypothesis_state, extract_hypothesis_state, normalize_hypothesis_state,
    seed_hypothesis_state_from_logic_hypotheses,
};
use crate::services::system_agents::verifier::verify_finding_for_runtime;
use crate::services::AiServiceManager;
use sentinel_tools::buildin_tools::SopsTool;

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
    semantic_cache: Arc<RwLock<HashMap<String, Value>>>,
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
            semantic_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn app_handle(&self) -> AppHandle {
        self.app_handle.clone()
    }

    pub(crate) fn db(&self) -> Arc<DatabaseService> {
        self.db.clone()
    }

    pub async fn recover_pending_runs(&self) -> Result<usize> {
        let runs = self
            .db
            .list_incomplete_system_agent_runs_internal(Some(200))
            .await?;
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
        let run =
            self.build_initial_run(&run_id, &profile, payload, trigger_event, "running", None)?;
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
                None,
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
            if profile.capability == "verification" && !safety_policy.allows_auto_mode() {
                continue;
            }
            if matches!(
                profile.capability.as_str(),
                "context" | "hypothesis" | "decision"
            ) {
                let payload_url = extract_scope_url(&event.payload);
                let payload_host = extract_scope_host(&event.payload);
                if let Err(error) =
                    safety_policy.ensure_url_or_host_in_scope(payload_url, payload_host)
                {
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
                        Ok(filter) => {
                            matches_event_filter(&filter, &event).then_some(binding.priority)
                        }
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
        context_extraction_settings: TrafficContextExtractionSettings,
    ) -> Result<()> {
        let payload = build_raw_context_request_payload(
            record,
            behavior_signal,
            browser_extension_behavior,
            context_extraction_settings,
        );
        let _ = self
            .dispatch_event(EVENT_TRAFFIC_RAW_READY, payload, "traffic_history")
            .await?;
        Ok(())
    }

    pub(crate) fn schedule_event_dispatch(
        &self,
        event_name: String,
        payload: Value,
        source: String,
    ) {
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
        let payload = self.augment_payload_for_profile(&profile, payload);
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
                .complete_background_run(
                    profile,
                    payload,
                    trigger_event,
                    event,
                    run_id_for_task,
                    0,
                    priority,
                )
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
        let run_result = if profile.id == TRAFFIC_VERIFICATION_AGENT_PROFILE_ID {
            verify_finding_for_runtime(&self, self.db.as_ref(), &self.app_handle, &payload, &run_id)
                .await
                .and_then(|value| serde_json::to_string(&value).map_err(Into::into))
        } else if profile.id == TRAFFIC_CONTEXT_AGENT_PROFILE_ID {
            self.run_context_agent_for_runtime(&payload)
                .await
                .and_then(|value| serde_json::to_string(&value).map_err(Into::into))
        } else if profile.id == TRAFFIC_DECISION_AGENT_PROFILE_ID {
            self.run_decision_agent_for_runtime(&profile, &payload, &event)
                .await
                .and_then(|value| serde_json::to_string(&value).map_err(Into::into))
        } else {
            self.run_profile_llm(&profile, &payload, trigger_event.as_deref(), &run_id)
                .await
        };
        let tool_calls_json = tool_calls_json_from_records(&take_execution_tool_trace(&run_id));

        match run_result {
            Ok(output_json) => {
                let persistence_warning: Option<String> = None;
                let mut output_json = output_json;
                if profile.mode == "passive" && profile.capability == "context" {
                    match serde_json::from_str::<Value>(&output_json) {
                        Ok(output_value) => {
                            self.schedule_event_dispatch(
                                EVENT_TRAFFIC_CONTEXT_READY.to_string(),
                                output_value.clone(),
                                "system_agent_context".to_string(),
                            );
                            let _ = self
                                .app_handle
                                .emit(EVENT_TRAFFIC_CONTEXT_READY, output_value);
                        }
                        Err(error) => {
                            tracing::warn!(
                                "Failed to parse system-agent output as JSON for {}: {}",
                                profile.id,
                                error
                            );
                        }
                    }
                } else if profile.mode == "passive" && profile.capability == "hypothesis" {
                    match serde_json::from_str::<Value>(&output_json) {
                        Ok(output_value) => {
                            let output_value = self
                                .bootstrap_triage_verification_plan(&payload, output_value)
                                .await;
                            let output_value =
                                ensure_triage_output_hypothesis_state(output_value, &payload);
                            output_json =
                                serde_json::to_string(&output_value).unwrap_or(output_json);
                            let event_payload = build_hypothesis_ready_payload(
                                payload.clone(),
                                output_value.clone(),
                                &profile.id,
                            );
                            self.schedule_event_dispatch(
                                EVENT_TRAFFIC_HYPOTHESIS_READY.to_string(),
                                event_payload.clone(),
                                "system_agent_hypothesis".to_string(),
                            );
                            let _ = self
                                .app_handle
                                .emit(EVENT_TRAFFIC_HYPOTHESIS_READY, event_payload);
                        }
                        Err(error) => {
                            tracing::warn!(
                                "Failed to parse system-agent output as JSON for {}: {}",
                                profile.id,
                                error
                            );
                        }
                    }
                } else if profile.id == TRAFFIC_VERIFICATION_AGENT_PROFILE_ID {
                    if let Ok(output_value) = serde_json::from_str::<Value>(&output_json) {
                        self.schedule_event_dispatch(
                            EVENT_TRAFFIC_VERIFICATION_COMPLETED.to_string(),
                            output_value,
                            "system_agent_verification".to_string(),
                        );
                    }
                }

                self.db
                    .update_system_agent_run(
                        &run_id,
                        "completed",
                        tool_calls_json.as_deref(),
                        Some(&output_json),
                        persistence_warning.as_deref(),
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
                        tool_calls_json.as_deref(),
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
        let payload = self.augment_payload_for_profile(&profile, payload);
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
        let tool_calls_json = tool_calls_json_from_records(&take_execution_tool_trace(&run_id));
        let updated = match result {
            Ok(output_json) => {
                self.db
                    .update_system_agent_run(
                        &run_id,
                        "completed",
                        tool_calls_json.as_deref(),
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
                        tool_calls_json.as_deref(),
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
            tool_calls: None,
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
        let ui_language = resolve_ui_language(&self.db).await;
        let tool_policy = SystemAgentToolPolicy::from_profile(profile);
        tool_policy.validate()?;

        let mut prompt_sections = vec![base_prompt.to_string()];
        prompt_sections.push(output_language_instruction(&ui_language).to_string());
        if let Some(prompt_patch) = &profile.prompt_patch {
            if !prompt_patch.trim().is_empty() {
                prompt_sections.push(format!("Additional instructions:\n{}", prompt_patch));
            }
        }
        if let Some(sop_prompt) = render_logic_sop_prompt(profile, payload) {
            prompt_sections.push(format!(
                "Matched SOP guidance. Use these procedures only when the current evidence fits the same scenario:\n{}",
                sop_prompt
            ));
        }
        prompt_sections.push(format!(
            "For additional SOP lookup, use the `sops` tool with profile_id=\"{}\". Treat `sops` as the source of truth for this background agent's registered SOP catalog.",
            profile.id
        ));
        if let Some(tool_policy_note) = tool_policy.prompt_note() {
            prompt_sections.push(tool_policy_note);
        }
        let system_prompt = prompt_sections.join("\n\n");

        let mut effective_payload = payload.clone();
        if let Some(object) = effective_payload.as_object_mut() {
            if !object.contains_key("logicSopContext") {
                let logic_sop_context = build_logic_sop_context(profile, payload);
                if logic_sop_context
                    .as_array()
                    .is_some_and(|items| !items.is_empty())
                {
                    object.insert("logicSopContext".to_string(), logic_sop_context);
                }
            }
        }

        let user_input = serde_json::to_string_pretty(&json!({
            "profileId": profile.id,
            "mode": profile.mode,
            "capability": profile.capability,
            "triggerEvent": trigger_event,
            "payload": effective_payload,
        }))?;

        if let Some(tool_config) =
            Self::build_system_agent_runtime_tool_config(profile, &tool_policy)
        {
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

        match self
            .ai_manager
            .resolve_generation_llm_config(
                profile.llm_provider_override.as_deref(),
                profile.llm_model_override.as_deref(),
            )
            .await
        {
            Ok(llm_config) => {
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
            Err(error)
                if profile.llm_provider_override.is_some()
                    || profile.llm_model_override.is_some() =>
            {
                return Err(error);
            }
            Err(_) => {}
        }

        Ok(serde_json::to_string(&json!({
            "summary": if ui_language == "zh" {
                "当前未配置 AI 服务，已生成确定性的兜底输出。"
            } else {
                "No AI service configured; generated deterministic fallback output."
            },
            "signals": [
                format!("profile={}", profile.id),
                format!("trigger={}", trigger_event.unwrap_or("manual"))
            ],
            "payloadEcho": payload
        }))?)
    }

    pub(crate) async fn run_ad_hoc_json_llm(
        &self,
        system_prompt: &str,
        user_input: &str,
    ) -> Result<Value> {
        let llm_config = self
            .ai_manager
            .resolve_generation_llm_config(None, None)
            .await?;
        let client = sentinel_llm::LlmClient::new(llm_config);
        let raw = client.completion(Some(system_prompt), user_input).await?;
        Ok(normalize_llm_json_output(&raw))
    }

    async fn bootstrap_triage_verification_plan(&self, payload: &Value, output: Value) -> Value {
        if !should_attempt_triage_bootstrap(&output) {
            return output;
        }

        let user_input = match serde_json::to_string_pretty(&json!({
            "payload": payload,
            "triageOutput": &output,
        })) {
            Ok(value) => value,
            Err(error) => {
                tracing::warn!("Failed to serialize triage bootstrap input: {}", error);
                return output;
            }
        };

        let bootstrap_raw = match self
            .run_ad_hoc_json_llm(triage_verification_bootstrap_prompt(), &user_input)
            .await
        {
            Ok(value) => value,
            Err(error) => {
                tracing::debug!("Skipped LLM triage bootstrap planning: {}", error);
                return output;
            }
        };

        let decision = match serde_json::from_value::<TriageBootstrapDecision>(bootstrap_raw) {
            Ok(value) => value,
            Err(error) => {
                tracing::warn!("Invalid triage bootstrap planner output: {}", error);
                return output;
            }
        };

        merge_triage_bootstrap_decision(&output, decision)
    }

    pub(crate) async fn build_semantic_abstraction(&self, payload: &Value) -> Value {
        let signature = build_semantic_signature(payload);
        if let Some(cached) = self.semantic_cache.read().await.get(&signature).cloned() {
            return cached;
        }

        let fallback = deterministic_semantic_abstraction(payload);
        let result = if should_attempt_ai_semantic_mapping(payload) {
            match self.run_semantic_mapper_llm(payload, &fallback).await {
                Ok(value) => value,
                Err(error) => {
                    tracing::debug!("Semantic mapper fallback used: {}", error);
                    fallback
                }
            }
        } else {
            fallback
        };

        let mut cache = self.semantic_cache.write().await;
        cache.insert(signature, result.clone());
        result
    }

    async fn run_semantic_mapper_llm(&self, payload: &Value, fallback: &Value) -> Result<Value> {
        let llm_config = self
            .ai_manager
            .resolve_generation_llm_config(None, None)
            .await?;
        let client = sentinel_llm::LlmClient::new(llm_config);
        let user_input = serde_json::to_string_pretty(&build_semantic_feature_payload(payload))?;
        let raw = client
            .completion(Some(semantic_mapper_prompt()), &user_input)
            .await?;
        Ok(normalize_semantic_abstraction_output(
            &raw, payload, fallback,
        ))
    }

    async fn run_profile_with_agent_executor(
        &self,
        profile: &SystemAgentProfileRecord,
        run_id: &str,
        system_prompt: String,
        task: String,
        tool_config: crate::agents::ToolConfig,
    ) -> Result<String> {
        let config = self
            .ai_manager
            .resolve_generation_llm_config(
                profile.llm_provider_override.as_deref(),
                profile.llm_model_override.as_deref(),
            )
            .await?;
        let params = AgentExecuteParams {
            execution_id: run_id.to_string(),
            cancellation_generation: None,
            model: config.model.clone(),
            system_prompt,
            task,
            active_browser_shell_direct_write_enabled: false,
            active_browser_shell_session_id: None,
            active_terminal_session_fingerprint: None,
            active_terminal_session_id: None,
            working_directory: None,
            rig_provider: config
                .rig_provider
                .clone()
                .unwrap_or_else(|| config.provider.clone()),
            api_key: config.api_key.clone(),
            api_base: config.base_url.clone(),
            max_iterations: 6,
            timeout_secs: 120,
            tool_config: Some(tool_config),
            enable_tenth_man_rule: false,
            tenth_man_config: None,
            document_attachments: None,
            image_attachments: None,
            referenced_traffic: None,
            persist_messages: false,
            subagent_run_id: None,
            context_policy: None,
            context_engine_mode: Some(crate::agents::ContextEngineMode::CodexLike),
            recursion_depth: 0,
        };

        execute_agent(&self.app_handle, params)
            .await
            .map_err(|error| anyhow!("System agent '{}' execution failed: {}", profile.id, error))
    }

    fn augment_payload_for_profile(
        &self,
        profile: &SystemAgentProfileRecord,
        payload: Value,
    ) -> Value {
        let mut enriched = payload;
        if let Some(mut hypothesis_state) = extract_hypothesis_state(&enriched)
            .or_else(|| seed_hypothesis_state_from_logic_hypotheses(&enriched))
        {
            normalize_hypothesis_state(&mut hypothesis_state);
            if let Some(object) = enriched.as_object_mut() {
                object.insert(
                    "hypothesisState".to_string(),
                    serde_json::to_value(hypothesis_state).unwrap_or(Value::Null),
                );
            }
        }
        let logic_sop_context = build_logic_sop_context(profile, &enriched);
        if logic_sop_context
            .as_array()
            .is_some_and(|items| !items.is_empty())
        {
            if let Some(object) = enriched.as_object_mut() {
                object.insert("logicSopContext".to_string(), logic_sop_context);
            }
        }
        enriched
    }

    fn build_system_agent_runtime_tool_config(
        profile: &SystemAgentProfileRecord,
        tool_policy: &SystemAgentToolPolicy,
    ) -> Option<crate::agents::ToolConfig> {
        let config = tool_policy.build_runtime_tool_config();
        let should_inject_sops = profile.mode == "passive"
            && !tool_policy
                .forbidden
                .iter()
                .any(|tool_id| tool_id == SopsTool::NAME);

        if !should_inject_sops {
            return config;
        }

        let mut effective = config.unwrap_or_default();
        if !effective
            .preselected_tools
            .iter()
            .any(|tool_id| tool_id == SopsTool::NAME)
        {
            effective.preselected_tools.push(SopsTool::NAME.to_string());
        }
        if !effective.allowed_tools.is_empty()
            && !effective
                .allowed_tools
                .iter()
                .any(|tool_id| tool_id == SopsTool::NAME)
        {
            effective.allowed_tools.push(SopsTool::NAME.to_string());
        }
        effective.max_tools = effective
            .max_tools
            .max(effective.preselected_tools.len())
            .max(1);
        effective.enabled = true;
        Some(effective)
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
            tool_calls: run
                .tool_calls
                .as_deref()
                .and_then(|raw| serde_json::from_str(raw).ok()),
            started_at: run.started_at,
            finished_at: run.finished_at,
            error_message: run.error_message.clone(),
        };
        let _ = self.app_handle.emit("system-agent:run-updated", payload);
    }

    pub(crate) async fn get_recent_sequence(&self, sequence_key: &str) -> Vec<String> {
        let sequences = self.recent_sequences.read().await;
        sequences.get(sequence_key).cloned().unwrap_or_default()
    }

    pub(crate) async fn push_recent_sequence(&self, sequence_key: &str, action_kind: String) {
        let mut sequences = self.recent_sequences.write().await;
        let entry = sequences.entry(sequence_key.to_string()).or_default();
        entry.push(action_kind);
        if entry.len() > 6 {
            let overflow = entry.len() - 6;
            entry.drain(0..overflow);
        }
    }

    pub(crate) async fn update_cluster_summary(&self, payload: &Value) -> Option<Value> {
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
            tracing::warn!(
                "Failed to persist queued system agent run {}: {}",
                run_id,
                error
            );
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
                        let wait_secs = runtime
                            .cooldown_wait_seconds(
                                &queued.profile,
                                &queued.payload,
                                &queued.cooldown_key,
                            )
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

    async fn start_background_run_from_queue(
        &self,
        queued: QueuedSystemAgentRun,
    ) -> Result<String> {
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
            Some(EVENT_TRAFFIC_RAW_READY) => {
                if let Some(url) = payload
                    .get("record")
                    .and_then(|value| value.get("url"))
                    .and_then(Value::as_str)
                {
                    return format!("{}::raw::{url}", profile.id);
                }
            }
            Some(EVENT_TRAFFIC_CONTEXT_READY) => {
                if let Some(cluster_key) = payload.get("clusterKey").and_then(Value::as_str) {
                    return format!("{}::cluster::{cluster_key}", profile.id);
                }
                if let Some(path_template) = payload.get("pathTemplate").and_then(Value::as_str) {
                    return format!("{}::path::{path_template}", profile.id);
                }
            }
            Some(EVENT_TRAFFIC_HYPOTHESIS_READY) => {
                if let Some(cluster_key) = payload
                    .get("contextPayload")
                    .and_then(|value| value.get("clusterKey"))
                    .and_then(Value::as_str)
                {
                    return format!("{}::hypothesis::{cluster_key}", profile.id);
                }
            }
            Some("traffic.verification.requested") | Some("traffic.verification.completed") => {
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
        if queue
            .iter()
            .any(|item| item.cooldown_key == queued.cooldown_key)
        {
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
    if profile.capability != "hypothesis" {
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
            semantic_cache: self.semantic_cache.clone(),
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
    matches!(
        category,
        "llm_service" | "verification_execution" | "runtime_recovery" | "unknown"
    )
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

fn tool_calls_json_from_records(tool_calls: &[ToolCallRecord]) -> Option<String> {
    if tool_calls.is_empty() {
        None
    } else {
        serde_json::to_string(tool_calls).ok()
    }
}

fn ensure_triage_output_hypothesis_state(output: Value, payload: &Value) -> Value {
    let Some(mut hypothesis_state) = derive_triage_hypothesis_state(&output, payload) else {
        return output;
    };
    normalize_hypothesis_state(&mut hypothesis_state);

    let mut enriched = output;
    let Some(object) = enriched.as_object_mut() else {
        return enriched;
    };
    object.insert(
        "hypothesisState".to_string(),
        serde_json::to_value(hypothesis_state).unwrap_or(Value::Null),
    );
    enriched
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn preserves_seeded_hypothesis_state_when_triage_output_omits_it() {
        let payload = json!({
            "logicHypotheses": [{
                "id": "repeatable_single_use_action",
                "riskType": "logic",
                "confidence": "high",
                "summary": "The action may violate single-use expectations."
            }]
        });
        let output = json!({
            "summary": "需要进一步确认",
            "riskType": "logic",
            "confidence": "medium"
        });

        let enriched = ensure_triage_output_hypothesis_state(output, &payload);

        assert_eq!(
            enriched
                .get("hypothesisState")
                .and_then(|value| value.get("active"))
                .and_then(Value::as_array)
                .and_then(|items| items.first())
                .and_then(Value::as_str),
            Some("The action may violate single-use expectations.")
        );
    }

    #[test]
    fn passive_profiles_inject_sops_tool_by_default() {
        let profile = SystemAgentProfileRecord {
            id: "traffic_logic_triage".to_string(),
            name: "Traffic Logic Triage".to_string(),
            description: String::new(),
            mode: "passive".to_string(),
            capability: "triage".to_string(),
            enabled: true,
            trigger_mode: "event".to_string(),
            llm_provider_override: None,
            llm_model_override: None,
            base_prompt_id: None,
            prompt_patch: None,
            sop_definitions_json: "[]".to_string(),
            input_schema_json: "{}".to_string(),
            output_schema_json: "{}".to_string(),
            required_tools_json: "[]".to_string(),
            optional_tools_json: "[]".to_string(),
            forbidden_tools_json: "[]".to_string(),
            trigger_events_json: "[]".to_string(),
            budget_json: "{}".to_string(),
            safety_policy_json: "{}".to_string(),
            cooldown_secs: 0,
            max_concurrency: 1,
            risk_level: "high".to_string(),
            visibility: "system".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let config = SystemAgentRuntime::build_system_agent_runtime_tool_config(
            &profile,
            &SystemAgentToolPolicy::from_profile(&profile),
        )
        .expect("sops tool should be injected");

        assert!(matches!(
            config.selection_strategy,
            crate::agents::ToolSelectionStrategy::Keyword
        ));
        assert!(config
            .preselected_tools
            .iter()
            .any(|tool| tool == SopsTool::NAME));
    }

    #[test]
    fn forbidden_sops_tool_is_respected() {
        let profile = SystemAgentProfileRecord {
            id: "traffic_logic_triage".to_string(),
            name: "Traffic Logic Triage".to_string(),
            description: String::new(),
            mode: "passive".to_string(),
            capability: "triage".to_string(),
            enabled: true,
            trigger_mode: "event".to_string(),
            llm_provider_override: None,
            llm_model_override: None,
            base_prompt_id: None,
            prompt_patch: None,
            sop_definitions_json: "[]".to_string(),
            input_schema_json: "{}".to_string(),
            output_schema_json: "{}".to_string(),
            required_tools_json: "[]".to_string(),
            optional_tools_json: "[]".to_string(),
            forbidden_tools_json: serde_json::to_string(&vec![SopsTool::NAME]).unwrap(),
            trigger_events_json: "[]".to_string(),
            budget_json: "{}".to_string(),
            safety_policy_json: "{}".to_string(),
            cooldown_secs: 0,
            max_concurrency: 1,
            risk_level: "high".to_string(),
            visibility: "system".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let config = SystemAgentRuntime::build_system_agent_runtime_tool_config(
            &profile,
            &SystemAgentToolPolicy::from_profile(&profile),
        )
        .expect("forbidden policy should still return runtime restrictions");

        assert!(!config
            .preselected_tools
            .iter()
            .any(|tool| tool == SopsTool::NAME));
        assert!(config
            .disabled_tools
            .iter()
            .any(|tool| tool == SopsTool::NAME));
    }
}
