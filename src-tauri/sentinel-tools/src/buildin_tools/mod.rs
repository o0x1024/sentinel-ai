pub mod audit_tools;
pub mod audit_tools_advanced;
pub mod browser;
pub mod cpg;
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
pub use audit_tools::{CallGraphLiteTool, CodeSearchTool, GitCloneRepoTool, GitDiffScopeTool, TaintSliceLiteTool};
pub use cpg::{BuildCpgTool, QueryCpgTool, CpgTaintAnalysisTool, CpgSecurityScanTool};
pub use cpg::tools::{try_get_cpg_audit_context, try_auto_build_cpg};
pub use audit_tools_advanced::{
    AuditCoverageTool, AuditReportTool, CrossFileTaintTool, DependencyAuditTool,
    ProjectOverviewTool, ReadFileTool,
};
pub use http_request::HttpRequestTool;
pub use local_time::LocalTimeTool;
pub use memory::MemoryManagerTool;
pub use ocr::OcrTool;
pub use port_scan::PortScanTool;
pub use shell::ShellTool;
pub use skills::SkillsTool;
pub use subdomain_brute::SubdomainBruteTool;
pub use subagent_tool::{
    SubagentAwaitTool, SubagentChannelTool, SubagentExecuteTool,
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
    toolset.add_tool(CodeSearchTool);
    toolset.add_tool(GitCloneRepoTool);
    toolset.add_tool(GitDiffScopeTool);
    toolset.add_tool(CallGraphLiteTool);
    toolset.add_tool(TaintSliceLiteTool);
    toolset.add_tool(ShellTool::new());
    toolset.add_tool(SubdomainBruteTool);
    toolset.add_tool(TodosTool);
    toolset.add_tool(WebSearchTool::default());
    toolset.add_tool(MemoryManagerTool);
    toolset.add_tool(OcrTool);
    toolset.add_tool(SkillsTool);
    // Advanced audit tools
    toolset.add_tool(ReadFileTool);
    toolset.add_tool(ProjectOverviewTool);
    toolset.add_tool(AuditCoverageTool);
    toolset.add_tool(DependencyAuditTool);
    toolset.add_tool(CrossFileTaintTool);
    toolset.add_tool(AuditReportTool);
    // Code Property Graph (CPG) tools
    toolset.add_tool(BuildCpgTool);
    toolset.add_tool(QueryCpgTool);
    toolset.add_tool(CpgTaintAnalysisTool);
    toolset.add_tool(CpgSecurityScanTool);
    // Condensed subagent tools
    toolset.add_tool(SubagentExecuteTool::new());
    toolset.add_tool(SubagentAwaitTool::new());
    toolset.add_tool(SubagentChannelTool::new());
    toolset
}

/// Get all builtin tool definitions
pub async fn get_tool_definitions() -> Vec<rig::completion::ToolDefinition> {
    let tools: Vec<Box<dyn rig::tool::ToolDyn>> = vec![
        Box::new(PortScanTool),
        Box::new(HttpRequestTool::default()),
        Box::new(LocalTimeTool),
        Box::new(CodeSearchTool),
        Box::new(GitCloneRepoTool),
        Box::new(GitDiffScopeTool),
        Box::new(CallGraphLiteTool),
        Box::new(TaintSliceLiteTool),
        Box::new(ShellTool::new()),
        Box::new(SubdomainBruteTool),
        Box::new(TodosTool),
        Box::new(WebSearchTool::default()),
        Box::new(MemoryManagerTool),
        Box::new(OcrTool),
        Box::new(SkillsTool),
        // Advanced audit tools
        Box::new(ReadFileTool),
        Box::new(ProjectOverviewTool),
        Box::new(AuditCoverageTool),
        Box::new(DependencyAuditTool),
        Box::new(CrossFileTaintTool),
        Box::new(AuditReportTool),
        // Code Property Graph (CPG) tools
        Box::new(BuildCpgTool),
        Box::new(QueryCpgTool),
        Box::new(CpgTaintAnalysisTool),
        Box::new(CpgSecurityScanTool),
        // Condensed subagent tools
        Box::new(SubagentExecuteTool::new()),
        Box::new(SubagentAwaitTool::new()),
        Box::new(SubagentChannelTool::new()),
    ];
    
    let mut definitions = Vec::new();
    for tool in tools {
        definitions.push(tool.definition(String::new()).await);
    }
    definitions
}
