//! VisionExplorer 消息发送器
//!
//! 专门用于 VisionExplorer 架构的流式消息发送
//! 发送 vision_step 格式以与前端对接

use crate::utils::ordered_message::{emit_message_chunk_with_arch, ArchitectureType, ChunkType};
use serde::Serialize;
use std::sync::{Arc, Mutex};
use tauri::AppHandle;
use tracing::debug;

/// VisionExplorer 消息发送器
pub struct VisionExplorerMessageEmitter {
    app_handle: Arc<AppHandle>,
    execution_id: String,
    message_id: String,
    conversation_id: Option<String>,
    /// 当前迭代号
    current_iteration: Arc<Mutex<u32>>, 
    /// 是否在完成时终结消息流
    finalize_on_complete: bool,
    /// 父架构类型（当作为子流运行时，如 Travel）
    /// 设置后消息将使用父架构类型发送，保持消息流顺序
    parent_architecture: Arc<Mutex<Option<ArchitectureType>>>,
}

/// Vision 探索步骤（与前端对齐）
#[derive(Debug, Clone, Serialize)]
pub struct VisionStep {
    /// 迭代号
    pub iteration: u32,
    /// 阶段: screenshot, analyze, action, verify
    pub phase: String,
    /// 状态: running, completed, failed
    pub status: String,
    /// 页面 URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// 页面标题
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// 截图 (base64)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub screenshot: Option<String>,
    /// VLM 分析结果
    #[serde(skip_serializing_if = "Option::is_none")]
    pub analysis: Option<VisionAnalysis>,
    /// 执行的操作
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<VisionAction>,
    /// 错误信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// VLM 分析结果
#[derive(Debug, Clone, Serialize)]
pub struct VisionAnalysis {
    pub page_analysis: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_apis: Option<Vec<String>>,
    pub exploration_progress: f32,
}

/// 执行的操作
#[derive(Debug, Clone, Serialize)]
pub struct VisionAction {
    pub action_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub element_index: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    pub reason: String,
    /// 操作是否成功
    pub success: bool,
    /// 操作耗时 (ms)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
}

/// 探索统计
#[derive(Debug, Clone, Serialize)]
pub struct VisionExplorationStats {
    pub total_iterations: u32,
    pub pages_visited: usize,
    pub apis_discovered: usize,
    pub elements_interacted: usize,
    pub total_duration_ms: u64,
    pub status: String,
}

impl VisionExplorerMessageEmitter {
    pub fn new(
        app_handle: Arc<AppHandle>,
        execution_id: String,
        message_id: String,
        conversation_id: Option<String>,
        finalize_on_complete: bool,
    ) -> Self {
        Self {
            app_handle,
            execution_id,
            message_id,
            conversation_id,
            current_iteration: Arc::new(Mutex::new(1)),
            finalize_on_complete,
            parent_architecture: Arc::new(Mutex::new(None)),
        }
    }

    /// 设置父架构类型（当作为子流运行时）
    /// 设置后，所有消息将使用父架构类型发送，保持与父流的消息顺序一致
    pub fn set_parent_architecture(&self, arch: ArchitectureType) {
        *self.parent_architecture.lock().unwrap() = Some(arch);
    }

    /// 获取用于发送消息的架构类型
    fn get_architecture(&self) -> ArchitectureType {
        self.parent_architecture.lock().unwrap()
            .clone()
            .unwrap_or(ArchitectureType::VisionExplorer)
    }

    /// 获取当前迭代号
    fn get_iteration(&self) -> u32 {
        *self.current_iteration.lock().unwrap()
    }

    /// 设置当前迭代号
    pub fn set_iteration(&self, iteration: u32) {
        *self.current_iteration.lock().unwrap() = iteration;
    }

    /// 发送探索开始信号
    pub fn emit_start(&self, target_url: &str) {
        self.emit_meta("start", serde_json::json!({
            "type": "start",
            "target_url": target_url
        }));
        
        let content = format!("🚀 **开始探索**: {}\n\n", target_url);
        self.emit_content(&content, false);
    }

    /// 发送探索完成信号
    pub fn emit_complete(&self, stats: VisionExplorationStats) {
        let content = format!(
            "\n✅ **探索完成**\n- 迭代次数: {}\n- 访问页面: {}\n- 发现 API: {}\n- 交互元素: {}\n- 总耗时: {}ms\n",
            stats.total_iterations,
            stats.pages_visited,
            stats.apis_discovered,
            stats.elements_interacted,
            stats.total_duration_ms
        );
        self.emit_content(&content, false);

        emit_message_chunk_with_arch(
            &self.app_handle,
            &self.execution_id,
            &self.message_id,
            self.conversation_id.as_deref(),
            ChunkType::StreamComplete,
            "",
            self.finalize_on_complete,
            Some("complete"),
            None,
            Some(self.get_architecture()),
            Some(serde_json::json!({
                "type": "complete",
                "statistics": stats
            })),
        );
    }

