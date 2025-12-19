//! Global shared state for multi-agent exploration
//!
//! Provides thread-safe deduplication and aggregation across all workers

use super::types::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// Global exploration state shared across all agents
pub struct GlobalExplorerState {
    /// Visited URLs (for deduplication)
    visited_urls: Arc<RwLock<HashSet<String>>>,
    /// Discovered APIs (deduplicated by fingerprint)
    discovered_apis: Arc<RwLock<HashMap<String, DiscoveredApi>>>,
    /// URL -> Scope mapping (which worker is responsible)
    url_scope_map: Arc<RwLock<HashMap<String, String>>>,
    /// Cross-scope links waiting to be processed
    pending_cross_scope: Arc<RwLock<Vec<CrossScopeLink>>>,
    /// Worker results
    worker_results: Arc<RwLock<Vec<WorkerResult>>>,
    /// Target domain for filtering
    target_domain: String,
    /// Global ignore patterns
    ignore_patterns: Vec<String>,
    /// Blacklisted URLs (dead ends, infinite loops, etc.)
    blacklisted_urls: Arc<RwLock<HashSet<String>>>,
    /// Blacklist reasons for debugging
    blacklist_reasons: Arc<RwLock<HashMap<String, String>>>,
    /// Global pause flag
    is_paused: Arc<RwLock<bool>>,
    /// Shared authentication state (Cookies, LocalStorage, etc.)
    auth_state: Arc<RwLock<Option<AuthSnapshot>>>,
    /// Dynamically discovered scopes (reported by Workers)
    discovered_scopes: Arc<RwLock<Vec<ExplorationScope>>>,
    /// Pending scope tasks (to be assigned to new workers)
    pending_scope_tasks: Arc<RwLock<Vec<WorkerTask>>>,
    /// Navigation patterns discovered during exploration
    discovered_nav_patterns: Arc<RwLock<HashSet<String>>>,
}

/// Snapshot of authentication state
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AuthSnapshot {
    /// Cookies in Playwright format
    pub cookies: Vec<serde_json::Value>,
    /// Local Storage items
    pub local_storage: HashMap<String, String>,
    /// Session Storage items
    pub session_storage: HashMap<String, String>,
    /// Timestamp of capture
    pub timestamp: i64,
}

