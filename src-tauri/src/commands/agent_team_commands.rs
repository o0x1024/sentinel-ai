//! Agent Team Tauri 命令层

use std::sync::Arc;

use tauri::{AppHandle, State};
use tracing::{error, info};

use sentinel_db::DatabaseService;

use crate::agent_team::{
    engine::{start_agent_team_run_async, stop_agent_team_run_async},
    models::*,
    repository_runtime as repo_rt,
};

type DbState<'r> = State<'r, Arc<DatabaseService>>;

// ==================== 模板命令 ====================

/// 创建 Agent Team 模板
#[tauri::command]
pub async fn agent_team_create_template(
    db: DbState<'_>,
    request: CreateAgentTeamTemplateRequest,
) -> Result<AgentTeamTemplate, String> {
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    repo_rt::create_agent_team_template(&runtime_pool, &request, None)
        .await
        .map_err(|e| {
            error!("Failed to create agent team template: {:#}", e);
            e.to_string()
        })
}

/// 列出 Agent Team 模板
#[tauri::command]
pub async fn agent_team_list_templates(
    db: DbState<'_>,
    domain: Option<String>,
) -> Result<Vec<AgentTeamTemplate>, String> {
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    repo_rt::list_agent_team_templates(&runtime_pool, domain.as_deref())
        .await
        .map_err(|e| e.to_string())
}

/// 获取 Agent Team 模板详情
#[tauri::command]
pub async fn agent_team_get_template(
    db: DbState<'_>,
    template_id: String,
) -> Result<Option<AgentTeamTemplate>, String> {
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    repo_rt::get_agent_team_template_detail(&runtime_pool, &template_id)
        .await
        .map_err(|e| e.to_string())
}

/// 更新 Agent Team 模板
#[tauri::command]
pub async fn agent_team_update_template(
    db: DbState<'_>,
    template_id: String,
    request: UpdateAgentTeamTemplateRequest,
) -> Result<(), String> {
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    repo_rt::update_agent_team_template(&runtime_pool, &template_id, &request)
        .await
        .map_err(|e| e.to_string())
}

/// 删除 Agent Team 模板（只允许非系统模板）
#[tauri::command]
pub async fn agent_team_delete_template(
    db: DbState<'_>,
    template_id: String,
) -> Result<(), String> {
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;

    // 查询，确保不是系统模板
    let template = repo_rt::get_agent_team_template_detail(&runtime_pool, &template_id)
        .await
        .map_err(|e| e.to_string())?;

    match template {
        None => Err("模板不存在".to_string()),
        Some(t) if t.is_system => Err("系统内置模板不允许删除".to_string()),
        Some(_) => repo_rt::delete_agent_team_template(&runtime_pool, &template_id)
            .await
            .map_err(|e| e.to_string()),
    }
}

// ==================== 会话命令 ====================

/// 创建 Agent Team 会话
#[tauri::command]
pub async fn agent_team_create_session(
    db: DbState<'_>,
    request: CreateAgentTeamSessionRequest,
) -> Result<AgentTeamSession, String> {
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    repo_rt::create_agent_team_session(&runtime_pool, &request)
        .await
        .map_err(|e| {
            error!("Failed to create agent team session: {:#}", e);
            e.to_string()
        })
}

/// 获取 Agent Team 会话详情
#[tauri::command]
pub async fn agent_team_get_session(
    db: DbState<'_>,
    session_id: String,
) -> Result<Option<AgentTeamSession>, String> {
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    repo_rt::get_agent_team_session(&runtime_pool, &session_id)
        .await
        .map_err(|e| e.to_string())
}

