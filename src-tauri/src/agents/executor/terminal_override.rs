use std::sync::Arc;

use sentinel_tools::dynamic_tool::{DynamicToolDef, ToolCategory, ToolExecutor, ToolSource};
use sentinel_tools::terminal::server::TerminalServer;
use sentinel_tools::terminal::unified_exec_tool::execute_interactive_shell;
use sentinel_tools::ToolServer;

use crate::agents::executor::terminal_session_store::{
    get_active_terminal_session, set_active_terminal_session,
};

pub(super) async fn build_interactive_shell_override_def(
    tool_server: &ToolServer,
    execution_id: &str,
    fallback_active_session_id: Option<&str>,
    working_directory: Option<&str>,
) -> Option<DynamicToolDef> {
    let info = tool_server.get_tool(TerminalServer::NAME).await?;
    let execution_id_for_terminal = execution_id.to_string();
    let fallback_active_session_id = fallback_active_session_id.map(str::to_string);
    let working_directory = working_directory.map(str::to_string);
    let input_schema = info.input_schema.clone();
    let description = info.description.clone();
    let execution_policy = info.execution_policy.clone();
    let terminal_executor: ToolExecutor = Arc::new(move |args: serde_json::Value| {
        let execution_id_for_terminal = execution_id_for_terminal.clone();
        let fallback_active_session_id = fallback_active_session_id.clone();
        let working_directory = working_directory.clone();
        Box::pin(async move {
            let mut patched_args = args;

            if let Some(obj) = patched_args.as_object_mut() {
                obj.insert(
                    "execution_id".to_string(),
                    serde_json::Value::String(execution_id_for_terminal.clone()),
                );
                if !obj.contains_key("session_policy") {
                    obj.insert(
                        "session_policy".to_string(),
                        serde_json::Value::String("reuse".to_string()),
                    );
                }
                if let Some(active_session_id) =
                    get_active_terminal_session(&execution_id_for_terminal)
                        .or_else(|| fallback_active_session_id.clone())
                {
                    obj.insert(
                        "active_session_id".to_string(),
                        serde_json::Value::String(active_session_id),
                    );
                }
                if let Some(working_directory) = working_directory.clone() {
                    obj.insert(
                        "working_dir".to_string(),
                        serde_json::Value::String(working_directory),
                    );
                }
            }

            let output = execute_interactive_shell(
                patched_args,
                Some(execution_id_for_terminal.clone()),
                fallback_active_session_id.clone(),
                working_directory.clone(),
            )
            .await?;

            if let Some(session_id) = output.get("session_id").and_then(|value| value.as_str()) {
                set_active_terminal_session(&execution_id_for_terminal, Some(session_id));
            }

            Ok(output)
        })
    });

    Some(DynamicToolDef {
        name: TerminalServer::NAME.to_string(),
        description,
        input_schema,
        output_schema: None,
        source: ToolSource::Builtin,
        category: ToolCategory::Terminal,
        tags: info.tags.clone(),
        search_hint: info.search_hint.clone(),
        exposure: info.exposure.clone(),
        execution_policy,
        executor: terminal_executor,
    })
}
