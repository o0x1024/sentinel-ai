use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone, Default)]
pub struct InterceptTracking {
    request_ids: Arc<RwLock<HashSet<String>>>,
}

impl InterceptTracking {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn mark_request_intercepted(&self, request_id: &str) {
        let mut guard = self.request_ids.write().await;
        guard.insert(request_id.to_string());
    }

    pub async fn was_request_intercepted(&self, request_id: &str) -> bool {
        let guard = self.request_ids.read().await;
        guard.contains(request_id)
    }

    pub async fn clear_request(&self, request_id: &str) {
        let mut guard = self.request_ids.write().await;
        guard.remove(request_id);
    }
}
