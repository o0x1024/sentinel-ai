//! 视觉探索引擎核心实现
//!
//! 实现VLM驱动的迭代式网站探索循环

use super::state::{StateManager, ExplorationSummary};
use super::tools::BrowserTools;
use super::types::*;
use crate::engines::llm_client::{LlmClient, LlmConfig};
use crate::services::mcp::McpService;
use anyhow::{anyhow, Result};
use chrono::Utc;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// 系统提示词模板
const SYSTEM_PROMPT_TEMPLATE: &str = include_str!("prompt.md");

/// 视觉探索引擎
pub struct VisionExplorer {
    config: VisionExplorerConfig,
    browser_tools: BrowserTools,
    state_manager: Arc<RwLock<StateManager>>,
    llm_config: LlmConfig,
    is_running: Arc<RwLock<bool>>,
}

impl VisionExplorer {
    /// 创建新的视觉探索引擎
    pub fn new(
        config: VisionExplorerConfig,
        mcp_service: Arc<McpService>,
        llm_config: LlmConfig,
    ) -> Self {
        let browser_tools = BrowserTools::new(mcp_service, config.clone());
        let state_manager = Arc::new(RwLock::new(StateManager::new(
            config.target_url.clone(),
            config.max_iterations,
        )));

        Self {
            config,
            browser_tools,
            state_manager,
            llm_config,
            is_running: Arc::new(RwLock::new(false)),
        }
    }

    /// 使用AI服务配置创建
    pub fn with_ai_config(
        config: VisionExplorerConfig,
        mcp_service: Arc<McpService>,
        provider: String,
        model: String,
    ) -> Self {
        let llm_config = LlmConfig {
            provider,
            model,
            api_key: None,
            base_url: None,
            timeout_secs: 120,
        };
        Self::new(config, mcp_service, llm_config)
    }

    /// 开始探索
    pub async fn start(&self) -> Result<ExplorationSummary> {
        // 检查是否已在运行
        {
            let mut is_running = self.is_running.write().await;
            if *is_running {
                return Err(anyhow!("Explorer is already running"));
            }
            *is_running = true;
        }

        info!("Starting vision exploration for: {}", self.config.target_url);

        // 更新状态
        {
            let mut state = self.state_manager.write().await;
            state.update_status(ExplorationStatus::Exploring);
        }

        // 执行探索循环
        let result = self.exploration_loop().await;

        // 标记停止
        {
            let mut is_running = self.is_running.write().await;
            *is_running = false;
        }

        // 返回摘要
        let state = self.state_manager.read().await;
        let summary = state.get_summary();

        match result {
            Ok(_) => Ok(summary),
            Err(e) => {
                error!("Exploration failed: {}", e);
                Ok(summary)
            }
        }
    }

    /// 停止探索
    pub async fn stop(&self) {
        let mut is_running = self.is_running.write().await;
        *is_running = false;
        
        let mut state = self.state_manager.write().await;
        state.update_status(ExplorationStatus::Paused);
        
        info!("Vision exploration stopped");
    }

    /// 获取当前状态
    pub async fn get_state(&self) -> ExplorationState {
        let state = self.state_manager.read().await;
        state.state().clone()
    }

    /// 获取探索摘要
    pub async fn get_summary(&self) -> ExplorationSummary {
        let state = self.state_manager.read().await;
        state.get_summary()
    }

    /// 主探索循环
    async fn exploration_loop(&self) -> Result<()> {
        // 第1步：导航到目标URL
        info!("Step 1: Navigating to target URL");
        let navigate_action = BrowserAction::Navigate {
            url: self.config.target_url.clone(),
        };
        let result = self.browser_tools.execute_action(&navigate_action).await?;
        
        {
            let mut state = self.state_manager.write().await;
            state.record_action(navigate_action, result, vec![]);
        }

        // 第2步：初始截图
        info!("Step 2: Taking initial screenshot");
        let page_state = self.browser_tools.capture_page_state().await?;
        
        {
            let mut state = self.state_manager.write().await;
            state.update_page_state(page_state);
        }

        // 第3步：迭代探索循环
        info!("Step 3: Starting exploration loop");
        loop {
            // 检查是否应该继续
            let should_continue = {
                let state = self.state_manager.read().await;
                state.should_continue()
            };

            if !should_continue {
                break;
            }

            // 检查是否被停止
            {
                let is_running = self.is_running.read().await;
                if !*is_running {
                    break;
                }
            }

            // 执行一次迭代
            match self.run_iteration().await {
                Ok(should_stop) => {
                    if should_stop {
                        info!("Exploration completed by VLM decision");
                        break;
                    }
                }
                Err(e) => {
                    error!("Iteration failed: {}", e);
                    // 继续尝试，但记录错误
                    let mut state = self.state_manager.write().await;
                    if state.state().iteration_count > 3 {
                        // 连续失败太多次，停止
                        state.set_error(e.to_string());
                        break;
                    }
                }
            }
        }

        Ok(())
    }