    /// 发送截图阶段
    pub fn emit_screenshot(&self, iteration: u32, url: &str, title: &str, screenshot: Option<&str>) {
        self.set_iteration(iteration);
        
        let step = VisionStep {
            iteration,
            phase: "screenshot".to_string(),
            status: "completed".to_string(),
            url: Some(url.to_string()),
            title: Some(title.to_string()),
            screenshot: screenshot.map(|s| s.to_string()),
            analysis: None,
            action: None,
            error: None,
        };
        self.emit_vision_step(&step);

        // 发送简短的文本内容
        let content = format!(
            "\n---\n**迭代 {}** | 📸 截图完成\n- URL: {}\n- 标题: {}\n",
            iteration, url, title
        );
        self.emit_content(&content, false);
    }

    /// 发送 VLM 分析结果
    pub fn emit_analysis(&self, iteration: u32, analysis: VisionAnalysis) {
        self.set_iteration(iteration);
        
        let step = VisionStep {
            iteration,
            phase: "analyze".to_string(),
            status: "completed".to_string(),
            url: None,
            title: None,
            screenshot: None,
            analysis: Some(analysis.clone()),
            action: None,
            error: None,
        };
        self.emit_vision_step(&step);

        // 发送分析内容
        let apis_str = analysis.estimated_apis
            .as_ref()
            .map(|apis| apis.join(", "))
            .unwrap_or_else(|| "无".to_string());
        
        let content = format!(
            "🧠 **分析结果**\n{}\n- 预估 API: {}\n- 进度: {:.0}%\n",
            analysis.page_analysis,
            apis_str,
            analysis.exploration_progress * 100.0
        );
        self.emit_content(&content, false);
    }

    /// 发送操作执行
    pub fn emit_action(&self, iteration: u32, action: VisionAction) {
        self.set_iteration(iteration);
        
        let status = if action.success { "completed" } else { "failed" };
        
        let step = VisionStep {
            iteration,
            phase: "action".to_string(),
            status: status.to_string(),
            url: None,
            title: None,
            screenshot: None,
            analysis: None,
            action: Some(action.clone()),
            error: None,
        };
        self.emit_vision_step(&step);

        // 发送操作内容
        let status_icon = if action.success { "✅" } else { "❌" };
        let element_info = action.element_index
            .map(|idx| format!("[{}]", idx))
            .unwrap_or_default();
        let value_info = action.value
            .as_ref()
            .map(|v| format!(" = \"{}\"", v))
            .unwrap_or_default();
        let duration_info = action.duration_ms
            .map(|d| format!(" ({}ms)", d))
            .unwrap_or_default();
        
        let content = format!(
            "{} **{}** {}{}{}\n- 原因: {}\n",
            status_icon,
            action.action_type,
            element_info,
            value_info,
            duration_info,
            action.reason
        );
        self.emit_content(&content, false);
    }

    /// 发送错误信息
    pub fn emit_error(&self, iteration: u32, error: &str) {
        self.set_iteration(iteration);
        
        let step = VisionStep {
            iteration,
            phase: "action".to_string(),
            status: "failed".to_string(),
            url: None,
            title: None,
            screenshot: None,
            analysis: None,
            action: None,
            error: Some(error.to_string()),
        };
        self.emit_vision_step(&step);

        let content = format!("\n❌ **错误**: {}\n", error);
        self.emit_content(&content, false);
    }

    /// 发送 API 发现
    pub fn emit_api_discovered(&self, api: &str, method: &str) {
        let content = format!("🔍 **发现 API**: {} {}\n", method, api);
        self.emit_content(&content, false);
        
        self.emit_meta("api_discovered", serde_json::json!({
            "type": "api_discovered",
            "api": api,
            "method": method
        }));
    }

    /// 发送流式内容
    pub fn emit_content(&self, content: &str, is_final: bool) {
        emit_message_chunk_with_arch(
            &self.app_handle,
            &self.execution_id,
            &self.message_id,
            self.conversation_id.as_deref(),
            ChunkType::Content,
            content,
            is_final,
            None,
            None,
            Some(self.get_architecture()),
            None,
        );
    }

    /// 发送 Vision 步骤数据
    fn emit_vision_step(&self, step: &VisionStep) {
        let meta_data = serde_json::json!({
            "type": "vision_step",
            "step": step,
        });

        emit_message_chunk_with_arch(
            &self.app_handle,
            &self.execution_id,
            &self.message_id,
            self.conversation_id.as_deref(),
            ChunkType::Meta,
            "",
            false,
            Some(&step.phase),
            None,
            Some(self.get_architecture()),
            Some(meta_data),
        );

        debug!(
            "Vision step emitted: iteration={}, phase={}, status={}, arch={:?}",
            step.iteration, step.phase, step.status, self.get_architecture()
        );
    }

    /// 发送元数据
    fn emit_meta(&self, stage: &str, data: serde_json::Value) {
        emit_message_chunk_with_arch(
            &self.app_handle,
            &self.execution_id,
            &self.message_id,
            self.conversation_id.as_deref(),
            ChunkType::Meta,
            "",
            false,
            Some(stage),
            None,
            Some(self.get_architecture()),
            Some(data),
        );
    }
}
