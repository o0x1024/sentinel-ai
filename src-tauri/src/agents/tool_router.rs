//! Tool Router - 智能工具选择和路由
//!
//! 根据任务内容选择相关工具，避免将所有工具传给 LLM 造成 token 浪费。

use anyhow::Result;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 工具元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMetadata {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: ToolCategory,
    pub tags: Vec<String>,
    pub cost_estimate: ToolCost,
    pub always_available: bool,
}

/// 工具分类
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ToolCategory {
    Network,
    Security,
    Data,
    AI,
    System,
    MCP,
    Plugin,
    Workflow,
}

/// 工具成本估算（token 数量）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolCost {
    Low,    // < 100 tokens
    Medium, // 100-500 tokens
    High,   // > 500 tokens
}

/// 工具选择策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolSelectionStrategy {
    /// 全部工具（不推荐，仅用于测试）
    All,
    /// 关键词匹配（快速，免费）
    Keyword,
    /// LLM 智能分析（准确，有成本）
    LLM,
    /// 混合策略（关键词 + LLM）
    Hybrid,
    /// 用户手动指定
    Manual(Vec<String>),
    /// 不使用工具
    None,
}

impl Default for ToolSelectionStrategy {
    fn default() -> Self {
        Self::Keyword
    }
}

/// 工具配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolConfig {
    /// 工具选择策略
    pub selection_strategy: ToolSelectionStrategy,
    /// 最大工具数量
    pub max_tools: usize,
    /// 固定启用的工具
    pub fixed_tools: Vec<String>,
    /// 禁用的工具
    pub disabled_tools: Vec<String>,
    /// 是否启用工具调用
    pub enabled: bool,
}

/// 工具统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolStatistics {
    pub total_tools: usize,
    pub builtin_tools: usize,
    pub workflow_tools: usize,
    pub mcp_tools: usize,
    pub plugin_tools: usize,
    pub always_available: usize,
    pub by_category: HashMap<String, usize>,
    pub by_cost: HashMap<String, usize>,
}

/// 工具使用记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUsageRecord {
    pub tool_id: String,
    pub tool_name: String,
    pub execution_id: String,
    pub timestamp: i64,
    pub success: bool,
    pub execution_time_ms: u64,
    pub error_message: Option<String>,
}

/// 工具使用统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUsageStatistics {
    pub total_executions: usize,
    pub successful_executions: usize,
    pub failed_executions: usize,
    pub by_tool: HashMap<String, ToolUsageStats>,
    pub recent_executions: Vec<ToolUsageRecord>,
}

/// 单个工具的使用统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUsageStats {
    pub tool_id: String,
    pub tool_name: String,
    pub execution_count: usize,
    pub success_count: usize,
    pub failure_count: usize,
    pub avg_execution_time_ms: f64,
    pub last_used: i64,
}

impl Default for ToolConfig {
    fn default() -> Self {
        Self {
            selection_strategy: ToolSelectionStrategy::Keyword,
            max_tools: 5,
            fixed_tools: vec!["local_time".to_string()],
            disabled_tools: vec![],
            enabled: false, // 默认关闭，避免意外消耗
        }
    }
}

/// 全局工具使用记录
static TOOL_USAGE_RECORDS: Lazy<Arc<RwLock<Vec<ToolUsageRecord>>>> =
    Lazy::new(|| Arc::new(RwLock::new(Vec::new())));

/// 工具路由器
pub struct ToolRouter {
    all_tools: Vec<ToolMetadata>,
    workflow_tools: Vec<ToolMetadata>,
    mcp_tools: Vec<ToolMetadata>,
    plugin_tools: Vec<ToolMetadata>,
}

impl ToolRouter {
    /// 创建新的工具路由器
    pub fn new() -> Self {
        Self {
            all_tools: Self::build_default_tools(),
            workflow_tools: Vec::new(),
            mcp_tools: Vec::new(),
            plugin_tools: Vec::new(),
        }
    }

    /// 创建工具路由器并加载动态工具（工作流、MCP、插件）
    pub async fn new_with_dynamic_tools(
        db_service: Option<&std::sync::Arc<sentinel_db::DatabaseService>>,
    ) -> Self {
        let mut router = Self::new();

        // 加载工作流工具
        if let Some(db) = db_service {
            if let Ok(workflows) = router.load_workflow_tools(db).await {
                router.workflow_tools = workflows;
            }
        }

        router
    }