    /// 执行单次迭代
    async fn run_iteration(&self) -> Result<bool> {
        // 1. 获取当前页面状态
        let page_state = self.browser_tools.capture_page_state().await?;
        
        // 2. 更新状态
        {
            let mut state = self.state_manager.write().await;
            state.update_page_state(page_state.clone());
        }

        // 3. 构建VLM提示词
        let prompt = self.build_vlm_prompt(&page_state).await?;

        // 4. 调用VLM获取下一步操作
        let vlm_response = self.call_vlm(&prompt, page_state.screenshot.as_deref()).await?;

        // 5. 解析VLM响应
        let analysis = self.parse_vlm_response(&vlm_response)?;

        // 6. 检查是否完成
        if analysis.is_exploration_complete {
            let mut state = self.state_manager.write().await;
            state.mark_completed(analysis.completion_reason.as_deref().unwrap_or("VLM decided exploration is complete"));
            return Ok(true);
        }

        // 7. 执行下一步操作
        let action = self.build_action_from_analysis(&analysis)?;
        let result = self.browser_tools.execute_action(&action).await?;

        // 8. 记录操作
        {
            let mut state = self.state_manager.write().await;
            state.record_action(action, result, analysis.estimated_apis.clone());
            
            // 标记元素已交互
            if let Some(element_id) = &analysis.next_action.element_id {
                state.mark_element_interacted(element_id);
            }
            
            // 更新进度
            state.calculate_progress();
        }

        // 检查是否需要帮助
        if analysis.next_action.action_type == "needs_help" {
            let mut state = self.state_manager.write().await;
            state.mark_needs_help(&analysis.next_action.reason);
            return Ok(true);
        }

        Ok(false)
    }

    /// 构建VLM提示词
    async fn build_vlm_prompt(&self, page_state: &PageState) -> Result<String> {
        let state = self.state_manager.read().await;
        
        // 格式化可交互元素
        let elements_json = serde_json::to_string_pretty(&page_state.interactable_elements)?;
        
        // 格式化操作历史
        let action_history = state.format_action_history(5);
        
        // 统计信息
        let visited_count = state.state().visited_urls.len();
        let api_count = state.state().discovered_apis.len();
        let interacted_count = state.state().interacted_elements.len();
        let unexplored_count = state.get_unexplored_elements().len();
        
        // 替换模板变量
        let prompt = SYSTEM_PROMPT_TEMPLATE
            .replace("{viewport_width}", &self.config.viewport_width.to_string())
            .replace("{viewport_height}", &self.config.viewport_height.to_string())
            .replace("{current_date}", &Utc::now().format("%Y-%m-%d").to_string())
            .replace("{current_time}", &Utc::now().format("%H:%M:%S").to_string())
            .replace("{target_url}", &self.config.target_url)
            .replace("{visited_count}", &visited_count.to_string())
            .replace("{api_count}", &api_count.to_string())
            .replace("{interacted_count}", &interacted_count.to_string())
            .replace("{unexplored_count}", &unexplored_count.to_string())
            .replace("{action_history}", &action_history);
        
        // 添加当前页面信息
        let user_message = format!(
            "Current page: {} ({})\n\nInteractable elements ({}):\n{}\n\nWhat should I do next?",
            page_state.url,
            page_state.title,
            page_state.interactable_elements.len(),
            elements_json
        );
        
        Ok(format!("{}\n\n---\n\n{}", prompt, user_message))
    }

    /// 调用VLM
    async fn call_vlm(&self, prompt: &str, _screenshot_base64: Option<&str>) -> Result<String> {
        // 使用LlmClient调用VLM
        // 注意：目前使用纯文本调用，截图信息已包含在prompt中作为描述
        // TODO: 未来可以支持真正的多模态调用
        
        let llm_client = LlmClient::new(self.llm_config.clone());
        
        // 调用LLM
        let response = llm_client
            .completion(Some(SYSTEM_PROMPT_TEMPLATE), prompt)
            .await?;

        Ok(response)
    }

    /// 解析VLM响应
    fn parse_vlm_response(&self, response: &str) -> Result<VlmAnalysisResult> {
        // 尝试提取JSON
        let json_str = self.extract_json_from_response(response)?;
        
        // 解析JSON
        let parsed: Value = serde_json::from_str(&json_str)?;
        
        // 提取字段
        let page_analysis = parsed.get("page_analysis")
            .and_then(|v| v.as_str())
            .unwrap_or("No analysis provided")
            .to_string();
        
        let next_action = parsed.get("next_action")
            .map(|v| VlmNextAction {
                action_type: v.get("type")
                    .or_else(|| v.get("action_type"))
                    .and_then(|t| t.as_str())
                    .unwrap_or("screenshot")
                    .to_string(),
                element_id: v.get("element_id")
                    .or_else(|| v.get("selector"))
                    .and_then(|e| e.as_str())
                    .map(String::from),
                value: v.get("value")
                    .and_then(|v| v.as_str())
                    .map(String::from),
                reason: v.get("reason")
                    .and_then(|r| r.as_str())
                    .unwrap_or("No reason provided")
                    .to_string(),
            })
            .unwrap_or(VlmNextAction {
                action_type: "screenshot".to_string(),
                element_id: None,
                value: None,
                reason: "Default action".to_string(),
            });
        
        let estimated_apis: Vec<String> = parsed.get("estimated_apis")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();
        
        let exploration_progress = parsed.get("exploration_progress")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as f32;
        
        let is_exploration_complete = parsed.get("is_exploration_complete")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
            || next_action.action_type == "completed"
            || next_action.action_type == "done";
        
        let completion_reason = parsed.get("completion_reason")
            .and_then(|v| v.as_str())
            .map(String::from);
        
        Ok(VlmAnalysisResult {
            page_analysis,
            next_action,
            estimated_apis,
            exploration_progress,
            is_exploration_complete,
            completion_reason,
        })
    }

