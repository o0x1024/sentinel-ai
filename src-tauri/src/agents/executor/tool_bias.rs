use tauri::AppHandle;

use sentinel_tools::buildin_tools::{
    BrowserShellTool, FileReadTool, ShellTool, ToolSearchRuntimeContext, ToolSearchTool,
};
use serde_json::Value;

use crate::agents::context_engineering::checkpoint::ContextRunState;
use crate::agents::context_engineering::has_incomplete_host_artifacts;
use crate::agents::load_run_state;
use crate::agents::{ToolConfig, ToolDigest};

pub async fn bias_tool_ids_for_recent_file_changes(
    app_handle: &AppHandle,
    execution_id: &str,
    task: &str,
    tool_ids: Vec<String>,
    tool_config: &ToolConfig,
    active_browser_shell_session_id: Option<&str>,
) -> Vec<String> {
    let run_state = load_run_state(app_handle, execution_id)
        .await
        .ok()
        .flatten();
    let tool_ids =
        apply_browser_shell_bias(tool_ids, task, active_browser_shell_session_id, tool_config);
    apply_file_read_bias(tool_ids, run_state.as_ref(), tool_config)
}

fn apply_browser_shell_bias(
    mut tool_ids: Vec<String>,
    task: &str,
    active_browser_shell_session_id: Option<&str>,
    tool_config: &ToolConfig,
) -> Vec<String> {
    let has_bound_browser_shell = active_browser_shell_session_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_some();
    if !has_bound_browser_shell {
        return tool_ids;
    }

    if task_explicitly_targets_bound_browser_shell(task) {
        tool_ids.retain(|id| id != ShellTool::NAME);
    }

    let browser_shell_id = BrowserShellTool::NAME.to_string();
    if tool_ids.iter().any(|id| id == &browser_shell_id) {
        return tool_ids;
    }
    if tool_config
        .disabled_tools
        .iter()
        .any(|id| id == &browser_shell_id)
    {
        return tool_ids;
    }
    if !tool_config.allowed_tools.is_empty()
        && !tool_config
            .allowed_tools
            .iter()
            .any(|id| id == &browser_shell_id)
    {
        return tool_ids;
    }

    if tool_ids.len() >= tool_config.max_tools {
        if let Some(index) = tool_ids.iter().position(|id| id == ToolSearchTool::NAME) {
            tool_ids.remove(index);
        } else {
            return tool_ids;
        }
    }

    tool_ids.push(browser_shell_id);
    tool_ids
}

fn task_explicitly_targets_bound_browser_shell(task: &str) -> bool {
    let task = task.trim().to_lowercase();
    if task.is_empty() {
        return false;
    }

    let explicit_target_phrases = [
        "browser shell",
        "current browser shell",
        "current browser terminal",
        "browser websocket shell",
        "third-party shell",
        "third party shell",
        "third-party terminal",
        "third party terminal",
        "浏览器shell",
        "浏览器 shell",
        "当前browser shell",
        "当前 browser shell",
        "当前浏览器shell",
        "当前浏览器 shell",
        "当前浏览器终端",
        "浏览器终端",
        "第三方shell",
        "第三方 shell",
        "第三方终端",
        "网页shell",
        "网页 shell",
        "websocket shell",
    ];

    explicit_target_phrases
        .iter()
        .any(|phrase| task.contains(phrase))
}

pub(crate) async fn build_tool_search_runtime_context(
    app_handle: &AppHandle,
    execution_id: &str,
) -> Option<ToolSearchRuntimeContext> {
    let run_state = load_run_state(app_handle, execution_id)
        .await
        .ok()
        .flatten();
    if !should_prioritize_file_read(run_state.as_ref()) {
        return None;
    }

    Some(ToolSearchRuntimeContext {
        prefer_file_read_backfill: true,
        reason: Some(
            "recent file changes detected; prefer reading back the changed result before more edits or the final answer"
                .to_string(),
        ),
    })
}

fn apply_file_read_bias(
    mut tool_ids: Vec<String>,
    run_state: Option<&ContextRunState>,
    tool_config: &ToolConfig,
) -> Vec<String> {
    if !should_prioritize_file_read(run_state) {
        return tool_ids;
    }

    let file_read_id = FileReadTool::NAME.to_string();
    if tool_ids.iter().any(|id| id == &file_read_id) {
        return tool_ids;
    }
    if tool_config
        .disabled_tools
        .iter()
        .any(|id| id == &file_read_id)
    {
        return tool_ids;
    }
    if !tool_config.allowed_tools.is_empty()
        && !tool_config
            .allowed_tools
            .iter()
            .any(|id| id == &file_read_id)
    {
        return tool_ids;
    }

    if tool_ids.len() >= tool_config.max_tools {
        if let Some(index) = tool_ids.iter().position(|id| id == ToolSearchTool::NAME) {
            tool_ids.remove(index);
        } else {
            return tool_ids;
        }
    }

    tool_ids.push(file_read_id);
    tool_ids
}

