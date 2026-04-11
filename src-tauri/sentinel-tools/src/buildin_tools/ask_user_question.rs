use once_cell::sync::Lazy;
use rig::tool::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AskUserQuestionOption {
    /// Short label shown to the user.
    pub label: String,
    /// Short description explaining the choice.
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AskUserQuestionItem {
    /// Very short chip/header for the question.
    pub header: String,
    /// The question text shown to the user.
    pub question: String,
    /// Available choices presented to the user.
    pub options: Vec<AskUserQuestionOption>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct AskUserQuestionArgs {
    /// Questions to ask the user.
    pub questions: Vec<AskUserQuestionItem>,
    /// Internal execution id for session-scoped question delivery.
    #[serde(default)]
    pub execution_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct AskUserQuestionOutput {
    pub questions: Vec<AskUserQuestionItem>,
    pub answers: HashMap<String, String>,
}

#[derive(Debug, thiserror::Error)]
pub enum AskUserQuestionError {
    #[error("Invalid ask_user_question input: {0}")]
    InvalidInput(String),
    #[error("No ask_user_question handler registered")]
    NoHandler,
    #[error("User declined to answer the questions")]
    Rejected,
    #[error("Failed to collect user answers: {0}")]
    RequestFailed(String),
}

#[async_trait::async_trait]
pub trait AskUserQuestionHandler: Send + Sync {
    async fn ask_questions(
        &self,
        questions: &[AskUserQuestionItem],
        execution_id: Option<&str>,
    ) -> Result<HashMap<String, String>, AskUserQuestionError>;
}

static ASK_USER_QUESTION_HANDLER: Lazy<RwLock<Option<Arc<dyn AskUserQuestionHandler>>>> =
    Lazy::new(|| RwLock::new(None));

pub async fn set_ask_user_question_handler(handler: Arc<dyn AskUserQuestionHandler>) {
    let mut guard = ASK_USER_QUESTION_HANDLER.write().await;
    *guard = Some(handler);
}

async fn request_user_answers(
    questions: &[AskUserQuestionItem],
    execution_id: Option<&str>,
) -> Result<HashMap<String, String>, AskUserQuestionError> {
    let guard = ASK_USER_QUESTION_HANDLER.read().await;
    let Some(handler) = &*guard else {
        return Err(AskUserQuestionError::NoHandler);
    };
    handler.ask_questions(questions, execution_id).await
}

fn validate_questions(questions: &[AskUserQuestionItem]) -> Result<(), AskUserQuestionError> {
    if questions.is_empty() {
        return Err(AskUserQuestionError::InvalidInput(
            "at least one question is required".to_string(),
        ));
    }
    if questions.len() > 4 {
        return Err(AskUserQuestionError::InvalidInput(
            "at most four questions are allowed".to_string(),
        ));
    }

    let mut seen_questions = HashSet::new();
    for item in questions {
        let question_text = item.question.trim();
        let header = item.header.trim();
        if question_text.is_empty() {
            return Err(AskUserQuestionError::InvalidInput(
                "question text cannot be empty".to_string(),
            ));
        }
        if header.is_empty() {
            return Err(AskUserQuestionError::InvalidInput(
                "question header cannot be empty".to_string(),
            ));
        }
        if !seen_questions.insert(question_text.to_string()) {
            return Err(AskUserQuestionError::InvalidInput(format!(
                "duplicate question text: {}",
                question_text
            )));
        }
        if item.options.len() < 2 || item.options.len() > 4 {
            return Err(AskUserQuestionError::InvalidInput(format!(
                "question '{}' must contain 2-4 options",
                question_text
            )));
        }
        let mut seen_labels = HashSet::new();
        for option in &item.options {
            let label = option.label.trim();
            let description = option.description.trim();
            if label.is_empty() {
                return Err(AskUserQuestionError::InvalidInput(format!(
                    "question '{}' has an empty option label",
                    question_text
                )));
            }
            if description.is_empty() {
                return Err(AskUserQuestionError::InvalidInput(format!(
                    "question '{}' has an empty option description",
                    question_text
                )));
            }
            if !seen_labels.insert(label.to_string()) {
                return Err(AskUserQuestionError::InvalidInput(format!(
                    "question '{}' contains duplicate option label '{}'",
                    question_text, label
                )));
            }
        }
    }

    Ok(())
}

#[derive(Debug, Clone, Default)]
pub struct AskUserQuestionTool;

impl AskUserQuestionTool {
    pub const NAME: &'static str = "ask_user_question";
    pub const DESCRIPTION: &'static str = concat!(
        "Ask the user one or more structured multiple-choice questions and wait for answers. ",
        "Use when requirements are ambiguous, when you need the user to choose between concrete options, ",
        "or when you must confirm a preference before continuing. Keep questions concise and decision-focused."
    );

    pub fn new() -> Self {
        Self
    }
}

impl Tool for AskUserQuestionTool {
    const NAME: &'static str = Self::NAME;
    type Args = AskUserQuestionArgs;
    type Output = AskUserQuestionOutput;
    type Error = AskUserQuestionError;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: Self::NAME.to_string(),
            description: Self::DESCRIPTION.to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "questions": {
                        "type": "array",
                        "description": "Questions to ask the user (1-4 questions).",
                        "minItems": 1,
                        "maxItems": 4,
                        "items": {
                            "type": "object",
                            "properties": {
                                "header": {
                                    "type": "string",
                                    "description": "Very short chip/header for the question."
                                },
                                "question": {
                                    "type": "string",
                                    "description": "The question text shown to the user."
                                },
                                "options": {
                                    "type": "array",
                                    "description": "Available choices for the question.",
                                    "minItems": 2,
                                    "maxItems": 4,
                                    "items": {
                                        "type": "object",
                                        "properties": {
                                            "label": {
                                                "type": "string",
                                                "description": "Short option label."
                                            },
                                            "description": {
                                                "type": "string",
                                                "description": "Short explanation of the option."
                                            }
                                        },
                                        "required": ["label", "description"]
                                    }
                                }
                            },
                            "required": ["header", "question", "options"]
                        }
                    }
                },
                "required": ["questions"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        validate_questions(&args.questions)?;
        let answers = request_user_answers(&args.questions, args.execution_id.as_deref()).await?;
        Ok(AskUserQuestionOutput {
            questions: args.questions,
            answers,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_question() -> AskUserQuestionItem {
        AskUserQuestionItem {
            header: "Mode".to_string(),
            question: "Which mode should we use?".to_string(),
            options: vec![
                AskUserQuestionOption {
                    label: "Safe".to_string(),
                    description: "Conservative execution".to_string(),
                },
                AskUserQuestionOption {
                    label: "Fast".to_string(),
                    description: "Optimize for speed".to_string(),
                },
            ],
        }
    }

    #[test]
    fn validates_question_shape() {
        assert!(validate_questions(&[sample_question()]).is_ok());
    }

    #[test]
    fn rejects_duplicate_labels() {
        let mut q = sample_question();
        q.options.push(AskUserQuestionOption {
            label: "Safe".to_string(),
            description: "Duplicate".to_string(),
        });
        assert!(validate_questions(&[q]).is_err());
    }
}
