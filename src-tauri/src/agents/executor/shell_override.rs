use std::sync::Arc;

use sentinel_tools::buildin_tools::ShellTool;
use sentinel_tools::dynamic_tool::{DynamicToolDef, ToolExecutor, ToolSource};
use sentinel_tools::ToolServer;

pub(super) async fn build_shell_override_def(
    tool_server: &ToolServer,
    execution_id: &str,
) -> Option<DynamicToolDef> {
    let shell_info = tool_server.get_tool(ShellTool::NAME).await?;
    let execution_id_for_shell = execution_id.to_string();
    let shell_input_schema = shell_info.input_schema.clone();
    let shell_description = shell_info.description.clone();
    let shell_executor: ToolExecutor = Arc::new(move |args: serde_json::Value| {
        let execution_id_for_shell = execution_id_for_shell.clone();
        Box::pin(async move {
            use rig::tool::Tool;
            use sentinel_tools::buildin_tools::shell::{ShellArgs, ShellTool};

            let mut patched_args = args;
            if let Some(obj) = patched_args.as_object_mut() {
                obj.insert(
                    "execution_id".to_string(),
                    serde_json::Value::String(execution_id_for_shell.clone()),
                );
                obj.insert(
                    "enable_large_output_storage".to_string(),
                    serde_json::Value::Bool(true),
                );
            }

            let tool_args: ShellArgs = serde_json::from_value(patched_args)
                .map_err(|e| format!("Invalid arguments: {}", e))?;

            let tool = ShellTool::new();
            let result = tool
                .call(tool_args)
                .await
                .map_err(|e| format!("Shell execution failed: {}", e))?;

            serde_json::to_value(result)
                .map_err(|e| format!("Failed to serialize shell result: {}", e))
        })
    });

    Some(DynamicToolDef {
        name: ShellTool::NAME.to_string(),
        description: shell_description,
        input_schema: shell_input_schema,
        output_schema: None,
        source: ToolSource::Builtin,
        category: "system".to_string(),
        tags: shell_info.tags.clone(),
        search_hint: shell_info.search_hint.clone(),
        exposure: shell_info.exposure.clone(),
        execution_policy: shell_info.execution_policy.clone(),
        executor: shell_executor,
    })
}
