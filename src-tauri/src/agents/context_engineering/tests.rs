use sentinel_llm::ChatMessage;

use crate::agents::context_engineering::memory_index::{
    ingest_memory_items, retrieve_memory_items, MemoryQuery,
};
use crate::agents::context_engineering::tool_digest::build_tool_digest;
use crate::agents::context_engineering::types::trim_history_preserve_tool_pairs;
use crate::agents::context_engineering::ContextRunState;

#[test]
fn trim_history_preserves_tool_pairs() {
    let mut assistant = ChatMessage::assistant("calling tool");
    assistant.tool_calls = Some(
        r#"[{"id":"call_1","type":"function","function":{"name":"shell","arguments":"{}"}}]"#
            .to_string(),
    );
    let tool = ChatMessage::tool(r#"{"stdout":"ok"}"#, "call_1".to_string());
    let user = ChatMessage::user("follow up");
    let history = vec![assistant, tool, user];

    let trimmed = trim_history_preserve_tool_pairs(&history, 200, 80, |_| 70);
    assert_eq!(trimmed.len(), 1);
    assert_eq!(trimmed[0].role, "user");
}

#[test]
fn memory_retrieval_prefers_relevant_items() {
    let mut state = ContextRunState::default();
    ingest_memory_items(
        &mut state,
        &[
            String::from("service runs on port 8080"),
            String::from("project root is /tmp/demo"),
        ],
        &[String::from("use ripgrep for searches")],
        &[String::from("write regression tests")],
    );

    let query = MemoryQuery {
        execution_id: "test-exec".to_string(),
        query: "which port does service run".to_string(),
        top_k: 3,
    };
    let items = retrieve_memory_items(&mut state, &query);
    assert!(!items.is_empty());
    assert!(items.iter().any(|item| item.text.contains("8080")));
}

#[test]
fn digest_extracts_artifact_reference() {
    let digest = build_tool_digest(
        "shell",
        &serde_json::json!({"command":"cat output.txt"}),
        r#"{"command":"cat output.txt","stdout":"","stderr":"","output_stored":true,"container_path":"/workspace/context/shell_1.txt"}"#,
    );
    assert_eq!(
        digest.artifact_id.as_deref(),
        Some("/workspace/context/shell_1.txt")
    );
    assert!(!digest.preview_snippets.is_empty() || digest.artifact_kind.is_some());
}