    /// 构建默认工具列表
    fn build_default_tools() -> Vec<ToolMetadata> {
        vec![
            // 网络工具
            ToolMetadata {
                id: "port_scan".to_string(),
                name: "port_scan".to_string(),
                description:
                    "Scan TCP ports on target IP address to discover open ports and services"
                        .to_string(),
                category: ToolCategory::Network,
                tags: vec![
                    "network".to_string(),
                    "scan".to_string(),
                    "port".to_string(),
                    "tcp".to_string(),
                ],
                cost_estimate: ToolCost::Medium,
                always_available: false,
            },
            ToolMetadata {
                id: "http_request".to_string(),
                name: "http_request".to_string(),
                description: "Make HTTP/HTTPS requests to any URL with custom headers and body"
                    .to_string(),
                category: ToolCategory::Network,
                tags: vec![
                    "http".to_string(),
                    "request".to_string(),
                    "api".to_string(),
                    "web".to_string(),
                ],
                cost_estimate: ToolCost::Medium,
                always_available: false,
            },
            // 系统工具
            ToolMetadata {
                id: "local_time".to_string(),
                name: "local_time".to_string(),
                description: "Get current local or UTC time in various formats".to_string(),
                category: ToolCategory::System,
                tags: vec!["time".to_string(), "date".to_string(), "clock".to_string()],
                cost_estimate: ToolCost::Low,
                always_available: true,
            },
            ToolMetadata {
                id: "shell".to_string(),
                name: "shell".to_string(),
                description: "Execute shell commands on the system (use with caution)".to_string(),
                category: ToolCategory::System,
                tags: vec![
                    "shell".to_string(),
                    "command".to_string(),
                    "execute".to_string(),
                    "bash".to_string(),
                ],
                cost_estimate: ToolCost::Medium,
                always_available: false,
            },
            // AI工具
            ToolMetadata {
                id: "vision_explorer".to_string(),
                name: "vision_explorer".to_string(),
                description: "Explore a website using vision capabilities to discover APIs, pages, and interactive elements.".to_string(),
                category: ToolCategory::AI,
                tags: vec![
                    "vision".to_string(),
                    "explorer".to_string(),
                    "web".to_string(),
                    "crawl".to_string(),
                    "api".to_string(),
                ],
                cost_estimate: ToolCost::High,
                always_available: false,
            },
        ]
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

        match &config.selection_strategy {
            ToolSelectionStrategy::None => Ok(vec![]),
            ToolSelectionStrategy::All => {
                let mut all = self.get_all_available_tools();
                all.retain(|t| !config.disabled_tools.contains(&t.id));
                Ok(all.into_iter().map(|t| t.id).collect())
            }
            ToolSelectionStrategy::Manual(tools) => {
                let all_tools = self.get_all_available_tools();
                let mut seen = std::collections::HashSet::new();
                let result: Vec<String> = tools
                    .iter()
                    .filter(|t| !config.disabled_tools.contains(t))
                    .filter_map(|t| {
                        let tool_id = {
                            // 1. Exact match
                            if let Some(found) = all_tools.iter().find(|meta| &meta.id == t) {
                                found.id.clone()
                            } else {
                                // 2. Legacy :: to __ match
                                let replaced = t.replace("::", "__");
                                if let Some(found) = all_tools.iter().find(|meta| meta.id == replaced) {
                                    found.id.clone()
                                } else {
                                    // 3. Strict sanitization match (for plugins etc)
                                    let sanitized =
                                        replaced.replace(|c: char| !c.is_alphanumeric() && c != '_', "_");
                                    if let Some(found) = all_tools.iter().find(|meta| meta.id == sanitized) {
                                        found.id.clone()
                                    } else {
                                        // Fallback
                                        replaced
                                    }
                                }
                            }
                        };
                        // Deduplicate: only include if not seen before
                        if seen.insert(tool_id.clone()) {
                            Some(tool_id)
                        } else {
                            None
                        }
                    })
                    .collect();
                Ok(result)
            }
            ToolSelectionStrategy::Keyword => self.select_by_keywords(task, config),
            ToolSelectionStrategy::LLM => self.select_by_llm(task, config, llm_config).await,
            ToolSelectionStrategy::Hybrid => self.select_hybrid(task, config, llm_config).await,
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

    /// 关键词匹配选择工具（快速，无额外成本）
    fn select_by_keywords(&self, task: &str, config: &ToolConfig) -> Result<Vec<String>> {
        let task_lower = task.to_lowercase();
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
            if task_lower.contains("http") || task_lower.contains("api") {
                if tool.id == "http_request" {
                    score += 15;
                }
            }
            if task_lower.contains("time") || task_lower.contains("date") {
                if tool.id == "local_time" {
                    score += 15;
                }
            }
            if task_lower.contains("command")
                || task_lower.contains("shell")
                || task_lower.contains("execute")
            {
                if tool.id == "shell" {
                    score += 15;
                }
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
        let mut workflow_tools = Vec::new();

        match db_service.list_workflow_definitions(Some(false)).await {
            Ok(workflows) => {
                for workflow in workflows {
                    // 只加载被标记为工具的工作流 (is_tool = true)
                    let is_tool = workflow
                        .get("is_tool")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);

                    if !is_tool {
                        continue;
                    }

                    if let (Some(id), Some(name), description) = (
                        workflow.get("id").and_then(|v| v.as_str()),
                        workflow.get("name").and_then(|v| v.as_str()),
                        workflow.get("description").and_then(|v| v.as_str()),
                    ) {
                        // 提取工作流的标签
                        let tags = extract_workflow_tags(name, description);

                        workflow_tools.push(ToolMetadata {
                            id: format!("workflow__{}", id),
                            name: name.to_string(), // Workflow name usually doesn't have ::, but id does. name is safe?
                            // Workflow name comes from DB. Could be anything.
                            // But ID construction used ::.
                            // Here name is just name.to_string().
                            description: description.unwrap_or("Workflow tool").to_string(),
                            category: ToolCategory::Workflow,
                            tags,
                            cost_estimate: ToolCost::High, // 工作流通常较复杂
                            always_available: false,
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
                            cost_estimate: ToolCost::Medium,
                            always_available: false,
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

        // 查询所有已启用的 agent 类型插件
        match db_service.get_plugins_from_registry().await {
            Ok(plugins) => {
                for plugin in plugins {
                    // 只加载已启用的 agent 类型插件
                    let enabled = plugin
                        .get("enabled")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    let main_category = plugin
                        .get("main_category")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");

                    if enabled && main_category == "agent" {
                        let id = plugin.get("id").and_then(|v| v.as_str()).unwrap_or("");
                        let name = plugin
                            .get("name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Unknown");
                        let description = plugin
                            .get("description")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Agent plugin tool");

                        // 从插件元数据中提取标签
                        let mut tags = vec!["plugin".to_string(), "agent".to_string()];
                        if let Some(plugin_tags) = plugin.get("tags").and_then(|v| v.as_array()) {
                            for tag in plugin_tags {
                                if let Some(tag_str) = tag.as_str() {
                                    tags.push(tag_str.to_string());
                                }
                            }
                        }

                        let sanitized_id =
                            id.replace(|c: char| !c.is_alphanumeric() && c != '_', "_");
                        plugin_tools.push(ToolMetadata {
                            id: format!("plugin__{}", sanitized_id),
                            name: name.to_string(), // Plugin name usually safe.
                            description: description.to_string(),
                            category: ToolCategory::Plugin,
                            tags,
                            cost_estimate: ToolCost::Medium,
                            always_available: false,
                        });
                    }
                }

                tracing::info!("Loaded {} plugin tools", plugin_tools.len());
            }
            Err(e) => {
                tracing::warn!("Failed to load plugin tools: {}", e);
            }
        }

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
        let prompt = format!(
            r#"Task: {}

Available tools:
{}

Select the {} most relevant tools for this task. Consider:
1. Which tools are directly needed for the task
2. Which tools provide complementary functionality
3. Prioritize tools that are essential over optional ones

Return ONLY the tool names, one per line, no explanations or extra text."#,
            task, tools_summary, remaining_slots
        );

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

        match client.completion(None, &prompt).await {
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

        let prompt = format!(
            r#"Task: {}

Pre-selected candidate tools (from keyword matching):
{}

From these {} candidates, select the {} most essential tools for the task.
Focus on tools that are directly needed, not just potentially useful.

Return ONLY the tool names, one per line."#,
            task,
            tools_summary,
            candidate_tools.len(),
            config.max_tools
        );

        use sentinel_llm::{LlmClient, LlmConfig};

        let llm_cfg = if let Some(cfg) = llm_config {
            cfg.clone()
        } else {
            LlmConfig::new("openai", "gpt-3.5-turbo").with_timeout(30)
        };

        let client = LlmClient::new(llm_cfg);

        match client.completion(None, &prompt).await {
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

/// 从工作流名称和描述中提取标签
fn extract_workflow_tags(name: &str, description: Option<&str>) -> Vec<String> {
    let mut tags = Vec::new();

    // 从名称提取
    let name_lower = name.to_lowercase();
    let name_words: Vec<&str> = name_lower.split(|c: char| !c.is_alphanumeric()).collect();

    for word in name_words {
        if word.len() > 2 {
            tags.push(word.to_string());
        }
    }

    // 从描述提取关键词
    if let Some(desc) = description {
        let desc_lower = desc.to_lowercase();

        // 常见的工作流类型关键词
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

    // 去重
    tags.sort();
    tags.dedup();

    tags
}

/// 从 MCP 工具名称和描述中提取标签
fn extract_mcp_tool_tags(name: &str, description: &str) -> Vec<String> {
    let mut tags = Vec::new();

    // 从名称提取
    let name_lower = name.to_lowercase();
    let name_words: Vec<&str> = name_lower.split(|c: char| !c.is_alphanumeric()).collect();

    for word in name_words {
        if word.len() > 2 {
            tags.push(word.to_string());
        }
    }

    // 从描述提取关键词
    let desc_lower = description.to_lowercase();

    // 常见的 MCP 工具类型关键词
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

    // 添加 MCP 标签
    tags.push("mcp".to_string());

    // 去重
    tags.sort();
    tags.dedup();

    tags
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_keyword_matching() {
        let router = ToolRouter::new();
        let config = ToolConfig {
            enabled: true,
            selection_strategy: ToolSelectionStrategy::Keyword,
            max_tools: 3,
            fixed_tools: vec![],
            disabled_tools: vec![],
        };

        // 测试端口扫描任务
        let task = "Scan ports on 192.168.1.1 to find open services";
        let selected = router.select_tools(task, &config, None).await.unwrap();
        assert!(selected.contains(&"port_scan".to_string()));

        // 测试 HTTP 请求任务
        let task = "Make an HTTP request to https://api.example.com";
        let selected = router.select_tools(task, &config, None).await.unwrap();
        assert!(selected.contains(&"http_request".to_string()));

        // 测试时间查询任务
        let task = "What is the current time?";
        let selected = router.select_tools(task, &config, None).await.unwrap();
        assert!(selected.contains(&"local_time".to_string()));
    }

    #[tokio::test]
    async fn test_manual_selection() {
        let router = ToolRouter::new();
        let config = ToolConfig {
            enabled: true,
            selection_strategy: ToolSelectionStrategy::Manual(vec![
                "port_scan".to_string(),
                "http_request".to_string(),
            ]),
            max_tools: 5,
            fixed_tools: vec![],
            disabled_tools: vec![],
        };

        let selected = router
            .select_tools("any task", &config, None)
            .await
            .unwrap();
        assert_eq!(selected.len(), 2);
        assert!(selected.contains(&"port_scan".to_string()));
        assert!(selected.contains(&"http_request".to_string()));
    }

    #[tokio::test]
    async fn test_disabled_tools() {
        let router = ToolRouter::new();
        let config = ToolConfig {
            enabled: true,
            selection_strategy: ToolSelectionStrategy::All,
            max_tools: 10,
            fixed_tools: vec![],
            disabled_tools: vec!["shell".to_string()],
        };

        let selected = router
            .select_tools("any task", &config, None)
            .await
            .unwrap();
        assert!(!selected.contains(&"shell".to_string()));
    }
}
