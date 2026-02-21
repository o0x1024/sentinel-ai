use sentinel_llm::ChatMessage;

use crate::agents::context_engineering::checkpoint::ContextRunState;
use crate::agents::context_engineering::memory_index::{ingest_memory_items, retrieve_memory_items, MemoryQuery};
use crate::agents::context_engineering::tool_digest::build_tool_digest;
use crate::agents::context_engineering::types::trim_history_preserve_tool_pairs;

fn estimate_tokens_for_test(msg: &ChatMessage) -> usize {
    let mut tokens = (msg.content.len() as f64 * 0.4).ceil() as usize + 12;
    if let Some(tc) = &msg.tool_calls {
        tokens += (tc.len() as f64 * 0.4).ceil() as usize + 16;
    }
    tokens
}

#[test]
fn long_conversation_50_turns_preserves_tool_message_integrity() {
    let mut history = Vec::new();
    for i in 0..60 {
        history.push(ChatMessage::user(format!("turn {} user asks to continue task", i)));

        let mut assistant = ChatMessage::assistant(format!("turn {} assistant starts tool call", i));
        assistant.tool_calls = Some(format!(
            r#"[{{"id":"call_{}","type":"function","function":{{"name":"shell","arguments":"{{\"command\":\"echo {}\"}}"}}}}]"#,
            i, i
        ));
        history.push(assistant);
        history.push(ChatMessage::tool(
            format!(r#"{{"command":"echo {}","stdout":"ok {}","exit_code":0}}"#, i, i),
            format!("call_{}", i),
        ));
    }

    let total_tokens: usize = history.iter().map(estimate_tokens_for_test).sum();
    let trimmed = trim_history_preserve_tool_pairs(&history, total_tokens, 2400, estimate_tokens_for_test);

    assert!(!trimmed.is_empty());
    assert!(trimmed.len() < history.len());

    for idx in 0..trimmed.len() {
        let msg = &trimmed[idx];
        if msg.role == "tool" {
            assert!(idx > 0, "tool message cannot be first after trimming");
            let prev = &trimmed[idx - 1];
            assert!(
                prev.role == "assistant" || prev.role == "tool",
                "tool message should not be orphaned from assistant/tool chain"
            );
        }
    }
}

#[test]
fn long_conversation_retrieval_keeps_core_goal_after_50_turns() {
    let mut state = ContextRunState::default();
    ingest_memory_items(
        &mut state,
        &[String::from("Primary goal: keep service bound to 127.0.0.1:8080 only")],
        &[String::from("Decision: never expose admin endpoint to public network")],
        &[String::from("Todo: add regression for auth bypass")],
    );

    for i in 0..60 {
        ingest_memory_items(
            &mut state,
            &[format!("noise fact {}", i)],
            &[format!("noise decision {}", i)],
            &[format!("noise todo {}", i)],
        );
    }

    let query = MemoryQuery {
        execution_id: "regression-long-50".to_string(),
        query: "what is the binding and exposure constraint for the service".to_string(),
        top_k: 5,
    };

    let items = retrieve_memory_items(&mut state, &query);
    assert!(!items.is_empty());
    assert!(
        items.iter().any(|item| item.text.contains("127.0.0.1:8080") || item.text.contains("never expose admin endpoint")),
        "retrieval should keep core constraints discoverable after long dialogue"
    );
}

#[test]
fn long_tool_output_digest_stays_compact_and_referenced() {
    let large_output = "x".repeat(20_000);
    let result = format!(
        r#"{{"command":"cat huge.log","stdout":"{}","output_stored":true,"container_path":"/workspace/context/shell_big.txt"}}"#,
        large_output
    );
    let digest = build_tool_digest("shell", &serde_json::json!({"command":"cat huge.log"}), &result);

    assert!(digest.summary.len() < 400);
    assert_eq!(digest.artifact_id.as_deref(), Some("/workspace/context/shell_big.txt"));
}
