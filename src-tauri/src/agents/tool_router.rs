//! Tool Router - 智能工具选择和路由
//!
//! 根据任务内容选择相关工具，避免将所有工具传给 LLM 造成 token 浪费。

mod catalog;
pub(crate) mod tool_server_catalog;
mod types;

use anyhow::Result;
use once_cell::sync::Lazy;
#[allow(unused_imports)]
use sentinel_db::Database;
use serde_json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use sentinel_tools::buildin_tools::{
    CloseAgentTool, HttpRequestTool, ListAgentsTool, MemoryManagerTool, ShellTool, SkillsTool,
    SpawnAgentTool, TasksTool, TenthManTool, WaitAgentsTool,
};
pub use types::{
    SelectedSkill, ToolCategory, ToolConfig, ToolCost, ToolExposure, ToolMetadata,
    ToolSelectionPlan, ToolSelectionStrategy, ToolStatistics, ToolUsageRecord, ToolUsageStatistics,
    ToolUsageStats,
};

use self::catalog::{extract_mcp_tool_tags, extract_workflow_tags, score_skill_match};
use self::tool_server_catalog::build_builtin_tool_metadata;

/// 全局工具使用记录
static TOOL_USAGE_RECORDS: Lazy<Arc<RwLock<Vec<ToolUsageRecord>>>> =
    Lazy::new(|| Arc::new(RwLock::new(Vec::new())));

/// 工具路由器
pub struct ToolRouter {
    all_tools: Vec<ToolMetadata>,
    workflow_tools: Vec<ToolMetadata>,
    mcp_tools: Vec<ToolMetadata>,
    plugin_tools: Vec<ToolMetadata>,
    db_service: Option<Arc<sentinel_db::DatabaseService>>,
}

impl ToolRouter {
    /// 创建新的工具路由器
    pub fn new() -> Self {
        Self {
            all_tools: Vec::new(),
            workflow_tools: Vec::new(),
            mcp_tools: Vec::new(),
            plugin_tools: Vec::new(),
            db_service: None,
        }
    }

    /// 创建工具路由器并加载动态工具（工作流、MCP、插件）
    pub async fn new_with_dynamic_tools(
        db_service: Option<&std::sync::Arc<sentinel_db::DatabaseService>>,
    ) -> Self {
        let mut router = Self::new();
        router.db_service = db_service.cloned();
        router.all_tools = router.load_builtin_tools_from_server().await;

        // 加载工作流工具
        if let Some(db) = db_service {
            if let Ok(workflows) = router.load_workflow_tools(db).await {
                router.workflow_tools = workflows;
            }
        }

        router
    }

    fn select_deferred_core_tools(&self, config: &ToolConfig) -> Vec<String> {
        let mut selected = Vec::new();
        for tool in self.get_all_available_tools() {
            if config.disabled_tools.contains(&tool.id) {
                continue;
            }
            if matches!(tool.exposure, ToolExposure::Always | ToolExposure::Core) {
                selected.push(tool.id);
            }
        }
        selected.extend(config.fixed_tools.clone());

        let available_tools = self.get_all_available_tools();
        let mut seen = std::collections::HashSet::new();
        selected.retain(|tool_id| seen.insert(tool_id.clone()));
        selected.retain(|tool_id| available_tools.iter().any(|tool| &tool.id == tool_id));

        if selected.len() > config.max_tools {
            tracing::warn!(
                "Deferred base toolset exceeds max_tools ({} > {}), truncating.",
                selected.len(),
                config.max_tools
            );
            selected.truncate(config.max_tools);
        }

        selected
    }

    async fn build_deferred_prompt_injection(&self) -> Option<String> {
        Some(
            "\n<tool_mode>\nDeferred tool mode is active. You currently have a minimal core toolset.\nWhen the current tools are insufficient, use `tool_search` action=search to discover relevant tools. If the search result includes `recommended_tool_ids`, you may call `tool_search` action=activate with the same query and omit `tool_ids` to activate the recommended bundle automatically. Otherwise, call action=activate with explicit tool_ids before trying to use them.\nDo not guess hidden tool names. Search first, activate second, then use the activated tool.\n</tool_mode>\n".to_string(),
        )
    }

    /// 根据任务选择相关工具
    pub async fn select_tools(
        &self,
        task: &str,
        config: &ToolConfig,
        llm_config: Option<&sentinel_llm::LlmConfig>,
    ) -> Result<Vec<String>> {
        if !config.enabled {
            return Ok(vec![]);
        }

        let selected = match &config.selection_strategy {
            ToolSelectionStrategy::None => vec![],
            ToolSelectionStrategy::All => {
                let mut all = self.get_all_available_tools();
                all.retain(|t| !config.disabled_tools.contains(&t.id));
                all.into_iter().map(|t| t.id).collect()
            }
            ToolSelectionStrategy::Manual(tools) => {
                let all_tools = self.get_all_available_tools();
                let mut unknown_tools = Vec::new();
                let mut seen = std::collections::HashSet::new();
                let result: Vec<String> = tools
                    .iter()
                    .filter(|t| !config.disabled_tools.contains(t))
                    .filter_map(|t| {
                        let tool_id = {
                            // 1. Exact match
                            if let Some(found) = all_tools.iter().find(|meta| &meta.id == t) {
                                Some(found.id.clone())
                            } else {
                                // 2. Legacy :: to __ match
                                let replaced = t.replace("::", "__");
                                if let Some(found) =
                                    all_tools.iter().find(|meta| meta.id == replaced)
                                {
                                    Some(found.id.clone())
                                } else {
                                    // 3. Strict sanitization match (for plugins etc)
                                    let sanitized = replaced
                                        .replace(|c: char| !c.is_alphanumeric() && c != '_', "_");
                                    if let Some(found) =
                                        all_tools.iter().find(|meta| meta.id == sanitized)
                                    {
                                        Some(found.id.clone())
                                    } else {
                                        None
                                    }
                                }
                            }
                        };
                        let Some(tool_id) = tool_id else {
                            unknown_tools.push(t.clone());
                            return None;
                        };
                        if seen.insert(tool_id.clone()) {
                            Some(tool_id)
                        } else {
                            None
                        }
                    })
                    .collect();
                if !unknown_tools.is_empty() {
                    tracing::warn!(
                        "Manual tool selection requested unknown tools: {:?}",
                        unknown_tools
                    );
                }
                result
            }
            ToolSelectionStrategy::Keyword => self.select_by_keywords(task, config)?,
            ToolSelectionStrategy::LLM => self.select_by_llm(task, config, llm_config).await?,
            ToolSelectionStrategy::Hybrid => self.select_hybrid(task, config, llm_config).await?,
            ToolSelectionStrategy::Skills(_allowed_groups) => {
                if !self.is_skills_enabled().await {
                    tracing::info!("Skills tool disabled via config; returning no tools.");
                    return Ok(self.merge_always_available_tools(vec![], &config.disabled_tools));
                }
                // Skills mode default toolset
                let mut base_tools = vec![
                    SkillsTool::NAME.to_string(),
                    ShellTool::NAME.to_string(),
                    HttpRequestTool::NAME.to_string(),
                    SpawnAgentTool::NAME.to_string(),
                    WaitAgentsTool::NAME.to_string(),
                    ListAgentsTool::NAME.to_string(),
                    CloseAgentTool::NAME.to_string(),
                    TenthManTool::NAME.to_string(),
                ];
                if !config
                    .disabled_tools
                    .contains(&MemoryManagerTool::NAME.to_string())
                {
                    base_tools.push(MemoryManagerTool::NAME.to_string());
                }
                if !config.disabled_tools.contains(&TasksTool::NAME.to_string()) {
                    base_tools.push(TasksTool::NAME.to_string());
                }
                base_tools
            }
            ToolSelectionStrategy::Deferred => self.select_deferred_core_tools(config),
        };

        Ok(self.merge_always_available_tools(selected, &config.disabled_tools))
    }

