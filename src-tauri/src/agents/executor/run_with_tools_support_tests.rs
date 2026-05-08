use super::*;
use crate::agents::{ToolConfig, ToolSelectionStrategy};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[test]
fn infer_tool_result_success_detects_timeout_failure() {
    assert!(!infer_tool_result_success(
        "Tool execution failed: command timeout after 180000ms"
    ));
}

#[test]
fn infer_tool_result_success_respects_success_field() {
    assert!(infer_tool_result_success(
        r#"{"success":true,"output":"ok"}"#
    ));
    assert!(!infer_tool_result_success(
        r#"{"success":false,"error":"boom"}"#
    ));
}

#[test]
fn infer_tool_result_success_treats_http_response_body_as_payload() {
    assert!(infer_tool_result_success(
        r#"{"url":"http://example.com","status_code":200,"status_text":"200 OK","headers":{"content-type":"text/html"},"body":"Warning: file_get_contents(/home/flag): failed to open stream: No such file or directory","body_length":123}"#
    ));
}

#[test]
fn infer_tool_result_success_treats_http_404_as_tool_success() {
    assert!(infer_tool_result_success(
        r#"{"url":"http://example.com/missing","status_code":404,"status_text":"404 Not Found","headers":{"content-type":"application/json"},"body":"{\"error\":\"resource not found\",\"code\":404}","body_length":43}"#
    ));
}

#[test]
fn manual_tool_scope_blocks_unselected_tools_when_allowed_tools_is_empty() {
    let config = ToolConfig {
        enabled: true,
        selection_strategy: ToolSelectionStrategy::Manual(vec![
            "file_read".to_string(),
            "grep".to_string(),
        ]),
        max_tools: 4,
        preselected_tools: vec![],
        disabled_tools: vec![],
        allowed_tools: vec![],
    };

    let scoped = apply_tool_config_scope_policy(
        vec![
            "file_read".to_string(),
            "grep".to_string(),
            "tenth_man_review".to_string(),
        ],
        &config,
    );

    assert_eq!(scoped, vec!["file_read".to_string(), "grep".to_string()]);
}

#[test]
fn explicit_allowed_tools_takes_precedence_over_manual_strategy_scope() {
    let config = ToolConfig {
        enabled: true,
        selection_strategy: ToolSelectionStrategy::Manual(vec!["file_read".to_string()]),
        max_tools: 4,
        preselected_tools: vec![],
        disabled_tools: vec![],
        allowed_tools: vec!["http_request".to_string()],
    };

    let scoped = apply_tool_config_scope_policy(
        vec!["file_read".to_string(), "http_request".to_string()],
        &config,
    );

    assert_eq!(scoped, vec!["http_request".to_string()]);
}

#[test]
fn infer_tool_result_success_prefers_http_shape_over_generic_error_field() {
    assert!(infer_tool_result_success(
        r#"{"url":"http://example.com","status_code":500,"status_text":"500 Internal Server Error","headers":{"content-type":"application/json"},"error":"upstream application error","body":"{\"message\":\"boom\"}","body_length":18}"#
    ));
}

#[test]
fn build_retry_history_compacts_large_json_tool_results() {
    let large_body = "line\n".repeat(4_000);
    let raw_result = serde_json::json!({
        "command": "cat huge.log",
        "stdout": large_body,
        "stderr": "",
        "exit_code": 0
    })
    .to_string();
    let history = build_retry_history(
        &[],
        vec![ToolCallRecord {
            id: "tool-1".to_string(),
            name: "shell".to_string(),
            arguments: r#"{"command":"cat huge.log"}"#.to_string(),
            result: Some(raw_result),
            success: true,
            sequence: 0,
            started_at_ms: 0,
            completed_at_ms: 1,
            duration_ms: 1,
        }],
        String::new(),
        1,
        true,
    );

    let tool_message = history
        .iter()
        .find(|message| message.role == "tool")
        .expect("tool result message should be present");

    assert!(tool_message.content.contains("_context_microcompact"));
    assert!(tool_message.content.contains("cat huge.log"));
    assert!(tool_message.content.len() < 13_000);
}

