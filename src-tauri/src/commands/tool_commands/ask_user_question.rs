use std::collections::HashMap;
use std::sync::Arc;

use once_cell::sync::Lazy;
use serde::Serialize;
use tokio::sync::RwLock;

use sentinel_tools::buildin_tools::ask_user_question::{
    set_ask_user_question_handler, AskUserQuestionCollectedResponse, AskUserQuestionError,
    AskUserQuestionHandler, AskUserQuestionItem, AskUserQuestionRequest,
    AskUserQuestionResponseSource, AskUserQuestionResponseStatus, AskUserQuestionTimeoutPolicy,
};

#[derive(Debug, Clone, Serialize)]
pub struct PendingAskUserQuestionRequest {
    pub id: String,
    pub execution_id: Option<String>,
    pub questions: Vec<AskUserQuestionItem>,
    pub timestamp: u64,
    pub timeout_secs: u64,
    pub timeout_policy: AskUserQuestionTimeoutPolicy,
    pub default_answers: HashMap<String, String>,
    pub expires_at: u64,
}

enum AskUserQuestionResponse {
    Response(AskUserQuestionCollectedResponse),
    Rejected,
}

static ASK_USER_QUESTION_SENDERS: Lazy<
    RwLock<HashMap<String, tokio::sync::oneshot::Sender<AskUserQuestionResponse>>>,
> = Lazy::new(|| RwLock::new(HashMap::new()));

static PENDING_ASK_USER_QUESTIONS: Lazy<RwLock<Vec<PendingAskUserQuestionRequest>>> =
    Lazy::new(|| RwLock::new(Vec::new()));

struct AskUserQuestionHandlerImpl {
    app: tauri::AppHandle,
}

#[async_trait::async_trait]
impl AskUserQuestionHandler for AskUserQuestionHandlerImpl {
    async fn ask_questions(
        &self,
        request: AskUserQuestionRequest,
    ) -> Result<AskUserQuestionCollectedResponse, AskUserQuestionError> {
        let id = uuid::Uuid::new_v4().to_string();
        let (tx, rx) = tokio::sync::oneshot::channel();
        let timeout_secs = normalize_timeout_secs(request.timeout_secs);
        let timeout_policy = request.timeout_policy.unwrap_or_default();
        let default_answers = request.default_answers.unwrap_or_default();
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let expires_at = timestamp.saturating_add(timeout_secs);

        {
            let mut senders = ASK_USER_QUESTION_SENDERS.write().await;
            senders.insert(id.clone(), tx);
        }

        {
            let mut pending = PENDING_ASK_USER_QUESTIONS.write().await;
            pending.push(PendingAskUserQuestionRequest {
                id: id.clone(),
                execution_id: request.execution_id.clone(),
                questions: request.questions.clone(),
                timestamp,
                timeout_secs,
                timeout_policy,
                default_answers: default_answers.clone(),
                expires_at,
            });
        }

        use tauri::Emitter;
        if let Err(error) = self.app.emit(
            "ask-user-question-request",
            serde_json::json!({
                "id": id,
                "execution_id": request.execution_id,
                "questions": request.questions,
                "timeout_secs": timeout_secs,
                "timeout_policy": timeout_policy,
                "default_answers": default_answers,
                "expires_at": expires_at,
            }),
        ) {
            cleanup_pending_request(&id).await;
            return Err(AskUserQuestionError::RequestFailed(format!(
                "failed to emit question request: {}",
                error
            )));
        }

        let response =
            match tokio::time::timeout(std::time::Duration::from_secs(timeout_secs), rx).await {
                Ok(Ok(value)) => value,
                Ok(Err(_)) => {
                    cleanup_pending_request(&id).await;
                    return Err(AskUserQuestionError::RequestFailed(
                        "question response channel dropped".to_string(),
                    ));
                }
                Err(_) => {
                    cleanup_pending_request(&id).await;
                    return handle_timeout_response(
                        &request.questions,
                        &default_answers,
                        timeout_policy,
                    );
                }
            };

        cleanup_pending_request(&id).await;

        match response {
            AskUserQuestionResponse::Response(payload) => Ok(payload),
            AskUserQuestionResponse::Rejected => Err(AskUserQuestionError::Rejected),
        }
    }
}