    /// Plan tools with full selection plan (supports Skills context injection)
    pub async fn plan_tools(
        &self,
        task: &str,
        config: &ToolConfig,
        llm_config: Option<&sentinel_llm::LlmConfig>,
    ) -> Result<ToolSelectionPlan> {
        if !config.enabled {
            return Ok(ToolSelectionPlan {
                tool_ids: vec![],
                injected_system_prompt: None,
                selected_skill: None,
            });
        }

        match &config.selection_strategy {
            ToolSelectionStrategy::Deferred => {
                let tool_ids = self.merge_always_available_tools(
                    self.select_deferred_core_tools(config),
                    &config.disabled_tools,
                );
                Ok(ToolSelectionPlan {
                    tool_ids,
                    injected_system_prompt: self.build_deferred_prompt_injection().await,
                    selected_skill: None,
                })
            }
            ToolSelectionStrategy::Skills(_allowed_groups) => {
                if !self.is_skills_enabled().await {
                    tracing::info!("Skills tool disabled via config; skipping skills injection.");
                    return Ok(ToolSelectionPlan {
                        tool_ids: vec![],
                        injected_system_prompt: None,
                        selected_skill: None,
                    });
                }
                // Skills mode default toolset
                let mut all = vec![
                    SkillsTool::NAME.to_string(),
                    ShellTool::NAME.to_string(),
                    HttpRequestTool::NAME.to_string(),
                    SpawnAgentTool::NAME.to_string(),
                    WaitAgentsTool::NAME.to_string(),
                    ListAgentsTool::NAME.to_string(),
                    CloseAgentTool::NAME.to_string(),
                    TenthManTool::NAME.to_string(),
                ];
                if !config
                    .disabled_tools
                    .contains(&MemoryManagerTool::NAME.to_string())
                {
                    all.push(MemoryManagerTool::NAME.to_string());
                }
                if !config.disabled_tools.contains(&TasksTool::NAME.to_string()) {
                    all.push(TasksTool::NAME.to_string());
                }
                let injected = self.build_skills_prompt_injection(Some(task)).await;
                Ok(ToolSelectionPlan {
                    tool_ids: self.merge_always_available_tools(all, &config.disabled_tools),
                    injected_system_prompt: injected,
                    selected_skill: None,
                })
            }
            _ => {
                // For non-Skills strategies, just wrap select_tools result
                let tool_ids = self.select_tools(task, config, llm_config).await?;
                let injected_system_prompt = if tool_ids.iter().any(|id| id == SkillsTool::NAME)
                    && self.is_skills_enabled().await
                {
                    self.build_skills_prompt_injection(Some(task)).await
                } else {
                    None
                };
                Ok(ToolSelectionPlan {
                    tool_ids,
                    injected_system_prompt,
                    selected_skill: None,
                })
            }
        }
    }

