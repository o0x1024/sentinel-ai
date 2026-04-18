use std::collections::HashSet;

use sentinel_tools::buildin_tools::{
    AskUserQuestionTool, CloseAgentTool, FileEditTool, FileReadTool, FileWriteTool, GlobTool,
    GrepTool, HttpRequestTool, ListAgentsTool, LspTool, MemoryManagerTool, OcrTool,
    SearchExploitTool, ShellTool, SkillsTool, SpawnAgentTool, TenthManTool, TodosTool,
    ToolSearchTool, WaitAgentsTool, WebSearchTool,
};
use sentinel_tools::terminal::server::TerminalServer;

use super::types::{ToolCategory, ToolCost, ToolExposure, ToolMetadata};

pub fn build_default_tools() -> Vec<ToolMetadata> {
    vec![
        ToolMetadata {
            id: AskUserQuestionTool::NAME.to_string(),
            name: AskUserQuestionTool::NAME.to_string(),
            description: AskUserQuestionTool::DESCRIPTION.to_string(),
            category: ToolCategory::System,
            tags: vec![
                "question".to_string(),
                "clarification".to_string(),
                "user".to_string(),
                "decision".to_string(),
            ],
            search_hint: Some("ask the user a focused follow-up question".to_string()),
            cost_estimate: ToolCost::Low,
            always_available: true,
            exposure: ToolExposure::Always,
        },
        ToolMetadata {
            id: ToolSearchTool::NAME.to_string(),
            name: ToolSearchTool::NAME.to_string(),
            description: ToolSearchTool::DESCRIPTION.to_string(),
            category: ToolCategory::System,
            tags: vec![
                "tool".to_string(),
                "search".to_string(),
                "catalog".to_string(),
                "activate".to_string(),
                "discover".to_string(),
            ],
            search_hint: Some("find hidden or deferred tools and activate them".to_string()),
            cost_estimate: ToolCost::Low,
            always_available: true,
            exposure: ToolExposure::Always,
        },
        ToolMetadata {
            id: ShellTool::NAME.to_string(),
            name: ShellTool::NAME.to_string(),
            description: ShellTool::DESCRIPTION.to_string(),
            category: ToolCategory::System,
            tags: vec![
                "shell".to_string(),
                "command".to_string(),
                "execute".to_string(),
                "bash".to_string(),
            ],
            search_hint: Some("run one-off shell commands".to_string()),
            cost_estimate: ToolCost::Medium,
            always_available: true,
            exposure: ToolExposure::Core,
        },
        ToolMetadata {
            id: GlobTool::NAME.to_string(),
            name: GlobTool::NAME.to_string(),
            description: GlobTool::DESCRIPTION.to_string(),
            category: ToolCategory::Utility,
            tags: vec![
                "file".to_string(),
                "glob".to_string(),
                "path".to_string(),
                "wildcard".to_string(),
            ],
            search_hint: Some("find files by wildcard path pattern".to_string()),
            cost_estimate: ToolCost::Low,
            always_available: false,
            exposure: ToolExposure::Deferred,
        },
        ToolMetadata {
            id: GrepTool::NAME.to_string(),
            name: GrepTool::NAME.to_string(),
            description: GrepTool::DESCRIPTION.to_string(),
            category: ToolCategory::Utility,
            tags: vec![
                "search".to_string(),
                "grep".to_string(),
                "regex".to_string(),
                "content".to_string(),
            ],
            search_hint: Some("search file contents with a regex pattern".to_string()),
            cost_estimate: ToolCost::Low,
            always_available: false,
            exposure: ToolExposure::Deferred,
        },
        ToolMetadata {
            id: FileReadTool::NAME.to_string(),
            name: FileReadTool::NAME.to_string(),
            description: FileReadTool::DESCRIPTION.to_string(),
            category: ToolCategory::Utility,
            tags: vec![
                "file".to_string(),
                "read".to_string(),
                "code".to_string(),
                "lines".to_string(),
            ],
            search_hint: Some("read a text file with line-range controls".to_string()),
            cost_estimate: ToolCost::Low,
            always_available: false,
            exposure: ToolExposure::Deferred,
        },
        ToolMetadata {
            id: FileEditTool::NAME.to_string(),
            name: FileEditTool::NAME.to_string(),
            description: FileEditTool::DESCRIPTION.to_string(),
            category: ToolCategory::Utility,
            tags: vec![
                "file".to_string(),
                "edit".to_string(),
                "replace".to_string(),
                "patch".to_string(),
            ],
            search_hint: Some("edit an existing text file by exact string replacement".to_string()),
            cost_estimate: ToolCost::Medium,
            always_available: false,
            exposure: ToolExposure::Deferred,
        },
        ToolMetadata {
            id: FileWriteTool::NAME.to_string(),
            name: FileWriteTool::NAME.to_string(),
            description: FileWriteTool::DESCRIPTION.to_string(),
            category: ToolCategory::Utility,
            tags: vec![
                "file".to_string(),
                "write".to_string(),
                "create".to_string(),
                "overwrite".to_string(),
            ],
            search_hint: Some("create a file or overwrite one when explicitly allowed".to_string()),
            cost_estimate: ToolCost::Medium,
            always_available: false,
            exposure: ToolExposure::Deferred,
        },
        ToolMetadata {
            id: LspTool::NAME.to_string(),
            name: LspTool::NAME.to_string(),
            description: LspTool::DESCRIPTION.to_string(),
            category: ToolCategory::Utility,
            tags: vec![
                "code".to_string(),
                "symbol".to_string(),
                "definition".to_string(),
                "reference".to_string(),
                "navigation".to_string(),
            ],
            search_hint: Some(
                "navigate source code by symbols, definitions, and references".to_string(),
            ),
            cost_estimate: ToolCost::Low,
            always_available: false,
            exposure: ToolExposure::Deferred,
        },
        ToolMetadata {
            id: TerminalServer::NAME.to_string(),
            name: TerminalServer::NAME.to_string(),
            description: TerminalServer::DESCRIPTION.to_string(),
            category: ToolCategory::System,
            tags: vec![
                "terminal".to_string(),
                "interactive".to_string(),
                "session".to_string(),
                "persistent".to_string(),
                "shell".to_string(),
            ],
            search_hint: Some("run persistent interactive terminal sessions".to_string()),
            cost_estimate: ToolCost::Low,
            always_available: false,
            exposure: ToolExposure::Deferred,
        },
        ToolMetadata {
            id: HttpRequestTool::NAME.to_string(),
            name: HttpRequestTool::NAME.to_string(),
            description: HttpRequestTool::DESCRIPTION.to_string(),
            category: ToolCategory::Network,
            tags: vec!["http".to_string(), "request".to_string(), "api".to_string()],
            search_hint: Some("send direct HTTP requests to known URLs".to_string()),
            cost_estimate: ToolCost::Medium,
            always_available: false,
            exposure: ToolExposure::Core,
        },
        ToolMetadata {
            id: WebSearchTool::NAME.to_string(),
            name: WebSearchTool::NAME.to_string(),
            description: WebSearchTool::DESCRIPTION.to_string(),
            category: ToolCategory::Network,
            tags: vec![
                "search".to_string(),
                "web".to_string(),
                "internet".to_string(),
                "research".to_string(),
            ],
            search_hint: Some("search the public web for current information".to_string()),
            cost_estimate: ToolCost::Medium,
            always_available: false,
            exposure: ToolExposure::Deferred,
        },
        ToolMetadata {
            id: SearchExploitTool::NAME.to_string(),
            name: SearchExploitTool::NAME.to_string(),
            description: SearchExploitTool::DESCRIPTION.to_string(),
            category: ToolCategory::Exploitation,
            tags: vec![
                "exploit".to_string(),
                "cve".to_string(),
                "poc".to_string(),
                "exploitdb".to_string(),
            ],
            search_hint: Some("search exploit references and payload candidates".to_string()),
            cost_estimate: ToolCost::Medium,
            always_available: false,
            exposure: ToolExposure::Deferred,
        },
        ToolMetadata {
            id: SkillsTool::NAME.to_string(),
            name: SkillsTool::NAME.to_string(),
            description: SkillsTool::DESCRIPTION.to_string(),
            category: ToolCategory::System,
            tags: vec![
                "skills".to_string(),
                "load".to_string(),
                "read".to_string(),
                "workflow".to_string(),
            ],
            search_hint: Some(
                "discover reusable local skills and load their instructions".to_string(),
            ),
            cost_estimate: ToolCost::Low,
            always_available: true,
            exposure: ToolExposure::Core,
        },
        ToolMetadata {
            id: TodosTool::NAME.to_string(),
            name: TodosTool::NAME.to_string(),
            description: TodosTool::DESCRIPTION.to_string(),
            category: ToolCategory::System,
            tags: vec![
                "plan".to_string(),
                "task".to_string(),
                "workflow".to_string(),
                "todos".to_string(),
            ],
            search_hint: Some("track execution tasks and progress".to_string()),
            cost_estimate: ToolCost::Low,
            always_available: true,
            exposure: ToolExposure::Core,
        },
        ToolMetadata {
            id: MemoryManagerTool::NAME.to_string(),
            name: MemoryManagerTool::NAME.to_string(),
            description: MemoryManagerTool::DESCRIPTION.to_string(),
            category: ToolCategory::AI,
            tags: vec![
                "memory".to_string(),
                "store".to_string(),
                "retrieve".to_string(),
                "recall".to_string(),
            ],
            search_hint: Some("store or retrieve execution memory".to_string()),
            cost_estimate: ToolCost::Low,
            always_available: false,
            exposure: ToolExposure::Deferred,
        },
        ToolMetadata {
            id: OcrTool::NAME.to_string(),
            name: OcrTool::NAME.to_string(),
            description: OcrTool::DESCRIPTION.to_string(),
            category: ToolCategory::AI,
            tags: vec![
                "ocr".to_string(),
                "image".to_string(),
                "text".to_string(),
                "extract".to_string(),
            ],
            search_hint: Some("extract text content from images".to_string()),
            cost_estimate: ToolCost::Medium,
            always_available: false,
            exposure: ToolExposure::Deferred,
        },
        ToolMetadata {
            id: TenthManTool::NAME.to_string(),
            name: TenthManTool::NAME.to_string(),
            description: TenthManTool::DESCRIPTION.to_string(),
            category: ToolCategory::AI,
            tags: vec![
                "review".to_string(),
                "risk".to_string(),
                "analysis".to_string(),
                "verification".to_string(),
            ],
            search_hint: Some("stress-test assumptions and review risk".to_string()),
            cost_estimate: ToolCost::Medium,
            always_available: false,
            exposure: ToolExposure::Core,
        },
        ToolMetadata {
            id: SpawnAgentTool::NAME.to_string(),
            name: SpawnAgentTool::NAME.to_string(),
            description: SpawnAgentTool::DESCRIPTION.to_string(),
            category: ToolCategory::AI,
            tags: vec![
                "subagent".to_string(),
                "spawn".to_string(),
                "delegate".to_string(),
                "parallel".to_string(),
            ],
            search_hint: Some("delegate work to a background subagent".to_string()),
            cost_estimate: ToolCost::High,
            always_available: false,
            exposure: ToolExposure::Core,
        },
        ToolMetadata {
            id: WaitAgentsTool::NAME.to_string(),
            name: WaitAgentsTool::NAME.to_string(),
            description: WaitAgentsTool::DESCRIPTION.to_string(),
            category: ToolCategory::AI,
            tags: vec![
                "subagent".to_string(),
                "wait".to_string(),
                "join".to_string(),
                "completion".to_string(),
            ],
            search_hint: Some("wait for background subagents to finish".to_string()),
            cost_estimate: ToolCost::Low,
            always_available: false,
            exposure: ToolExposure::Core,
        },
        ToolMetadata {
            id: ListAgentsTool::NAME.to_string(),
            name: ListAgentsTool::NAME.to_string(),
            description: ListAgentsTool::DESCRIPTION.to_string(),
            category: ToolCategory::AI,
            tags: vec![
                "subagent".to_string(),
                "list".to_string(),
                "status".to_string(),
                "inspect".to_string(),
            ],
            search_hint: Some("list background subagents and their status".to_string()),
            cost_estimate: ToolCost::Low,
            always_available: false,
            exposure: ToolExposure::Core,
        },
        ToolMetadata {
            id: CloseAgentTool::NAME.to_string(),
            name: CloseAgentTool::NAME.to_string(),
            description: CloseAgentTool::DESCRIPTION.to_string(),
            category: ToolCategory::AI,
            tags: vec![
                "subagent".to_string(),
                "close".to_string(),
                "cancel".to_string(),
            ],
            search_hint: Some("close a background subagent".to_string()),
            cost_estimate: ToolCost::Low,
            always_available: false,
            exposure: ToolExposure::Core,
        },
    ]
}

