use sentinel_llm::ChatMessage;

use crate::agents::context_engineering::memory_index::{
    ingest_memory_items, retrieve_memory_items, MemoryQuery,
};
use crate::agents::context_engineering::engine::ContextEngineMode;
use crate::agents::context_engineering::tool_digest::build_tool_digest;
use crate::agents::context_engineering::types::{
    trim_history_preserve_tool_pairs, ContextPacket, RetrievedMemorySection, ToolDigestEntry,
};
use crate::agents::context_engineering::{ContextMessageLayout, ContextRunState};

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
fn memory_ingestion_promotes_fact_signals_into_richer_kinds() {
    let mut state = ContextRunState::default();
    ingest_memory_items(
        &mut state,
        &[
            String::from("Prefer ripgrep for searches in large repos"),
            String::from("Avoid broad wildcard scans on production"),
        ],
        &[],
        &[],
    );

    assert!(state
        .memory_items
        .iter()
        .any(|item| item.kind == "preference" && item.importance == 4));
    assert!(state
        .memory_items
        .iter()
        .any(|item| item.kind == "anti_pattern" && item.importance == 4));
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

#[test]
fn system_prompt_stays_static_without_runtime_sections() {
    let mut packet = ContextPacket::new("STATIC_RULES".to_string());
    packet.run_state = "Goals:\n- dynamic task".to_string();
    packet.retrieved_memories = vec!["memory item".to_string()];
    packet.tool_digests = vec![ToolDigestEntry {
        status: "ok".to_string(),
        tool_name: "shell".to_string(),
        summary: "listed files".to_string(),
        artifact_id: None,
    }];

    let rendered = packet.render_system_prompt();
    assert_eq!(rendered, "STATIC_RULES");
    assert!(!rendered.contains("[RunState]"));
    assert!(!rendered.contains("[RetrievedMemory]"));
    assert!(!rendered.contains("[Recent Tool Digests]"));
}

#[test]
fn orchestrator_context_contains_runtime_sections() {
    let mut packet = ContextPacket::new("STATIC_RULES".to_string());
    packet.run_state = "Goals:\n- dynamic task".to_string();
    packet.retrieved_memories = vec!["memory item".to_string()];
    packet.tool_digests = vec![ToolDigestEntry {
        status: "ok".to_string(),
        tool_name: "shell".to_string(),
        summary: "listed files".to_string(),
        artifact_id: Some("/tmp/out.txt".to_string()),
    }];

    let rendered = packet.render_orchestrator_context();
    assert!(rendered.contains("[RunState]"));
    assert!(rendered.contains("[RetrievedMemory]"));
    assert!(rendered.contains("[Recent Tool Digests]"));
    assert!(!rendered.contains("STATIC_RULES"));
}

#[test]
fn codex_layout_splits_runtime_sections_into_multiple_messages() {
    let mut packet = ContextPacket::new("STATIC_RULES".to_string());
    packet.run_state = "Goals:\n- dynamic task".to_string();
    packet.retrieved_memories = vec!["memory item".to_string()];
    packet.tool_digests = vec![ToolDigestEntry {
        status: "ok".to_string(),
        tool_name: "shell".to_string(),
        summary: "listed files".to_string(),
        artifact_id: None,
    }];

    let messages = packet.render_context_messages(ContextMessageLayout::SplitUserMessages);
    assert_eq!(messages.len(), 3);
    assert!(messages[0].content.contains("[RunState]"));
    assert!(messages[1].content.contains("[RetrievedMemory]"));
    assert!(messages[2].content.contains("[Recent Tool Digests]"));
}

#[test]
fn orchestrator_context_renders_typed_memory_sections() {
    let mut packet = ContextPacket::new("STATIC_RULES".to_string());
    packet.retrieved_memory_sections = vec![
        RetrievedMemorySection {
            title: "Relevant Decisions".to_string(),
            items: vec!["[decision|importance=4|score=0.91] use ripgrep for searches".to_string()],
        },
        RetrievedMemorySection {
            title: "Known Anti-patterns".to_string(),
            items: vec![
                "[anti_pattern|importance=4|score=0.77] avoid broad wildcard scans".to_string(),
            ],
        },
    ];

    let rendered = packet.render_orchestrator_context();
    assert!(rendered.contains("[RetrievedMemory]"));
    assert!(rendered.contains("Relevant Decisions:"));
    assert!(rendered.contains("Known Anti-patterns:"));
}

#[test]
fn split_layout_uses_typed_memory_sections_when_available() {
    let mut packet = ContextPacket::new("STATIC_RULES".to_string());
    packet.retrieved_memory_sections = vec![RetrievedMemorySection {
        title: "Reusable SOP Hints".to_string(),
        items: vec![
            "[sop|importance=4|score=0.88] verify host, path, and auth state before replay"
                .to_string(),
        ],
    }];

    let messages = packet.render_context_messages(ContextMessageLayout::SplitUserMessages);
    assert_eq!(messages.len(), 1);
    assert!(messages[0].content.contains("[RetrievedMemory]"));
    assert!(messages[0].content.contains("Reusable SOP Hints:"));
}

#[test]
fn ask_user_question_digest_is_summarized_as_question_collection() {
    let digest = build_tool_digest(
        "ask_user_question",
        &serde_json::json!({
            "questions": [
                {
                    "header": "Mode",
                    "question": "Which mode should we use?",
                    "options": [
                        {"label": "Safe", "description": "Conservative"},
                        {"label": "Fast", "description": "Quicker"}
                    ]
                }
            ]
        }),
        r#"{"questions":[{"header":"Mode","question":"Which mode should we use?","options":[{"label":"Safe","description":"Conservative"},{"label":"Fast","description":"Quicker"}]}],"answers":{"Which mode should we use?":"Safe"},"status":"resolved","source":"user"}"#,
    );

    assert_eq!(digest.status, "ok");
    assert!(digest
        .summary
        .contains("AskUserQuestion collected 1 / 1 answers (user)"));
}

#[test]
fn ask_user_question_digest_marks_timeout_defaults() {
    let digest = build_tool_digest(
        "ask_user_question",
        &serde_json::json!({
            "questions": [
                {
                    "header": "Mode",
                    "question": "Which mode should we use?",
                    "options": [
                        {"label": "Safe", "description": "Conservative"},
                        {"label": "Fast", "description": "Quicker"}
                    ]
                }
            ]
        }),
        r#"{"questions":[{"header":"Mode","question":"Which mode should we use?","options":[{"label":"Safe","description":"Conservative"},{"label":"Fast","description":"Quicker"}]}],"answers":{"Which mode should we use?":"Safe"},"status":"timeout_with_default","source":"system_default"}"#,
    );

    assert!(digest.summary.contains("timed out and used defaults"));
}

#[test]
fn sentinel_like_context_mode_is_supported() {
    assert_eq!(
        ContextEngineMode::from_str("sentinel-like"),
        Some(ContextEngineMode::SentinelLike)
    );
}

#[test]
fn background_shell_digest_is_not_treated_as_failed_exit() {
    let digest = build_tool_digest(
        "shell",
        &serde_json::json!({
            "command": "python3 -m http.server 8000",
            "run_in_background": true
        }),
        r#"{"command":"python3 -m http.server 8000","backgrounded":true,"background_task_id":"task-1","background_session_id":"session-1","background_status":"running"}"#,
    );

    assert_eq!(digest.status, "ok");
    assert!(digest.summary.contains("background running"));
    assert!(digest.summary.contains("task task-1"));
    assert!(digest.summary.contains("session session-1"));
}