    async fn build_skills_prompt_injection(&self, task: Option<&str>) -> Option<String> {
        const MAX_SKILLS_INJECTION: usize = 8;
        const MAX_DESC_CHARS: usize = 220;

        if let Some(db_service) = &self.db_service {
            let root = db_service.get_skills_root_dir();
            let mut summaries: Vec<(String, String)> = Vec::new();
            if let Ok(entries) = std::fs::read_dir(&root) {
                for entry in entries.flatten() {
                    if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                        continue;
                    }
                    let skill_dir = entry.path();
                    let skill_id = entry.file_name().to_string_lossy().to_string();
                    if !self.is_skill_enabled(&skill_id).await {
                        continue;
                    }
                    let skill_md = skill_dir.join("SKILL.md");
                    if !skill_md.exists() {
                        continue;
                    }
                    let content = match std::fs::read_to_string(&skill_md) {
                        Ok(c) => c,
                        Err(_) => continue,
                    };
                    let doc = match crate::skills::parse_skill_markdown(&content) {
                        Ok(doc) => doc,
                        Err(_) => continue,
                    };
                    if doc.frontmatter.description.trim().is_empty()
                        && doc
                            .frontmatter
                            .when_to_use
                            .as_ref()
                            .map(|s| s.trim().is_empty())
                            .unwrap_or(true)
                    {
                        continue;
                    }
                    let name = doc.frontmatter.name.trim().to_string();
                    if name.is_empty() {
                        continue;
                    }
                    let description = if let Some(when) = doc.frontmatter.when_to_use.as_ref() {
                        if when.trim().is_empty() {
                            doc.frontmatter.description.trim().to_string()
                        } else {
                            format!("{} - {}", doc.frontmatter.description.trim(), when.trim())
                        }
                    } else {
                        doc.frontmatter.description.trim().to_string()
                    };
                    let description = if description.len() > MAX_DESC_CHARS {
                        let mut trimmed =
                            description.chars().take(MAX_DESC_CHARS).collect::<String>();
                        trimmed.push_str("...");
                        trimmed
                    } else {
                        description
                    };
                    summaries.push((name, description));
                }
            }
            let total_count = summaries.len();
            let task_text = task.unwrap_or_default().to_lowercase();
            summaries.sort_by(|(name_a, desc_a), (name_b, desc_b)| {
                let score_a = score_skill_match(&task_text, name_a, desc_a);
                let score_b = score_skill_match(&task_text, name_b, desc_b);
                score_b
                    .cmp(&score_a)
                    .then_with(|| name_a.to_lowercase().cmp(&name_b.to_lowercase()))
            });
            if summaries.len() > MAX_SKILLS_INJECTION {
                summaries.truncate(MAX_SKILLS_INJECTION);
            }
            let mut rendered = summaries
                .into_iter()
                .map(|(name, description)| format!("\"{}\": {}", name, description))
                .collect::<Vec<_>>();
            if total_count > rendered.len() {
                rendered.push(format!(
                    "... {} more skills omitted for brevity. Use `skills` tool action=list to enumerate all.",
                    total_count - rendered.len()
                ));
            }
            let skills_block = if rendered.is_empty() {
                "No skills available.".to_string()
            } else {
                rendered.join("\n")
            };
            Some(format!(
                "\n<available_skills>\n{}\n</available_skills>\n\nWhen a task requires specialized workflows, use the `skills` tool. If <available_skills> already lists relevant skills, you can call action=load directly without calling action=list. Use action=read_file for referenced files as needed. Do not assume skill details without loading.",
                skills_block
            ))
        } else {
            Some("When a task requires specialized workflows, use the `skills` tool. If <available_skills> is provided, you can call action=load directly; otherwise call action=list. Use action=read_file for referenced files as needed. Do not assume skill details without loading.".to_string())
        }
    }

    async fn is_skills_enabled(&self) -> bool {
        let Some(db) = &self.db_service else {
            return true;
        };
        match db.get_config("agent", "skills_enabled").await {
            Ok(Some(val)) => {
                let v = val.trim().to_lowercase();
                matches!(v.as_str(), "true" | "1" | "yes" | "on")
            }
            _ => true,
        }
    }

    async fn is_skill_enabled(&self, skill_id: &str) -> bool {
        let Some(db) = &self.db_service else {
            return true;
        };
        let key = format!("enabled::{}", skill_id);
        match db.get_config("skills", &key).await {
            Ok(Some(val)) => {
                let v = val.trim().to_lowercase();
                matches!(v.as_str(), "true" | "1" | "yes" | "on")
            }
            _ => true,
        }
    }

    /// 获取所有可用工具（包括动态工具）
    fn get_all_available_tools(&self) -> Vec<ToolMetadata> {
        let mut tools = self.all_tools.clone();
        tools.extend(self.workflow_tools.clone());
        tools.extend(self.mcp_tools.clone());
        tools.extend(self.plugin_tools.clone());

        // Deduplicate by tool ID
        let mut seen = std::collections::HashSet::new();
        tools.retain(|t| seen.insert(t.id.clone()));

        tools
    }

    fn merge_always_available_tools(
        &self,
        selected: Vec<String>,
        disabled_tools: &[String],
    ) -> Vec<String> {
        let mut merged = std::collections::HashSet::new();
        for id in selected {
            merged.insert(id);
        }
        for tool in self.get_all_available_tools() {
            if disabled_tools.contains(&tool.id) {
                continue;
            }
            if tool.always_available {
                merged.insert(tool.id);
            }
        }
        merged.into_iter().collect()
    }

    /// 关键词匹配选择工具（快速，无额外成本）
    fn select_by_keywords(&self, task: &str, config: &ToolConfig) -> Result<Vec<String>> {
        let task_lower = task.to_lowercase();
        let is_binary_security_task = [
            "pwn",
            "reverse",
            "binary",
            "elf",
            "rop",
            "heap",
            "format string",
            "ret2libc",
            "shellcode",
            "gdb",
            "pwndbg",
            "gef",
            "ctf",
            "exploit",
            "逆向",
            "二进制",
            "漏洞利用",
            "缓冲区溢出",
            "栈溢出",
            "堆溢出",
        ]
        .iter()
        .any(|kw| task_lower.contains(kw));
        let mut scored_tools = Vec::new();

        // 先添加固定工具
        let all_available_tools = self.get_all_available_tools();
        let mut selected: Vec<String> = config
            .fixed_tools
            .iter()
            .map(|t| {
                if let Some(found) = all_available_tools.iter().find(|meta| &meta.id == t) {
                    return found.id.clone();
                }
                let replaced = t.replace("::", "__");
                if let Some(found) = all_available_tools.iter().find(|meta| meta.id == replaced) {
                    return found.id.clone();
                }
                let sanitized = replaced.replace(|c: char| !c.is_alphanumeric() && c != '_', "_");
                if let Some(found) = all_available_tools.iter().find(|meta| meta.id == sanitized) {
                    return found.id.clone();
                }
                replaced
            })
            .collect();

        // 合并所有工具
        // let all_available_tools = self.get_all_available_tools(); // Moved up

        for tool in &all_available_tools {
            // 跳过已禁用的工具
            if config.disabled_tools.contains(&tool.id) {
                continue;
            }

            // 跳过已在固定工具中的
            if selected.contains(&tool.id) {
                continue;
            }

            let mut score = 0;

            // 始终可用的工具优先级更高
            if tool.always_available {
                score += 5;
            }

            // 检查工具名称
            if task_lower.contains(&tool.name.to_lowercase()) {
                score += 20;
            }

            // 检查标签
            for tag in &tool.tags {
                if task_lower.contains(&tag.to_lowercase()) {
                    score += 10;
                }
            }

            // 检查描述中的关键词
            let description_words = tool.description.to_lowercase();
            let task_words: Vec<&str> = task_lower.split_whitespace().collect();

            for word in task_words {
                if word.len() > 3 && description_words.contains(word) {
                    score += 3;
                }
            }

            // 特殊关键词匹配
            if task_lower.contains("scan") && tool.category == ToolCategory::Network {
                score += 15;
            }
            if (task_lower.contains("http") || task_lower.contains("api"))
                && tool.id == "http_request"
            {
                score += 15;
            }
            if (task_lower.contains("command")
                || task_lower.contains("shell")
                || task_lower.contains("execute"))
                && tool.id == "shell"
            {
                score += 15;
            }
            if is_binary_security_task && tool.id == "interactive_shell" {
                // For reverse/pwn flows, prefer persistent interactive sessions (gdb/pwndbg/python repl).
                score += 35;
            }
            if is_binary_security_task && tool.id == "skills" {
                // Encourage loading specialized pwn/rev skills early.
                score += 20;
            }
            if is_binary_security_task && tool.id == "shell" {
                // Keep shell available, but lower priority than interactive_shell in binary tasks.
                score += 3;
            }
            let memory_intent = task_lower.contains("memory")
                || task_lower.contains("remember")
                || task_lower.contains("recall")
                || task_lower.contains("store")
                || task_lower.contains("save")
                || task_lower.contains("记忆")
                || task_lower.contains("回忆")
                || task_lower.contains("回顾")
                || task_lower.contains("复盘")
                || task_lower.contains("历史思路")
                || task_lower.contains("历史分析")
                || task_lower.contains("之前分析")
                || task_lower.contains("之前结论")
                || task_lower.contains("解题思路")
                || (task_lower.contains("之前") && task_lower.contains("思路"))
                || (task_lower.contains("历史") && task_lower.contains("思路"));
            if memory_intent && tool.id == MemoryManagerTool::NAME {
                score += 25; // High priority for memory operations
            }
            if (task_lower.contains("ocr")
                || task_lower.contains("text from image")
                || task_lower.contains("read image")
                || task_lower.contains("图片文字")
                || task_lower.contains("文字识别"))
                && tool.id == "ocr"
            {
                score += 30;
            }

            // 工作流工具匹配
            if tool.category == ToolCategory::Workflow {
                if task_lower.contains("workflow") || task_lower.contains("工作流") {
                    score += 20;
                }
                // 根据工作流名称匹配
                if tool
                    .name
                    .to_lowercase()
                    .split('_')
                    .any(|word| task_lower.contains(word))
                {
                    score += 10;
                }
            }

            if score > 0 {
                scored_tools.push((tool.id.clone(), score));
            }
        }

        // 排序并选择 top-k
        scored_tools.sort_by(|a, b| b.1.cmp(&a.1));

        let remaining_slots = config.max_tools.saturating_sub(selected.len());
        let additional_tools: Vec<String> = scored_tools
            .into_iter()
            .take(remaining_slots)
            .map(|(id, _)| id)
            .collect();

        selected.extend(additional_tools);

        tracing::info!(
            "Tool selection for task (first 100 chars): '{}...' -> {} tools: {:?}",
            task.chars().take(100).collect::<String>(),
            selected.len(),
            selected
        );

        Ok(selected)
    }

    /// 获取工具元数据
    pub fn get_tool_metadata(&self, tool_id: &str) -> Option<ToolMetadata> {
        self.get_all_available_tools()
            .into_iter()
            .find(|t| t.id == tool_id)
    }

    /// 获取所有工具 ID
    pub fn all_tool_ids(&self) -> Vec<String> {
        self.get_all_available_tools()
            .into_iter()
            .map(|t| t.id)
            .collect()
    }

    /// 加载工作流工具
    async fn load_workflow_tools(
        &self,
        db_service: &std::sync::Arc<sentinel_db::DatabaseService>,
    ) -> Result<Vec<ToolMetadata>> {
        use sentinel_db::Database;
        let mut workflow_tools = Vec::new();

        // Use list_workflow_tools to get workflows marked as tools
        match db_service.list_workflow_tools().await {
            Ok(workflows) => {
                for workflow in workflows {
                    if let (Some(id), Some(name), description) = (
                        workflow.get("id").and_then(|v| v.as_str()),
                        workflow.get("name").and_then(|v| v.as_str()),
                        workflow.get("description").and_then(|v| v.as_str()),
                    ) {
                        // 提取工作流的标签
                        let tags = extract_workflow_tags(name, description);

                        workflow_tools.push(ToolMetadata {
                            id: format!("workflow__{}", id),
                            name: name.to_string(),
                            description: description.unwrap_or("Workflow tool").to_string(),
                            category: ToolCategory::Workflow,
                            tags,
                            search_hint: Some("run a workflow-defined tool".to_string()),
                            cost_estimate: ToolCost::High, // 工作流通常较复杂
                            always_available: false,
                            exposure: ToolExposure::Deferred,
                        });
                    }
                }

                tracing::info!(
                    "Loaded {} workflow tools (is_tool=true)",
                    workflow_tools.len()
                );
            }
            Err(e) => {
                tracing::warn!("Failed to load workflow tools: {}", e);
            }
        }

        Ok(workflow_tools)
    }

    async fn load_builtin_tools_from_server(&self) -> Vec<ToolMetadata> {
        let tool_server = sentinel_tools::get_tool_server();
        tool_server.init_builtin_tools().await;
        build_builtin_tool_metadata(tool_server.list_tools().await)
    }

    /// 添加自定义工具（用于 MCP、插件等）
    pub fn add_tool(&mut self, metadata: ToolMetadata) {
        self.all_tools.push(metadata);
    }

    /// 批量添加工具
    pub fn add_tools(&mut self, tools: Vec<ToolMetadata>) {
        self.all_tools.extend(tools);
    }

    /// 移除工具
    pub fn remove_tool(&mut self, tool_id: &str) -> bool {
        let before_len = self.all_tools.len();
        self.all_tools.retain(|t| t.id != tool_id);
        self.mcp_tools.retain(|t| t.id != tool_id);
        self.plugin_tools.retain(|t| t.id != tool_id);
        self.workflow_tools.retain(|t| t.id != tool_id);
        before_len != self.all_tools.len()
    }

    /// 更新工具元数据
    pub fn update_tool(&mut self, metadata: ToolMetadata) -> bool {
        // 查找并更新工具
        if let Some(tool) = self.all_tools.iter_mut().find(|t| t.id == metadata.id) {
            *tool = metadata;
            return true;
        }
        if let Some(tool) = self.mcp_tools.iter_mut().find(|t| t.id == metadata.id) {
            *tool = metadata;
            return true;
        }
        if let Some(tool) = self.plugin_tools.iter_mut().find(|t| t.id == metadata.id) {
            *tool = metadata;
            return true;
        }
        if let Some(tool) = self.workflow_tools.iter_mut().find(|t| t.id == metadata.id) {
            *tool = metadata;
            return true;
        }
        false
    }

    /// 列出所有工具元数据
    pub fn list_all_tools(&self) -> Vec<ToolMetadata> {
        self.get_all_available_tools()
    }

    /// 按分类列出工具
    pub fn list_tools_by_category(&self, category: ToolCategory) -> Vec<ToolMetadata> {
        self.get_all_available_tools()
            .into_iter()
            .filter(|t| t.category == category)
            .collect()
    }

    /// 搜索工具（按名称或描述）
    pub fn search_tools(&self, query: &str) -> Vec<ToolMetadata> {
        let query_lower = query.to_lowercase();
        self.get_all_available_tools()
            .into_iter()
            .filter(|t| {
                t.name.to_lowercase().contains(&query_lower)
                    || t.description.to_lowercase().contains(&query_lower)
                    || t.search_hint
                        .as_deref()
                        .map(|hint| hint.to_lowercase().contains(&query_lower))
                        .unwrap_or(false)
                    || t.tags
                        .iter()
                        .any(|tag| tag.to_lowercase().contains(&query_lower))
            })
            .collect()
    }

    /// 获取工具统计信息
    pub fn get_statistics(&self) -> ToolStatistics {
        let all_tools = self.get_all_available_tools();

        let mut by_category = std::collections::HashMap::new();
        let mut by_cost = std::collections::HashMap::new();

        for tool in &all_tools {
            *by_category
                .entry(format!("{:?}", tool.category))
                .or_insert(0) += 1;
            *by_cost
                .entry(format!("{:?}", tool.cost_estimate))
                .or_insert(0) += 1;
        }

        ToolStatistics {
            total_tools: all_tools.len(),
            builtin_tools: self.all_tools.len(),
            workflow_tools: self.workflow_tools.len(),
            mcp_tools: self.mcp_tools.len(),
            plugin_tools: self.plugin_tools.len(),
            always_available: all_tools.iter().filter(|t| t.always_available).count(),
            by_category,
            by_cost,
        }
    }

    /// 添加 MCP 工具
    pub fn add_mcp_tool(&mut self, metadata: ToolMetadata) {
        self.mcp_tools.push(metadata);
    }

    /// 批量添加 MCP 工具
    pub fn add_mcp_tools(&mut self, tools: Vec<ToolMetadata>) {
        self.mcp_tools.extend(tools);
    }

    /// 清空 MCP 工具
    pub fn clear_mcp_tools(&mut self) {
        self.mcp_tools.clear();
    }

    /// 添加插件工具
    pub fn add_plugin_tool(&mut self, metadata: ToolMetadata) {
        self.plugin_tools.push(metadata);
    }

    /// 批量添加插件工具
    pub fn add_plugin_tools(&mut self, tools: Vec<ToolMetadata>) {
        self.plugin_tools.extend(tools);
    }

    /// 清空插件工具
    pub fn clear_plugin_tools(&mut self) {
        self.plugin_tools.clear();
    }

    /// 刷新工作流工具
    pub async fn refresh_workflow_tools(
        &mut self,
        db_service: &std::sync::Arc<sentinel_db::DatabaseService>,
    ) -> Result<()> {
        self.workflow_tools = self.load_workflow_tools(db_service).await?;
        Ok(())
    }

    /// 加载 MCP 工具
    pub async fn load_mcp_tools(&self) -> Result<Vec<ToolMetadata>> {
        let mut mcp_tools = Vec::new();

        // 调用 MCP 命令获取所有工具
        match crate::commands::mcp_commands::mcp_get_all_tools().await {
            Ok(tools) => {
                for tool in tools {
                    if let (Some(server_name), Some(tool_name)) = (
                        tool.get("server_name").and_then(|v| v.as_str()),
                        tool.get("name").and_then(|v| v.as_str()),
                    ) {
                        let description = tool
                            .get("description")
                            .and_then(|v| v.as_str())
                            .unwrap_or("MCP tool");

                        // 从工具名称和描述中提取标签
                        let tags = extract_mcp_tool_tags(tool_name, description);

                        mcp_tools.push(ToolMetadata {
                            id: format!("mcp__{}__{}", server_name, tool_name),
                            name: format!("mcp__{}__{}", server_name, tool_name), // name for display can stay as is, or change? Let's check.
                            description: description.to_string(),
                            category: ToolCategory::MCP,
                            tags,
                            search_hint: Some("use a connected MCP server capability".to_string()),
                            cost_estimate: ToolCost::Medium,
                            always_available: false,
                            exposure: ToolExposure::Deferred,
                        });
                    }
                }

                tracing::info!("Loaded {} MCP tools", mcp_tools.len());
            }
            Err(e) => {
                tracing::warn!("Failed to load MCP tools: {}", e);
            }
        }

        Ok(mcp_tools)
    }

    /// 刷新 MCP 工具
    pub async fn refresh_mcp_tools(&mut self) -> Result<()> {
        self.mcp_tools = self.load_mcp_tools().await?;
        Ok(())
    }

    /// 加载插件工具
    pub async fn load_plugin_tools(
        &self,
        db_service: &std::sync::Arc<sentinel_db::DatabaseService>,
    ) -> Result<Vec<ToolMetadata>> {
        let mut plugin_tools = Vec::new();

        use sentinel_db::Database;
        let plugins = db_service
            .get_plugins_from_registry(Some("default"))
            .await
            .map_err(|e| anyhow::anyhow!("Failed to query database plugins: {}", e))?;

        for p in plugins {
            // 只查询已启用的 agent 类型插件
            if p.metadata.main_category == "agent"
                && p.status == sentinel_plugins::PluginStatus::Enabled
            {
                let description_str = p
                    .metadata
                    .description
                    .as_deref()
                    .unwrap_or("Agent plugin tool");

                // 从 tags 提取标签
                let mut tags = vec!["plugin".to_string(), "agent".to_string()];
                for tag in p.metadata.tags {
                    tags.push(tag);
                }

                let sanitized_id = p
                    .metadata
                    .id
                    .replace(|c: char| !c.is_alphanumeric() && c != '_', "_");
                plugin_tools.push(ToolMetadata {
                    id: format!("plugin__{}", sanitized_id),
                    name: p.metadata.name.clone(),
                    description: description_str.to_string(),
                    category: ToolCategory::Plugin,
                    tags,
                    search_hint: Some("run a plugin-provided tool".to_string()),
                    cost_estimate: ToolCost::Medium,
                    always_available: false,
                    exposure: ToolExposure::Deferred,
                });
            }
        }

        tracing::info!("Loaded {} plugin tools from database", plugin_tools.len());

        Ok(plugin_tools)
    }

    /// 刷新插件工具
    pub async fn refresh_plugin_tools(
        &mut self,
        db_service: &std::sync::Arc<sentinel_db::DatabaseService>,
    ) -> Result<()> {
        self.plugin_tools = self.load_plugin_tools(db_service).await?;
        Ok(())
    }

    /// 创建工具路由器并加载所有动态工具（工作流、MCP、插件）
    pub async fn new_with_all_tools(
        db_service: Option<&std::sync::Arc<sentinel_db::DatabaseService>>,
    ) -> Self {
        let mut router = Self::new();
        router.db_service = db_service.cloned();
        router.all_tools = router.load_builtin_tools_from_server().await;

        // 加载工作流工具
        if let Some(db) = db_service {
            if let Ok(workflows) = router.load_workflow_tools(db).await {
                router.workflow_tools = workflows;
            }

            // 加载插件工具
            if let Ok(plugin_tools) = router.load_plugin_tools(db).await {
                router.plugin_tools = plugin_tools;
            }
        }

        // 加载 MCP 工具
        if let Ok(mcp_tools) = router.load_mcp_tools().await {
            router.mcp_tools = mcp_tools;
        }

        router
    }

    /// 使用 LLM 智能选择工具
    async fn select_by_llm(
        &self,
        task: &str,
        config: &ToolConfig,
        llm_config: Option<&sentinel_llm::LlmConfig>,
    ) -> Result<Vec<String>> {
        use sentinel_llm::{LlmClient, LlmConfig};

        // 先添加固定工具
        let mut selected = config.fixed_tools.clone();

        // 获取所有可用工具
        let all_tools = self.get_all_available_tools();

        // 构建工具列表摘要
        let tools_summary = all_tools
            .iter()
            .filter(|t| !config.disabled_tools.contains(&t.id))
            .filter(|t| !selected.contains(&t.id))
            .map(|t| format!("- {}: {}", t.name, t.description))
            .collect::<Vec<_>>()
            .join("\n");

        if tools_summary.is_empty() {
            return Ok(selected);
        }

        let remaining_slots = config.max_tools.saturating_sub(selected.len());
        if remaining_slots == 0 {
            return Ok(selected);
        }

        // 构建 LLM prompt
        let system_prompt = format!(
            r#"Available tools:
{}

Select the {} most relevant tools for this task. Consider:
1. Which tools are directly needed for the task
2. Which tools provide complementary functionality
3. Prioritize tools that are essential over optional ones

Return ONLY the tool names, one per line, no explanations or extra text."#,
            tools_summary, remaining_slots
        );

        let user_prompt = format!("Task: {}", task);

        // 使用快速模型（优先使用用户配置，否则使用默认）
        let llm_cfg = if let Some(cfg) = llm_config {
            cfg.clone()
        } else {
            // 默认使用 gpt-3.5-turbo 进行工具选择
            LlmConfig::new("openai", "gpt-3.5-turbo").with_timeout(30)
        };

        let client = LlmClient::new(llm_cfg);

        tracing::info!(
            "Using LLM to select tools for task: {}",
            task.chars().take(100).collect::<String>()
        );

        match client.completion(Some(&system_prompt), &user_prompt).await {
            Ok(response) => {
                // 解析响应，提取工具名称
                let tool_names: Vec<String> = response
                    .lines()
                    .filter_map(|line| {
                        let name = line.trim().trim_start_matches('-').trim();
                        if name.is_empty() {
                            return None;
                        }
                        // 查找匹配的工具
                        all_tools
                            .iter()
                            .find(|t| t.name == name || t.id == name)
                            .map(|t| t.id.clone())
                    })
                    .take(remaining_slots)
                    .collect();

                tracing::info!("LLM selected {} tools: {:?}", tool_names.len(), tool_names);

                selected.extend(tool_names);
                Ok(selected)
            }
            Err(e) => {
                tracing::warn!(
                    "LLM tool selection failed: {}, falling back to keyword matching",
                    e
                );
                // 失败时回退到关键词匹配
                Ok(self.select_by_keywords(task, config)?)
            }
        }
    }

    /// 混合策略：关键词匹配 + LLM 验证
    async fn select_hybrid(
        &self,
        task: &str,
        config: &ToolConfig,
        llm_config: Option<&sentinel_llm::LlmConfig>,
    ) -> Result<Vec<String>> {
        // 1. 先用关键词匹配获取候选工具（扩大范围）
        let mut expanded_config = config.clone();
        expanded_config.max_tools = (config.max_tools * 2).min(15); // 扩大到 2 倍，最多 15 个

        let keyword_candidates = self.select_by_keywords(task, &expanded_config)?;

        if keyword_candidates.len() <= config.max_tools {
            // 候选工具数量已经合适，直接返回
            return Ok(keyword_candidates);
        }

        // 2. 使用 LLM 从候选工具中精选
        let all_tools = self.get_all_available_tools();
        let candidate_tools: Vec<_> = keyword_candidates
            .iter()
            .filter_map(|id| all_tools.iter().find(|t| &t.id == id))
            .collect();

        let tools_summary = candidate_tools
            .iter()
            .map(|t| format!("- {}: {}", t.name, t.description))
            .collect::<Vec<_>>()
            .join("\n");

        let system_prompt = format!(
            r#"Pre-selected candidate tools (from keyword matching):
{}

From these {} candidates, select the {} most essential tools for the task.
Focus on tools that are directly needed, not just potentially useful.

Return ONLY the tool names, one per line."#,
            tools_summary,
            candidate_tools.len(),
            config.max_tools
        );

        let user_prompt = format!("Task: {}", task);

        use sentinel_llm::{LlmClient, LlmConfig};

        let llm_cfg = if let Some(cfg) = llm_config {
            cfg.clone()
        } else {
            LlmConfig::new("openai", "gpt-3.5-turbo").with_timeout(30)
        };

        let client = LlmClient::new(llm_cfg);

        match client.completion(Some(&system_prompt), &user_prompt).await {
            Ok(response) => {
                let tool_names: Vec<String> = response
                    .lines()
                    .filter_map(|line| {
                        let name = line.trim().trim_start_matches('-').trim();
                        if name.is_empty() {
                            return None;
                        }
                        candidate_tools
                            .iter()
                            .find(|t| t.name == name || t.id == name)
                            .map(|t| t.id.clone())
                    })
                    .take(config.max_tools)
                    .collect();

                tracing::info!(
                    "Hybrid selection: {} keywords -> {} LLM refined -> {} final",
                    keyword_candidates.len(),
                    tool_names.len(),
                    tool_names.len()
                );

                Ok(tool_names)
            }
            Err(e) => {
                tracing::warn!("Hybrid LLM refinement failed: {}, using keyword results", e);
                // LLM 失败时，返回关键词匹配结果（截断到 max_tools）
                Ok(keyword_candidates
                    .into_iter()
                    .take(config.max_tools)
                    .collect())
            }
        }
    }

    /// Skills mode: two-phase progressive disclosure
    /// Phase 1: Show skill summaries to LLM, let it pick one
    /// Phase 2: Load full skill (content + tools), inject context
    #[allow(dead_code)]
    async fn plan_tools_skills(
        &self,
        task: &str,
        config: &ToolConfig,
        llm_config: Option<&sentinel_llm::LlmConfig>,
        _allowed_groups: &[String],
        _db_pool: Option<&sentinel_db::sqlx_compat::PgPool>,
    ) -> Result<ToolSelectionPlan> {
        use sentinel_db::Database;
        use sentinel_llm::{LlmClient, LlmConfig};

        // Need DB service for skill queries
        let db = match &self.db_service {
            Some(db) => db,
            None => {
                tracing::warn!("Skills mode requires db_service, falling back to Keyword");
                let tool_ids = self.select_by_keywords(task, config)?;
                return Ok(ToolSelectionPlan {
                    tool_ids,
                    injected_system_prompt: None,
                    selected_skill: None,
                });
            }
        };

        // Phase 1: Load all skill summaries (Claude-style auto discovery)
        let skills = db.list_skills_summary().await?;
        let mut enabled_skills = Vec::new();
        for skill in skills {
            if self.is_skill_enabled(&skill.id).await {
                enabled_skills.push(skill);
            }
        }

        if enabled_skills.is_empty() {
            tracing::warn!("No skills found, falling back to Keyword");
            let tool_ids = self.select_by_keywords(task, config)?;
            return Ok(ToolSelectionPlan {
                tool_ids,
                injected_system_prompt: None,
                selected_skill: None,
            });
        }

        // Build skill selection prompt
        let skills_summary = enabled_skills
            .iter()
            .map(|s| format!("- {}: {}", s.name, s.description))
            .collect::<Vec<_>>()
            .join("\n");

        let system_prompt = format!(
            r#"Available skills:
{}

Instructions:
- If the task requires specialized tools or workflows, select the most relevant skills.
- If multiple skills are needed, return a JSON array of skill names (max 3).
- If the task is general chat (e.g., "who are you", "hello"), simple Q&A, or does not require any tools, choose "General".

Return ONLY:
- "General" (one word), or
- A JSON array of skill names (example: ["ctf-pwn","binary-analysis"]), or
- A single skill name."#,
            skills_summary
        );

        let user_prompt = format!("Task: {}", task);

        // Call LLM to select skill
        let llm_cfg = llm_config
            .cloned()
            .unwrap_or_else(|| LlmConfig::new("openai", "gpt-3.5-turbo").with_timeout(30));
        let client = LlmClient::new(llm_cfg);

        let selected_skill_raw = match client.completion(Some(&system_prompt), &user_prompt).await {
            Ok(response) => response.trim().to_string(),
            Err(e) => {
                tracing::warn!(
                    "Skill selection LLM call failed: {}, falling back to Keyword",
                    e
                );
                let tool_ids = self.select_by_keywords(task, config)?;
                return Ok(ToolSelectionPlan {
                    tool_ids,
                    injected_system_prompt: None,
                    selected_skill: None,
                });
            }
        };

        // Check for General/None response
        let selected_skill_raw = selected_skill_raw.trim().to_string();
        if selected_skill_raw.eq_ignore_ascii_case("General")
            || selected_skill_raw.eq_ignore_ascii_case("None")
        {
            tracing::info!("Tool Router: 'General' mode selected, skipping specialized tools.");
            return Ok(ToolSelectionPlan {
                tool_ids: vec![],
                injected_system_prompt: None,
                selected_skill: None,
            });
        }

        let mut requested_names: Vec<String> = if selected_skill_raw.starts_with('[') {
            match serde_json::from_str::<Vec<String>>(&selected_skill_raw) {
                Ok(list) => list,
                Err(_) => vec![selected_skill_raw.clone()],
            }
        } else {
            vec![selected_skill_raw.clone()]
        };

        requested_names.retain(|s| !s.trim().is_empty());
        if requested_names.is_empty() {
            tracing::warn!("Empty skill selection, falling back to Keyword");
            let tool_ids = self.select_by_keywords(task, config)?;
            return Ok(ToolSelectionPlan {
                tool_ids,
                injected_system_prompt: None,
                selected_skill: None,
            });
        }

        if requested_names.len() > 3 {
            requested_names.truncate(3);
        }

        let mut matched_skill_ids: Vec<String> = Vec::new();
        for name in requested_names.iter() {
            // Try exact match by name or id
            if let Some(s) = enabled_skills
                .iter()
                .find(|s| s.name.eq_ignore_ascii_case(name) || s.id.eq_ignore_ascii_case(name))
            {
                matched_skill_ids.push(s.id.clone());
                continue;
            }
            // Fuzzy match: contains skill name
            if let Some(s) = enabled_skills.iter().find(|s| {
                name.to_lowercase().contains(&s.name.to_lowercase())
                    || name.to_lowercase().contains(&s.id.to_lowercase())
            }) {
                tracing::info!("Fuzzy matched skill: '{}'", s.name);
                matched_skill_ids.push(s.id.clone());
            }
        }

        matched_skill_ids.sort();
        matched_skill_ids.dedup();

        if matched_skill_ids.is_empty() {
            tracing::warn!(
                "Could not match LLM response '{}' to any skill, falling back to Keyword",
                selected_skill_raw
            );
            let tool_ids = self.select_by_keywords(task, config)?;
            return Ok(ToolSelectionPlan {
                tool_ids,
                injected_system_prompt: None,
                selected_skill: None,
            });
        }

        let mut full_skills = Vec::new();
        for skill_id in matched_skill_ids.iter() {
            if !self.is_skill_enabled(skill_id).await {
                tracing::info!("Skill {} is disabled; skipping", skill_id);
                continue;
            }
            match db.get_skill(skill_id).await? {
                Some(s) => full_skills.push(s),
                None => {
                    tracing::warn!("Skill {} not found, skipping", skill_id);
                }
            }
        }

        if full_skills.is_empty() {
            tracing::warn!("No skills loaded, falling back to Keyword");
            let tool_ids = self.select_by_keywords(task, config)?;
            return Ok(ToolSelectionPlan {
                tool_ids,
                injected_system_prompt: None,
                selected_skill: None,
            });
        }

        // Compute final tool_ids: fixed_tools + skill.allowed_tools - disabled_tools
        let all_available = self.get_all_available_tools();
        let available_ids: std::collections::HashSet<_> =
            all_available.iter().map(|t| &t.id).collect();

        let mut final_tools: Vec<String> = config.fixed_tools.clone();

        // Add skill tools (filter out non-existent and disabled)
        for skill in &full_skills {
            for tool_id in &skill.allowed_tools {
                if config.disabled_tools.contains(tool_id) {
                    continue;
                }
                let normalized_id = tool_id.replace("::", "__");
                let exists =
                    available_ids.contains(tool_id) || available_ids.contains(&normalized_id);
                if exists && !final_tools.contains(tool_id) && !final_tools.contains(&normalized_id)
                {
                    final_tools.push(if available_ids.contains(&normalized_id) {
                        normalized_id
                    } else {
                        tool_id.clone()
                    });
                }
            }
        }

        // Remove disabled from fixed_tools too
        final_tools.retain(|t| !config.disabled_tools.contains(t));

        // Respect max_tools
        if final_tools.len() > config.max_tools {
            final_tools.truncate(config.max_tools);
        }

        // Log if no tools available (this is valid - skill may not need tools)
        if final_tools.is_empty() {
            tracing::info!("Selected skills have no tools configured, proceeding without tools");
        }

        // Build injected system prompt (load SKILL.md body only, one block per skill)
        let mut injected_blocks: Vec<String> = Vec::new();
        if let Some(db_service) = &self.db_service {
            let root = db_service.get_skills_root_dir();
            for skill in &full_skills {
                let skill_path = if !skill.source_path.is_empty() {
                    root.join(&skill.source_path)
                } else {
                    root.join(&skill.id).join("SKILL.md")
                };
                match crate::skills::read_skill_markdown(&skill_path) {
                    Ok(doc) => {
                        let mut body = doc.body;
                        if body.contains("$ARGUMENTS") {
                            body = body.replace("$ARGUMENTS", task);
                        }
                        injected_blocks.push(format!(
                            "\n\n[SkillContentBegin: {}]\n{}\n[SkillContentEnd]",
                            skill.name, body
                        ));
                    }
                    Err(e) => tracing::warn!("Failed to load SKILL.md for {}: {}", skill.id, e),
                }
            }
        }

        let injected = if injected_blocks.is_empty() {
            None
        } else {
            Some(injected_blocks.join(""))
        };

        tracing::info!(
            "Skills selection: skills={:?}, tools={:?}, content_blocks={}",
            full_skills
                .iter()
                .map(|s| s.name.clone())
                .collect::<Vec<_>>(),
            final_tools,
            injected_blocks.len()
        );

        Ok(ToolSelectionPlan {
            tool_ids: final_tools,
            injected_system_prompt: injected,
            selected_skill: Some(SelectedSkill {
                id: full_skills[0].id.clone(),
                name: full_skills[0].name.clone(),
            }),
        })
    }
}