pub fn score_skill_match(task_lower: &str, skill_name: &str, description: &str) -> usize {
    if task_lower.trim().is_empty() {
        return 0;
    }

    let mut score = 0usize;
    let name_lower = skill_name.to_lowercase();
    let desc_lower = description.to_lowercase();

    if task_lower.contains(&name_lower) {
        score += 8;
    }

    let mut keywords = HashSet::new();
    for part in name_lower
        .split(|c: char| !c.is_alphanumeric() && c != '_' && c != '-')
        .chain(desc_lower.split(|c: char| !c.is_alphanumeric() && c != '_' && c != '-'))
    {
        let token = part.trim();
        if token.len() >= 3 && token.len() <= 32 {
            keywords.insert(token.to_string());
        }
        if keywords.len() >= 36 {
            break;
        }
    }

    for token in keywords {
        if task_lower.contains(&token) {
            score += 1;
        }
    }

    score
}

pub fn extract_workflow_tags(name: &str, description: Option<&str>) -> Vec<String> {
    let mut tags = Vec::new();

    let name_lower = name.to_lowercase();
    let name_words: Vec<&str> = name_lower.split(|c: char| !c.is_alphanumeric()).collect();
    for word in name_words {
        if word.len() > 2 {
            tags.push(word.to_string());
        }
    }

    if let Some(desc) = description {
        let desc_lower = desc.to_lowercase();
        let keywords = [
            "scan",
            "test",
            "analyze",
            "report",
            "monitor",
            "alert",
            "security",
            "vulnerability",
            "penetration",
            "reconnaissance",
            "扫描",
            "测试",
            "分析",
            "报告",
            "监控",
            "告警",
            "安全",
            "漏洞",
        ];

        for keyword in keywords {
            if desc_lower.contains(keyword) {
                tags.push(keyword.to_string());
            }
        }
    }

    tags.sort();
    tags.dedup();
    tags
}

pub fn extract_mcp_tool_tags(name: &str, description: &str) -> Vec<String> {
    let mut tags = Vec::new();
    let name_lower = name.to_lowercase();
    let name_words: Vec<&str> = name_lower.split(|c: char| !c.is_alphanumeric()).collect();

    for word in name_words {
        if word.len() > 2 {
            tags.push(word.to_string());
        }
    }

    let desc_lower = description.to_lowercase();
    let keywords = [
        "file",
        "read",
        "write",
        "search",
        "query",
        "fetch",
        "get",
        "list",
        "create",
        "update",
        "delete",
        "execute",
        "run",
        "call",
        "invoke",
        "database",
        "api",
        "web",
        "http",
        "git",
        "github",
        "filesystem",
        "文件",
        "读取",
        "写入",
        "搜索",
        "查询",
        "获取",
        "列表",
        "创建",
        "更新",
        "删除",
        "执行",
        "运行",
        "调用",
        "数据库",
        "接口",
    ];

    for keyword in keywords {
        if desc_lower.contains(keyword) {
            tags.push(keyword.to_string());
        }
    }

    tags.push("mcp".to_string());
    tags.sort();
    tags.dedup();
    tags
}
