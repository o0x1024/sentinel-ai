//! 覆盖率引擎
//!
//! 计算和跟踪多维度探索覆盖率，判断探索是否完成

use super::route_tracker::RouteStats;
use super::element_manager::ElementStats;
use tracing::info;

/// 覆盖率目标阈值（95%）
pub const COVERAGE_TARGET: f32 = 95.0;

/// 稳定性阈值（连续无新发现轮次）
pub const STABILITY_THRESHOLD: u32 = 5;

/// 覆盖率引擎
#[derive(Debug, Clone)]
pub struct CoverageEngine {
    /// 路由覆盖率
    pub route_coverage: f32,
    /// 元素覆盖率  
    pub element_coverage: f32,
    /// API 发现数量
    pub api_count: usize,
    /// 组件覆盖率
    pub component_coverage: f32,
    /// 连续无新发现轮次
    pub consecutive_no_discovery: u32,
    /// 稳定性阈值
    pub stability_threshold: u32,
    /// 覆盖率目标
    pub coverage_target: f32,
    /// 上一轮的发现计数
    last_discovery_counts: DiscoveryCounts,
}

/// 发现计数（用于检测是否有新发现）
#[derive(Debug, Clone, Default)]
struct DiscoveryCounts {
    routes: usize,
    elements: usize,
    apis: usize,
    components: usize,
}

impl CoverageEngine {
    /// 创建新的覆盖率引擎
    pub fn new() -> Self {
        Self {
            route_coverage: 0.0,
            element_coverage: 0.0,
            api_count: 0,
            component_coverage: 100.0,
            consecutive_no_discovery: 0,
            stability_threshold: STABILITY_THRESHOLD,
            coverage_target: COVERAGE_TARGET,
            last_discovery_counts: DiscoveryCounts::default(),
        }
    }

    /// 更新覆盖率数据
    pub fn update(
        &mut self,
        route_stats: &RouteStats,
        element_stats: &ElementStats,
        api_count: usize,
        component_coverage: f32,
    ) {
        self.route_coverage = route_stats.coverage;
        self.element_coverage = element_stats.coverage;
        self.api_count = api_count;
        self.component_coverage = component_coverage;

        // 检查是否有新发现
        let current_counts = DiscoveryCounts {
            routes: route_stats.discovered,
            elements: element_stats.total,
            apis: api_count,
            components: 0, // 组件计数暂不使用
        };

        let has_new_discovery = 
            current_counts.routes > self.last_discovery_counts.routes ||
            current_counts.elements > self.last_discovery_counts.elements ||
            current_counts.apis > self.last_discovery_counts.apis;

        if has_new_discovery {
            self.consecutive_no_discovery = 0;
            info!(
                "New discoveries: routes +{}, elements +{}, apis +{}",
                current_counts.routes.saturating_sub(self.last_discovery_counts.routes),
                current_counts.elements.saturating_sub(self.last_discovery_counts.elements),
                current_counts.apis.saturating_sub(self.last_discovery_counts.apis)
            );
        } else {
            self.consecutive_no_discovery += 1;
            info!(
                "No new discoveries, stable rounds: {}/{}",
                self.consecutive_no_discovery,
                self.stability_threshold
            );
        }

        self.last_discovery_counts = current_counts;
    }

    /// 计算综合覆盖率
    pub fn overall_coverage(&self) -> f32 {
        // 加权平均：路由 30%，元素 50%，组件 20%
        self.route_coverage * 0.3 + 
        self.element_coverage * 0.5 + 
        self.component_coverage * 0.2
    }

    /// 检查是否达到稳定完成状态
    pub fn is_stable_complete(&self) -> bool {
        self.consecutive_no_discovery >= self.stability_threshold
    }

    /// 检查是否满足完成条件
    pub fn is_completion_ready(&self, pending_routes: usize) -> bool {
        // 条件1: 待访问路由为空
        let routes_done = pending_routes == 0;
        
        // 条件2: 元素覆盖率达标
        let elements_done = self.element_coverage >= self.coverage_target;
        
        // 条件3: 稳定性确认
        let stable = self.is_stable_complete();

        routes_done && elements_done && stable
    }

    /// 生成覆盖率报告
    pub fn generate_report(&self, route_stats: &RouteStats, element_stats: &ElementStats) -> CoverageReport {
        CoverageReport {
            route_coverage: self.route_coverage,
            element_coverage: self.element_coverage,
            component_coverage: self.component_coverage,
            overall_coverage: self.overall_coverage(),
            api_count: self.api_count,
            routes_discovered: route_stats.discovered,
            routes_visited: route_stats.visited,
            routes_pending: route_stats.pending,
            elements_total: element_stats.total,
            elements_interacted: element_stats.interacted,
            hover_candidates: element_stats.hover_candidates,
            consecutive_no_discovery: self.consecutive_no_discovery,
            stability_threshold: self.stability_threshold,
            is_stable_complete: self.is_stable_complete(),
            coverage_target: self.coverage_target,
        }
    }