    /// 从响应中提取JSON
    fn extract_json_from_response(&self, response: &str) -> Result<String> {
        // 尝试找到JSON块
        if let Some(start) = response.find('{') {
            if let Some(end) = response.rfind('}') {
                if end > start {
                    return Ok(response[start..=end].to_string());
                }
            }
        }
        
        // 尝试找到代码块中的JSON
        if let Some(start) = response.find("```json") {
            let json_start = start + 7;
            if let Some(end) = response[json_start..].find("```") {
                return Ok(response[json_start..json_start + end].trim().to_string());
            }
        }
        
        // 尝试找到普通代码块
        if let Some(start) = response.find("```") {
            let code_start = response[start + 3..].find('\n').map(|i| start + 4 + i).unwrap_or(start + 3);
            if let Some(end) = response[code_start..].find("```") {
                return Ok(response[code_start..code_start + end].trim().to_string());
            }
        }
        
        Err(anyhow!("No JSON found in response"))
    }

    /// 根据分析结果构建浏览器操作
    fn build_action_from_analysis(&self, analysis: &VlmAnalysisResult) -> Result<BrowserAction> {
        let action = &analysis.next_action;
        
        match action.action_type.as_str() {
            "screenshot" => Ok(BrowserAction::Screenshot),
            
            "click" | "click_mouse" | "computer_click_mouse" => {
                if let Some(element_id) = &action.element_id {
                    // 尝试解析坐标
                    if element_id.contains(',') {
                        let parts: Vec<&str> = element_id.split(',').collect();
                        if parts.len() == 2 {
                            let x: i32 = parts[0].trim().parse().unwrap_or(0);
                            let y: i32 = parts[1].trim().parse().unwrap_or(0);
                            return Ok(BrowserAction::ClickMouse {
                                coordinates: Some(Coordinates { x, y }),
                                button: MouseButton::Left,
                                click_count: 1,
                            });
                        }
                    }
                    // 使用选择器
                    Ok(BrowserAction::ClickElement {
                        selector: element_id.clone(),
                    })
                } else {
                    // 默认点击当前位置
                    Ok(BrowserAction::ClickMouse {
                        coordinates: None,
                        button: MouseButton::Left,
                        click_count: 1,
                    })
                }
            }
            
            "type" | "type_text" | "computer_type_text" | "fill" => {
                let text = action.value.clone().unwrap_or_default();
                if let Some(selector) = &action.element_id {
                    Ok(BrowserAction::FillInput {
                        selector: selector.clone(),
                        value: text,
                    })
                } else {
                    Ok(BrowserAction::TypeText { text })
                }
            }
            
            "scroll" | "computer_scroll" => {
                let direction = action.value.as_deref()
                    .map(|v| match v.to_lowercase().as_str() {
                        "up" => ScrollDirection::Up,
                        "left" => ScrollDirection::Left,
                        "right" => ScrollDirection::Right,
                        _ => ScrollDirection::Down,
                    })
                    .unwrap_or(ScrollDirection::Down);
                
                Ok(BrowserAction::Scroll {
                    coordinates: None,
                    direction,
                    scroll_count: 3,
                })
            }
            
            "navigate" | "computer_navigate" => {
                let url = action.value.clone().unwrap_or(self.config.target_url.clone());
                Ok(BrowserAction::Navigate { url })
            }
            
            "wait" | "computer_wait" => {
                let duration_ms = action.value.as_ref()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(500);
                Ok(BrowserAction::Wait { duration_ms })
            }
            
            "keys" | "type_keys" | "computer_type_keys" => {
                let keys = action.value.as_ref()
                    .map(|v| v.split(',').map(|s| s.trim().to_string()).collect())
                    .unwrap_or_else(|| vec!["Enter".to_string()]);
                Ok(BrowserAction::TypeKeys { keys })
            }
            
            "completed" | "done" | "set_exploration_status" => {
                // 这些不是浏览器操作，返回截图作为默认
                Ok(BrowserAction::Screenshot)
            }
            
            "needs_help" => {
                // 标记需要帮助
                Ok(BrowserAction::Screenshot)
            }
            
            _ => {
                warn!("Unknown action type: {}, defaulting to screenshot", action.action_type);
                Ok(BrowserAction::Screenshot)
            }
        }
    }
}

// 测试模块暂时禁用，需要mock依赖
// #[cfg(test)]
// mod tests {
//     use super::*;
// }

