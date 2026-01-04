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
#[derive(Default)]
pub enum ToolSelectionStrategy {
    /// 全部工具（不推荐，仅用于测试）
    All,
    /// 关键词匹配（快速，免费）
    #[default]
    Keyword,
    /// LLM 智能分析（准确，有成本）
    LLM,
    /// 混合策略（关键词 + LLM）
    Hybrid,
    /// 用户手动指定
    Manual(Vec<String>),
    /// 能力组模式（渐进式披露）
    /// Vec<String> 为允许参与选择的 ability_group_id 列表；空表示全部
    Ability(Vec<String>),
    /// 不使用工具
    None,
}

/// 选中的 Ability Group 摘要
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectedAbilityGroup {
    pub id: String,
    pub name: String,
}

/// 工具选择计划（扩展返回类型，支持注入上下文）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSelectionPlan {
    /// 最终选中的工具 ID 列表
    pub tool_ids: Vec<String>,
    /// 需要注入到 system_prompt 的额外内容（来自 Ability instructions）
    pub injected_system_prompt: Option<String>,
    /// 选中的 Ability 组信息
    pub selected_ability_group: Option<SelectedAbilityGroup>,
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
    db_service: Option<Arc<sentinel_db::DatabaseService>>,
}

impl ToolRouter {
    /// 创建新的工具路由器
    pub fn new() -> Self {
        Self {
            all_tools: Self::build_default_tools(),
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
        use sentinel_tools::buildin_tools::*;

        vec![
            // 网络工具
            ToolMetadata {
                id: PortScanTool::NAME.to_string(),
                name: PortScanTool::NAME.to_string(),
                description: PortScanTool::DESCRIPTION.to_string(),
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
                id: HttpRequestTool::NAME.to_string(),
                name: HttpRequestTool::NAME.to_string(),
                description: HttpRequestTool::DESCRIPTION.to_string(),
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
            ToolMetadata {
                id: SubdomainBruteTool::NAME.to_string(),
                name: SubdomainBruteTool::NAME.to_string(),
                description: SubdomainBruteTool::DESCRIPTION.to_string(),
                category: ToolCategory::Network,
                tags: vec![
                    "subdomain".to_string(),
                    "brute".to_string(),
                    "dns".to_string(),
                    "scan".to_string(),
                    "network".to_string(),
                    "security".to_string(),
                ],
                cost_estimate: ToolCost::High,
                always_available: false,
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
                    "information".to_string(),
                    "research".to_string(),
                    "tavily".to_string(),
                ],
                cost_estimate: ToolCost::Medium,
                always_available: false,
            },
            // 系统工具
            ToolMetadata {
                id: LocalTimeTool::NAME.to_string(),
                name: LocalTimeTool::NAME.to_string(),
                description: LocalTimeTool::DESCRIPTION.to_string(),
                category: ToolCategory::System,
                tags: vec!["time".to_string(), "date".to_string(), "clock".to_string()],
                cost_estimate: ToolCost::Low,
                always_available: true,
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
                cost_estimate: ToolCost::Medium,
                always_available: false,
            },
            // Planning tool
            ToolMetadata {
                id: TaskPlannerTool::NAME.to_string(),
                name: TaskPlannerTool::NAME.to_string(),
                description: TaskPlannerTool::DESCRIPTION.to_string(),
                category: ToolCategory::System,
                tags: vec!["plan".to_string(), "task".to_string(), "autonomous".to_string(), "workflow".to_string()],
                cost_estimate: ToolCost::Low,
                always_available: true,
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
            // Test Explorer (unified tool)
            ToolMetadata {
                id: "test_explorer".to_string(),
                name: "test_explorer".to_string(),
                description: "Explore a website using text-based automation with LLM. Automatically navigates, interacts with elements, captures API requests, and completes exploration tasks.".to_string(),
                category: ToolCategory::AI,
                tags: vec![
                    "browser".to_string(),
                    "automation".to_string(),
                    "web".to_string(),
                    "api".to_string(),
                    "crawl".to_string(),
                    "exploration".to_string(),
                ],
                cost_estimate: ToolCost::Medium,
                always_available: false,
            },
            // Memory Manager
            ToolMetadata {
                id: MemoryManagerTool::NAME.to_string(),
                name: MemoryManagerTool::NAME.to_string(),
                description: MemoryManagerTool::DESCRIPTION.to_string(),
                category: ToolCategory::AI,
                tags: vec![
                    "memory".to_string(),
                    "store".to_string(),
                    "retrieve".to_string(),
                    "remember".to_string(),
                    "recall".to_string(),
                    "vector".to_string(),
                    "knowledge".to_string(),
                ],
                cost_estimate: ToolCost::Low,
                always_available: true,
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
            ToolSelectionStrategy::Ability(allowed_groups) => {
                // Ability mode: delegate to plan_tools and return only tool_ids
                let plan = self
                    .plan_tools_ability(task, config, llm_config, allowed_groups, None)
                    .await?;
                Ok(plan.tool_ids)
            }
        }
    }

    /// Plan tools with full selection plan (supports Ability context injection)
    pub async fn plan_tools(
        &self,
        task: &str,
        config: &ToolConfig,
        llm_config: Option<&sentinel_llm::LlmConfig>,
        db_pool: Option<&sqlx::sqlite::SqlitePool>,
    ) -> Result<ToolSelectionPlan> {
        if !config.enabled {
            return Ok(ToolSelectionPlan {
                tool_ids: vec![],
                injected_system_prompt: None,
                selected_ability_group: None,
            });
        }

        match &config.selection_strategy {
            ToolSelectionStrategy::Ability(allowed_groups) => {
                self.plan_tools_ability(task, config, llm_config, allowed_groups, db_pool)
                    .await
            }
            _ => {
                // For non-Ability strategies, just wrap select_tools result
                let tool_ids = self.select_tools(task, config, llm_config).await?;
                Ok(ToolSelectionPlan {
                    tool_ids,
                    injected_system_prompt: None,
                    selected_ability_group: None,
                })
            }
        }
    }

    /// 获取所有可用工具（包括动态工具）
    pub fn get_all_available_tools(&self) -> Vec<ToolMetadata> {
        let mut tools = self.all_tools.clone();
        tools.extend(self.workflow_tools.clone());
        tools.extend(self.mcp_tools.clone());
        tools.extend(self.plugin_tools.clone());
        
        // Deduplicate by tool ID
        let mut seen = std::collections::HashSet::new();
        tools.retain(|t| seen.insert(t.id.clone()));
        
        tools
    }

    /// 获取工具的完整定义（用于渐进式披露）
    pub async fn get_tool_full_definition(&self, tool_id: &str) -> Result<crate::agents::ToolFullDefinition> {
        
        // Get tool metadata first
        let all_tools = self.get_all_available_tools();
        let tool_meta = all_tools
            .iter()
            .find(|t| t.id == tool_id)
            .ok_or_else(|| anyhow::anyhow!("Tool '{}' not found", tool_id))?;
        
        // Try to get full definition from tool server
        // For now, return basic structure with metadata
        // In the future, this could query the actual tool for its schema
        
        let parameters = serde_json::json!({
            "type": "object",
            "description": format!("Parameters for {}", tool_meta.name),
            "note": "Use the tool to see actual parameter validation"
        });
        
        let usage_notes = Some(format!(
            "Category: {:?}\nCost: {:?}\nTags: {}",
            tool_meta.category,
            tool_meta.cost_estimate,
            tool_meta.tags.join(", ")
        ));
        
        let examples = vec![
            format!("Example: Use {} for {}", tool_meta.name, tool_meta.description)
        ];
        
        Ok(crate::agents::ToolFullDefinition {
            parameters,
            usage_notes,
            examples,
        })
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
            if (task_lower.contains("http") || task_lower.contains("api"))
                && tool.id == "http_request" {
                    score += 15;
                }
            if (task_lower.contains("time") || task_lower.contains("date"))
                && tool.id == "local_time" {
                    score += 15;
                }
            if (task_lower.contains("command")
                || task_lower.contains("shell")
                || task_lower.contains("execute"))
                && tool.id == "shell" {
                    score += 15;
                }
            if (task_lower.contains("memory")
                || task_lower.contains("remember")
                || task_lower.contains("recall")
                || task_lower.contains("store")
                || task_lower.contains("save"))
                && tool.id == "memory_manager" {
                    score += 25; // High priority for memory operations
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

        // 直接查询数据库获取已启用的 agent 类型插件
        // 使用与 TrafficAnalysisState::list_plugins_internal 相同的查询方式
        let rows = sqlx::query_as::<
            _,
            (
                String,         // id
                String,         // name
                Option<String>, // description
                Option<String>, // tags (JSON)
            ),
        >(
            r#"
            SELECT id, name, description, tags
            FROM plugin_registry
            WHERE main_category = 'agent' 
              AND enabled = 1 
              AND validation_status = 'Approved'
            "#,
        )
        .fetch_all(db_service.pool())
        .await;

        match rows {
            Ok(plugins) => {
                for (id, name, description, tags_json) in plugins {
                    let description_str = description.as_deref().unwrap_or("Agent plugin tool");

                    // 从 tags JSON 提取标签
                    let mut tags = vec!["plugin".to_string(), "agent".to_string()];
                    if let Some(tags_str) = tags_json {
                        if let Ok(parsed_tags) = serde_json::from_str::<Vec<String>>(&tags_str) {
                            for tag in parsed_tags {
                                tags.push(tag);
                            }
                        }
                    }

                    let sanitized_id = id.replace(|c: char| !c.is_alphanumeric() && c != '_', "_");
                    plugin_tools.push(ToolMetadata {
                        id: format!("plugin__{}", sanitized_id),
                        name: name.clone(),
                        description: description_str.to_string(),
                        category: ToolCategory::Plugin,
                        tags,
                        cost_estimate: ToolCost::Medium,
                        always_available: false,
                    });
                }

                tracing::info!("Loaded {} plugin tools from database", plugin_tools.len());
            }
            Err(e) => {
                tracing::warn!("Failed to load plugin tools from database: {}", e);
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
        router.db_service = db_service.cloned();

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

    /// Ability mode: two-phase progressive disclosure
    /// Phase 1: Show group summaries to LLM, let it pick one
    /// Phase 2: Load full group (instructions + tools), inject context
    async fn plan_tools_ability(
        &self,
        task: &str,
        config: &ToolConfig,
        _llm_config: Option<&sentinel_llm::LlmConfig>,
        allowed_groups: &[String],
        _db_pool: Option<&sqlx::sqlite::SqlitePool>,
    ) -> Result<ToolSelectionPlan> {
        use sentinel_db::Database;

        // Need DB service for ability group queries
        let db = match &self.db_service {
            Some(db) => db,
            None => {
                tracing::warn!("Ability mode requires db_service, falling back to Keyword");
                let tool_ids = self.select_by_keywords(task, config)?;
                return Ok(ToolSelectionPlan {
                    tool_ids,
                    injected_system_prompt: None,
                    selected_ability_group: None,
                });
            }
        };

        // 🔑 新方案：不使用单独的LLM调用选择能力组
        // 直接使用第一个匹配的能力组（如果指定了allowed_groups）
        // 或者使用所有能力组的工具（如果没有指定）
        
        let groups = if allowed_groups.is_empty() {
            db.list_ability_groups_summary().await?
        } else {
            db.list_ability_groups_summary_by_ids(allowed_groups).await?
        };

        if groups.is_empty() {
            tracing::warn!("No ability groups found, falling back to Keyword");
            let tool_ids = self.select_by_keywords(task, config)?;
            return Ok(ToolSelectionPlan {
                tool_ids,
                injected_system_prompt: None,
                selected_ability_group: None,
            });
        }

        // 如果只有一个能力组，直接使用它
        // 如果有多个，使用第一个（或者可以合并所有组的工具）
        let selected_group_summary = if groups.len() == 1 {
            groups.first().unwrap()
        } else {
            // 多个能力组时，使用第一个
            // TODO: 未来可以考虑合并多个组的工具
            tracing::info!("Multiple ability groups found ({}), using first one: {}", 
                groups.len(), groups[0].name);
            &groups[0]
        };

        tracing::info!("Using ability group: '{}'", selected_group_summary.name);

        // Load full group details
        let full_group = match db.get_ability_group(&selected_group_summary.id).await? {
            Some(g) => g,
            None => {
                tracing::warn!("Ability group {} not found, falling back to Keyword", selected_group_summary.id);
                let tool_ids = self.select_by_keywords(task, config)?;
                return Ok(ToolSelectionPlan {
                    tool_ids,
                    injected_system_prompt: None,
                    selected_ability_group: None,
                });
            }
        };

        // 🔑 渐进式披露改进：注册所有工具但鼓励按需查看详情
        // 这样既能调用工具，又能节省初始 token（通过简化的工具描述）
        
        let all_available = self.get_all_available_tools();
        let available_ids: std::collections::HashSet<_> = all_available.iter().map(|t| &t.id).collect();

        // Include fixed_tools + all group tools + get_tool_definition
        let mut final_tools: Vec<String> = config.fixed_tools.clone();
        
        // Add get_tool_definition tool (this will be registered separately)
        if !final_tools.contains(&"get_tool_definition".to_string()) {
            final_tools.push("get_tool_definition".to_string());
        }

        // Build tool list summary for system prompt
        let mut tool_list_items = Vec::new();
        
        // Add group tools (filter out non-existent and disabled)
        for tool_id in &full_group.tool_ids {
            if config.disabled_tools.contains(tool_id) {
                continue;
            }
            
            let normalized_id = tool_id.replace("::", "__");
            let exists = available_ids.contains(tool_id) || available_ids.contains(&normalized_id);
            
            if exists {
                // Find tool metadata
                let tool_meta = all_available.iter().find(|t| {
                    &t.id == tool_id || t.id == normalized_id
                });
                
                if let Some(meta) = tool_meta {
                    tool_list_items.push(format!("- **{}**: {}", meta.name, meta.description));
                    
                    // Add to final_tools if not already present
                    let tool_id_to_add = if available_ids.contains(&normalized_id) {
                        normalized_id.clone()
                    } else {
                        tool_id.clone()
                    };
                    
                    if !final_tools.contains(&tool_id_to_add) {
                        final_tools.push(tool_id_to_add);
                    }
                }
            }
        }

        // Remove disabled from final_tools
        final_tools.retain(|t| !config.disabled_tools.contains(t));

        // Respect max_tools
        if final_tools.len() > config.max_tools {
            final_tools.truncate(config.max_tools);
        }

        // Warn if no tools available
        if tool_list_items.is_empty() {
            tracing::warn!(
                "Ability group '{}' has no available tools after filtering, falling back to Keyword",
                full_group.name
            );
            let tool_ids = self.select_by_keywords(task, config)?;
            return Ok(ToolSelectionPlan {
                tool_ids,
                injected_system_prompt: None,
                selected_ability_group: None,
            });
        }

        // Build injected system prompt with progressive disclosure instructions
        let tool_list = tool_list_items.join("\n");
        
        let mut injected_content = full_group.instructions.clone();
        
        // Append additional_notes if present
        if !full_group.additional_notes.is_empty() {
            injected_content.push_str("\n\n");
            injected_content.push_str(&full_group.additional_notes);
        }
        
        let injected = format!(
            r#"

[AbilityContext: {}]
Description: {}

{}

## Available Tools

You have access to the following tools:
{}

### Tool Usage Guidelines

**All tools are available for immediate use.** If you're unsure about a tool's parameters or usage:

1. **Option 1 (Recommended)**: Try using the tool directly - the system will validate parameters
2. **Option 2**: Call `get_tool_definition("tool_name")` to see detailed parameter specifications and examples

**When to check tool definition**:
- Complex tools with many parameters
- When you need to see usage examples
- When a tool call fails due to parameter errors

**Example**:
```
User: "Scan the target website"
Assistant: I'll use vision_explorer to scan the website.
[If unsure about parameters, can call get_tool_definition("vision_explorer") first]
[Then calls vision_explorer with parameters]
```
[End of AbilityContext]
"#,
            full_group.name, 
            full_group.description,
            injected_content, 
            tool_list
        );

        tracing::info!(
            "Ability selection (Progressive Disclosure): group='{}', exposed_tools={:?}, available_tools_count={}, instructions_len={}",
            full_group.name,
            final_tools,
            tool_list_items.len(),
            full_group.instructions.len()
        );

        Ok(ToolSelectionPlan {
            tool_ids: final_tools,
            injected_system_prompt: Some(injected),
            selected_ability_group: Some(SelectedAbilityGroup {
                id: full_group.id,
                name: full_group.name,
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
