//! Agent 步骤执行器
//!
//! 负责执行计划中的单个步骤，调用工具并收集结果

use super::config::ToolConfig;
use super::emitter::AgentMessageEmitter;
use super::types::{ExecutionContext, PlanStep, StepResult, ToolCall};
use anyhow::{anyhow, Result};
use std::sync::Arc;
use std::time::Instant;

/// 工具执行器 trait
#[async_trait::async_trait]
pub trait ToolExecutor: Send + Sync {
    async fn execute(&self, tool_name: &str, args: &serde_json::Value) -> Result<serde_json::Value>;
    fn list_tools(&self) -> Vec<String>;
    fn is_available(&self, tool_name: &str) -> bool;
}

/// 步骤执行器
pub struct StepExecutor {
    tool_executor: Arc<dyn ToolExecutor>,
    config: ToolConfig,
    emitter: Option<Arc<AgentMessageEmitter>>,
}

impl StepExecutor {
    pub fn new(tool_executor: Arc<dyn ToolExecutor>, config: ToolConfig) -> Self {
        Self {
            tool_executor,
            config,
            emitter: None,
        }
    }

    pub fn with_emitter(mut self, emitter: Arc<AgentMessageEmitter>) -> Self {
        self.emitter = Some(emitter);
        self
    }

    /// 执行单个步骤
    pub async fn execute(
        &self,
        step: &PlanStep,
        _context: &ExecutionContext,
    ) -> Result<StepResult> {
        let start = Instant::now();

        // 如果步骤没有工具调用，直接返回
        let tool_call = match &step.tool {
            Some(tc) => tc,
            None => return Ok(StepResult::no_tool(&step.id)),
        };

        // 检查工具是否启用
        if !self.config.is_tool_enabled(&tool_call.name) {
            return Ok(StepResult::failed(
                &step.id,
                format!("Tool {} is disabled", tool_call.name),
                start.elapsed().as_millis() as u64,
            ));
        }

        // 发送工具调用事件
        if let Some(emitter) = &self.emitter {
            emitter.emit_tool_call(&tool_call.name, &tool_call.args);
        }

        // 执行工具
        let result = tokio::time::timeout(
            self.config.tool_timeout(),
            self.tool_executor.execute(&tool_call.name, &tool_call.args),
        )
        .await;

        let duration = start.elapsed().as_millis() as u64;

        match result {
            Ok(Ok(output)) => {
                // 发送工具结果
                if let Some(emitter) = &self.emitter {
                    let output_str = serde_json::to_string_pretty(&output).unwrap_or_default();
                    emitter.emit_tool_result(&tool_call.name, &output_str, true, duration);
                }
                Ok(StepResult::success(&step.id, output, duration))
            }
            Ok(Err(e)) => {
                let error_msg = e.to_string();
                if let Some(emitter) = &self.emitter {
                    emitter.emit_tool_result(&tool_call.name, &error_msg, false, duration);
                }
                Ok(StepResult::failed(&step.id, error_msg, duration))
            }
            Err(_) => {
                let error_msg = format!("Tool {} timed out", tool_call.name);
                if let Some(emitter) = &self.emitter {
                    emitter.emit_tool_result(&tool_call.name, &error_msg, false, duration);
                }
                Ok(StepResult::failed(&step.id, error_msg, duration))
            }
        }
    }

    /// 批量执行步骤
    pub async fn execute_steps(
        &self,
        steps: &[PlanStep],
        context: &mut ExecutionContext,
    ) -> Vec<StepResult> {
        let mut results = Vec::new();
        let total = steps.len();

        for (index, step) in steps.iter().enumerate() {
            // 检查取消
            if context.is_cancelled() {
                break;
            }

            // 发送进度
            if let Some(emitter) = &self.emitter {
                emitter.emit_progress(index + 1, total, &step.description);
            }

            // 检查依赖
            let deps_satisfied = step.depends_on.iter().all(|dep_id| {
                context
                    .results
                    .get(dep_id)
                    .map(|r| r.success)
                    .unwrap_or(false)
            });

            if !deps_satisfied {
                let result = StepResult::failed(
                    &step.id,
                    "Dependencies not satisfied",
                    0,
                );
                results.push(result.clone());
                context.add_result(step.id.clone(), result);
                continue;
            }

            // 执行步骤
            match self.execute(step, context).await {
                Ok(result) => {
                    context.add_result(step.id.clone(), result.clone());
                    results.push(result);
                }
                Err(e) => {
                    let result = StepResult::failed(&step.id, e.to_string(), 0);
                    context.add_result(step.id.clone(), result.clone());
                    results.push(result);
                }
            }
        }

        results
    }
}