#[test]
fn build_retry_history_leaves_small_tool_results_unchanged() {
    let raw_result = r#"{"stdout":"ok","exit_code":0}"#.to_string();
    let history = build_retry_history(
        &[],
        vec![ToolCallRecord {
            id: "tool-1".to_string(),
            name: "shell".to_string(),
            arguments: r#"{"command":"echo ok"}"#.to_string(),
            result: Some(raw_result.clone()),
            success: true,
            sequence: 0,
            started_at_ms: 0,
            completed_at_ms: 1,
            duration_ms: 1,
        }],
        String::new(),
        1,
        true,
    );

    let tool_message = history
        .iter()
        .find(|message| message.role == "tool")
        .expect("tool result message should be present");

    assert_eq!(tool_message.content, raw_result);
}

#[test]
fn accumulate_retry_progress_compacts_large_tool_results_once() {
    let collector = Arc::new(Mutex::new(vec![ToolCallRecord {
        id: "tool-1".to_string(),
        name: "shell".to_string(),
        arguments: r#"{"command":"cat huge.log"}"#.to_string(),
        result: Some("line\n".repeat(4_000)),
        success: true,
        sequence: 0,
        started_at_ms: 0,
        completed_at_ms: 1,
        duration_ms: 1,
    }]));
    let accumulated = Arc::new(Mutex::new(Vec::new()));
    let assistant = Arc::new(Mutex::new(String::new()));
    let accumulated_output = Arc::new(Mutex::new(String::new()));

    accumulate_retry_progress(&collector, &accumulated, &assistant, &accumulated_output);

    let result = accumulated.lock().unwrap()[0].result.clone().unwrap();
    assert!(result.contains("context_microcompact"));
    assert!(result.len() < 13_000);
}

#[test]
fn detects_context_length_errors_for_reactive_compaction() {
    assert!(is_context_length_error(
        "This model's maximum context length is 128000 tokens"
    ));
    assert!(is_context_length_error("prompt too long"));
    assert!(is_context_length_error("too many tokens in request"));
    assert!(!is_context_length_error("connection reset by peer"));
}

#[test]
fn detects_high_risk_mutating_tool_calls() {
    assert!(is_high_risk_tool_call(
        "shell",
        r#"{"command":"sed -i 's/a/b/' src/app.rs"}"#
    ));
    assert!(is_high_risk_tool_call(
        "http_request",
        r#"{"method":"POST","url":"https://example.com/api"}"#
    ));
    assert!(!is_high_risk_tool_call(
        "shell",
        r#"{"command":"rg tenth_man src-tauri/src"}"#
    ));
}

#[test]
fn counts_trailing_failed_tool_calls() {
    let records = vec![
        ToolCallRecord {
            id: "1".to_string(),
            name: "shell".to_string(),
            arguments: r#"{"command":"rg foo ."}"#.to_string(),
            result: Some("ok".to_string()),
            success: true,
            sequence: 0,
            started_at_ms: 0,
            completed_at_ms: 1,
            duration_ms: 1,
        },
        ToolCallRecord {
            id: "2".to_string(),
            name: "shell".to_string(),
            arguments: r#"{"command":"cargo test"}"#.to_string(),
            result: Some("failed".to_string()),
            success: false,
            sequence: 1,
            started_at_ms: 2,
            completed_at_ms: 3,
            duration_ms: 1,
        },
        ToolCallRecord {
            id: "3".to_string(),
            name: "shell".to_string(),
            arguments: r#"{"command":"cargo test"}"#.to_string(),
            result: Some("failed".to_string()),
            success: false,
            sequence: 2,
            started_at_ms: 4,
            completed_at_ms: 5,
            duration_ms: 1,
        },
    ];

    assert_eq!(trailing_failed_tool_calls(&records), 2);
}

