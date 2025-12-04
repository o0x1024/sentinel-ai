//! 探索状态管理
//!
//! 管理视觉探索过程中的状态跟踪、进度评估和API发现记录

use super::types::*;
use chrono::Utc;
use std::collections::HashSet;
use tracing::{debug, info};

/// 探索状态管理器
pub struct StateManager {
    state: ExplorationState,
}

impl StateManager {
    /// 创建新的状态管理器
    pub fn new(target_url: String, max_iterations: u32) -> Self {
        let mut state = ExplorationState::default();
        state.target_url = target_url;
        state.max_iterations = max_iterations;
        
        Self { state }
    }

    /// 获取当前状态的引用
    pub fn state(&self) -> &ExplorationState {
        &self.state
    }

    /// 获取当前状态的可变引用
    pub fn state_mut(&mut self) -> &mut ExplorationState {
        &mut self.state
    }

    /// 更新当前页面状态
    pub fn update_page_state(&mut self, page_state: PageState) {
        // 记录已访问URL
        self.state.visited_urls.insert(page_state.url.clone());
        
        // 更新当前页面
        self.state.current_page = Some(page_state);
    }

    /// 记录操作
    pub fn record_action(&mut self, action: BrowserAction, result: ActionResult, triggered_apis: Vec<String>) {
        let record = ActionRecord {
            id: uuid::Uuid::new_v4().to_string(),
            action,
            target_element: None,
            executed_at: Utc::now(),
            result,
            triggered_apis,
        };
        
        self.state.action_history.push(record);
        self.state.iteration_count += 1;
    }

    /// 标记元素已交互
    pub fn mark_element_interacted(&mut self, element_id: &str) {
        self.state.interacted_elements.insert(element_id.to_string());
    }

    /// 检查元素是否已交互
    pub fn is_element_interacted(&self, element_id: &str) -> bool {
        self.state.interacted_elements.contains(element_id)
    }

    /// 添加发现的API
    pub fn add_discovered_api(&mut self, api: ApiEndpoint) {
        // 检查是否已存在相同的API
        let exists = self.state.discovered_apis.iter().any(|existing| {
            existing.method == api.method && existing.path == api.path
        });
        
        if !exists {
            info!("Discovered new API: {} {}", api.method, api.path);
            self.state.discovered_apis.push(api);
        }
    }

    /// 批量添加发现的API
    pub fn add_discovered_apis(&mut self, apis: Vec<ApiEndpoint>) {
        for api in apis {
            self.add_discovered_api(api);
        }
    }

    /// 添加发现的表单
    pub fn add_discovered_form(&mut self, form: FormInfo) {
        let exists = self.state.discovered_forms.iter().any(|f| f.id == form.id);
        if !exists {
            self.state.discovered_forms.push(form);
        }
    }

    /// 更新探索状态
    pub fn update_status(&mut self, status: ExplorationStatus) {
        self.state.status = status;
    }

    /// 设置错误信息
    pub fn set_error(&mut self, error: String) {
        self.state.error_message = Some(error);
        self.state.status = ExplorationStatus::Failed;
    }

    /// 计算探索进度
    pub fn calculate_progress(&mut self) -> f32 {
        let progress = if let Some(page) = &self.state.current_page {
            let total_elements = page.interactable_elements.len() as f32;
            let interacted = self.state.interacted_elements.len() as f32;
            
            if total_elements > 0.0 {
                (interacted / total_elements * 100.0).min(100.0)
            } else {
                // 基于迭代次数估算
                (self.state.iteration_count as f32 / self.state.max_iterations as f32 * 100.0).min(100.0)
            }
        } else {
            0.0
        };
        
        self.state.exploration_progress = progress;
        progress
    }

    /// 检查是否应该继续探索
    pub fn should_continue(&self) -> bool {
        // 检查状态
        if matches!(self.state.status, 
            ExplorationStatus::Completed | 
            ExplorationStatus::Failed | 
            ExplorationStatus::WaitingForInput
        ) {
            return false;
        }
        
        // 检查迭代次数
        if self.state.iteration_count >= self.state.max_iterations {
            debug!("Reached max iterations: {}", self.state.max_iterations);
            return false;
        }
        
        true
    }