/// 列出 Agent Team 会话（用于历史恢复）
#[tauri::command]
pub async fn agent_team_list_sessions(
    db: DbState<'_>,
    conversation_id: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Vec<AgentTeamSession>, String> {
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    repo_rt::list_agent_team_sessions(
        &runtime_pool,
        conversation_id.as_deref(),
        limit.unwrap_or(20),
        offset.unwrap_or(0),
    )
    .await
    .map_err(|e| e.to_string())
}

/// 更新 Agent Team 会话
#[tauri::command]
pub async fn agent_team_update_session(
    db: DbState<'_>,
    session_id: String,
    request: UpdateAgentTeamSessionRequest,
) -> Result<(), String> {
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    repo_rt::update_agent_team_session(&runtime_pool, &session_id, &request)
        .await
        .map_err(|e| e.to_string())
}

/// 删除 Agent Team 会话
#[tauri::command]
pub async fn agent_team_delete_session(db: DbState<'_>, session_id: String) -> Result<(), String> {
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    repo_rt::delete_agent_team_session(&runtime_pool, &session_id)
        .await
        .map_err(|e| e.to_string())
}

/// 列出 V2 任务看板
#[tauri::command]
pub async fn agent_team_list_tasks(
    db: DbState<'_>,
    session_id: String,
) -> Result<Vec<TeamTask>, String> {
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    repo_rt::list_tasks(&runtime_pool, &session_id)
        .await
        .map_err(|e| e.to_string())
}

/// 更新 V2 任务
#[tauri::command]
pub async fn agent_team_update_task(
    db: DbState<'_>,
    session_id: String,
    task_id: String,
    patch: UpdateTaskRequest,
) -> Result<(), String> {
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    let req = UpdateTaskRequest {
        task_id,
        status: patch.status,
        assignee_agent_id: patch.assignee_agent_id,
        last_error: patch.last_error,
    };
    repo_rt::update_task(&runtime_pool, &session_id, &req)
        .await
        .map_err(|e| e.to_string())
}

/// 列出 Agent 收件箱
#[tauri::command]
pub async fn agent_team_list_mailbox(
    db: DbState<'_>,
    session_id: String,
    agent_id: Option<String>,
) -> Result<Vec<MailboxMessage>, String> {
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    repo_rt::list_mailbox(&runtime_pool, &session_id, agent_id.as_deref())
        .await
        .map_err(|e| e.to_string())
}

/// 确认收件箱消息
#[tauri::command]
pub async fn agent_team_ack_mailbox(db: DbState<'_>, message_id: String) -> Result<(), String> {
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    repo_rt::ack_mailbox_message(&runtime_pool, &message_id)
        .await
        .map_err(|e| e.to_string())
}

/// 强制升级模板到 V2
#[tauri::command]
pub async fn agent_team_upgrade_templates_to_v2(
    db: DbState<'_>,
    force: Option<bool>,
) -> Result<i64, String> {
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    repo_rt::upgrade_templates_to_v2(&runtime_pool, force.unwrap_or(false))
        .await
        .map_err(|e| e.to_string())
}

/// 启动 Agent Team 运行（异步后台执行）
#[tauri::command]
pub async fn agent_team_start_run(
    app_handle: AppHandle,
    db: DbState<'_>,
    session_id: String,
) -> Result<(), String> {
    info!("Starting Agent Team run for session: {}", session_id);
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    let session = repo_rt::get_agent_team_session(&runtime_pool, &session_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Session not found".to_string())?;
    if session.schema_version < 2 {
        return Err(format!(
            "Session schema_version={} is not supported. Please upgrade to V2 first.",
            session.schema_version
        ));
    }
    let runtime_spec = session
        .runtime_spec_v2
        .as_ref()
        .ok_or_else(|| "runtime_spec_v2 is required".to_string())?;
    let agents = runtime_spec
        .get("agents")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "runtime_spec_v2.agents is missing".to_string())?;
    if agents.is_empty() {
        return Err("runtime_spec_v2.agents cannot be empty".to_string());
    }
    let nodes = runtime_spec
        .get("task_graph")
        .and_then(|v| v.get("nodes"))
        .and_then(|v| v.as_array())
        .ok_or_else(|| "runtime_spec_v2.task_graph.nodes is missing".to_string())?;
    if nodes.is_empty() {
        return Err("runtime_spec_v2.task_graph.nodes cannot be empty".to_string());
    }
    start_agent_team_run_async(app_handle, session_id)
        .await
        .map_err(|e| {
            error!("Failed to start agent team run: {:#}", e);
            e.to_string()
        })
}

/// 停止 Agent Team 运行
#[tauri::command]
pub async fn agent_team_stop_run(db: DbState<'_>, session_id: String) -> Result<(), String> {
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    let stopped = stop_agent_team_run_async(&session_id)
        .await
        .map_err(|e| e.to_string())?;

    // 无论是否命中运行句柄，只要当前会话还处于运行态，都强制落库为 FAILED，
    // 避免出现前端提示“已停止”但状态仍停留在运行中阶段。
    let session = repo_rt::get_agent_team_session(&runtime_pool, &session_id)
        .await
        .map_err(|e| e.to_string())?;
    let should_mark_failed = session
        .as_ref()
        .map(|s| {
            !matches!(
                s.state.as_str(),
                "PENDING" | "SUSPENDED_FOR_HUMAN" | "COMPLETED" | "FAILED" | "ARCHIVED"
            )
        })
        .unwrap_or(false);

    if should_mark_failed {
        repo_rt::update_agent_team_session(
            &runtime_pool,
            &session_id,
            &UpdateAgentTeamSessionRequest {
                name: None,
                goal: None,
                state: Some("FAILED".to_string()),
                max_rounds: None,
                schema_version: None,
                runtime_spec_v2: None,
                state_machine: None,
                error_message: Some("Execution stopped by user".to_string()),
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    }

    if !stopped {
        info!(
            "Stop requested for session {} but no running handle found; state patched={}",
            session_id, should_mark_failed
        );
    }

    Ok(())
}

// ==================== 消息命令 ====================

/// 获取 Agent Team 消息列表
#[tauri::command]
pub async fn agent_team_get_messages(
    db: DbState<'_>,
    session_id: String,
) -> Result<Vec<AgentTeamMessage>, String> {
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    repo_rt::get_messages(&runtime_pool, &session_id)
        .await
        .map_err(|e| e.to_string())
}

/// 获取 Agent Team 讨论轮次
#[tauri::command]
pub async fn agent_team_get_rounds(
    db: DbState<'_>,
    session_id: String,
) -> Result<Vec<AgentTeamRound>, String> {
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    repo_rt::get_rounds(&runtime_pool, &session_id)
        .await
        .map_err(|e| e.to_string())
}

/// 提交人工介入消息
#[tauri::command]
pub async fn agent_team_submit_message(
    db: DbState<'_>,
    app_handle: AppHandle,
    request: SubmitAgentTeamMessageRequest,
) -> Result<(), String> {
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;

    // 存储人工消息
    repo_rt::create_message(
        &runtime_pool,
        &request.session_id,
        None,
        None,
        Some("用户"),
        "user",
        &request.content,
        None,
    )
    .await
    .map_err(|e| e.to_string())?;

    // 如果需要恢复，重新启动引擎
    if request.resume {
        start_agent_team_run_async(app_handle, request.session_id)
            .await
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

/// 保存 Team 运行中断前的流式消息片段
#[tauri::command]
pub async fn agent_team_append_partial_message(
    db: DbState<'_>,
    request: AppendAgentTeamPartialMessageRequest,
) -> Result<(), String> {
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    let content = request.content.trim();
    let has_tool_calls = request
        .tool_calls
        .as_ref()
        .and_then(|v| v.as_array())
        .map(|arr| !arr.is_empty())
        .unwrap_or(false);

    if content.is_empty() && !has_tool_calls {
        return Ok(());
    }

    let role = request.role.trim().to_lowercase();
    if role != "assistant" && role != "user" && role != "system" {
        return Err("invalid role".to_string());
    }

    let msg = repo_rt::create_message(
        &runtime_pool,
        &request.session_id,
        None,
        request.member_id.as_deref(),
        request.member_name.as_deref(),
        &role,
        content,
        None,
    )
    .await
    .map_err(|e| e.to_string())?;

    if has_tool_calls {
        if let Some(tool_calls) = request.tool_calls.as_ref() {
            repo_rt::update_message_tool_calls(&runtime_pool, &msg.id, tool_calls)
                .await
                .map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}

// ==================== 白板/产物命令 ====================

/// 获取白板条目
#[tauri::command]
pub async fn agent_team_get_blackboard(
    db: DbState<'_>,
    session_id: String,
) -> Result<Vec<AgentTeamBlackboardEntry>, String> {
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    repo_rt::get_blackboard_entries(&runtime_pool, &session_id)
        .await
        .map_err(|e| e.to_string())
}

/// 手动添加白板条目
#[tauri::command]
pub async fn agent_team_add_blackboard_entry(
    db: DbState<'_>,
    request: UpdateBlackboardRequest,
) -> Result<AgentTeamBlackboardEntry, String> {
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    repo_rt::upsert_blackboard_entry(&runtime_pool, &request)
        .await
        .map_err(|e| e.to_string())
}

/// 标记白板条目为已解决
#[tauri::command]
pub async fn agent_team_resolve_blackboard_entry(
    db: DbState<'_>,
    session_id: String,
    entry_id: String,
) -> Result<AgentTeamBlackboardEntry, String> {
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    repo_rt::resolve_blackboard_entry(&runtime_pool, &session_id, &entry_id)
        .await
        .map_err(|e| e.to_string())
}

/// 获取白板条目关联的原始归档（消息明细）
#[tauri::command]
pub async fn agent_team_get_blackboard_entry_archive(
    db: DbState<'_>,
    session_id: String,
    entry_id: String,
    limit: Option<i64>,
) -> Result<AgentTeamBlackboardArchive, String> {
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    repo_rt::get_blackboard_entry_archive(
        &runtime_pool,
        &session_id,
        &entry_id,
        limit.unwrap_or(80),
    )
    .await
    .map_err(|e| e.to_string())
}

/// 列出产物
#[tauri::command]
pub async fn agent_team_list_artifacts(
    db: DbState<'_>,
    session_id: String,
) -> Result<Vec<AgentTeamArtifact>, String> {
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    repo_rt::list_artifacts(&runtime_pool, &session_id)
        .await
        .map_err(|e| e.to_string())
}

/// 获取产物详情
#[tauri::command]
pub async fn agent_team_get_artifact(
    db: DbState<'_>,
    artifact_id: String,
) -> Result<Option<AgentTeamArtifact>, String> {
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    repo_rt::get_artifact_detail(&runtime_pool, &artifact_id)
        .await
        .map_err(|e| e.to_string())
}

/// 获取 Team 运行状态（轮询用）
#[tauri::command]
pub async fn agent_team_get_run_status(
    db: DbState<'_>,
    session_id: String,
) -> Result<AgentTeamRunStatus, String> {
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;

    let session = repo_rt::get_agent_team_session(&runtime_pool, &session_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Session not found".to_string())?;

    let messages = repo_rt::get_messages(&runtime_pool, &session_id)
        .await
        .map_err(|e| e.to_string())?;

    let latest_message = messages
        .iter()
        .filter(|m| m.role == "assistant")
        .last()
        .map(|m| {
            let s: String = m.content.chars().take(200).collect();
            if m.content.chars().count() > 200 {
                format!("{}...", s)
            } else {
                s
            }
        });

    let is_suspended = session.state == "SUSPENDED_FOR_HUMAN";

    Ok(AgentTeamRunStatus {
        session_id: session_id.clone(),
        state: session.state,
        current_round: session.current_round,
        blackboard_snapshot: session.blackboard_state,
        latest_message,
        divergence_score: None,
        is_suspended,
    })
}

/// 触发种子数据（内置模板）
#[tauri::command]
pub async fn agent_team_seed_builtin_templates(db: DbState<'_>) -> Result<(), String> {
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    repo_rt::seed_builtin_templates(&runtime_pool)
        .await
        .map_err(|e| e.to_string())
}

// ==================== AI 生成模板命令 ====================

/// AI 模板生成请求
#[derive(Debug, serde::Deserialize)]
pub struct GenerateTemplateRequest {
    /// 用户对场景的自然语言描述
    pub description: String,
    /// 领域提示（可选，例如 product / security / audit）
    pub domain: Option<String>,
    /// 期望的 Agent 数量（可选，默认 3）
    pub agent_count: Option<u8>,
}

/// AI 生成的模板预览（保存前返回给前端确认）
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct GeneratedTemplate {
    pub name: String,
    pub description: String,
    pub domain: String,
    pub agents: Vec<GeneratedAgent>,
    /// LLM 原始输出（供调试）
    pub raw_json: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct GeneratedAgent {
    pub name: String,
    pub responsibility: String,
    pub system_prompt: String,
    pub decision_style: String,
    pub risk_preference: String,
    pub weight: f64,
}

/// 使用 LLM 生成 Agent Team 模板（只生成，不保存）
#[tauri::command]
pub async fn agent_team_generate_template(
    app_handle: AppHandle,
    request: GenerateTemplateRequest,
) -> Result<GeneratedTemplate, String> {
    use crate::agent_team::engine::AgentTeamEngine;

    let engine = AgentTeamEngine::new(app_handle);
    let llm_config = engine
        .get_llm_config_pub()
        .await
        .map_err(|e| e.to_string())?;

    let agent_count = request.agent_count.unwrap_or(3).clamp(2, 6);
    let domain_hint = request.domain.as_deref().unwrap_or("custom");

    let system_prompt = format!(
        r#"你是一个专业的 AI Agent Team 架构师。你的任务是根据用户的场景描述，设计一个高质量的多 Agent 协作团队模板。

## 输出要求
必须严格输出 **合法 JSON**，格式如下（不要包含任何其他文字）：

{{
  "name": "模板名称（简洁、专业，≤30字）",
  "description": "模板说明（≤100字，说明适用场景和能解决的问题）",
  "domain": "领域（product / security / audit / ops / redblue / custom 之一）",
  "agents": [
    {{
      "name": "Agent 名称（如：产品经理、安全分析师）",
      "responsibility": "该 Agent 的核心职责（≤80字）",
      "system_prompt": "专属 System Prompt（150-300字，使用第一人称，包含 Agent 定位、分析框架、输出要求）",
      "decision_style": "conservative | balanced | aggressive | pragmatic | risk_aware",
      "risk_preference": "low | medium | high",
      "weight": 1.0
    }}
  ]
}}

## 设计原则
1. **Agent 互补**：每个 Agent 必须有独特的视角和专业领域，避免重叠
2. **代表核心利益**：第一个 Agent 通常是主提案人，其余 Agent 代表不同立场（技术/业务/风险）
3. **system_prompt 专业度**：包含具体的分析方法论（如 STRIDE、CVSS、ITIL 等）
4. **分歧设计**：Agent 之间应有天然的视角分歧，推动高质量讨论
5. **正好 {agent_count} 个 Agent**

领域参考：`{domain_hint}`"#,
        agent_count = agent_count,
        domain_hint = domain_hint
    );

    let user_msg = format!(
        "请为以下场景设计 {} 个 Agent 的协作团队模板：\n\n{}",
        agent_count, request.description
    );

    let history = vec![sentinel_llm::ChatMessage {
        role: "user".to_string(),
        content: user_msg,
        tool_calls: None,
        tool_call_id: None,
        reasoning_content: None,
    }];

    let raw = engine
        .invoke_llm_pub(&llm_config, &system_prompt, &history)
        .await
        .map_err(|e| format!("LLM 调用失败: {}", e))?;

    // 从 LLM 响应中提取 JSON
    let json_str = extract_json_block(&raw).ok_or_else(|| {
        format!(
            "LLM 未返回合法 JSON。原始输出：\n{}",
            &raw[..raw.len().min(500)]
        )
    })?;

    // 解析 JSON
    let parsed: serde_json::Value = serde_json::from_str(&json_str).map_err(|e| {
        format!(
            "JSON 解析失败: {}。内容：{}",
            e,
            &json_str[..json_str.len().min(300)]
        )
    })?;

    // 提取字段
    let name = parsed["name"].as_str().unwrap_or("AI 生成模板").to_string();
    let description = parsed["description"].as_str().unwrap_or("").to_string();
    let domain = parsed["domain"].as_str().unwrap_or(domain_hint).to_string();

    let agents_raw = parsed["agents"]
        .as_array()
        .ok_or_else(|| "生成结果缺少 agents 字段".to_string())?;

    if agents_raw.is_empty() {
        return Err("AI 生成的模板没有 Agent，请重试".to_string());
    }

    let agents: Vec<GeneratedAgent> = agents_raw
        .iter()
        .map(|m| GeneratedAgent {
            name: m["name"].as_str().unwrap_or("Agent").to_string(),
            responsibility: m["responsibility"].as_str().unwrap_or("").to_string(),
            system_prompt: m["system_prompt"].as_str().unwrap_or("").to_string(),
            decision_style: m["decision_style"]
                .as_str()
                .unwrap_or("balanced")
                .to_string(),
            risk_preference: m["risk_preference"]
                .as_str()
                .unwrap_or("medium")
                .to_string(),
            weight: m["weight"].as_f64().unwrap_or(1.0),
        })
        .collect();

    info!(
        "AI generated template '{}' with {} agents for domain '{}'",
        name,
        agents.len(),
        domain
    );

    Ok(GeneratedTemplate {
        name,
        description,
        domain,
        agents,
        raw_json: json_str,
    })
}

/// 保存 AI 生成的模板到数据库
#[tauri::command]
pub async fn agent_team_save_generated_template(
    db: DbState<'_>,
    generated: GeneratedTemplate,
) -> Result<AgentTeamTemplate, String> {
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    let mut agents: Vec<AgentProfile> = Vec::new();
    let mut nodes: Vec<TeamTaskNode> = Vec::new();
    let mut prev_task_id: Option<String> = None;

    for (i, agent) in generated.agents.into_iter().enumerate() {
        let base = agent
            .name
            .trim()
            .to_lowercase()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .collect::<String>()
            .trim_matches('-')
            .to_string();
        let agent_id = if base.is_empty() {
            format!("agent-{}", i + 1)
        } else {
            format!("agent-{}", base)
        };
        agents.push(AgentProfile {
            id: agent_id.clone(),
            name: agent.name.clone(),
            system_prompt: Some(agent.system_prompt.clone()),
            model: None,
            tool_policy: None,
            skills: vec![],
            max_parallel_tasks: Some(1),
        });

        let task_id = format!("task-{}", i + 1);
        let mut depends_on = Vec::new();
        if let Some(prev) = prev_task_id.as_ref() {
            depends_on.push(prev.clone());
        }
        nodes.push(TeamTaskNode {
            id: task_id.clone(),
            title: format!("{} 执行任务", agent.name),
            instruction: if agent.responsibility.trim().is_empty() {
                format!("请作为 {} 完成当前任务。", agent.name)
            } else {
                agent.responsibility.clone()
            },
            depends_on,
            assignee_strategy: Some(serde_json::json!({
                "mode": "fixed_agent",
                "agent_id": agent_id,
                "agent_name": agent.name,
            })),
            retry: Some(TeamTaskRetryPolicy {
                max_attempts: Some(1),
                backoff_ms: Some(300),
            }),
            sla: None,
            input_schema: None,
            output_schema: None,
            phase: Some("task_execution".to_string()),
        });
        prev_task_id = Some(task_id);
    }

    let request = CreateAgentTeamTemplateRequest {
        name: generated.name,
        description: Some(generated.description),
        domain: generated.domain,
        default_rounds_config: None,
        default_tool_policy: None,
        schema_version: Some(2),
        agents,
        task_graph: TeamTaskGraph {
            version: Some(1),
            nodes,
        },
        hook_policy: None,
    };

    repo_rt::create_agent_team_template(&runtime_pool, &request, None)
        .await
        .map_err(|e| {
            error!("Failed to save generated template: {:#}", e);
            e.to_string()
        })
}

/// 从文本中提取第一个合法 JSON 对象/数组
fn extract_json_block(text: &str) -> Option<String> {
    // 优先从 ```json 代码块提取
    if let Some(start) = text.find("```json").or_else(|| text.find("```JSON")) {
        let after = &text[start + 7..];
        if let Some(end) = after.find("```") {
            let candidate = after[..end].trim();
            if serde_json::from_str::<serde_json::Value>(candidate).is_ok() {
                return Some(candidate.to_string());
            }
        }
    }

    // 其次从 ``` 代码块提取
    if let Some(start) = text.find("```") {
        let after = &text[start + 3..];
        // skip optional language tag line
        let trimmed = after.trim_start_matches(|c: char| c.is_alphanumeric());
        if let Some(end) = trimmed.find("```") {
            let candidate = trimmed[..end].trim();
            if serde_json::from_str::<serde_json::Value>(candidate).is_ok() {
                return Some(candidate.to_string());
            }
        }
    }

    // 最后尝试直接找 { ... } 范围
    let start = text.find('{')?;
    let mut depth = 0i32;
    let mut end = start;
    for (i, ch) in text[start..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    end = start + i + 1;
                    break;
                }
            }
            _ => {}
        }
    }
    if end > start {
        let candidate = &text[start..end];
        if serde_json::from_str::<serde_json::Value>(candidate).is_ok() {
            return Some(candidate.to_string());
        }
    }

    None
}
