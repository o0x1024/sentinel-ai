use std::sync::{Arc, Mutex};

use crate::agents::executor::types::ToolCallRecord;

type PendingToolCalls = std::collections::HashMap<String, (String, String, i64, u32)>;

pub(super) fn accumulate_progress(
    tool_calls_collector: &Arc<Mutex<Vec<ToolCallRecord>>>,
    accumulated_tool_calls: &Arc<Mutex<Vec<ToolCallRecord>>>,
    assistant_segment_buf: &Arc<Mutex<String>>,
    accumulated_assistant_output: &Arc<Mutex<String>>,
    pending: Option<&Arc<Mutex<PendingToolCalls>>>,
) {
    if let Ok(current_calls) = tool_calls_collector.lock() {
        if let Ok(mut acc) = accumulated_tool_calls.lock() {
            acc.extend(current_calls.clone());
        }
    }

    if let Ok(current_output) = assistant_segment_buf.lock() {
        if !current_output.is_empty() {
            if let Ok(mut acc) = accumulated_assistant_output.lock() {
                if !acc.is_empty() {
                    acc.push_str("\n\n");
                }
                acc.push_str(current_output.as_str());
            }
        }
    }

    if let Some(pending) = pending {
        if let Ok(mut pending_map) = pending.lock() {
            pending_map.clear();
        }
    }
}