impl GlobalExplorerState {
    /// Create new global state
    pub fn new(target_url: &str, ignore_patterns: Vec<String>) -> Self {
        let target_domain = extract_domain(target_url).unwrap_or_default();

        Self {
            visited_urls: Arc::new(RwLock::new(HashSet::new())),
            discovered_apis: Arc::new(RwLock::new(HashMap::new())),
            url_scope_map: Arc::new(RwLock::new(HashMap::new())),
            pending_cross_scope: Arc::new(RwLock::new(Vec::new())),
            worker_results: Arc::new(RwLock::new(Vec::new())),
            target_domain,
            ignore_patterns,
            blacklisted_urls: Arc::new(RwLock::new(HashSet::new())),
            blacklist_reasons: Arc::new(RwLock::new(HashMap::new())),
            is_paused: Arc::new(RwLock::new(false)),
            auth_state: Arc::new(RwLock::new(None)),
            discovered_scopes: Arc::new(RwLock::new(Vec::new())),
            pending_scope_tasks: Arc::new(RwLock::new(Vec::new())),
            discovered_nav_patterns: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    /// Check if URL was already visited (and mark as visited if not)
    pub async fn try_visit_url(&self, url: &str) -> bool {
        // Normalize URL
        let normalized = normalize_url(url);

        // Check ignore patterns
        if self.should_ignore(&normalized) {
            debug!("URL ignored by pattern: {}", normalized);
            return false;
        }

        // Check blacklist
        if self.is_blacklisted(&normalized).await {
            debug!("URL blacklisted: {}", normalized);
            return false;
        }

        // Check domain
        if !self.is_target_domain(&normalized) {
            debug!("URL outside target domain: {}", normalized);
            return false;
        }

        // Try to insert (atomic check-and-set)
        let mut visited = self.visited_urls.write().await;
        if visited.contains(&normalized) {
            false
        } else {
            visited.insert(normalized);
            true
        }
    }

    /// Check if URL was visited without marking
    pub async fn is_visited(&self, url: &str) -> bool {
        let normalized = normalize_url(url);
        let visited = self.visited_urls.read().await;
        visited.contains(&normalized)
    }

    /// Mark URL as visited (force)
    pub async fn mark_visited(&self, url: &str) {
        let normalized = normalize_url(url);
        let mut visited = self.visited_urls.write().await;
        visited.insert(normalized);
    }

    /// Register discovered API (with deduplication)
    pub async fn register_api(&self, api: DiscoveredApi) -> bool {
        let fingerprint = generate_api_fingerprint(&api);

        let mut apis = self.discovered_apis.write().await;
        if apis.contains_key(&fingerprint) {
            false
        } else {
            info!("Global: New API discovered: {} {}", api.method, api.path);
            apis.insert(fingerprint, api);
            true
        }
    }

    /// Register multiple APIs
    pub async fn register_apis(&self, apis: Vec<DiscoveredApi>) -> usize {
        let mut count = 0;
        for api in apis {
            if self.register_api(api).await {
                count += 1;
            }
        }
        count
    }

    /// Get all discovered APIs
    pub async fn get_all_apis(&self) -> Vec<DiscoveredApi> {
        let apis = self.discovered_apis.read().await;
        apis.values().cloned().collect()
    }

    /// Register URL scope mapping
    pub async fn register_scope(&self, url_prefix: &str, scope_id: &str) {
        let mut map = self.url_scope_map.write().await;
        map.insert(url_prefix.to_string(), scope_id.to_string());
    }

    /// Find scope for URL
    pub async fn find_scope(&self, url: &str) -> Option<String> {
        let map = self.url_scope_map.read().await;

        // Find longest matching prefix
        let mut best_match: Option<(&String, &String)> = None;
        for (prefix, scope_id) in map.iter() {
            if url.starts_with(prefix) {
                match best_match {
                    None => best_match = Some((prefix, scope_id)),
                    Some((best_prefix, _)) if prefix.len() > best_prefix.len() => {
                        best_match = Some((prefix, scope_id));
                    }
                    _ => {}
                }
            }
        }

        best_match.map(|(_, id)| id.clone())
    }

    /// Add cross-scope link for later processing
    pub async fn add_cross_scope_link(&self, link: CrossScopeLink) {
        let mut pending = self.pending_cross_scope.write().await;
        pending.push(link);
    }

    /// Get and clear pending cross-scope links
    pub async fn drain_cross_scope_links(&self) -> Vec<CrossScopeLink> {
        let mut pending = self.pending_cross_scope.write().await;
        std::mem::take(&mut *pending)
    }

    /// Store worker result
    pub async fn store_worker_result(&self, result: WorkerResult) {
        let mut results = self.worker_results.write().await;
        results.push(result);
    }

    /// Get all worker results
    pub async fn get_worker_results(&self) -> Vec<WorkerResult> {
        let results = self.worker_results.read().await;
        results.clone()
    }

    /// Get exploration statistics
    pub async fn get_stats(&self) -> GlobalStats {
        let visited = self.visited_urls.read().await;
        let apis = self.discovered_apis.read().await;
        let results = self.worker_results.read().await;

        GlobalStats {
            total_urls_visited: visited.len(),
            total_apis_discovered: apis.len(),
            workers_completed: results.len(),
            total_elements_interacted: results.iter().map(|r| r.stats.elements_interacted).sum(),
        }
    }

    /// Check if URL should be ignored
    fn should_ignore(&self, url: &str) -> bool {
        let lower = url.to_lowercase();
        self.ignore_patterns.iter().any(|p| lower.contains(p))
    }

    /// Check if URL belongs to target domain
    fn is_target_domain(&self, url: &str) -> bool {
        if self.target_domain.is_empty() {
            return true;
        }

        extract_domain(url)
            .map(|d| d.contains(&self.target_domain) || self.target_domain.contains(&d))
            .unwrap_or(false)
    }

    /// Check if URL is blacklisted
    pub async fn is_blacklisted(&self, url: &str) -> bool {
        let normalized = normalize_url(url);
        let blacklist = self.blacklisted_urls.read().await;
        blacklist.contains(&normalized)
    }

    /// Add URL to blacklist with reason
    pub async fn blacklist_url(&self, url: &str, reason: &str) {
        let normalized = normalize_url(url);

        let mut blacklist = self.blacklisted_urls.write().await;
        if blacklist.insert(normalized.clone()) {
            info!("URL blacklisted: {} (reason: {})", &normalized, reason);

            // Store reason
            let mut reasons = self.blacklist_reasons.write().await;
            reasons.insert(normalized, reason.to_string());
        }
    }

    /// Get blacklist reason for URL
    pub async fn get_blacklist_reason(&self, url: &str) -> Option<String> {
        let normalized = normalize_url(url);
        let reasons = self.blacklist_reasons.read().await;
        reasons.get(&normalized).cloned()
    }

    /// Get all blacklisted URLs
    pub async fn get_blacklisted_urls(&self) -> Vec<(String, String)> {
        let blacklist = self.blacklisted_urls.read().await;
        let reasons = self.blacklist_reasons.read().await;

        blacklist
            .iter()
            .map(|url| {
                let reason = reasons.get(url).cloned().unwrap_or_default();
                (url.clone(), reason)
            })
            .collect()
    }

    /// Pause exploration
    pub async fn pause(&self) {
        let mut paused = self.is_paused.write().await;
        *paused = true;
        info!("Global exploration paused");
    }

    /// Resume exploration
    pub async fn resume(&self) {
        let mut paused = self.is_paused.write().await;
        *paused = false;
        info!("Global exploration resumed");
    }

    /// Check if paused
    pub async fn is_paused(&self) -> bool {
        *self.is_paused.read().await
    }

    /// Wait if paused (blocks until resumed)
    pub async fn wait_if_paused(&self) {
        if !self.is_paused().await {
            return;
        }

        info!("Worker waiting for resume...");
        loop {
            tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
            if !self.is_paused().await {
                info!("Worker resumed");
                break;
            }
        }
    }

    /// Save authentication snapshot
    pub async fn save_auth_state(&self, snapshot: AuthSnapshot) {
        let mut auth = self.auth_state.write().await;
        // Only update if newer
        if let Some(current) = &*auth {
            if snapshot.timestamp > current.timestamp {
                info!("Global: Updating auth state (timestamp: {})", snapshot.timestamp);
                *auth = Some(snapshot);
            }
        } else {
            info!("Global: Setting initial auth state");
            *auth = Some(snapshot);
        }
    }

    /// Get current authentication snapshot
    pub async fn get_auth_state(&self) -> Option<AuthSnapshot> {
        let auth = self.auth_state.read().await;
        auth.clone()
    }

    // ========== Dynamic Scope Discovery Methods ==========

    /// Report a newly discovered scope from a Worker
    /// Returns true if the scope was added (not a duplicate)
    pub async fn report_discovered_scope(&self, scope: ExplorationScope) -> bool {
        let mut scopes = self.discovered_scopes.write().await;
        
        // Check for duplicate by ID
        if scopes.iter().any(|s| s.id == scope.id) {
            debug!("Scope already exists: {}", scope.id);
            return false;
        }

        // Check if any pattern already registered
        let map = self.url_scope_map.read().await;
        for pattern in &scope.url_patterns {
            if map.contains_key(pattern) {
                debug!("Scope pattern already registered: {}", pattern);
                return false;
            }
        }
        drop(map);

        info!(
            "Global: New scope discovered by worker: {} (entry: {}, patterns: {:?})",
            scope.name, scope.entry_url, scope.url_patterns
        );
        
        // Register patterns
        for pattern in &scope.url_patterns {
            self.register_scope(pattern, &scope.id).await;
        }

        scopes.push(scope);
        true
    }

    /// Create and queue a worker task for a discovered scope
    pub async fn queue_scope_task(&self, scope: &ExplorationScope, max_iterations: u32) {
        let task = WorkerTask {
            id: format!("dynamic-task-{}", scope.id),
            scope: scope.clone(),
            context: format!(
                "Dynamically discovered scope: {}. Entry: {}",
                scope.name, scope.entry_url
            ),
            max_iterations,
        };

        let mut tasks = self.pending_scope_tasks.write().await;
        
        // Avoid duplicates
        if !tasks.iter().any(|t| t.scope.id == scope.id) {
            info!("Global: Queuing new worker task for scope: {}", scope.name);
            tasks.push(task);
        }
    }

    /// Get pending scope tasks (and clear the queue)
    pub async fn drain_pending_scope_tasks(&self) -> Vec<WorkerTask> {
        let mut tasks = self.pending_scope_tasks.write().await;
        std::mem::take(&mut *tasks)
    }

    /// Check if there are pending scope tasks
    pub async fn has_pending_scope_tasks(&self) -> bool {
        let tasks = self.pending_scope_tasks.read().await;
        !tasks.is_empty()
    }

    /// Get all dynamically discovered scopes
    pub async fn get_discovered_scopes(&self) -> Vec<ExplorationScope> {
        let scopes = self.discovered_scopes.read().await;
        scopes.clone()
    }

    /// Record a navigation pattern (for deduplication)
    pub async fn record_nav_pattern(&self, pattern: &str) -> bool {
        let mut patterns = self.discovered_nav_patterns.write().await;
        patterns.insert(pattern.to_string())
    }

    /// Check if a navigation pattern was already discovered
    pub async fn is_nav_pattern_known(&self, pattern: &str) -> bool {
        let patterns = self.discovered_nav_patterns.read().await;
        patterns.contains(pattern)
    }

    /// Report a potential new scope from URL discovery during exploration
    /// This is a convenience method that creates an ExplorationScope from minimal info
    pub async fn report_discovered_url_scope(
        &self,
        scope_id: &str,
        scope_name: &str,
        entry_url: &str,
        url_patterns: Vec<String>,
        priority: u32,
    ) -> bool {
        let scope = ExplorationScope {
            id: scope_id.to_string(),
            name: scope_name.to_string(),
            url_patterns,
            entry_url: entry_url.to_string(),
            max_depth: 5, // Default depth for dynamically discovered scopes
            priority,
        };
        self.report_discovered_scope(scope).await
    }
}

/// Global exploration statistics
#[derive(Debug, Clone)]
pub struct GlobalStats {
    pub total_urls_visited: usize,
    pub total_apis_discovered: usize,
    pub workers_completed: usize,
    pub total_elements_interacted: usize,
}

/// Extract domain from URL
fn extract_domain(url: &str) -> Option<String> {
    url::Url::parse(url)
        .ok()
        .and_then(|u| u.host_str().map(String::from))
}

/// Normalize URL for deduplication
fn normalize_url(url: &str) -> String {
    // Remove trailing slash, fragment, and normalize
    let mut normalized = url.trim().to_string();

    // Remove fragment
    if let Some(idx) = normalized.find('#') {
        normalized.truncate(idx);
    }

    // Remove trailing slash (except for root)
    if normalized.len() > 1 && normalized.ends_with('/') {
        normalized.pop();
    }

    // Sort query parameters for consistency
    if let Ok(mut parsed) = url::Url::parse(&normalized) {
        let mut params: Vec<_> = parsed.query_pairs().into_owned().collect();
        params.sort_by(|a, b| a.0.cmp(&b.0));

        if params.is_empty() {
            parsed.set_query(None);
        } else {
            let query: String = params
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join("&");
            parsed.set_query(Some(&query));
        }

        normalized = parsed.to_string();
    }

    normalized
}

/// Generate fingerprint for API deduplication
fn generate_api_fingerprint(api: &DiscoveredApi) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    api.method.hash(&mut hasher);
    api.path.hash(&mut hasher);

    // Include sorted parameter keys
    let mut param_keys: Vec<_> = api.parameters.keys().collect();
    param_keys.sort();
    for key in param_keys {
        key.hash(&mut hasher);
    }

    format!("{:x}", hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_url_deduplication() {
        let state = GlobalExplorerState::new("https://example.com", vec![]);

        // First visit should succeed
        assert!(state.try_visit_url("https://example.com/page1").await);

        // Second visit to same URL should fail
        assert!(!state.try_visit_url("https://example.com/page1").await);

        // Different URL should succeed
        assert!(state.try_visit_url("https://example.com/page2").await);
    }

    #[tokio::test]
    async fn test_api_deduplication() {
        let state = GlobalExplorerState::new("https://example.com", vec![]);

        let api1 = DiscoveredApi {
            method: "GET".to_string(),
            path: "/api/users".to_string(),
            full_url: "https://example.com/api/users".to_string(),
            parameters: Default::default(),
            status_code: Some(200),
        };

        // First registration should succeed
        assert!(state.register_api(api1.clone()).await);

        // Same API should be deduplicated
        assert!(!state.register_api(api1).await);
    }

    #[test]
    fn test_normalize_url() {
        assert_eq!(
            normalize_url("https://example.com/page/"),
            "https://example.com/page"
        );

        assert_eq!(
            normalize_url("https://example.com/page#section"),
            "https://example.com/page"
        );
    }
}