impl Default for ToolRouter {
    fn default() -> Self {
        Self::new()
    }
}

/// 记录工具使用
pub async fn record_tool_usage(
    tool_id: &str,
    tool_name: &str,
    execution_id: &str,
    success: bool,
    execution_time_ms: u64,
    error_message: Option<String>,
) {
    let record = ToolUsageRecord {
        tool_id: tool_id.to_string(),
        tool_name: tool_name.to_string(),
        execution_id: execution_id.to_string(),
        timestamp: chrono::Utc::now().timestamp(),
        success,
        execution_time_ms,
        error_message,
    };

    let mut records = TOOL_USAGE_RECORDS.write().await;
    records.push(record);

    // 保持最近 1000 条记录
    if records.len() > 1000 {
        let excess = records.len() - 1000;
        records.drain(0..excess);
    }
}

/// 获取工具使用统计
pub async fn get_tool_usage_statistics() -> ToolUsageStatistics {
    let records = TOOL_USAGE_RECORDS.read().await;

    let total_executions = records.len();
    let successful_executions = records.iter().filter(|r| r.success).count();
    let failed_executions = total_executions - successful_executions;

    // 按工具统计
    let mut by_tool: HashMap<String, ToolUsageStats> = HashMap::new();

    for record in records.iter() {
        let stats = by_tool
            .entry(record.tool_id.clone())
            .or_insert_with(|| ToolUsageStats {
                tool_id: record.tool_id.clone(),
                tool_name: record.tool_name.clone(),
                execution_count: 0,
                success_count: 0,
                failure_count: 0,
                avg_execution_time_ms: 0.0,
                last_used: 0,
            });

        stats.execution_count += 1;
        if record.success {
            stats.success_count += 1;
        } else {
            stats.failure_count += 1;
        }

        // 更新平均执行时间
        let total_time = stats.avg_execution_time_ms * (stats.execution_count - 1) as f64;
        stats.avg_execution_time_ms =
            (total_time + record.execution_time_ms as f64) / stats.execution_count as f64;

        // 更新最后使用时间
        if record.timestamp > stats.last_used {
            stats.last_used = record.timestamp;
        }
    }

    // 获取最近 50 条记录
    let recent_executions: Vec<ToolUsageRecord> = records.iter().rev().take(50).cloned().collect();

    ToolUsageStatistics {
        total_executions,
        successful_executions,
        failed_executions,
        by_tool,
        recent_executions,
    }
}

