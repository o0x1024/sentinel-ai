//! ReWOO Solver 实现
//!
//! 负责根据工具执行结果生成最终答案

use super::*;
use crate::services::ai::AiServiceManager;
use crate::services::prompt_db::PromptRepository;
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{info, warn};

const DEFAULT_SOLVER_PROMPT: &str = r#"You are a ReWOO solving assistant. Your task is to generate a comprehensive final answer based on the execution plan and tool results.

**Original Query:** {query}

**Execution Plan:**
{plan}

**Tool Execution Results:**
{results}

**Instructions:**
1. Analyze all the tool execution results
2. Synthesize the information into a coherent answer
3. Address the original query directly and completely
4. Include relevant details from the tool results
5. Organize the information logically
6. If any steps failed, acknowledge it and work with available information
7. Provide actionable insights when applicable

Generate your final answer:"#;

/// ReWOO Solver - 负责生成最终答案
#[derive(Debug)]
pub struct ReWOOSolver {
    /// AI服务管理器
    ai_service_manager: Arc<AiServiceManager>,
    /// Prompt仓库
    prompt_repo: Arc<PromptRepository>,
    /// 配置
    config: SolverConfig,
}

impl ReWOOSolver {
    /// 创建新的 ReWOO Solver
    pub fn new(
        ai_service_manager: Arc<AiServiceManager>,
        prompt_repo: Arc<PromptRepository>,
        config: SolverConfig,
    ) -> Result<Self> {
        Ok(Self {
            ai_service_manager,
            prompt_repo,
            config,
        })
    }

    /// 生成最终答案
    pub async fn solve(
        &self,
        query: &str,
        plan_string: &str,
        tool_results: &HashMap<String, serde_json::Value>,
        execution_id: &str,
        emitter: Option<&crate::utils::message_emitter::StandardMessageEmitter>,
    ) -> Result<String> {
        info!("ReWOO Solver: Generating final answer for query: {}", query);

        // 构建求解prompt（返回system prompt和user prompt）
        let (system_prompt, user_prompt) = self
            .build_solving_prompt(query, plan_string, tool_results)
            .await?;

        // 调用LLM生成答案
        let answer = self
            .call_llm(&system_prompt, &user_prompt, execution_id, emitter)
            .await?;

        info!("ReWOO Solver: Generated answer with {} chars", answer.len());
        Ok(answer)
    }

    /// 构建求解prompt（返回system prompt和user prompt）
    async fn build_solving_prompt(
        &self,
        query: &str,
        plan_string: &str,
        tool_results: &HashMap<String, serde_json::Value>,
    ) -> Result<(String, String)> {
        use crate::models::prompt::{ArchitectureType, StageType};

        // 从数据库获取ReWOO solver模板作为system prompt
        let system_template = if let Ok(Some(template)) = self
            .prompt_repo
            .get_template_by_arch_stage(ArchitectureType::ReWOO, StageType::Solver)
            .await
        {
            template.content
        } else {
            // Fallback到默认模板
            warn!("ReWOO solver template not found in database, using default template");
            include_str!("../prompt_md/rewoo_prompts.md")
                .split("## rewoo_solver")
                .nth(1)
                .and_then(|s| s.split("---").next())
                .unwrap_or(DEFAULT_SOLVER_PROMPT)
                .to_string()
        };

        // 构建工具结果字符串
        let mut results_str = String::new();
        for (var, result) in tool_results {
            results_str.push_str(&format!("{} = {}\n", var, result));
        }

        // 填充system prompt中的占位符
        let mut system_prompt = system_template
            .replace("{execution_plan}", plan_string)
            .replace("{execution_results}", &results_str);

        // 替换{task}占位符为用户输入
        let system_part = system_prompt.replace("{task}", query);

        // user prompt就是用户的输入
        let user_part = query.to_string();

        Ok((system_part, user_part))
    }

    /// 调用LLM生成答案
    async fn call_llm(
        &self,
        system_prompt: &str,
        user_prompt: &str,
        execution_id: &str,
        emitter: Option<&crate::utils::message_emitter::StandardMessageEmitter>,
    ) -> Result<String> {
        // 从调度器配置获取评估器模型(Evaluation阶段)
        let ai_service = match self
            .ai_service_manager
            .get_service_for_stage(crate::services::ai::SchedulerStage::Evaluation)
            .await
        {
            Ok(Some(service)) => {
                info!("ReWOO Solver: Using scheduler config for Evaluation stage");
                service
            }
            Ok(None) | Err(_) => {
                // 回退到默认服务
                warn!(
                    "ReWOO Solver: Scheduler config not available, using fallback model: {}",
                    self.config.model_name
                );
                let provider = &self.config.model_name;
                self.ai_service_manager
                    .get_service(provider)
                    .ok_or_else(|| anyhow!("AI service '{}' not found", provider))?
            }
        };

        let config = ai_service.get_config();
        info!(
            "ReWOO Solver: Using provider={}, model={}, execution_id={}",
            config.provider, config.model, execution_id
        );

        // 如果有emitter,则流式发送到前端;否则使用非流式模式
        let stream_to_frontend = emitter.is_some();

        // 在开始生成答案前,发送solving阶段的Content chunk
        if let Some(emitter) = emitter {
            // 发送一个空的Content chunk来标记Solving阶段开始
            emitter.emit_content("", false);
        }

        // 调用 AiService,不保存到数据库(conversation_id=None)
        let content = ai_service
            .send_message_stream(
                Some(user_prompt),
                Some(system_prompt),
                None, // 不关联会话
                Some(execution_id.to_string()),
                stream_to_frontend, // 根据emitter决定是否流式发送
                false,              // 不是最终消息(最终消息由engine_adapter发送)
                None,  // chunk_type
                None,  // attachments
            )
            .await?;

        if content.is_empty() {
            return Err(anyhow!("LLM returned empty response"));
        }

        info!(
            "ReWOO Solver: Generated answer with {} chars",
            content.len()
        );
        Ok(content)
    }
}
