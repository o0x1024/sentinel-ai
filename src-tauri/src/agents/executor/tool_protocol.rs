//! Tracks tool-call protocol closure for one streamed agent turn.

#[derive(Debug, Default)]
pub struct ToolProtocolTracker {
    pending_tool_call_count: usize,
    saw_tool_call: bool,
    waiting_for_final_assistant_after_tool_result: bool,
    final_assistant_after_last_tool_result: bool,
}

impl ToolProtocolTracker {
    pub fn observe_text(&mut self, text: &str) {
        if self.waiting_for_final_assistant_after_tool_result && !text.trim().is_empty() {
            self.final_assistant_after_last_tool_result = true;
            self.waiting_for_final_assistant_after_tool_result = false;
        }
    }

    pub fn observe_tool_call(&mut self) {
        self.saw_tool_call = true;
        self.pending_tool_call_count = self.pending_tool_call_count.saturating_add(1);
        self.final_assistant_after_last_tool_result = false;
        self.waiting_for_final_assistant_after_tool_result = false;
    }

    pub fn observe_tool_result(&mut self) {
        self.pending_tool_call_count = self.pending_tool_call_count.saturating_sub(1);
        self.final_assistant_after_last_tool_result = false;
        self.waiting_for_final_assistant_after_tool_result = true;
    }

    pub fn pending_tool_call_count(&self) -> usize {
        self.pending_tool_call_count
    }

    pub fn final_assistant_after_last_tool_result(&self, final_response: &str) -> bool {
        if !self.saw_tool_call {
            return true;
        }
        self.final_assistant_after_last_tool_result && !final_response.trim().is_empty()
    }
}
