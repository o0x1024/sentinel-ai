use std::sync::Arc;

use sentinel_tools::dynamic_tool::{DynamicTool, DynamicToolDef, ToolExecutor};
use serde_json::{json, Value};

const MODEL_TOOL_RESULT_MAX_CHARS: usize = 12_000;
const MODEL_TOOL_RESULT_FIELD_MAX_CHARS: usize = 4_000;
const MODEL_TOOL_RESULT_ARRAY_MAX_ITEMS: usize = 20;

pub(super) fn wrap_dynamic_tool_for_model_context(
    tool: DynamicTool,
    execution_id: &str,
) -> DynamicTool {
    let original_def = tool.def().clone();
    let original_tool = DynamicTool::new(original_def.clone());
    let tool_name = original_def.name.clone();
    let execution_id = execution_id.to_string();
    let executor: ToolExecutor = Arc::new(move |args: Value| {
        let original_tool = original_tool.clone();
        let tool_name = tool_name.clone();
        let execution_id = execution_id.clone();
        Box::pin(async move {
            use rig::tool::Tool;

            let result = original_tool
                .call(args)
                .await
                .map_err(|error| error.to_string())?;
            Ok(compact_tool_value_for_model_context(&tool_name, &execution_id, result).await)
        })
    });

    DynamicTool::new(DynamicToolDef {
        output_schema: None,
        executor,
        ..original_def
    })
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
    if original_chars <= MODEL_TOOL_RESULT_MAX_CHARS {
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
        .map(|value| value.chars().count() <= MODEL_TOOL_RESULT_MAX_CHARS)
        .unwrap_or(false)
    {
        return compacted;
    }

    let mut replacement = json!({
        "_context_microcompact": build_microcompact_metadata(tool_name, original_chars, stored_artifact.as_ref()),
        "preview": compact_text_preview(&rendered, MODEL_TOOL_RESULT_MAX_CHARS),
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
            if original_len > MODEL_TOOL_RESULT_ARRAY_MAX_ITEMS {
                items.truncate(MODEL_TOOL_RESULT_ARRAY_MAX_ITEMS);
                items.push(json!({
                    "_context_microcompact_array": {
                        "original_items": original_len,
                        "kept_items": MODEL_TOOL_RESULT_ARRAY_MAX_ITEMS
                    }
                }));
            }
            for nested in items.iter_mut() {
                compact_json_value_for_model(nested);
            }
        }
        Value::String(text) => {
            if text.chars().count() > MODEL_TOOL_RESULT_FIELD_MAX_CHARS {
                let original_chars = text.chars().count();
                *text = compact_text_preview(text, MODEL_TOOL_RESULT_FIELD_MAX_CHARS);
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
        let wrapped = wrap_dynamic_tool_for_model_context(DynamicTool::new(def), &execution_id);
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
        assert!(rendered.chars().count() <= MODEL_TOOL_RESULT_MAX_CHARS + 2_000);
    }
}
