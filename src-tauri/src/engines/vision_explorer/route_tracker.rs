//! 路由追踪器
//!
//! 追踪 SPA 应用的路由变化，确保所有发现的路由都被访问

use std::collections::{HashMap, HashSet, VecDeque};
use regex::Regex;
use tracing::{debug, info, warn};
use url::Url;

/// Route ignore pattern (supports both string contains and regex)
#[derive(Debug, Clone)]
pub enum IgnorePattern {
    /// Simple string contains match
    Contains(String),
    /// Regex pattern match
    Regex(String),
}

/// 路由追踪器
#[derive(Clone)]
pub struct RouteTracker {
    /// 目标域名（用于过滤外部链接）
    target_domain: String,
    /// 已发现的所有路由
    discovered_routes: HashSet<String>,
    /// 已访问的路由
    visited_routes: HashSet<String>,
    /// 待访问队列
    pending_routes: VecDeque<String>,
    /// 路由来源记录 (route -> source_route)
    route_sources: HashMap<String, String>,
    /// 忽略的路由模式（支持字符串和正则）
    ignored_patterns: Vec<IgnorePattern>,
    /// 编译后的正则缓存
    #[allow(dead_code)]
    compiled_regexes: Vec<Regex>,
}

impl std::fmt::Debug for RouteTracker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RouteTracker")
            .field("target_domain", &self.target_domain)
            .field("discovered_routes", &self.discovered_routes.len())
            .field("visited_routes", &self.visited_routes.len())
            .field("pending_routes", &self.pending_routes.len())
            .finish()
    }
}

impl RouteTracker {
    /// 创建新的路由追踪器
    pub fn new(target_url: &str) -> Self {
        let target_domain = Self::extract_domain(target_url).unwrap_or_default();
        
        // Default ignore patterns with smart matching
        let ignored_patterns = vec![
            // Exact action paths (use word boundary regex)
            IgnorePattern::Regex(r"(?i)/logout\b".to_string()),
            IgnorePattern::Regex(r"(?i)/sign[-_]?out\b".to_string()),
            IgnorePattern::Regex(r"(?i)/log[-_]?out\b".to_string()),
            IgnorePattern::Regex(r"(?i)/exit\b".to_string()),
            // Destructive actions (only match as action, not as part of path)
            IgnorePattern::Regex(r"(?i)/delete/?\?".to_string()),  // /delete?id=xxx
            IgnorePattern::Regex(r"(?i)/remove/?\?".to_string()),  // /remove?id=xxx
            // Protocol prefixes
            IgnorePattern::Contains("javascript:".to_string()),
            IgnorePattern::Contains("mailto:".to_string()),
            IgnorePattern::Contains("tel:".to_string()),
            IgnorePattern::Contains("data:".to_string()),
            IgnorePattern::Contains("blob:".to_string()),
            // External services
            IgnorePattern::Contains("oauth".to_string()),
            IgnorePattern::Contains("sso/callback".to_string()),
        ];
        
        // Compile regexes
        let compiled_regexes = ignored_patterns.iter()
            .filter_map(|p| {
                if let IgnorePattern::Regex(pattern) = p {
                    match Regex::new(pattern) {
                        Ok(re) => Some(re),
                        Err(e) => {
                            warn!("Failed to compile ignore pattern '{}': {}", pattern, e);
                            None
                        }
                    }
                } else {
                    None
                }
            })
            .collect();
        
        let mut tracker = Self {
            target_domain,
            discovered_routes: HashSet::new(),
            visited_routes: HashSet::new(),
            pending_routes: VecDeque::new(),
            route_sources: HashMap::new(),
            ignored_patterns,
            compiled_regexes,
        };
        
        // 将目标 URL 标记为已发现和已访问
        let normalized = tracker.normalize_route(target_url);
        tracker.discovered_routes.insert(normalized.clone());
        tracker.visited_routes.insert(normalized);
        
        tracker
    }

    /// 从 URL 提取域名
    fn extract_domain(url: &str) -> Option<String> {
        Url::parse(url).ok().and_then(|u| u.host_str().map(|s| s.to_string()))
    }