/// 清空工具使用记录
pub async fn clear_tool_usage_records() {
    let mut records = TOOL_USAGE_RECORDS.write().await;
    records.clear();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_keyword_matching() {
        let router = ToolRouter::new_with_all_tools(None).await;
        let config = ToolConfig {
            enabled: true,
            selection_strategy: ToolSelectionStrategy::Keyword,
            max_tools: 3,
            fixed_tools: vec![],
            disabled_tools: vec![],
            allowed_tools: vec![],
        };

        // 测试 HTTP 请求任务
        let task = "Make an HTTP request to https://api.example.com";
        let selected = router.select_tools(task, &config, None).await.unwrap();
        assert!(selected.contains(&"http_request".to_string()));
    }

    #[tokio::test]
    async fn test_manual_selection() {
        let router = ToolRouter::new_with_all_tools(None).await;
        let config = ToolConfig {
            enabled: true,
            selection_strategy: ToolSelectionStrategy::Manual(vec!["http_request".to_string()]),
            max_tools: 5,
            fixed_tools: vec![],
            disabled_tools: vec![],
            allowed_tools: vec![],
        };

        let selected = router
            .select_tools("any task", &config, None)
            .await
            .unwrap();
        assert!(selected.contains(&"http_request".to_string()));
        assert!(selected.contains(&"ask_user_question".to_string()));
        assert!(selected.contains(&"tool_search".to_string()));
    }

    #[tokio::test]
    async fn test_disabled_tools() {
        let router = ToolRouter::new_with_all_tools(None).await;
        let config = ToolConfig {
            enabled: true,
            selection_strategy: ToolSelectionStrategy::All,
            max_tools: 10,
            fixed_tools: vec![],
            disabled_tools: vec!["shell".to_string()],
            allowed_tools: vec![],
        };

        let selected = router
            .select_tools("any task", &config, None)
            .await
            .unwrap();
        assert!(!selected.contains(&"shell".to_string()));
        assert!(selected.contains(&"ask_user_question".to_string()));
    }

    #[tokio::test]
    async fn test_deferred_plan_includes_tool_search_and_prompt() {
        let router = ToolRouter::new_with_all_tools(None).await;
        let config = ToolConfig {
            enabled: true,
            selection_strategy: ToolSelectionStrategy::Deferred,
            max_tools: 12,
            fixed_tools: vec![],
            disabled_tools: vec![],
            allowed_tools: vec![],
        };

        let plan = router
            .plan_tools("find the right tool and activate it", &config, None)
            .await
            .unwrap();

        assert!(plan.tool_ids.contains(&"tool_search".to_string()));
        assert!(plan.tool_ids.contains(&"ask_user_question".to_string()));
        assert!(plan
            .injected_system_prompt
            .as_deref()
            .unwrap_or_default()
            .contains("Deferred tool mode is active"));
        assert!(plan
            .injected_system_prompt
            .as_deref()
            .unwrap_or_default()
            .contains("recommended_tool_ids"));
    }
}
