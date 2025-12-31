//! Rig-core Tool implementations
//!
//! All tools implement the `rig::tool::Tool` trait for type-safe integration
//! with rig Agent framework.

pub mod port_scan;
pub mod http_request;
pub mod local_time;
pub mod shell;
pub mod subdomain_brute;
pub mod task_planner;
pub mod web_search;

pub use port_scan::PortScanTool;
pub use http_request::HttpRequestTool;
pub use local_time::LocalTimeTool;
pub use shell::ShellTool;
pub use subdomain_brute::SubdomainBruteTool;
pub use task_planner::TaskPlannerTool;
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
    toolset.add_tool(TaskPlannerTool);
    toolset.add_tool(WebSearchTool::default());
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
        Box::new(TaskPlannerTool),
        Box::new(WebSearchTool::default()),
    ];
    
    let mut definitions = Vec::new();
    for tool in tools {
        definitions.push(tool.definition(String::new()).await);
    }
    definitions
}