    /// 规范化路由（移除 hash、query 等）
    fn normalize_route(&self, url: &str) -> String {
        let trimmed = url.trim();

        // Hash-router (SPA) relative routes: "#/xxx" or "#!/xxx"
        if let Some(rest) = trimmed.strip_prefix("#/") {
            return format!("#/{}", rest.split('?').next().unwrap_or(rest));
        }
        if let Some(rest) = trimmed.strip_prefix("#!/") {
            return format!("#!/{}", rest.split('?').next().unwrap_or(rest));
        }

        // 尝试解析为完整 URL
        if let Ok(parsed) = Url::parse(url) {
            // 保留 scheme + host + path
            let mut normalized = format!(
                "{}://{}{}",
                parsed.scheme(),
                parsed.host_str().unwrap_or(""),
                parsed.path()
            );

            // Preserve hash fragment if it looks like a client-side route (hash-router).
            // Example: "https://a.com/#/dashboard" -> "https://a.com/#/dashboard"
            if let Some(frag) = parsed.fragment() {
                if frag.starts_with('/') || frag.starts_with("!/") {
                    normalized.push('#');
                    normalized.push_str(frag);
                }
            }

            normalized
        } else {
            // 可能是相对路径，直接使用
            let s = trimmed.split('?').next().unwrap_or(trimmed);
            // Keep hash-router style fragments, but drop plain anchors
            if s.starts_with("#/") || s.starts_with("#!/") {
                return s.to_string();
            }
            s.split('#').next().unwrap_or(s).to_string()
        }
    }

    /// 检查路由是否应该被忽略
    fn should_ignore(&self, url: &str) -> bool {
        let trimmed = url.trim();

        // Ignore pure anchors but keep hash-router routes like "#/xxx"
        if trimmed == "#" {
            return true;
        }
        if trimmed.starts_with("#/") || trimmed.starts_with("#!/") {
            return false;
        }

        // Check each pattern
        for pattern in &self.ignored_patterns {
            match pattern {
                IgnorePattern::Contains(s) => {
                    if trimmed.to_lowercase().contains(&s.to_lowercase()) {
                        debug!("Route ignored by contains pattern '{}': {}", s, trimmed);
                        return true;
                    }
                }
                IgnorePattern::Regex(pattern_str) => {
                    if let Ok(re) = Regex::new(pattern_str) {
                        if re.is_match(trimmed) {
                            debug!("Route ignored by regex pattern '{}': {}", pattern_str, trimmed);
                            return true;
                        }
                    }
                }
            }
        }
        
        false
    }

    /// 检查是否是同域路由
    fn is_same_domain(&self, url: &str) -> bool {
        if url.starts_with('/') || url.starts_with('#') || url.starts_with('.') {
            return true; // 相对路径
        }
        
        if let Ok(parsed) = Url::parse(url) {
            if let Some(host) = parsed.host_str() {
                return host == self.target_domain || host.ends_with(&format!(".{}", self.target_domain));
            }
        }
        
        false
    }

    /// 添加发现的路由
    pub fn add_discovered_route(&mut self, url: &str, source: &str) -> bool {
        // 检查是否应该忽略
        if self.should_ignore(url) {
            debug!("Ignoring route (matched ignore pattern): {}", url);
            return false;
        }

        // 检查是否同域
        if !self.is_same_domain(url) {
            debug!("Ignoring route (different domain): {}", url);
            return false;
        }

        let normalized = self.normalize_route(url);
        
        // 检查是否已发现
        if self.discovered_routes.contains(&normalized) {
            return false;
        }

        info!("Discovered new route: {} (from: {})", normalized, source);
        self.discovered_routes.insert(normalized.clone());
        self.route_sources.insert(normalized.clone(), source.to_string());
        
        // 如果未访问，加入队列
        if !self.visited_routes.contains(&normalized) {
            self.pending_routes.push_back(normalized);
        }

        true
    }

    /// 批量添加发现的路由
    pub fn add_discovered_routes(&mut self, urls: &[String], source: &str) -> usize {
        let mut count = 0;
        for url in urls {
            if self.add_discovered_route(url, source) {
                count += 1;
            }
        }
        count
    }

    /// 标记路由为已访问
    pub fn mark_visited(&mut self, url: &str) {
        let normalized = self.normalize_route(url);
        self.visited_routes.insert(normalized.clone());
        
        // 从待访问队列中移除
        self.pending_routes.retain(|r| r != &normalized);
        
        // 确保也在发现列表中
        self.discovered_routes.insert(normalized);
    }

    /// 获取下一个待访问路由
    pub fn next_pending(&mut self) -> Option<String> {
        self.pending_routes.pop_front()
    }

    /// 查看下一个待访问路由（不移除）
    pub fn peek_pending(&self) -> Option<&String> {
        self.pending_routes.front()
    }

