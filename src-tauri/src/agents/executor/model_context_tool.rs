use std::sync::{Arc, Mutex};

use sentinel_tools::dynamic_tool::{DynamicTool, DynamicToolDef, ToolExecutor};
use serde_json::{json, Value};

use super::tool_result_limits::{
    TOOL_RESULT_ARRAY_MAX_ITEMS, TOOL_RESULT_FIELD_MAX_CHARS, TOOL_RESULT_MAX_CHARS,
};

/// Holds skill metadata extracted from a `skills` invoke result for executor config
/// and compaction re-injection tracking.
#[derive(Debug, Clone)]
pub(super) struct PendingSkillInjection {
    pub skill_id: String,
    pub skill_name: String,
    pub body: String,
    pub allowed_tools: Vec<String>,
    pub model_override: Option<String>,
    pub effort: Option<String>,
}

pub(super) type SkillInjectionSink = Arc<Mutex<Vec<PendingSkillInjection>>>;

pub(super) fn new_skill_injection_sink() -> SkillInjectionSink {
    Arc::new(Mutex::new(Vec::new()))
}

pub(super) fn wrap_dynamic_tool_for_model_context(
    tool: DynamicTool,
    execution_id: &str,
    skill_injection_sink: Option<SkillInjectionSink>,
) -> DynamicTool {
    let original_def = tool.def().clone();
    let original_tool = DynamicTool::new(original_def.clone());
    let tool_name = original_def.name.clone();
    let execution_id = execution_id.to_string();
    let executor: ToolExecutor = Arc::new(move |args: Value| {
        let original_tool = original_tool.clone();
        let tool_name = tool_name.clone();
        let execution_id = execution_id.clone();
        let sink = skill_injection_sink.clone();
        Box::pin(async move {
            use rig::tool::Tool;

            let result = original_tool
                .call(args)
                .await
                .map_err(|error| error.to_string())?;

            let result =
                extract_skill_metadata_for_executor(&tool_name, result, sink.as_ref()).await;

            if should_skip_compaction_for_tool_result(&tool_name, &result) {
                return Ok(result);
            }

            Ok(compact_tool_value_for_model_context(&tool_name, &execution_id, result).await)
        })
    });

    DynamicTool::new(DynamicToolDef {
        output_schema: None,
        executor,
        ..original_def
    })
}

fn should_skip_compaction_for_tool_result(tool_name: &str, result: &Value) -> bool {
    tool_name == "skills"
        && result
            .get("action")
            .and_then(|value| value.as_str())
            == Some("invoke")
}

