use std::sync::Arc;

use sentinel_tools::buildin_tools::ToolSearchTool;
use sentinel_tools::dynamic_tool::{DynamicToolDef, ToolCategory, ToolExecutor, ToolSource};
use sentinel_tools::ToolServer;

pub(super) async fn build_tool_search_override_def(
    tool_server: &ToolServer,
    execution_id: &str,
) -> Option<DynamicToolDef> {
    let info = tool_server.get_tool(ToolSearchTool::NAME).await?;
    let execution_id_for_tool_search = execution_id.to_string();
    let input_schema = info.input_schema.clone();
    let description = info.description.clone();
    let execution_policy = info.execution_policy.clone();
    let executor: ToolExecutor = Arc::new(move |args: serde_json::Value| {
        let execution_id_for_tool_search = execution_id_for_tool_search.clone();
        Box::pin(async move {
            let tool_server = sentinel_tools::get_tool_server();
            let mut patched_args = args;
            if let Some(obj) = patched_args.as_object_mut() {
                obj.insert(
                    "execution_id".to_string(),
                    serde_json::Value::String(execution_id_for_tool_search.clone()),
                );
            }

            let result = tool_server
                .execute(ToolSearchTool::NAME, patched_args)
                .await;
            if !result.success {
                return Err(result
                    .error
                    .unwrap_or_else(|| "tool_search execution failed".to_string()));
            }

            result
                .output
                .ok_or_else(|| "tool_search returned no output".to_string())
        })
    });

    Some(DynamicToolDef {
        name: ToolSearchTool::NAME.to_string(),
        description,
        input_schema,
        output_schema: info.output_schema.clone(),
        source: ToolSource::Builtin,
        category: ToolCategory::KnowledgeExtension,
        tags: info.tags.clone(),
        search_hint: info.search_hint.clone(),
        exposure: info.exposure.clone(),
        execution_policy,
        executor,
    })
}
