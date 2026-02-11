pub mod browser;
pub mod http_request;
pub mod local_time;
pub mod memory;
pub mod ocr;
pub mod port_scan;
pub mod shell;
pub mod skills;
pub mod subdomain_brute;
pub mod subagent_tool;
pub mod tenth_man_tool;
pub mod todos;
pub mod web_search;

pub use browser::*;
pub use http_request::HttpRequestTool;
pub use local_time::LocalTimeTool;
pub use memory::MemoryManagerTool;
pub use ocr::OcrTool;
pub use port_scan::PortScanTool;
pub use shell::ShellTool;
pub use skills::SkillsTool;
pub use subdomain_brute::SubdomainBruteTool;
pub use subagent_tool::{
    SubagentTool, SubagentRunTool, SubagentSpawnTool, SubagentWaitTool, SubagentWaitAnyTool,
    SubagentWorkflowRunTool,
    SubagentStatePutTool, SubagentStateGetTool,
    SubagentEventPublishTool, SubagentEventPollTool,
};
pub use tenth_man_tool::TenthManTool;
pub use todos::TodosTool;
pub use web_search::WebSearchTool;

use rig::tool::ToolSet;

/// Create a ToolSet with all builtin security tools
pub fn create_buildin_toolset() -> ToolSet {
    let mut toolset = ToolSet::default();
    toolset.add_tool(PortScanTool);
    toolset.add_tool(HttpRequestTool::default());
    toolset.add_tool(LocalTimeTool);
    toolset.add_tool(ShellTool::new());
    toolset.add_tool(SubdomainBruteTool);
    toolset.add_tool(TodosTool);
    toolset.add_tool(WebSearchTool::default());
    toolset.add_tool(MemoryManagerTool);
    toolset.add_tool(OcrTool);
    toolset.add_tool(SkillsTool);
    // Subagent tools: spawn (async), wait, run (sync/legacy)
    toolset.add_tool(SubagentSpawnTool::new());
    toolset.add_tool(SubagentWaitTool::new());
    toolset.add_tool(SubagentWaitAnyTool::new());
    toolset.add_tool(SubagentRunTool::new());
    toolset.add_tool(SubagentWorkflowRunTool::new());
    toolset.add_tool(SubagentStatePutTool::new());
    toolset.add_tool(SubagentStateGetTool::new());
    toolset.add_tool(SubagentEventPublishTool::new());
    toolset.add_tool(SubagentEventPollTool::new());
    toolset
}

/// Get all builtin tool definitions
pub async fn get_tool_definitions() -> Vec<rig::completion::ToolDefinition> {
    let tools: Vec<Box<dyn rig::tool::ToolDyn>> = vec![
        Box::new(PortScanTool),
        Box::new(HttpRequestTool::default()),
        Box::new(LocalTimeTool),
        Box::new(ShellTool::new()),
        Box::new(SubdomainBruteTool),
        Box::new(TodosTool),
        Box::new(WebSearchTool::default()),
        Box::new(MemoryManagerTool),
        Box::new(OcrTool),
        Box::new(SkillsTool),
        // Subagent tools
        Box::new(SubagentSpawnTool::new()),
        Box::new(SubagentWaitTool::new()),
        Box::new(SubagentWaitAnyTool::new()),
        Box::new(SubagentRunTool::new()),
        Box::new(SubagentWorkflowRunTool::new()),
        Box::new(SubagentStatePutTool::new()),
        Box::new(SubagentStateGetTool::new()),
        Box::new(SubagentEventPublishTool::new()),
        Box::new(SubagentEventPollTool::new()),
    ];
    
    let mut definitions = Vec::new();
    for tool in tools {
        definitions.push(tool.definition(String::new()).await);
    }
    definitions
}