/// If this is a `skills.invoke` result, extract executor metadata into the sink while
/// keeping the full inline skill body in the tool result for the model.
async fn extract_skill_metadata_for_executor(
    tool_name: &str,
    mut result: Value,
    sink: Option<&SkillInjectionSink>,
) -> Value {
    if tool_name != "skills" {
        return result;
    }
    let Some(obj) = result.as_object_mut() else {
        return result;
    };
    let action = obj
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    if action != "invoke" {
        return result;
    }
    let body = obj
        .get("content")
        .and_then(|v| v.as_str())
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .unwrap_or_default();
    if body.is_empty() {
        return result;
    }
    let skill_id = obj
        .get("skill")
        .and_then(|s| s.get("id"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let skill_name = obj
        .get("skill")
        .and_then(|s| s.get("name"))
        .and_then(|v| v.as_str())
        .unwrap_or(&skill_id)
        .to_string();
    let allowed_tools = obj
        .get("allowed_tools")
        .and_then(|v| v.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(str::to_string))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let model_override = obj
        .get("model_override")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    let effort = obj.get("effort").and_then(|v| v.as_str()).map(str::to_string);
    obj.remove("allowed_tools");
    obj.remove("model_override");
    obj.remove("effort");
    if let Some(sink) = sink {
        if let Ok(mut guard) = sink.lock() {
            let tracking_body =
                crate::agents::skills_injection::extract_skill_body_from_inline_tool_result(
                    &body,
                );
            tracing::info!(
                "Tracked skill metadata for inline tool result: skill_id={}, skill_name={}, body_chars={}",
                skill_id,
                skill_name,
                tracking_body.len()
            );
            guard.push(PendingSkillInjection {
                skill_id,
                skill_name,
                body: tracking_body,
                allowed_tools,
                model_override,
                effort,
            });
        }
    }
    result
}

async fn compact_tool_value_for_model_context(
    tool_name: &str,
    execution_id: &str,
    value: Value,
) -> Value {
    let Ok(rendered) = serde_json::to_string(&value) else {
        return value;
    };
    let original_chars = rendered.chars().count();
    if original_chars <= TOOL_RESULT_MAX_CHARS {
        return value;
    }

    let stored_artifact = store_raw_tool_result(tool_name, execution_id, &rendered).await;
    let mut compacted = value;
    compact_json_value_for_model(&mut compacted);
    if let Some(object) = compacted.as_object_mut() {
        object.insert(
            "_context_microcompact".to_string(),
            build_microcompact_metadata(tool_name, original_chars, stored_artifact.as_ref()),
        );
        if let Some(artifact) = stored_artifact.clone() {
            object.insert("stored_artifacts".to_string(), json!([artifact]));
        }
    }
    if serde_json::to_string(&compacted)
        .map(|value| value.chars().count() <= TOOL_RESULT_MAX_CHARS)
        .unwrap_or(false)
    {
        return compacted;
    }

    let mut replacement = json!({
        "_context_microcompact": build_microcompact_metadata(tool_name, original_chars, stored_artifact.as_ref()),
        "preview": compact_text_preview(&rendered, TOOL_RESULT_MAX_CHARS),
    });
    if let Some(artifact) = stored_artifact {
        replacement["stored_artifacts"] = json!([artifact]);
    }
    replacement
}

async fn store_raw_tool_result(
    tool_name: &str,
    execution_id: &str,
    rendered: &str,
) -> Option<Value> {
    let storage_tool_name = format!(
        "{}_raw_tool_result_{}",
        sanitize_tool_name(tool_name),
        uuid::Uuid::new_v4()
    );
    match sentinel_tools::output_storage::store_output_unified(
        &storage_tool_name,
        rendered,
        None,
        Some(execution_id),
    )
    .await
    {
        Ok(result) => result
            .to_stored_artifact("raw_tool_result")
            .and_then(|artifact| serde_json::to_value(artifact).ok()),
        Err(error) => {
            tracing::warn!(
                "Failed to store raw large tool result before model compaction - tool: {}, execution_id: {}, error: {}",
                tool_name,
                execution_id,
                error
            );
            None
        }
    }
}

fn build_microcompact_metadata(
    tool_name: &str,
    original_chars: usize,
    stored_artifact: Option<&Value>,
) -> Value {
    json!({
        "tool": tool_name,
        "original_chars": original_chars,
        "policy": "large raw tool result stored as artifact; model context receives bounded preview",
        "raw_result_artifact": stored_artifact,
    })
}

fn compact_json_value_for_model(value: &mut Value) {
    match value {
        Value::Object(object) => {
            for nested in object.values_mut() {
                compact_json_value_for_model(nested);
            }
        }
        Value::Array(items) => {
            let original_len = items.len();
            if original_len > TOOL_RESULT_ARRAY_MAX_ITEMS {
                items.truncate(TOOL_RESULT_ARRAY_MAX_ITEMS);
                items.push(json!({
                    "_context_microcompact_array": {
                        "original_items": original_len,
                        "kept_items": TOOL_RESULT_ARRAY_MAX_ITEMS
                    }
                }));
            }
            for nested in items.iter_mut() {
                compact_json_value_for_model(nested);
            }
        }
        Value::String(text) => {
            if text.chars().count() > TOOL_RESULT_FIELD_MAX_CHARS {
                let original_chars = text.chars().count();
                *text = compact_text_preview(text, TOOL_RESULT_FIELD_MAX_CHARS);
                text.push_str(&format!(
                    "\n[context microcompact: original field chars={}]",
                    original_chars
                ));
            }
        }
        _ => {}
    }
}

fn compact_text_preview(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }

    let half = max_chars / 2;
    let head = text.chars().take(half).collect::<String>();
    let tail = text
        .chars()
        .rev()
        .take(half)
        .collect::<String>()
        .chars()
        .rev()
        .collect::<String>();
    format!(
        "{}\n...[context microcompact: omitted middle chars]...\n{}",
        head, tail
    )
}

fn sanitize_tool_name(tool_name: &str) -> String {
    let sanitized = tool_name
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>();
    if sanitized.trim_matches('_').is_empty() {
        "tool".to_string()
    } else {
        sanitized
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sentinel_tools::dynamic_tool::{
        ToolCategory, ToolExecutionPolicy, ToolExposure, ToolSource,
    };

    #[tokio::test]
    async fn dynamic_tool_wrapper_stores_raw_result_and_compacts_model_context() {
        let executor: ToolExecutor = Arc::new(|_args: Value| {
            Box::pin(async move {
                Ok(json!({
                    "body": "x".repeat(80_000),
                    "items": (0..100).map(|index| json!({"index": index, "value": "y".repeat(1000)})).collect::<Vec<_>>(),
                }))
            })
        });
        let def = DynamicToolDef {
            name: "huge_tool".to_string(),
            description: "returns a huge payload".to_string(),
            input_schema: json!({"type": "object", "properties": {}}),
            output_schema: None,
            source: ToolSource::Builtin,
            category: ToolCategory::Other,
            tags: Vec::new(),
            search_hint: None,
            exposure: ToolExposure::Standard,
            execution_policy: ToolExecutionPolicy::default(),
            executor,
        };

        let execution_id = format!("test-{}", uuid::Uuid::new_v4());
        let wrapped =
            wrap_dynamic_tool_for_model_context(DynamicTool::new(def), &execution_id, None);
        let result = {
            use rig::tool::Tool;
            wrapped
                .call(json!({}))
                .await
                .expect("wrapped tool should execute")
        };
        let rendered = serde_json::to_string(&result).unwrap();

        assert!(rendered.contains("_context_microcompact"));
        assert!(rendered.contains("raw_result_artifact"));
        assert!(rendered.contains("stored_artifacts"));
        use crate::agents::executor::tool_result_limits::{
            TOOL_RESULT_COMPACT_SLACK_CHARS, TOOL_RESULT_MAX_CHARS,
        };
        assert!(
            rendered.chars().count() <= TOOL_RESULT_MAX_CHARS + TOOL_RESULT_COMPACT_SLACK_CHARS
        );
    }

    #[tokio::test]
    async fn fork_skill_result_is_not_tracked_for_metadata() {
        let sink = new_skill_injection_sink();
        let result = extract_skill_metadata_for_executor(
            "skills",
            json!({
                "action": "fork",
                "skill": {"id": "review", "name": "Review"},
                "content": "Skill completed in forked sub-agent.",
            }),
            Some(&sink),
        )
        .await;

        assert_eq!(result.get("action").and_then(|v| v.as_str()), Some("fork"));
        assert!(sink.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn invoke_skill_result_keeps_inline_content_and_skips_compaction() {
        let sink = new_skill_injection_sink();
        let inline_content = format!(
            "Skill loaded: audit\n\n<skill>\n<name>audit</name>\n<path>skill://audit/SKILL.md</path>\n{}\n</skill>",
            "x".repeat(20_000)
        );
        let result = extract_skill_metadata_for_executor(
            "skills",
            json!({
                "action": "invoke",
                "skill": {"id": "audit", "name": "audit"},
                "content": inline_content,
                "allowed_tools": ["grep"],
                "model_override": "gpt-4o",
            }),
            Some(&sink),
        )
        .await;

        assert!(result.get("content").and_then(|v| v.as_str()).is_some());
        assert!(result.get("allowed_tools").is_none());
        assert!(result.get("model_override").is_none());
        assert_eq!(sink.lock().unwrap().len(), 1);
        assert_eq!(sink.lock().unwrap()[0].body.chars().count(), 20_000);
        assert!(should_skip_compaction_for_tool_result("skills", &result));
    }
}
