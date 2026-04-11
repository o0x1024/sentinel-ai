use std::collections::HashMap;
use std::sync::Arc;

use once_cell::sync::Lazy;
use serde::Serialize;
use tokio::sync::RwLock;

use sentinel_tools::buildin_tools::ask_user_question::{
    set_ask_user_question_handler, AskUserQuestionError, AskUserQuestionHandler,
    AskUserQuestionItem,
};

#[derive(Debug, Clone, Serialize)]
pub struct PendingAskUserQuestionRequest {
    pub id: String,
    pub execution_id: Option<String>,
    pub questions: Vec<AskUserQuestionItem>,
    pub timestamp: u64,
}

enum AskUserQuestionResponse {
    Answers(HashMap<String, String>),
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
        questions: &[AskUserQuestionItem],
        execution_id: Option<&str>,
    ) -> Result<HashMap<String, String>, AskUserQuestionError> {
        let id = uuid::Uuid::new_v4().to_string();
        let (tx, rx) = tokio::sync::oneshot::channel();

        {
            let mut senders = ASK_USER_QUESTION_SENDERS.write().await;
            senders.insert(id.clone(), tx);
        }

        {
            let mut pending = PENDING_ASK_USER_QUESTIONS.write().await;
            pending.push(PendingAskUserQuestionRequest {
                id: id.clone(),
                execution_id: execution_id.map(|value| value.to_string()),
                questions: questions.to_vec(),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            });
        }

        use tauri::Emitter;
        if let Err(error) = self.app.emit(
            "ask-user-question-request",
            serde_json::json!({
                "id": id,
                "execution_id": execution_id,
                "questions": questions,
            }),
        ) {
            let mut pending = PENDING_ASK_USER_QUESTIONS.write().await;
            pending.retain(|request| request.id != id);
            return Err(AskUserQuestionError::RequestFailed(format!(
                "failed to emit question request: {}",
                error
            )));
        }

        let response = match tokio::time::timeout(std::time::Duration::from_secs(1800), rx).await {
            Ok(Ok(value)) => value,
            Ok(Err(_)) => {
                return Err(AskUserQuestionError::RequestFailed(
                    "question response channel dropped".to_string(),
                ))
            }
            Err(_) => {
                return Err(AskUserQuestionError::RequestFailed(
                    "question timed out".to_string(),
                ))
            }
        };

        {
            let mut pending = PENDING_ASK_USER_QUESTIONS.write().await;
            pending.retain(|request| request.id != id);
        }

        match response {
            AskUserQuestionResponse::Answers(answers) => Ok(answers),
            AskUserQuestionResponse::Rejected => Err(AskUserQuestionError::Rejected),
        }
    }
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
    tx.send(AskUserQuestionResponse::Answers(answers))
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