pub(crate) fn should_prioritize_file_read(run_state: Option<&ContextRunState>) -> bool {
    let Some(run_state) = run_state else {
        return false;
    };

    if has_incomplete_host_artifacts(Some(run_state)) {
        return true;
    }

    has_unverified_recent_file_change(&run_state.last_tool_digests)
}

fn has_unverified_recent_file_change(digests: &[ToolDigest]) -> bool {
    let mut verified_reads: Vec<(String, String)> = Vec::new();

    for digest in digests.iter().rev().take(8) {
        if digest.status != "ok" {
            continue;
        }

        match digest.tool_name.as_str() {
            "file_read" => {
                if let Some((artifact_id, content_hash)) = digest_artifact_and_hash(digest) {
                    verified_reads.push((artifact_id, content_hash));
                }
            }
            "file_edit" | "file_write" => {
                let Some((artifact_id, content_hash)) = digest_artifact_and_hash(digest) else {
                    return true;
                };
                return !verified_reads.iter().any(|(read_artifact_id, read_hash)| {
                    read_artifact_id == &artifact_id && read_hash == &content_hash
                });
            }
            _ => {}
        }
    }

    false
}

fn digest_artifact_and_hash(digest: &ToolDigest) -> Option<(String, String)> {
    let metadata = digest.metadata.as_ref()?.as_object()?;
    let content_hash = metadata
        .get("content_hash")
        .and_then(|value| value.as_str())?
        .to_string();
    let artifact_id =
        extract_artifact_id_from_metadata(metadata).or_else(|| digest.artifact_id.clone())?;
    Some((artifact_id, content_hash))
}