/// 空工具执行器（用于测试）
pub struct NoopToolExecutor;

#[async_trait::async_trait]
impl ToolExecutor for NoopToolExecutor {
    async fn execute(&self, tool_name: &str, _args: &serde_json::Value) -> Result<serde_json::Value> {
        Err(anyhow!("Tool {} not implemented", tool_name))
    }

    fn list_tools(&self) -> Vec<String> {
        vec![]
    }

    fn is_available(&self, _tool_name: &str) -> bool {
        false
    }
}

/// Mock 工具执行器（用于测试）
pub struct MockToolExecutor {
    responses: std::collections::HashMap<String, serde_json::Value>,
}

impl MockToolExecutor {
    pub fn new() -> Self {
        Self {
            responses: std::collections::HashMap::new(),
        }
    }

    pub fn with_response(mut self, tool_name: &str, response: serde_json::Value) -> Self {
        self.responses.insert(tool_name.to_string(), response);
        self
    }
}

impl Default for MockToolExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl ToolExecutor for MockToolExecutor {
    async fn execute(&self, tool_name: &str, _args: &serde_json::Value) -> Result<serde_json::Value> {
        self.responses
            .get(tool_name)
            .cloned()
            .ok_or_else(|| anyhow!("Tool {} not found in mock", tool_name))
    }

    fn list_tools(&self) -> Vec<String> {
        self.responses.keys().cloned().collect()
    }

    fn is_available(&self, tool_name: &str) -> bool {
        self.responses.contains_key(tool_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_execute_step_without_tool() {
        let executor = StepExecutor::new(
            Arc::new(NoopToolExecutor),
            ToolConfig::default(),
        );

        let step = PlanStep {
            id: "1".to_string(),
            description: "Analysis step".to_string(),
            tool: None,
            depends_on: vec![],
            fallback: None,
        };

        let ctx = ExecutionContext::new("exec-1", "Test", 10);
        let result = executor.execute(&step, &ctx).await.unwrap();
        
        assert!(result.success);
        assert_eq!(result.step_id, "1");
    }

    #[tokio::test]
    async fn test_execute_step_with_mock_tool() {
        let mock = MockToolExecutor::new()
            .with_response("test_tool", serde_json::json!({"result": "ok"}));
        
        let executor = StepExecutor::new(
            Arc::new(mock),
            ToolConfig::default(),
        );

        let step = PlanStep {
            id: "1".to_string(),
            description: "Test step".to_string(),
            tool: Some(ToolCall::new("test_tool", serde_json::json!({}))),
            depends_on: vec![],
            fallback: None,
        };

        let ctx = ExecutionContext::new("exec-1", "Test", 10);
        let result = executor.execute(&step, &ctx).await.unwrap();
        
        assert!(result.success);
    }

    #[tokio::test]
    async fn test_disabled_tool() {
        let mock = MockToolExecutor::new()
            .with_response("disabled_tool", serde_json::json!({}));
        
        let config = ToolConfig {
            disabled_tools: vec!["disabled_tool".to_string()],
            ..Default::default()
        };
        
        let executor = StepExecutor::new(Arc::new(mock), config);

        let step = PlanStep {
            id: "1".to_string(),
            description: "Test step".to_string(),
            tool: Some(ToolCall::new("disabled_tool", serde_json::json!({}))),
            depends_on: vec![],
            fallback: None,
        };

        let ctx = ExecutionContext::new("exec-1", "Test", 10);
        let result = executor.execute(&step, &ctx).await.unwrap();
        
        assert!(!result.success);
        assert!(result.error.unwrap().contains("disabled"));
    }
}

