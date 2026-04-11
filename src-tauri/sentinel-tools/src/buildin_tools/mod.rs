pub mod agent_control_tool;
pub mod ask_user_question;
pub mod http_request;
pub mod memory;
pub mod ocr;
pub mod search_exploit;
pub mod shell;
pub mod skills;
pub mod subagent_tool;
pub mod subdomain_brute;
pub mod tenth_man_tool;
pub mod todos;
pub mod web_search;

pub use agent_control_tool::{CloseAgentTool, ListAgentsTool, SpawnAgentTool, WaitAgentsTool};
pub use ask_user_question::AskUserQuestionTool;
pub use http_request::HttpRequestTool;
pub use memory::MemoryManagerTool;
pub use ocr::OcrTool;
pub use search_exploit::SearchExploitTool;
pub use shell::ShellTool;
pub use skills::SkillsTool;
pub use subdomain_brute::SubdomainBruteTool;
pub use tenth_man_tool::TenthManTool;
pub use todos::TodosTool;
pub use web_search::WebSearchTool;

use rig::tool::ToolSet;

/// Create a ToolSet with all builtin security tools
pub fn create_buildin_toolset() -> ToolSet {
    let mut toolset = ToolSet::default();
    toolset.add_tool(HttpRequestTool::default());
    toolset.add_tool(AskUserQuestionTool::new());
    toolset.add_tool(ShellTool::new());
    toolset.add_tool(TodosTool);
    toolset.add_tool(WebSearchTool::default());
    toolset.add_tool(SearchExploitTool);
    toolset.add_tool(MemoryManagerTool);
    toolset.add_tool(OcrTool);
    toolset.add_tool(SkillsTool);
    toolset.add_tool(SubdomainBruteTool);
    toolset.add_tool(SpawnAgentTool);
    toolset.add_tool(WaitAgentsTool);
    toolset.add_tool(ListAgentsTool);
    toolset.add_tool(CloseAgentTool);
    toolset
}

/// Get all builtin tool definitions
pub async fn get_tool_definitions() -> Vec<rig::completion::ToolDefinition> {
    let tools: Vec<Box<dyn rig::tool::ToolDyn>> = vec![
        Box::new(HttpRequestTool::default()),
        Box::new(AskUserQuestionTool::new()),
        Box::new(ShellTool::new()),
        Box::new(TodosTool),
        Box::new(WebSearchTool::default()),
        Box::new(SearchExploitTool),
        Box::new(MemoryManagerTool),
        Box::new(OcrTool),
        Box::new(SkillsTool),
        Box::new(SubdomainBruteTool),
        Box::new(SpawnAgentTool),
        Box::new(WaitAgentsTool),
        Box::new(ListAgentsTool),
        Box::new(CloseAgentTool),
    ];

    let mut definitions = Vec::new();
    for tool in tools {
        definitions.push(tool.definition(String::new()).await);
    }
    definitions
}