fn extract_artifact_id_from_metadata(metadata: &serde_json::Map<String, Value>) -> Option<String> {
    metadata
        .get("artifact_read")
        .and_then(|value| value.get("artifact_id"))
        .and_then(|value| value.as_str())
        .map(str::to_string)
        .or_else(|| {
            metadata
                .get("stored_artifacts")
                .and_then(|value| value.as_array())
                .and_then(|items| items.first())
                .and_then(|item| item.get("path"))
                .and_then(|value| value.as_str())
                .map(str::to_string)
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::context_engineering::TrackedArtifact;
    use crate::agents::ContextRunState;
    use serde_json::json;

    fn digest(tool_name: &str, status: &str) -> ToolDigest {
        ToolDigest {
            tool_name: tool_name.to_string(),
            status: status.to_string(),
            summary: String::new(),
            artifact_id: None,
            artifact_kind: None,
            preview_snippets: Vec::new(),
            metadata: None,
            created_at_ms: 0,
        }
    }

    fn digest_with_metadata(tool_name: &str, metadata: Value) -> ToolDigest {
        ToolDigest {
            tool_name: tool_name.to_string(),
            status: "ok".to_string(),
            summary: String::new(),
            artifact_id: None,
            artifact_kind: None,
            preview_snippets: Vec::new(),
            metadata: Some(metadata),
            created_at_ms: 0,
        }
    }

    #[test]
    fn adds_file_read_after_recent_file_write() {
        let run_state = ContextRunState {
            last_tool_digests: vec![digest("file_write", "ok")],
            ..Default::default()
        };
        let config = ToolConfig {
            max_tools: 4,
            ..Default::default()
        };

        let tool_ids =
            apply_file_read_bias(vec!["file_write".to_string()], Some(&run_state), &config);

        assert_eq!(
            tool_ids,
            vec!["file_write".to_string(), "file_read".to_string()]
        );
    }

    #[test]
    fn prefers_file_read_over_tool_search_when_budget_is_full() {
        let run_state = ContextRunState {
            last_tool_digests: vec![digest("file_edit", "ok")],
            ..Default::default()
        };
        let config = ToolConfig {
            max_tools: 2,
            ..Default::default()
        };

        let tool_ids = apply_file_read_bias(
            vec!["tool_search".to_string(), "file_edit".to_string()],
            Some(&run_state),
            &config,
        );

        assert_eq!(
            tool_ids,
            vec!["file_edit".to_string(), "file_read".to_string()]
        );
    }

    #[test]
    fn skips_bias_when_recent_relevant_file_tool_was_read() {
        let run_state = ContextRunState {
            last_tool_digests: vec![
                digest_with_metadata(
                    "file_write",
                    json!({
                        "content_hash": "abc123",
                        "stored_artifacts": [{
                            "path": "/tmp/out.txt"
                        }]
                    }),
                ),
                digest_with_metadata(
                    "file_read",
                    json!({
                        "content_hash": "abc123",
                        "artifact_read": {
                            "artifact_id": "/tmp/out.txt"
                        }
                    }),
                ),
            ],
            ..Default::default()
        };
        let config = ToolConfig::default();

        let tool_ids =
            apply_file_read_bias(vec!["file_write".to_string()], Some(&run_state), &config);

        assert_eq!(tool_ids, vec!["file_write".to_string()]);
    }

    #[test]
    fn keeps_bias_when_file_read_hash_does_not_match_recent_write() {
        let run_state = ContextRunState {
            last_tool_digests: vec![
                digest_with_metadata(
                    "file_write",
                    json!({
                        "content_hash": "abc123",
                        "stored_artifacts": [{
                            "path": "/tmp/out.txt"
                        }]
                    }),
                ),
                digest_with_metadata(
                    "file_read",
                    json!({
                        "content_hash": "different",
                        "artifact_read": {
                            "artifact_id": "/tmp/out.txt"
                        }
                    }),
                ),
            ],
            ..Default::default()
        };
        let config = ToolConfig::default();

        let tool_ids =
            apply_file_read_bias(vec!["file_write".to_string()], Some(&run_state), &config);

        assert_eq!(
            tool_ids,
            vec!["file_write".to_string(), "file_read".to_string()]
        );
    }

    #[test]
    fn skips_bias_when_file_read_is_not_allowed() {
        let run_state = ContextRunState {
            last_tool_digests: vec![digest("file_write", "ok")],
            ..Default::default()
        };
        let config = ToolConfig {
            allowed_tools: vec!["file_write".to_string()],
            ..Default::default()
        };

        let tool_ids =
            apply_file_read_bias(vec!["file_write".to_string()], Some(&run_state), &config);

        assert_eq!(tool_ids, vec!["file_write".to_string()]);
    }

    #[test]
    fn prioritizes_file_read_for_incomplete_host_artifacts() {
        let run_state = ContextRunState {
            tracked_artifacts: vec![TrackedArtifact {
                artifact_id: "/tmp/out.txt".to_string(),
                artifact_kind: "file".to_string(),
                storage_backend: "host".to_string(),
                source_tool: "http_request".to_string(),
                source_slot: Some("body".to_string()),
                size_bytes: Some(4096),
                total_lines: Some(380),
                read_ranges: vec![],
                contiguous_read_through_line: 0,
                last_read_start_line: None,
                last_read_end_line: None,
                fully_read: false,
                created_at_ms: 1,
                updated_at_ms: 2,
            }],
            ..Default::default()
        };
        let config = ToolConfig {
            max_tools: 4,
            ..Default::default()
        };

        let tool_ids =
            apply_file_read_bias(vec!["http_request".to_string()], Some(&run_state), &config);

        assert_eq!(
            tool_ids,
            vec!["http_request".to_string(), "file_read".to_string()]
        );
    }

    #[test]
    fn adds_browser_shell_when_session_is_bound() {
        let config = ToolConfig {
            max_tools: 4,
            ..Default::default()
        };

        let tool_ids = apply_browser_shell_bias(
            vec!["shell".to_string(), "tool_search".to_string()],
            "inspect current state",
            Some("browser-shell-1"),
            &config,
        );

        assert_eq!(
            tool_ids,
            vec![
                "shell".to_string(),
                "tool_search".to_string(),
                "browser_shell".to_string()
            ]
        );
    }

    #[test]
    fn prefers_browser_shell_over_tool_search_when_budget_is_full() {
        let config = ToolConfig {
            max_tools: 2,
            ..Default::default()
        };

        let tool_ids = apply_browser_shell_bias(
            vec!["tool_search".to_string(), "shell".to_string()],
            "inspect current state",
            Some("browser-shell-1"),
            &config,
        );

        assert_eq!(
            tool_ids,
            vec!["shell".to_string(), "browser_shell".to_string()]
        );
    }

    #[test]
    fn removes_local_terminal_tools_when_task_targets_bound_browser_shell() {
        let config = ToolConfig {
            max_tools: 4,
            ..Default::default()
        };

        let tool_ids = apply_browser_shell_bias(
            vec!["shell".to_string(), "tool_search".to_string()],
            "在当前浏览器shell中执行一个ls",
            Some("browser-shell-1"),
            &config,
        );

        assert_eq!(
            tool_ids,
            vec!["tool_search".to_string(), "browser_shell".to_string()]
        );
    }
}