fn normalize_timeout_secs(value: Option<u64>) -> u64 {
    value.unwrap_or(20).clamp(5, 1800)
}

fn build_default_answers(
    questions: &[AskUserQuestionItem],
    configured: &HashMap<String, String>,
) -> HashMap<String, String> {
    let mut answers = HashMap::new();
    for question in questions {
        if let Some(answer) = configured.get(&question.question) {
            answers.insert(question.question.clone(), answer.clone());
            continue;
        }
        if let Some(first_option) = question.options.first() {
            answers.insert(question.question.clone(), first_option.label.clone());
        }
    }
    answers
}

fn handle_timeout_response(
    questions: &[AskUserQuestionItem],
    default_answers: &HashMap<String, String>,
    timeout_policy: AskUserQuestionTimeoutPolicy,
) -> Result<AskUserQuestionCollectedResponse, AskUserQuestionError> {
    match timeout_policy {
        AskUserQuestionTimeoutPolicy::UseDefault => Ok(AskUserQuestionCollectedResponse {
            answers: build_default_answers(questions, default_answers),
            status: AskUserQuestionResponseStatus::TimeoutWithDefault,
            source: AskUserQuestionResponseSource::SystemDefault,
        }),
        AskUserQuestionTimeoutPolicy::ReturnTimeout => Ok(AskUserQuestionCollectedResponse {
            answers: HashMap::new(),
            status: AskUserQuestionResponseStatus::TimeoutWithoutDefault,
            source: AskUserQuestionResponseSource::Timeout,
        }),
        AskUserQuestionTimeoutPolicy::FailClosed => Err(AskUserQuestionError::RequestFailed(
            "question timed out".to_string(),
        )),
    }
}

async fn cleanup_pending_request(id: &str) {
    {
        let mut pending = PENDING_ASK_USER_QUESTIONS.write().await;
        pending.retain(|request| request.id != id);
    }

    let mut senders = ASK_USER_QUESTION_SENDERS.write().await;
    senders.remove(id);
}

pub async fn init_ask_user_question_handler(app: tauri::AppHandle) -> Result<(), String> {
    set_ask_user_question_handler(Arc::new(AskUserQuestionHandlerImpl { app })).await;
    Ok(())
}

pub async fn get_pending_ask_user_questions() -> Result<Vec<PendingAskUserQuestionRequest>, String>
{
    let pending = PENDING_ASK_USER_QUESTIONS.read().await;
    Ok(pending.clone())
}

pub async fn respond_ask_user_question(
    id: String,
    answers: HashMap<String, String>,
) -> Result<(), String> {
    {
        let mut pending = PENDING_ASK_USER_QUESTIONS.write().await;
        pending.retain(|request| request.id != id);
    }

    let mut senders = ASK_USER_QUESTION_SENDERS.write().await;
    let Some(tx) = senders.remove(&id) else {
        return Err(format!(
            "question request {} not found or already handled",
            id
        ));
    };
    tx.send(AskUserQuestionResponse::Response(
        AskUserQuestionCollectedResponse {
            answers,
            status: AskUserQuestionResponseStatus::Resolved,
            source: AskUserQuestionResponseSource::User,
        },
    ))
    .map_err(|_| format!("failed to deliver ask_user_question response for {}", id))
}

pub async fn reject_ask_user_question(id: String) -> Result<(), String> {
    {
        let mut pending = PENDING_ASK_USER_QUESTIONS.write().await;
        pending.retain(|request| request.id != id);
    }

    let mut senders = ASK_USER_QUESTION_SENDERS.write().await;
    let Some(tx) = senders.remove(&id) else {
        return Err(format!(
            "question request {} not found or already handled",
            id
        ));
    };
    tx.send(AskUserQuestionResponse::Rejected)
        .map_err(|_| format!("failed to reject ask_user_question request {}", id))
}