    /// 获取未探索的元素
    pub fn get_unexplored_elements(&self) -> Vec<&PageElement> {
        if let Some(page) = &self.state.current_page {
            page.interactable_elements.iter()
                .filter(|e| !self.state.interacted_elements.contains(&e.id))
                .collect()
        } else {
            Vec::new()
        }
    }

    /// 获取最近的操作历史（用于提示词）
    pub fn get_recent_actions(&self, count: usize) -> Vec<&ActionRecord> {
        let len = self.state.action_history.len();
        let start = if len > count { len - count } else { 0 };
        self.state.action_history[start..].iter().collect()
    }

    /// 格式化操作历史（用于提示词）
    pub fn format_action_history(&self, count: usize) -> String {
        let recent = self.get_recent_actions(count);
        if recent.is_empty() {
            return "No actions yet".to_string();
        }
        
        recent.iter().enumerate().map(|(i, record)| {
            let action_desc = match &record.action {
                BrowserAction::Screenshot => "Screenshot".to_string(),
                BrowserAction::ClickMouse { coordinates, .. } => {
                    if let Some(coords) = coordinates {
                        format!("Click at ({}, {})", coords.x, coords.y)
                    } else {
                        "Click".to_string()
                    }
                }
                BrowserAction::ClickByIndex { index } => format!("Click element [{}]", index),
                BrowserAction::FillByIndex { index, value } => format!("Fill [{}]: '{}'", index, value.chars().take(20).collect::<String>()),
                BrowserAction::Navigate { url } => format!("Navigate to: {}", url),
                BrowserAction::Scroll { direction, .. } => format!("Scroll {:?}", direction),
                _ => format!("{:?}", record.action),
            };
            
            let result_status = if record.result.success { "✓" } else { "✗" };
            format!("{}. {} {}", i + 1, result_status, action_desc)
        }).collect::<Vec<_>>().join("\n")
    }

    /// 获取统计摘要
    pub fn get_summary(&self) -> ExplorationSummary {
        ExplorationSummary {
            session_id: self.state.session_id.clone(),
            target_url: self.state.target_url.clone(),
            status: self.state.status.clone(),
            total_iterations: self.state.iteration_count,
            pages_visited: self.state.visited_urls.len(),
            elements_interacted: self.state.interacted_elements.len(),
            apis_discovered: self.state.discovered_apis.len(),
            forms_discovered: self.state.discovered_forms.len(),
            exploration_progress: self.state.exploration_progress,
            duration_seconds: (Utc::now() - self.state.start_time).num_seconds() as u64,
        }
    }

    /// 标记探索完成
    pub fn mark_completed(&mut self, reason: &str) {
        self.state.status = ExplorationStatus::Completed;
        info!("Exploration completed: {}", reason);
    }

    /// 标记需要帮助
    pub fn mark_needs_help(&mut self, reason: &str) {
        self.state.status = ExplorationStatus::WaitingForInput;
        self.state.error_message = Some(reason.to_string());
        info!("Exploration needs help: {}", reason);
    }
}

/// 探索摘要
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExplorationSummary {
    pub session_id: String,
    pub target_url: String,
    pub status: ExplorationStatus,
    pub total_iterations: u32,
    pub pages_visited: usize,
    pub elements_interacted: usize,
    pub apis_discovered: usize,
    pub forms_discovered: usize,
    pub exploration_progress: f32,
    pub duration_seconds: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_manager_creation() {
        let manager = StateManager::new("https://example.com".to_string(), 100);
        assert_eq!(manager.state().target_url, "https://example.com");
        assert_eq!(manager.state().max_iterations, 100);
    }

    #[test]
    fn test_mark_element_interacted() {
        let mut manager = StateManager::new("https://example.com".to_string(), 100);
        
        assert!(!manager.is_element_interacted("btn1"));
        manager.mark_element_interacted("btn1");
        assert!(manager.is_element_interacted("btn1"));
    }

    #[test]
    fn test_should_continue() {
        let mut manager = StateManager::new("https://example.com".to_string(), 5);
        
        assert!(manager.should_continue());
        
        // 模拟达到最大迭代
        manager.state_mut().iteration_count = 5;
        assert!(!manager.should_continue());
    }
}

