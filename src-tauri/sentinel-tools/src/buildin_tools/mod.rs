pub mod agent_control_tool;
pub mod ask_user_question;
pub mod browser;
mod file_context;
pub mod file_edit;
pub mod file_read;
pub mod file_runtime;
pub mod file_write;
pub mod glob;
pub mod grep;
pub mod http_request;
pub mod lsp;
pub mod memory;
#[cfg(feature = "ocr")]
pub mod ocr;
pub mod plugin_authoring;
pub mod route_discovery;
pub mod search_exploit;
pub mod shell;
pub mod shell_background;
pub mod shell_policy;
pub mod skills;
#[cfg(feature = "db")]
pub mod sops;
pub mod subagent_tool;
#[cfg(feature = "plugins")]
pub mod subdomain_brute;
#[cfg(feature = "db")]
pub mod tasks;
pub mod tenth_man_tool;
mod text_change;
mod text_preview;
pub mod tool_search;
pub mod web_search;

pub use agent_control_tool::{CloseAgentTool, ListAgentsTool, SpawnAgentTool, WaitAgentsTool};
pub use ask_user_question::AskUserQuestionTool;
pub use browser::BrowserTool;
pub use file_edit::FileEditTool;
pub use file_read::FileReadTool;
pub use file_write::FileWriteTool;
pub use glob::GlobTool;
pub use grep::GrepTool;
pub use http_request::HttpRequestTool;
pub use lsp::LspTool;
pub use memory::MemoryManagerTool;
#[cfg(feature = "ocr")]
pub use ocr::OcrTool;
pub use plugin_authoring::PluginAuthoringTool;
pub use route_discovery::RouteDiscoveryTool;
pub use search_exploit::SearchExploitTool;
pub use shell::ShellTool;
pub use skills::SkillsTool;
#[cfg(feature = "db")]
pub use sops::{set_sops_app_handle, SopsTool};
#[cfg(feature = "plugins")]
pub use subdomain_brute::SubdomainBruteTool;
#[cfg(feature = "db")]
pub use tasks::TasksTool;
pub use tenth_man_tool::TenthManTool;
pub use tool_search::{
    load_tool_search_runtime_context, set_tool_search_context_provider, set_tool_search_executor,
    ToolSearchAction, ToolSearchArgs, ToolSearchMatch, ToolSearchOutput, ToolSearchRuntimeContext,
    ToolSearchTool,
};
pub use web_search::WebSearchTool;

use rig::tool::ToolSet;

/// Create a ToolSet with all builtin security tools
pub fn create_buildin_toolset() -> ToolSet {
    let mut toolset = ToolSet::default();
    toolset.add_tool(BrowserTool::default());
    toolset.add_tool(HttpRequestTool::default());
    toolset.add_tool(AskUserQuestionTool::new());
    toolset.add_tool(GlobTool);
    toolset.add_tool(GrepTool);
    toolset.add_tool(FileReadTool);
    toolset.add_tool(FileEditTool);
    toolset.add_tool(FileWriteTool);
    toolset.add_tool(LspTool);
    toolset.add_tool(ShellTool::new());
    #[cfg(feature = "db")]
    toolset.add_tool(TasksTool);
    toolset.add_tool(WebSearchTool::default());
    toolset.add_tool(RouteDiscoveryTool);
    toolset.add_tool(SearchExploitTool);
    toolset.add_tool(MemoryManagerTool);
    toolset.add_tool(PluginAuthoringTool);
    #[cfg(feature = "ocr")]
    toolset.add_tool(OcrTool);
    toolset.add_tool(SkillsTool);
    #[cfg(feature = "plugins")]
    toolset.add_tool(SubdomainBruteTool);
    toolset.add_tool(ToolSearchTool);
    toolset.add_tool(SpawnAgentTool);
    toolset.add_tool(WaitAgentsTool);
    toolset.add_tool(ListAgentsTool);
    toolset.add_tool(CloseAgentTool);
    toolset
}

/// Create a minimal ToolSet for unattended contest solving.
pub fn create_contest_toolset() -> ToolSet {
    let mut toolset = ToolSet::default();
    toolset.add_tool(BrowserTool::default());
    toolset.add_tool(HttpRequestTool::default());
    toolset.add_tool(RouteDiscoveryTool);
    toolset.add_tool(ShellTool::new());
    toolset.add_tool(TenthManTool::new());
    toolset
}

/// Get all builtin tool definitions
pub async fn get_tool_definitions() -> Vec<rig::completion::ToolDefinition> {
    let tools: Vec<Box<dyn rig::tool::ToolDyn>> = vec![
        Box::new(BrowserTool::default()),
        Box::new(HttpRequestTool::default()),
        Box::new(AskUserQuestionTool::new()),
        Box::new(GlobTool),
        Box::new(GrepTool),
        Box::new(FileReadTool),
        Box::new(FileEditTool),
        Box::new(FileWriteTool),
        Box::new(LspTool),
        Box::new(ShellTool::new()),
        #[cfg(feature = "db")]
        Box::new(TasksTool),
        Box::new(WebSearchTool::default()),
        Box::new(RouteDiscoveryTool),
        Box::new(SearchExploitTool),
        Box::new(MemoryManagerTool),
        Box::new(PluginAuthoringTool),
        #[cfg(feature = "ocr")]
        Box::new(OcrTool),
        Box::new(SkillsTool),
        #[cfg(feature = "plugins")]
        Box::new(SubdomainBruteTool),
        Box::new(ToolSearchTool),
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

/// Get contest-only builtin tool definitions.
pub async fn get_contest_tool_definitions() -> Vec<rig::completion::ToolDefinition> {
    let tools: Vec<Box<dyn rig::tool::ToolDyn>> = vec![
        Box::new(BrowserTool::default()),
        Box::new(HttpRequestTool::default()),
        Box::new(RouteDiscoveryTool),
        Box::new(ShellTool::new()),
        Box::new(TenthManTool::new()),
    ];

    let mut definitions = Vec::new();
    for tool in tools {
        definitions.push(tool.definition(String::new()).await);
    }
    definitions
}
