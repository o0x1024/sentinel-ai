use anyhow::Result;
use rig::tool::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;
use std::sync::OnceLock;

pub type StoreMemoryFn = Box<
    dyn Fn(
            String,
            Option<String>,
            Vec<String>,
        ) -> Pin<Box<dyn Future<Output = Result<MemoryManagerStoreResult>> + Send>>
        + Send
        + Sync,
>;
pub type RetrieveMemoryFn = Box<
    dyn Fn(
            String,
            usize,
        ) -> Pin<Box<dyn Future<Output = Result<MemoryManagerRetrieveResult>> + Send>>
        + Send
        + Sync,
>;

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

    /// Optional title for the memory (only for "store"). If not provided, a title will be generated from content.
    pub title: Option<String>,

    /// Tags to categorize the memory (only for "store")
    pub tags: Option<Vec<String>>,

    /// Max number of results to return (only for "retrieve"), default 5
    pub limit: Option<usize>,
}

#[derive(Serialize, Clone)]
pub struct MemoryManagerResultItem {
    pub id: String,
    pub text: String,
    pub kind: String,
    pub scope: String,
    pub stability: String,
    pub source: String,
    pub confidence: f64,
    pub importance: u8,
    pub created_at_ms: i64,
    pub score: f64,
}

#[derive(Serialize, Clone)]
pub struct MemoryManagerProjectionState {
    pub memory_id: String,
    pub lexical_indexed: bool,
    pub vector_indexed: bool,
    pub skill_projected: bool,
    pub last_error: Option<String>,
    pub updated_at_ms: i64,
}

#[derive(Serialize, Clone)]
pub struct MemoryManagerStoreResult {
    pub memory_id: String,
    pub title: Option<String>,
    pub kind: String,
    pub scope: String,
    pub stability: String,
    pub source: String,
    pub confidence: f64,
    pub created_at_ms: i64,
    pub projection: MemoryManagerProjectionState,
}

#[derive(Serialize, Clone)]
pub struct MemoryManagerTraceCount {
    pub label: String,
    pub count: usize,
}

#[derive(Serialize, Clone)]
pub struct MemoryManagerRetrievalTrace {
    pub query_preview: String,
    pub requested_top_k: usize,
    pub hit_count: usize,
    pub used_canonical_fallback: bool,
    pub include_reflection: bool,
    pub source_breakdown: Vec<MemoryManagerTraceCount>,
    pub kind_breakdown: Vec<MemoryManagerTraceCount>,
}

#[derive(Serialize, Clone)]
pub struct MemoryManagerRetrieveResult {
    pub items: Vec<MemoryManagerResultItem>,
    pub trace: MemoryManagerRetrievalTrace,
}

#[derive(Serialize)]
pub struct MemoryManagerOutput {
    pub success: bool,
    pub message: String,
    pub results: Option<Vec<String>>,
    pub items: Option<Vec<MemoryManagerResultItem>>,
    pub store: Option<MemoryManagerStoreResult>,
    pub trace: Option<MemoryManagerRetrievalTrace>,
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

impl MemoryManagerTool {
    pub const NAME: &'static str = "memory";
    pub const DESCRIPTION: &'static str = concat!(
        "Long-term memory for durable agent knowledge across tasks. ",
        "Use action='retrieve' before non-trivial work when prior fixes, commands, environment quirks, ",
        "or workflows may help. Use action='store' after finishing a task to save reusable findings, ",
        "solutions, and playbooks. Do not store transient chat filler or incomplete guesses."
    );
}

impl Tool for MemoryManagerTool {
    const NAME: &'static str = Self::NAME;
    type Args = MemoryManagerArgs;
    type Output = MemoryManagerOutput;
    type Error = MemoryManagerError;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: Self::NAME.to_string(),
            description: Self::DESCRIPTION.to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(MemoryManagerArgs)).unwrap(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        match args.action.as_str() {
            "store" => {
                let handler = STORE_FN.get().ok_or(MemoryManagerError::MissingHandler)?;
                let tags = args.tags.unwrap_or_default();
                let store = handler(args.content, args.title, tags)
                    .await
                    .map_err(|e| MemoryManagerError::OperationFailed(e.to_string()))?;
                Ok(MemoryManagerOutput {
                    success: true,
                    message: format!(
                        "Memory stored as {} ({})",
                        store.memory_id,
                        if store.projection.lexical_indexed || store.projection.vector_indexed {
                            "retrievable"
                        } else {
                            "projection degraded"
                        }
                    ),
                    results: None,
                    items: None,
                    store: Some(store),
                    trace: None,
                })
            }
            "retrieve" => {
                let handler = RETRIEVE_FN
                    .get()
                    .ok_or(MemoryManagerError::MissingHandler)?;
                let limit = args.limit.unwrap_or(5);
                let payload = handler(args.content, limit)
                    .await
                    .map_err(|e| MemoryManagerError::OperationFailed(e.to_string()))?;
                let results = payload.items.iter().map(|item| item.text.clone()).collect();
                Ok(MemoryManagerOutput {
                    success: true,
                    message: format!(
                        "Retrieved {} matches via {}",
                        payload.trace.hit_count,
                        if payload.trace.used_canonical_fallback {
                            "canonical fallback"
                        } else {
                            "hybrid retrieval"
                        }
                    ),
                    results: Some(results),
                    items: Some(payload.items),
                    store: None,
                    trace: Some(payload.trace),
                })
            }
            _ => Err(MemoryManagerError::InvalidAction(args.action)),
        }
    }
}
