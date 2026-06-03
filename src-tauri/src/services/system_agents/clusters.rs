use serde_json::{json, Map, Value};
use std::collections::{HashMap, HashSet, VecDeque};

const MAX_RECENT_REQUESTS: usize = 8;
const MAX_RECENT_STATUS_CODES: usize = 8;
const MAX_CONTENT_TYPES: usize = 8;

#[derive(Debug, Default, Clone)]
pub struct TrafficClusterStore {
    clusters: HashMap<String, TrafficClusterState>,
}

impl TrafficClusterStore {
    pub fn update_with_payload(&mut self, payload: &Value) -> Option<Value> {
        let cluster_key = payload.get("clusterKey")?.as_str()?.to_string();
        let state = self.clusters.entry(cluster_key).or_default();
        state.update(payload);
        Some(state.to_summary())
    }
}

#[derive(Debug, Clone)]
struct TrafficClusterState {
    total_requests: u64,
    auth_fingerprints: HashSet<String>,
    principal_signal_keys: HashSet<String>,
    resource_field_names: HashSet<String>,
    action_counts: HashMap<String, u64>,
    status_counts: HashMap<String, u64>,
    content_types: HashSet<String>,
    recent_request_ids: VecDeque<i64>,
    recent_status_codes: VecDeque<i64>,
    last_request_id: Option<i64>,
    last_seen_at: Option<String>,
}

impl Default for TrafficClusterState {
    fn default() -> Self {
        Self {
            total_requests: 0,
            auth_fingerprints: HashSet::new(),
            principal_signal_keys: HashSet::new(),
            resource_field_names: HashSet::new(),
            action_counts: HashMap::new(),
            status_counts: HashMap::new(),
            content_types: HashSet::new(),
            recent_request_ids: VecDeque::new(),
            recent_status_codes: VecDeque::new(),
            last_request_id: None,
            last_seen_at: None,
        }
    }
}

impl TrafficClusterState {
    fn update(&mut self, payload: &Value) {
        self.total_requests += 1;

        if let Some(fingerprint) = payload
            .get("authContext")
            .and_then(|value| value.get("fingerprint"))
            .and_then(Value::as_str)
        {
            if !fingerprint.is_empty() {
                self.auth_fingerprints.insert(fingerprint.to_string());
            }
        }

        if let Some(principal_context) = payload.get("principalContext").and_then(Value::as_object)
        {
            for key in principal_context.keys() {
                self.principal_signal_keys.insert(key.to_string());
            }
        }

        if let Some(resource_keys) = payload.get("resourceKeys").and_then(Value::as_object) {
            for key in resource_keys.keys() {
                self.resource_field_names.insert(key.to_string());
            }
        }

        if let Some(action_kind) = payload.get("actionKind").and_then(Value::as_str) {
            *self
                .action_counts
                .entry(action_kind.to_string())
                .or_insert(0) += 1;
        }

        if let Some(status_code) = payload.get("statusCode").and_then(Value::as_i64) {
            *self
                .status_counts
                .entry(status_code.to_string())
                .or_insert(0) += 1;
            push_bounded(
                &mut self.recent_status_codes,
                status_code,
                MAX_RECENT_STATUS_CODES,
            );
        }

        if let Some(content_type) = payload
            .get("responseFingerprint")
            .and_then(|value| value.get("contentType"))
            .and_then(Value::as_str)
        {
            let normalized = content_type.trim();
            if !normalized.is_empty() && self.content_types.len() < MAX_CONTENT_TYPES {
                self.content_types.insert(normalized.to_string());
            }
        }

        if let Some(request_id) = payload.get("dbRequestId").and_then(Value::as_i64) {
            self.last_request_id = Some(request_id);
            push_bounded(
                &mut self.recent_request_ids,
                request_id,
                MAX_RECENT_REQUESTS,
            );
        }

        if let Some(timestamp) = payload.get("timestamp").and_then(Value::as_str) {
            self.last_seen_at = Some(timestamp.to_string());
        }
    }

    fn to_summary(&self) -> Value {
        json!({
            "totalRequests": self.total_requests,
            "distinctAuthContexts": self.auth_fingerprints.len(),
            "principalSignalKeys": sorted_set(&self.principal_signal_keys),
            "resourceFieldNames": sorted_set(&self.resource_field_names),
            "actionHistogram": sorted_map(&self.action_counts),
            "statusHistogram": sorted_map(&self.status_counts),
            "recentRequestIds": self.recent_request_ids.iter().copied().collect::<Vec<_>>(),
            "recentStatusCodes": self.recent_status_codes.iter().copied().collect::<Vec<_>>(),
            "responseContentTypes": sorted_set(&self.content_types),
            "lastRequestId": self.last_request_id,
            "lastSeenAt": self.last_seen_at,
        })
    }
}

fn push_bounded(queue: &mut VecDeque<i64>, value: i64, max_len: usize) {
    queue.push_back(value);
    while queue.len() > max_len {
        queue.pop_front();
    }
}

fn sorted_set(set: &HashSet<String>) -> Vec<String> {
    let mut values = set.iter().cloned().collect::<Vec<_>>();
    values.sort();
    values
}

fn sorted_map(map: &HashMap<String, u64>) -> Value {
    let mut entries = map.iter().collect::<Vec<_>>();
    entries.sort_by(|left, right| left.0.cmp(right.0));
    let mut object = Map::new();
    for (key, value) in entries {
        object.insert(key.clone(), Value::Number((*value).into()));
    }
    Value::Object(object)
}
