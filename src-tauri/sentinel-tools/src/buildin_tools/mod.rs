pub mod browser;
pub mod http_request;
pub mod local_time;
pub mod memory;
pub mod ocr;
pub mod port_scan;
pub mod shell;
pub mod subdomain_brute;
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
pub use subdomain_brute::SubdomainBruteTool;
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
    ];
    
    let mut definitions = Vec::new();
    for tool in tools {
        definitions.push(tool.definition(String::new()).await);
    }
    definitions
}

