use std::sync::Arc;

use sentinel_tools::buildin_tools::AskUserQuestionTool;
use sentinel_tools::dynamic_tool::{DynamicToolDef, ToolExecutor, ToolSource};
use sentinel_tools::ToolServer;

pub(super) async fn build_ask_user_question_override_def(
    tool_server: &ToolServer,
    execution_id: &str,
) -> Option<DynamicToolDef> {
    let info = tool_server.get_tool(AskUserQuestionTool::NAME).await?;
    let execution_id_for_questions = execution_id.to_string();
    let input_schema = info.input_schema.clone();
    let description = info.description.clone();
    let executor: ToolExecutor = Arc::new(move |args: serde_json::Value| {
        let execution_id_for_questions = execution_id_for_questions.clone();
        Box::pin(async move {
            use rig::tool::Tool;
            use sentinel_tools::buildin_tools::ask_user_question::{
                AskUserQuestionArgs, AskUserQuestionTool,
            };

            let mut patched_args = args;
            if let Some(obj) = patched_args.as_object_mut() {
                obj.insert(
                    "execution_id".to_string(),
                    serde_json::Value::String(execution_id_for_questions.clone()),
                );
            }

            let tool_args: AskUserQuestionArgs = serde_json::from_value(patched_args)
                .map_err(|e| format!("Invalid arguments: {}", e))?;

            let tool = AskUserQuestionTool::new();
            let result = tool
                .call(tool_args)
                .await
                .map_err(|e| format!("AskUserQuestion failed: {}", e))?;

            serde_json::to_value(result)
                .map_err(|e| format!("Failed to serialize ask_user_question result: {}", e))
        })
    });

    Some(DynamicToolDef {
        name: AskUserQuestionTool::NAME.to_string(),
        description,
        input_schema,
        output_schema: None,
        source: ToolSource::Builtin,
        category: "system".to_string(),
        tags: info.tags.clone(),
        search_hint: info.search_hint.clone(),
        exposure: info.exposure.clone(),
        execution_policy: info.execution_policy.clone(),
        executor,
    })
}
