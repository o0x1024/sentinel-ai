use crate::tool_args::tool_call_argument_repair_message;
use rig::agent::{PromptHook, ToolCallHookAction};
use rig::completion::CompletionModel;
use std::future::Future;

#[derive(Debug, Clone, Default)]
pub struct ToolArgumentGuardHook;

impl<M> PromptHook<M> for ToolArgumentGuardHook
where
    M: CompletionModel,
{
    fn on_tool_call(
        &self,
        tool_name: &str,
        _tool_call_id: Option<String>,
        _internal_call_id: &str,
        args: &str,
    ) -> impl Future<Output = ToolCallHookAction> + rig::wasm_compat::WasmCompatSend {
        let repair_message = tool_call_argument_repair_message(tool_name, args);
        async move {
            match repair_message {
                Some(reason) => ToolCallHookAction::skip(reason),
                None => ToolCallHookAction::cont(),
            }
        }
    }
}
