//! Progressive Executor - 真正的渐进式披露执行器
//! 
//! 实现 "Discovery -> Scoping -> Activation -> Execution" 的状态机循环。
//! 允许 Agent 在对话过程中动态请求并加载工具 Schema，极大降低 Token 消耗。

use anyhow::Result;
use rig::completion::ToolDefinition;
use rig::tool::{Tool, ToolError};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tauri::AppHandle;
use tokio::sync::Mutex;

use crate::agents::tool_router::ToolRouter;
use sentinel_tools::DynamicTool;

/// 激活工具的参数
#[derive(Deserialize)]
pub struct ActivateToolArgs {
    /// The name of the tool to activate
    pub tool_name: String,
}

/// 元工具：用于请求激活某个具体工具
#[derive(Clone)]
pub struct ActivateTool {
    tool_router: Arc<ToolRouter>,
}

impl ActivateTool {
    pub fn new(tool_router: Arc<ToolRouter>) -> Self {
        Self { tool_router }
    }
}

#[async_trait::async_trait]
impl Tool for ActivateTool {
    const NAME: &'static str = "activate_tool";

    type Error = ToolError;
    type Args = ActivateToolArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Activate a tool to make it available for use. You MUST activate a tool before you can use it.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "tool_name": {
                        "type": "string",
                        "description": "The name of the tool to activate (e.g., 'ping', 'http_request')"
                    }
                },
                "required": ["tool_name"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // 这个工具的主要作用是作为一个"信号"，告诉执行器需要加载新工具。
        // 实际的加载逻辑在 Executor 循环中处理。
        // 这里主要做校验。
        
        let all_tools = self.tool_router.get_all_available_tools();
        let tool_exists = all_tools.iter().any(|t| {
            t.name.eq_ignore_ascii_case(&args.tool_name) || t.id == args.tool_name
        });

        if tool_exists {
            Ok(format!("Tool '{}' activated successfully. You can now use it in the next step.", args.tool_name))
        } else {
            Err(ToolError::ToolCallError(
                format!("Tool '{}' not found in the current ability group.", args.tool_name).into()
            ))
        }
    }
}

/// 渐进式执行器状态
#[derive(Debug, Clone)]
pub enum ExecutorState {
    /// 初始状态，选择能力组
    Discovery,
    /// 能力组已选，工具未激活（只看到名称）
    Scoping {
        group_id: String,
        group_name: String,
    },
    /// 工具已激活，可以执行
    Execution {
        group_id: String,
        active_tool_names: Vec<String>,
    },
}

/// 渐进式执行器
pub struct ProgressiveExecutor {
    app_handle: AppHandle,
    tool_router: Arc<ToolRouter>,
    // 状态将由循环逻辑维护
}

impl ProgressiveExecutor {
    pub fn new(app_handle: AppHandle, tool_router: Arc<ToolRouter>) -> Self {
        Self {
            app_handle,
            tool_router,
        }
    }

    // TODO: 实现 execute_loop
}
