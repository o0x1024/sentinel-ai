use rig::tool::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use std::pin::Pin;
use std::future::Future;
use anyhow::Result;

pub type StoreMemoryFn = Box<dyn Fn(String, Vec<String>) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> + Send + Sync>;
pub type RetrieveMemoryFn = Box<dyn Fn(String, usize) -> Pin<Box<dyn Future<Output = Result<Vec<String>>> + Send>> + Send + Sync>;

static STORE_FN: OnceLock<StoreMemoryFn> = OnceLock::new();
static RETRIEVE_FN: OnceLock<RetrieveMemoryFn> = OnceLock::new();

pub fn register_memory_functions(store: StoreMemoryFn, retrieve: RetrieveMemoryFn) {
    // Ignore error if already set
    let _ = STORE_FN.set(store);
    let _ = RETRIEVE_FN.set(retrieve);
}

#[derive(Deserialize, JsonSchema)]
pub struct MemoryManagerArgs {
    /// The action to perform: "store" or "retrieve"
    pub action: String,
    
    /// Content to store (if action="store") or query to retrieve (if action="retrieve")
    pub content: String,
    
    /// Tags to categorize the memory (only for "store")
    pub tags: Option<Vec<String>>,
    
    /// Max number of results to return (only for "retrieve"), default 5
    pub limit: Option<usize>,
}

#[derive(Serialize)]
pub struct MemoryManagerOutput {
    pub success: bool,
    pub message: String,
    pub results: Option<Vec<String>>,
}

#[derive(Debug, thiserror::Error)]
pub enum MemoryManagerError {
    #[error("Missing global handler. Please initialize memory functions.")]
    MissingHandler,
    #[error("Invalid action: {0}")]
    InvalidAction(String),
    #[error("Operation failed: {0}")]
    OperationFailed(String),
}

#[derive(Default)]
pub struct MemoryManagerTool;

impl Tool for MemoryManagerTool {
    const NAME: &'static str = "memory_manager";
    type Args = MemoryManagerArgs;
    type Output = MemoryManagerOutput;
    type Error = MemoryManagerError;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Manage long-term memory for the agent. Use 'store' to save important solutions, workflows, or findings for future reference into the vector database. Use 'retrieve' to perform semantic search on past experiences when facing new problems.".to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(MemoryManagerArgs)).unwrap(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        match args.action.as_str() {
            "store" => {
                let handler = STORE_FN.get().ok_or(MemoryManagerError::MissingHandler)?;
                let tags = args.tags.unwrap_or_default();
                handler(args.content, tags).await
                    .map_err(|e| MemoryManagerError::OperationFailed(e.to_string()))?;
                Ok(MemoryManagerOutput {
                    success: true,
                    message: "Memory stored successfully".to_string(),
                    results: None,
                })
            }
            "retrieve" => {
                let handler = RETRIEVE_FN.get().ok_or(MemoryManagerError::MissingHandler)?;
                let limit = args.limit.unwrap_or(5);
                let results = handler(args.content, limit).await
                    .map_err(|e| MemoryManagerError::OperationFailed(e.to_string()))?;
                Ok(MemoryManagerOutput {
                    success: true,
                    message: format!("Retrieved {} matches", results.len()),
                    results: Some(results),
                })
            }
            _ => Err(MemoryManagerError::InvalidAction(args.action)),
        }
    }
}