#[test]
fn final_response_review_required_when_changes_lack_verification() {
    let records = vec![ToolCallRecord {
        id: "1".to_string(),
        name: "shell".to_string(),
        arguments: r#"{"command":"sed -i 's/a/b/' src/app.rs"}"#.to_string(),
        result: Some("ok".to_string()),
        success: true,
        sequence: 0,
        started_at_ms: 0,
        completed_at_ms: 1,
        duration_ms: 1,
    }];

    assert!(final_response_needs_verification_review(
        "问题已修复，已经完成。",
        &records
    ));
}

#[test]
fn final_response_review_skipped_when_verification_exists() {
    let records = vec![
        ToolCallRecord {
            id: "1".to_string(),
            name: "shell".to_string(),
            arguments: r#"{"command":"sed -i 's/a/b/' src/app.rs"}"#.to_string(),
            result: Some("ok".to_string()),
            success: true,
            sequence: 0,
            started_at_ms: 0,
            completed_at_ms: 1,
            duration_ms: 1,
        },
        ToolCallRecord {
            id: "2".to_string(),
            name: "shell".to_string(),
            arguments: r#"{"command":"cargo check"}"#.to_string(),
            result: Some("ok".to_string()),
            success: true,
            sequence: 1,
            started_at_ms: 2,
            completed_at_ms: 3,
            duration_ms: 1,
        },
    ];

    assert!(!final_response_needs_verification_review(
        "问题已修复，已经完成。",
        &records
    ));
}

#[test]
fn final_response_review_required_for_low_evidence_high_confidence() {
    let records = vec![ToolCallRecord {
        id: "1".to_string(),
        name: "shell".to_string(),
        arguments: r#"{"command":"ls src"}"#.to_string(),
        result: Some("ok".to_string()),
        success: true,
        sequence: 0,
        started_at_ms: 0,
        completed_at_ms: 1,
        duration_ms: 1,
    }];

    assert!(final_response_needs_evidence_review(
        "可以确定根因就是这里，最佳方案已经明确。",
        None,
        &records,
        3,
    ));
}

#[test]
fn final_response_review_skipped_when_evidence_is_sufficient() {
    let records = vec![
        ToolCallRecord {
            id: "1".to_string(),
            name: "shell".to_string(),
            arguments: r#"{"command":"rg tenth_man src-tauri/src"}"#.to_string(),
            result: Some("ok".to_string()),
            success: true,
            sequence: 0,
            started_at_ms: 0,
            completed_at_ms: 1,
            duration_ms: 1,
        },
        ToolCallRecord {
            id: "2".to_string(),
            name: "shell".to_string(),
            arguments: r#"{"command":"sed -n '1,120p' src-tauri/src/agents/tenth_man.rs"}"#
                .to_string(),
            result: Some("ok".to_string()),
            success: true,
            sequence: 1,
            started_at_ms: 2,
            completed_at_ms: 3,
            duration_ms: 1,
        },
    ];

    assert!(!final_response_needs_evidence_review(
        "可以确定根因就是这里，最佳方案已经明确。",
        None,
        &records,
        3,
    ));
}

#[test]
fn streaming_review_required_for_low_evidence_conclusion() {
    let records = vec![ToolCallRecord {
        id: "1".to_string(),
        name: "shell".to_string(),
        arguments: r#"{"command":"ls src"}"#.to_string(),
        result: Some("ok".to_string()),
        success: true,
        sequence: 0,
        started_at_ms: 0,
        completed_at_ms: 1,
        duration_ms: 1,
    }];

    assert!(streaming_content_needs_evidence_review(
        "基于当前情况，可以确定根因就是这里，因此应该直接按这个方向修改。",
        None,
        &records,
        3,
    ));
}