    /// 获取所有待访问路由
    pub fn get_pending_routes(&self) -> Vec<String> {
        self.pending_routes.iter().cloned().collect()
    }

    /// 获取待访问路由数量
    pub fn pending_count(&self) -> usize {
        self.pending_routes.len()
    }

    /// 计算路由覆盖率
    pub fn coverage_percentage(&self) -> f32 {
        if self.discovered_routes.is_empty() {
            return 100.0;
        }
        (self.visited_routes.len() as f32 / self.discovered_routes.len() as f32) * 100.0
    }

    /// 获取统计信息
    pub fn stats(&self) -> RouteStats {
        RouteStats {
            discovered: self.discovered_routes.len(),
            visited: self.visited_routes.len(),
            pending: self.pending_routes.len(),
            coverage: self.coverage_percentage(),
        }
    }

    /// 获取已发现路由列表
    pub fn discovered_routes(&self) -> &HashSet<String> {
        &self.discovered_routes
    }

    /// 获取已访问路由列表
    pub fn visited_routes(&self) -> &HashSet<String> {
        &self.visited_routes
    }

    /// 检查是否所有路由都已访问
    pub fn is_fully_covered(&self) -> bool {
        self.pending_routes.is_empty() && 
        self.discovered_routes.len() == self.visited_routes.len()
    }

    /// 添加忽略模式 (string contains)
    pub fn add_ignored_pattern(&mut self, pattern: &str) {
        self.ignored_patterns.push(IgnorePattern::Contains(pattern.to_lowercase()));
    }
    
    /// 添加正则忽略模式
    pub fn add_ignored_regex(&mut self, pattern: &str) {
        if Regex::new(pattern).is_ok() {
            self.ignored_patterns.push(IgnorePattern::Regex(pattern.to_string()));
        } else {
            warn!("Invalid regex pattern: {}", pattern);
        }
    }
}

/// 路由统计信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RouteStats {
    pub discovered: usize,
    pub visited: usize,
    pub pending: usize,
    pub coverage: f32,
}

impl Default for RouteStats {
    fn default() -> Self {
        Self {
            discovered: 0,
            visited: 0,
            pending: 0,
            coverage: 100.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_route_tracker_basic() {
        let mut tracker = RouteTracker::new("https://example.com/app");
        
        // 添加同域路由
        assert!(tracker.add_discovered_route("/page1", "/app"));
        assert!(tracker.add_discovered_route("/page2", "/app"));
        
        // 重复添加应该返回 false
        assert!(!tracker.add_discovered_route("/page1", "/app"));
        
        // 检查待访问
        assert_eq!(tracker.pending_count(), 2);
        
        // 标记访问
        tracker.mark_visited("/page1");
        assert_eq!(tracker.pending_count(), 1);
    }

    #[test]
    fn test_route_tracker_ignores() {
        let mut tracker = RouteTracker::new("https://example.com");
        
        // 应该被忽略的路由
        assert!(!tracker.add_discovered_route("/logout", "/"));
        assert!(!tracker.add_discovered_route("javascript:void(0)", "/"));
        assert!(!tracker.add_discovered_route("https://other.com/page", "/"));
    }

    #[test]
    fn test_coverage() {
        let mut tracker = RouteTracker::new("https://example.com");
        
        tracker.add_discovered_route("/page1", "/");
        tracker.add_discovered_route("/page2", "/");
        
        // 只访问了起始页
        assert!(tracker.coverage_percentage() < 100.0);
        
        // 访问所有
        tracker.mark_visited("/page1");
        tracker.mark_visited("/page2");
        assert!(tracker.is_fully_covered());
    }

    #[test]
    fn test_hash_router_routes_are_not_ignored_and_normalized() {
        let mut tracker = RouteTracker::new("https://example.com/#/login");

        // Pure anchor should be ignored
        assert!(!tracker.add_discovered_route("#", "src"));

        // Hash-router routes should be accepted
        assert!(tracker.add_discovered_route("https://example.com/#/dashboard", "src"));
        assert!(tracker.add_discovered_route("#/settings?tab=profile", "src"));

        // Ensure they are tracked as distinct routes
        let stats = tracker.stats();
        assert!(stats.discovered >= 3); // initial + 2 new
        assert!(tracker.discovered_routes().iter().any(|r| r.contains("#/dashboard")));
        assert!(tracker.discovered_routes().iter().any(|r| r.starts_with("#/settings")));
    }
}
