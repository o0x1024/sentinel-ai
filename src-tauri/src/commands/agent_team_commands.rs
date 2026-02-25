//! Agent Team Tauri 命令层

use std::sync::Arc;

use tauri::{AppHandle, State};
use tracing::{error, info};

use sentinel_db::DatabaseService;

use crate::agent_team::{
    engine::start_agent_team_run_async,
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
) -> Result<Vec<AgentTeamSession>, String> {
    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;
    repo_rt::list_agent_team_sessions(&runtime_pool, conversation_id.as_deref(), limit.unwrap_or(20))
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

/// 启动 Agent Team 运行（异步后台执行）
#[tauri::command]
pub async fn agent_team_start_run(
    app_handle: AppHandle,
    session_id: String,
) -> Result<(), String> {
    info!("Starting Agent Team run for session: {}", session_id);
    start_agent_team_run_async(app_handle, session_id)
        .await
        .map_err(|e| {
            error!("Failed to start agent team run: {:#}", e);
            e.to_string()
        })
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
pub async fn agent_team_seed_builtin_templates(
    db: DbState<'_>,
) -> Result<(), String> {
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
    /// 期望的角色数量（可选，默认 3）
    pub role_count: Option<u8>,
}

/// AI 生成的模板预览（保存前返回给前端确认）
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct GeneratedTemplate {
    pub name: String,
    pub description: String,
    pub domain: String,
    pub members: Vec<GeneratedMember>,
    /// LLM 原始输出（供调试）
    pub raw_json: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct GeneratedMember {
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
    let llm_config = engine.get_llm_config_pub().await.map_err(|e| e.to_string())?;

    let role_count = request.role_count.unwrap_or(3).clamp(2, 6);
    let domain_hint = request.domain.as_deref().unwrap_or("custom");

    let system_prompt = format!(
        r#"你是一个专业的 AI Agent Team 架构师。你的任务是根据用户的场景描述，设计一个高质量的多角色协作团队模板。

## 输出要求
必须严格输出 **合法 JSON**，格式如下（不要包含任何其他文字）：

{{
  "name": "模板名称（简洁、专业，≤30字）",
  "description": "模板说明（≤100字，说明适用场景和能解决的问题）",
  "domain": "领域（product / security / audit / ops / redblue / custom 之一）",
  "members": [
    {{
      "name": "角色名称（如：产品经理、安全分析师）",
      "responsibility": "该角色的核心职责（≤80字）",
      "system_prompt": "专属 System Prompt（150-300字，使用第一人称，包含角色定位、分析框架、输出要求）",
      "decision_style": "conservative | balanced | aggressive | pragmatic | risk_aware",
      "risk_preference": "low | medium | high",
      "weight": 1.0
    }}
  ]
}}

## 设计原则
1. **角色互补**：每个角色必须有独特的视角和专业领域，避免重叠
2. **代表核心利益**：第一个角色通常是主提案人，其余角色代表不同立场（技术/业务/风险）
3. **system_prompt 专业度**：包含具体的分析方法论（如 STRIDE、CVSS、ITIL 等）
4. **分歧设计**：角色之间应有天然的视角分歧，推动高质量讨论
5. **正好 {role_count} 个角色**

领域参考：`{domain_hint}`"#,
        role_count = role_count,
        domain_hint = domain_hint
    );

    let user_msg = format!(
        "请为以下场景设计 {} 个角色的协作团队模板：\n\n{}",
        role_count, request.description
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
    let json_str = extract_json_block(&raw)
        .ok_or_else(|| format!("LLM 未返回合法 JSON。原始输出：\n{}", &raw[..raw.len().min(500)]))?;

    // 解析 JSON
    let parsed: serde_json::Value = serde_json::from_str(&json_str)
        .map_err(|e| format!("JSON 解析失败: {}。内容：{}", e, &json_str[..json_str.len().min(300)]))?;

    // 提取字段
    let name = parsed["name"].as_str().unwrap_or("AI 生成模板").to_string();
    let description = parsed["description"].as_str().unwrap_or("").to_string();
    let domain = parsed["domain"].as_str().unwrap_or(domain_hint).to_string();

    let members_raw = parsed["members"].as_array()
        .ok_or_else(|| "生成结果缺少 members 字段".to_string())?;

    if members_raw.is_empty() {
        return Err("AI 生成的模板没有角色，请重试".to_string());
    }

    let members: Vec<GeneratedMember> = members_raw
        .iter()
        .map(|m| GeneratedMember {
            name: m["name"].as_str().unwrap_or("角色").to_string(),
            responsibility: m["responsibility"].as_str().unwrap_or("").to_string(),
            system_prompt: m["system_prompt"].as_str().unwrap_or("").to_string(),
            decision_style: m["decision_style"].as_str().unwrap_or("balanced").to_string(),
            risk_preference: m["risk_preference"].as_str().unwrap_or("medium").to_string(),
            weight: m["weight"].as_f64().unwrap_or(1.0),
        })
        .collect();

    info!(
        "AI generated template '{}' with {} members for domain '{}'",
        name, members.len(), domain
    );

    Ok(GeneratedTemplate {
        name,
        description,
        domain,
        members,
        raw_json: json_str,
    })
}

/// 保存 AI 生成的模板到数据库
#[tauri::command]
pub async fn agent_team_save_generated_template(
    db: DbState<'_>,
    generated: GeneratedTemplate,
) -> Result<AgentTeamTemplate, String> {
    use crate::agent_team::models::CreateAgentTeamTemplateMemberRequest;

    let runtime_pool = db.get_runtime_pool().map_err(|e| e.to_string())?;

    let request = CreateAgentTeamTemplateRequest {
        name: generated.name,
        description: Some(generated.description),
        domain: generated.domain,
        default_rounds_config: None,
        default_tool_policy: None,
        members: generated.members.into_iter().enumerate().map(|(i, m)| {
            CreateAgentTeamTemplateMemberRequest {
                name: m.name,
                responsibility: Some(m.responsibility),
                system_prompt: Some(m.system_prompt),
                decision_style: Some(m.decision_style),
                risk_preference: Some(m.risk_preference),
                weight: Some(m.weight),
                tool_policy: None,
                output_schema: None,
                sort_order: Some(i as i32),
            }
        }).collect(),
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