#[test]
fn streaming_review_skipped_for_short_conclusion_fragment() {
    let records = vec![ToolCallRecord {
        id: "1".to_string(),
        name: "shell".to_string(),
        arguments: r#"{"command":"ls src"}"#.to_string(),
        result: Some("ok".to_string()),
        success: true,
        sequence: 0,
        started_at_ms: 0,
        completed_at_ms: 1,
        duration_ms: 1,
    }];

    assert!(!streaming_content_needs_evidence_review(
        "因此可以先看这里。",
        None,
        &records,
        3,
    ));
}

#[test]
fn evidence_score_distinguishes_broad_scan_from_direct_inspection() {
    let broad_scan = evidence_score_for_tool_call("shell", r#"{"command":"ls src"}"#, true);
    let direct_read = evidence_score_for_tool_call(
        "shell",
        r#"{"command":"sed -n '1,120p' src/main.rs"}"#,
        true,
    );
    let verification = evidence_score_for_tool_call("shell", r#"{"command":"cargo check"}"#, true);

    assert_eq!(broad_scan, 1);
    assert_eq!(direct_read, 2);
    assert_eq!(verification, 3);
}

#[test]
fn total_evidence_score_adds_weighted_evidence() {
    let records = vec![
        ToolCallRecord {
            id: "1".to_string(),
            name: "shell".to_string(),
            arguments: r#"{"command":"ls src"}"#.to_string(),
            result: Some("ok".to_string()),
            success: true,
            sequence: 0,
            started_at_ms: 0,
            completed_at_ms: 1,
            duration_ms: 1,
        },
        ToolCallRecord {
            id: "2".to_string(),
            name: "shell".to_string(),
            arguments: r#"{"command":"sed -n '1,120p' src/main.rs"}"#.to_string(),
            result: Some("ok".to_string()),
            success: true,
            sequence: 1,
            started_at_ms: 2,
            completed_at_ms: 3,
            duration_ms: 1,
        },
    ];

    assert_eq!(total_evidence_score(&records), 3);
}

#[test]
fn relevant_evidence_score_ignores_unrelated_evidence() {
    let records = vec![
        ToolCallRecord {
            id: "1".to_string(),
            name: "shell".to_string(),
            arguments: r#"{"command":"rg metrics src-tauri/src"}"#.to_string(),
            result: Some("ok".to_string()),
            success: true,
            sequence: 0,
            started_at_ms: 0,
            completed_at_ms: 1,
            duration_ms: 1,
        },
        ToolCallRecord {
            id: "2".to_string(),
            name: "shell".to_string(),
            arguments: r#"{"command":"sed -n '1,120p' src-tauri/src/agents/metrics.rs"}"#
                .to_string(),
            result: Some("ok".to_string()),
            success: true,
            sequence: 1,
            started_at_ms: 2,
            completed_at_ms: 3,
            duration_ms: 1,
        },
    ];

    assert!(final_response_needs_evidence_review(
        "可以确定问题就在 src-tauri/src/agents/tenth_man.rs 这里。",
        None,
        &records,
        3,
    ));
}

#[test]
fn relevant_evidence_score_counts_matching_evidence() {
    let records = vec![
        ToolCallRecord {
            id: "1".to_string(),
            name: "shell".to_string(),
            arguments: r#"{"command":"rg tenth_man src-tauri/src"}"#.to_string(),
            result: Some("ok".to_string()),
            success: true,
            sequence: 0,
            started_at_ms: 0,
            completed_at_ms: 1,
            duration_ms: 1,
        },
        ToolCallRecord {
            id: "2".to_string(),
            name: "shell".to_string(),
            arguments: r#"{"command":"sed -n '1,120p' src-tauri/src/agents/tenth_man.rs"}"#
                .to_string(),
            result: Some("ok".to_string()),
            success: true,
            sequence: 1,
            started_at_ms: 2,
            completed_at_ms: 3,
            duration_ms: 1,
        },
    ];

    assert!(!final_response_needs_evidence_review(
        "可以确定问题就在 src-tauri/src/agents/tenth_man.rs 这里。",
        None,
        &records,
        3,
    ));
}

#[tokio::test]
async fn file_snapshot_requires_prior_read_and_fresh_mtime() {
    let execution_id = format!("exec-{}", uuid::Uuid::new_v4());
    let temp_dir = std::env::temp_dir().join(format!("file-state-{}", uuid::Uuid::new_v4()));
    tokio::fs::create_dir_all(&temp_dir).await.unwrap();
    let file_path: PathBuf = temp_dir.join("sample.txt");
    tokio::fs::write(&file_path, "a\nb\nc\n").await.unwrap();
    let revision_token =
        sentinel_tools::buildin_tools::file_runtime::revision_token(&file_path.to_string_lossy())
            .await
            .unwrap();

    record_file_read_snapshot(
        &execution_id,
        &file_path.to_string_lossy(),
        &revision_token,
        1,
        2,
        3,
        true,
    )
    .await
    .unwrap();
    ensure_file_snapshot_is_editable(&execution_id, &file_path.to_string_lossy())
        .await
        .unwrap();

    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    tokio::fs::write(&file_path, "changed\n").await.unwrap();
    let stale_err = ensure_file_snapshot_is_editable(&execution_id, &file_path.to_string_lossy())
        .await
        .unwrap_err()
        .to_string();
    assert!(stale_err.contains("changed since it was read"));

    crate::agents::executor::file_tool_state::clear_file_tool_state(&execution_id).await;
    let _ = tokio::fs::remove_dir_all(&temp_dir).await;
}

#[tokio::test]
async fn file_read_edit_override_allows_consecutive_edits_after_snapshot_update() {
    use rig::tool::Tool;
    use sentinel_tools::buildin_tools::shell::{
        get_shell_config, set_shell_config, ShellExecutionMode,
    };

    let execution_id = format!("exec-{}", uuid::Uuid::new_v4());
    let temp_dir =
        std::env::temp_dir().join(format!("file-override-state-{}", uuid::Uuid::new_v4()));
    tokio::fs::create_dir_all(&temp_dir).await.unwrap();
    let file_path: PathBuf = temp_dir.join("sample.txt");
    tokio::fs::write(&file_path, "alpha\nbeta\ngamma\n")
        .await
        .unwrap();
    let original_shell_config = get_shell_config().await;
    let mut host_shell_config = original_shell_config.clone();
    host_shell_config.default_execution_mode = ShellExecutionMode::Host;
    set_shell_config(host_shell_config).await;

    let tool_server = ToolServer::new();
    tool_server.init_builtin_tools().await;
    let working_dir = temp_dir.to_string_lossy().to_string();
    let read_def =
        build_file_read_override_def(&tool_server, &execution_id, None, Some(&working_dir))
            .await
            .expect("file_read override should be available");
    let edit_def =
        build_file_edit_override_def(&tool_server, &execution_id, None, Some(&working_dir))
            .await
            .expect("file_edit override should be available");
    let read_tool = DynamicTool::new(read_def);
    let edit_tool = DynamicTool::new(edit_def);

    read_tool
        .call(json!({
            "file_path": file_path.to_string_lossy(),
            "offset": 1,
            "limit": 200,
        }))
        .await
        .expect("file_read should record an editable snapshot");

    edit_tool
        .call(json!({
            "file_path": file_path.to_string_lossy(),
            "old_string": "beta",
            "new_string": "delta",
        }))
        .await
        .expect("first edit should update the snapshot to the new revision");

    edit_tool
        .call(json!({
            "file_path": file_path.to_string_lossy(),
            "old_string": "gamma",
            "new_string": "epsilon",
        }))
        .await
        .expect("second edit should not require another file_read");

    let updated = tokio::fs::read_to_string(&file_path).await.unwrap();
    assert_eq!(updated, "alpha\ndelta\nepsilon\n");

    crate::agents::executor::file_tool_state::clear_file_tool_state(&execution_id).await;
    set_shell_config(original_shell_config).await;
    let _ = tokio::fs::remove_dir_all(&temp_dir).await;
}
