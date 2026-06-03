use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::services::system_agents::findings::persist_passive_agent_finding;
use crate::services::system_agents::pipeline::{
    EVENT_TRAFFIC_CONTEXT_READY, EVENT_TRAFFIC_VERIFICATION_REQUESTED,
};
use crate::services::system_agents::runtime::SystemAgentRuntime;
use crate::services::system_agents::types::SystemAgentEvent;
use sentinel_db::SystemAgentProfileRecord;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DecisionAgentInput {
    pub context_payload: Value,
    pub hypothesis_output: Value,
    pub source_profile_id: String,
}

impl SystemAgentRuntime {
    pub async fn run_decision_agent_for_runtime(
        &self,
        profile: &SystemAgentProfileRecord,
        payload: &Value,
        event: &SystemAgentEvent,
    ) -> Result<Value> {
        let input = serde_json::from_value::<DecisionAgentInput>(payload.clone())
            .map_err(|error| anyhow!("Invalid decision-agent payload: {error}"))?;
        let context_event = SystemAgentEvent {
            event_name: EVENT_TRAFFIC_CONTEXT_READY.to_string(),
            payload: input.context_payload.clone(),
            source: input.source_profile_id.clone(),
            timestamp: event.timestamp,
        };

        let finding_id = persist_passive_agent_finding(
            self.db().as_ref(),
            &self.app_handle(),
            &profile.id,
            &profile.safety_policy_json,
            &context_event,
            &input.hypothesis_output,
        )
        .await?;

        if let Some(finding_id) = finding_id.clone() {
            self.schedule_event_dispatch(
                EVENT_TRAFFIC_VERIFICATION_REQUESTED.to_string(),
                json!({
                    "findingId": finding_id,
                    "sourceProfileId": input.source_profile_id,
                    "riskType": input.hypothesis_output.get("riskType").cloned().unwrap_or(Value::Null),
                    "confidence": input.hypothesis_output.get("confidence").cloned().unwrap_or(Value::Null),
                    "hypothesisState": input.hypothesis_output.get("hypothesisState").cloned().unwrap_or(Value::Null),
                    "verificationPlan": input.hypothesis_output.get("verificationPlan").cloned().unwrap_or(Value::Null),
                }),
                "system_agent_decision".to_string(),
            );
        }

        Ok(json!({
            "summary": if finding_id.is_some() {
                "Decision agent promoted the hypothesis into a finding."
            } else {
                "Decision agent rejected the hypothesis promotion."
            },
            "promoted": finding_id.is_some(),
            "findingId": finding_id,
            "sourceProfileId": input.source_profile_id,
        }))
    }
}
