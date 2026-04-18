use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use sentinel_traffic::HttpRequestRecord;

use crate::services::system_agents::behavior_session::build_behavior_session;
use crate::services::system_agents::behavior_signal::TrafficBehaviorSignalSettings;
use crate::services::system_agents::context::build_traffic_context_snapshot;
use crate::services::system_agents::context_settings::TrafficContextExtractionSettings;
use crate::services::system_agents::logic_hypotheses::build_logic_hypotheses;
use crate::services::system_agents::logic_invariants::evaluate_logic_invariants;
use crate::services::system_agents::logic_skill_context::build_logic_skill_context;
use crate::services::system_agents::process_graph::build_process_graph;
use crate::services::system_agents::runtime::SystemAgentRuntime;
use crate::services::system_agents::skill_recommendation::recommend_logic_skills;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrafficRawContextRequest {
    pub record: HttpRequestRecord,
    pub behavior_signal: TrafficBehaviorSignalSettings,
    #[serde(default)]
    pub browser_extension_behavior: Option<Value>,
    pub context_extraction_settings: TrafficContextExtractionSettings,
}

pub fn build_raw_context_request_payload(
    record: HttpRequestRecord,
    behavior_signal: TrafficBehaviorSignalSettings,
    browser_extension_behavior: Option<Value>,
    context_extraction_settings: TrafficContextExtractionSettings,
) -> Value {
    json!({
        "record": record,
        "behaviorSignal": behavior_signal,
        "browserExtensionBehavior": browser_extension_behavior,
        "contextExtractionSettings": context_extraction_settings,
    })
}

impl SystemAgentRuntime {
    pub async fn run_context_agent_for_runtime(&self, payload: &Value) -> Result<Value> {
        let request = serde_json::from_value::<TrafficRawContextRequest>(payload.clone())
            .map_err(|error| anyhow!("Invalid traffic raw context payload: {error}"))?;
        let initial_snapshot = build_traffic_context_snapshot(
            &request.record,
            &[],
            &request.context_extraction_settings,
        );
        let recent_sequence = self
            .get_recent_sequence(&initial_snapshot.sequence_key)
            .await;
        let mut snapshot = build_traffic_context_snapshot(
            &request.record,
            &recent_sequence,
            &request.context_extraction_settings,
        );

        if let Some(object) = snapshot.payload.as_object_mut() {
            object.insert(
                "behaviorSignal".to_string(),
                request.behavior_signal.to_payload(),
            );
            if let Some(browser_extension_behavior) = request.browser_extension_behavior {
                object.insert(
                    "browserExtensionBehavior".to_string(),
                    browser_extension_behavior,
                );
            }
        }

        self.push_recent_sequence(&snapshot.sequence_key, snapshot.action_kind.clone())
            .await;
        if let Some(summary) = self.update_cluster_summary(&snapshot.payload).await {
            if let Some(object) = snapshot.payload.as_object_mut() {
                object.insert("clusterSummary".to_string(), summary);
            }
        }
        if let Some(behavior_session) = build_behavior_session(&snapshot.payload) {
            if let Some(object) = snapshot.payload.as_object_mut() {
                object.insert("behaviorSession".to_string(), behavior_session);
            }
        }
        if let Some(process_graph) = build_process_graph(&snapshot.payload) {
            if let Some(object) = snapshot.payload.as_object_mut() {
                object.insert("processGraph".to_string(), process_graph);
            }
        }

        let semantic_abstraction = self.build_semantic_abstraction(&snapshot.payload).await;
        if let Some(object) = snapshot.payload.as_object_mut() {
            object.insert("semanticAbstraction".to_string(), semantic_abstraction);
        }
        let logic_invariants = evaluate_logic_invariants(&snapshot.payload);
        if let Some(object) = snapshot.payload.as_object_mut() {
            object.insert("logicInvariants".to_string(), logic_invariants);
        }
        let logic_hypotheses = build_logic_hypotheses(&snapshot.payload);
        if let Some(object) = snapshot.payload.as_object_mut() {
            object.insert("logicHypotheses".to_string(), logic_hypotheses);
        }
        let skill_recommendations = recommend_logic_skills(&snapshot.payload);
        if let Some(object) = snapshot.payload.as_object_mut() {
            object.insert("skillRecommendations".to_string(), skill_recommendations);
        }
        let db = self.db();
        let logic_skill_context = build_logic_skill_context(db.as_ref(), &snapshot.payload);
        if let Some(object) = snapshot.payload.as_object_mut() {
            object.insert("logicSkillContext".to_string(), logic_skill_context);
        }

        Ok(snapshot.payload)
    }
}