    /// 获取完成状态检查结果
    pub fn completion_check(&self, pending_routes: usize) -> CompletionCheck {
        CompletionCheck {
            routes_done: pending_routes == 0,
            elements_done: self.element_coverage >= self.coverage_target,
            stable: self.is_stable_complete(),
            pending_routes,
            element_coverage: self.element_coverage,
            stable_rounds: self.consecutive_no_discovery,
            stability_threshold: self.stability_threshold,
            coverage_target: self.coverage_target,
        }
    }

    /// 重置稳定性计数（当有重大状态变化时调用）
    pub fn reset_stability(&mut self) {
        self.consecutive_no_discovery = 0;
    }
}

impl Default for CoverageEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// 覆盖率报告
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CoverageReport {
    /// 路由覆盖率
    pub route_coverage: f32,
    /// 元素覆盖率
    pub element_coverage: f32,
    /// 组件覆盖率
    pub component_coverage: f32,
    /// 综合覆盖率
    pub overall_coverage: f32,
    /// API 发现数量
    pub api_count: usize,
    /// 发现的路由数
    pub routes_discovered: usize,
    /// 访问的路由数
    pub routes_visited: usize,
    /// 待访问路由数
    pub routes_pending: usize,
    /// 总元素数
    pub elements_total: usize,
    /// 已交互元素数
    pub elements_interacted: usize,
    /// 悬停候选数
    pub hover_candidates: usize,
    /// 连续无新发现轮次
    pub consecutive_no_discovery: u32,
    /// 稳定性阈值
    pub stability_threshold: u32,
    /// 是否稳定完成
    pub is_stable_complete: bool,
    /// 覆盖率目标
    pub coverage_target: f32,
}

/// 完成状态检查
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CompletionCheck {
    /// 路由是否完成
    pub routes_done: bool,
    /// 元素是否达标
    pub elements_done: bool,
    /// 是否稳定
    pub stable: bool,
    /// 待访问路由数
    pub pending_routes: usize,
    /// 当前元素覆盖率
    pub element_coverage: f32,
    /// 稳定轮次
    pub stable_rounds: u32,
    /// 稳定阈值
    pub stability_threshold: u32,
    /// 覆盖率目标
    pub coverage_target: f32,
}

impl CompletionCheck {
    /// 是否可以完成
    pub fn can_complete(&self) -> bool {
        self.routes_done && self.elements_done && self.stable
    }

    /// 生成指导信息
    pub fn guidance(&self) -> String {
        let mut issues = Vec::new();
        
        if !self.routes_done {
            issues.push(format!("还有 {} 个路由待访问", self.pending_routes));
        }
        if !self.elements_done {
            issues.push(format!(
                "元素覆盖率 {:.1}% 未达到目标 {:.0}%",
                self.element_coverage, self.coverage_target
            ));
        }
        if !self.stable {
            issues.push(format!(
                "稳定轮次 {}/{} 未达标",
                self.stable_rounds, self.stability_threshold
            ));
        }

        if issues.is_empty() {
            "✅ 所有完成条件已满足，可以设置 completed 状态".to_string()
        } else {
            format!("⚠️ 未满足完成条件:\n- {}", issues.join("\n- "))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coverage_engine_basic() {
        let mut engine = CoverageEngine::new();
        
        let route_stats = RouteStats {
            discovered: 10,
            visited: 10,
            pending: 0,
            coverage: 100.0,
        };
        
        let element_stats = ElementStats {
            total: 50,
            interacted: 48,
            hover_candidates: 5,
            coverage: 96.0,
        };
        
        engine.update(&route_stats, &element_stats, 15, 100.0);
        
        assert!(engine.overall_coverage() > 95.0);
    }

    #[test]
    fn test_stability_detection() {
        let mut engine = CoverageEngine::new();
        
        let route_stats = RouteStats::default();
        let element_stats = ElementStats::default();
        
        // 模拟 5 轮无新发现
        for _ in 0..5 {
            engine.update(&route_stats, &element_stats, 0, 100.0);
        }
        
        assert!(engine.is_stable_complete());
    }

    #[test]
    fn test_completion_check() {
        let mut engine = CoverageEngine::new();
        engine.element_coverage = 96.0;
        engine.consecutive_no_discovery = 5;
        
        let check = engine.completion_check(0);
        assert!(check.can_complete());
        
        let check2 = engine.completion_check(3);
        assert!(!check2.can_complete());
    }
}
