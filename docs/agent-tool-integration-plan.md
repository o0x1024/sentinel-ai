# Agent 工具集成方案

## 问题分析

如果把所有工具（内置工具、MCP 工具、插件工具）都传给 LLM，会导致：
1. **Token 消耗巨大** - 每次请求都要发送所有工具定义（可能 10k+ tokens）
2. **上下文污染** - 太多工具会降低 LLM 的选择准确性
3. **成本高昂** - 特别是使用 Claude/GPT-4 等模型
4. **响应变慢** - 更长的 prompt 导致推理时间增加

## 解决方案：两阶段工具选择 + 智能过滤

### 方案 1：智能工具路由（推荐）⭐

**核心思路**：根据用户任务，先用轻量级模型选择相关工具，再传给主模型。

```
用户任务 → [轻量级 LLM 分析] → 选择 3-5 个相关工具 → [主 LLM + 工具] → 执行
```

**优势**：
- Token 节省 80%+（只传递相关工具）
- 工具选择准确率高
- 支持大规模工具库（100+ 工具）

**实现步骤**：
1. 工具分类和标签化（network, security, data, ai, etc.）
2. 轻量级路由器（使用 embedding 或小模型）
3. 动态工具注入到 Agent

### 方案 2：工具分组 + 用户配置

**核心思路**：将工具分组，用户/系统预先选择工具组。

```
工具分组:
- 网络工具组: port_scan, http_request, subdomain_scan
- 数据工具组: rag_query, web_search
- 系统工具组: shell, local_time
- MCP 工具组: 按服务器分组
```

**优势**：
- 简单直接
- 用户可控
- 适合特定场景（如安全测试只需网络工具）

**劣势**：
- 需要用户手动选择
- 灵活性较差

### 方案 3：混合方案（最佳实践）⭐⭐⭐

结合方案 1 和 2：
1. **默认工具集**：始终可用的核心工具（3-5 个）
2. **智能推荐**：根据任务自动推荐相关工具
3. **用户自定义**：允许用户固定某些工具

## 技术实现

### 1. 工具元数据增强

```rust
pub struct ToolMetadata {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: ToolCategory,
    pub tags: Vec<String>,
    pub embedding: Option<Vec<f32>>,  // 用于语义搜索
    pub cost_estimate: ToolCost,       // token 成本估算
    pub always_available: bool,        // 是否始终可用
}

pub enum ToolCategory {
    Network,
    Security,
    Data,
    AI,
    System,
    MCP,
    Plugin,
}

pub enum ToolCost {
    Low,      // < 100 tokens
    Medium,   // 100-500 tokens
    High,     // > 500 tokens
}
```

### 2. 工具路由器

```rust
pub struct ToolRouter {
    all_tools: Vec<ToolMetadata>,
    default_tools: Vec<String>,
    embedding_model: Option<EmbeddingModel>,
}

impl ToolRouter {
    /// 根据任务选择相关工具（智能路由）
    pub async fn select_tools(
        &self,
        task: &str,
        max_tools: usize,
        strategy: ToolSelectionStrategy,
    ) -> Result<Vec<String>> {
        match strategy {
            ToolSelectionStrategy::Semantic => {
                // 使用 embedding 相似度
                self.select_by_embedding(task, max_tools).await
            }
            ToolSelectionStrategy::LLM => {
                // 使用轻量级 LLM 分析
                self.select_by_llm(task, max_tools).await
            }
            ToolSelectionStrategy::Keyword => {
                // 使用关键词匹配
                self.select_by_keywords(task, max_tools)
            }
            ToolSelectionStrategy::Hybrid => {
                // 混合策略
                self.select_hybrid(task, max_tools).await
            }
        }
    }
    
    /// 关键词匹配（最快，无额外成本）
    fn select_by_keywords(&self, task: &str, max_tools: usize) -> Vec<String> {
        let task_lower = task.to_lowercase();
        let mut scored_tools = Vec::new();
        
        for tool in &self.all_tools {
            let mut score = 0;
            
            // 检查工具名称
            if task_lower.contains(&tool.name.to_lowercase()) {
                score += 10;
            }
            
            // 检查标签
            for tag in &tool.tags {
                if task_lower.contains(&tag.to_lowercase()) {
                    score += 5;
                }
            }
            
            // 检查描述关键词
            let keywords = extract_keywords(&tool.description);
            for keyword in keywords {
                if task_lower.contains(&keyword.to_lowercase()) {
                    score += 3;
                }
            }
            
            if score > 0 {
                scored_tools.push((tool.id.clone(), score));
            }
        }
        
        // 排序并返回 top-k
        scored_tools.sort_by(|a, b| b.1.cmp(&a.1));
        scored_tools.into_iter()
            .take(max_tools)
            .map(|(id, _)| id)
            .collect()
    }
    
    /// 使用轻量级 LLM 选择工具
    async fn select_by_llm(&self, task: &str, max_tools: usize) -> Result<Vec<String>> {
        // 构建工具列表（简化版）
        let tools_summary = self.all_tools.iter()
            .map(|t| format!("- {}: {}", t.name, t.description))
            .collect::<Vec<_>>()
            .join("\n");
        
        let prompt = format!(
            "Task: {}\n\nAvailable tools:\n{}\n\nSelect the {} most relevant tools for this task. Return only tool names, one per line.",
            task, tools_summary, max_tools
        );
        
        // 使用快速模型（如 gpt-3.5-turbo, claude-haiku）
        let config = LlmConfig::new("openai", "gpt-3.5-turbo");
        let client = LlmClient::new(config);
        let response = client.completion(None, &prompt).await?;
        
        // 解析响应
        let selected_tools = response.lines()
            .filter_map(|line| {
                let name = line.trim().trim_start_matches('-').trim();
                self.all_tools.iter()
                    .find(|t| t.name == name)
                    .map(|t| t.id.clone())
            })
            .collect();
        
        Ok(selected_tools)
    }
}
```

### 3. Agent 配置增强

```rust
pub struct AgentExecuteConfig {
    pub conversation_id: Option<String>,
    pub message_id: Option<String>,
    pub enable_rag: Option<bool>,
    pub enable_web_search: Option<bool>,
    pub attachments: Option<serde_json::Value>,
    pub system_prompt: Option<String>,
    
    // 新增：工具配置
    pub tool_config: Option<ToolConfig>,
}

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

pub enum ToolSelectionStrategy {
    /// 全部工具（不推荐）
    All,
    /// 关键词匹配（快速，免费）
    Keyword,
    /// 语义搜索（需要 embedding）
    Semantic,
    /// LLM 分析（准确，有成本）
    LLM,
    /// 混合策略
    Hybrid,
    /// 用户指定
    Manual(Vec<String>),
}

impl Default for ToolConfig {
    fn default() -> Self {
        Self {
            selection_strategy: ToolSelectionStrategy::Keyword,
            max_tools: 5,
            fixed_tools: vec!["local_time".to_string()],
            disabled_tools: vec![],
            enabled: true,
        }
    }
}
```

### 4. 工具执行流程

```rust
pub async fn agent_execute_with_tools(
    task: String,
    config: AgentExecuteConfig,
    app_handle: AppHandle,
) -> Result<String, String> {
    let tool_config = config.tool_config.unwrap_or_default();
    
    if !tool_config.enabled {
        // 不使用工具，直接调用 LLM
        return agent_execute_simple(task, config, app_handle).await;
    }
    
    // 1. 工具选择
    let tool_router = get_global_tool_router().await?;
    let selected_tool_ids = match tool_config.selection_strategy {
        ToolSelectionStrategy::Manual(tools) => tools,
        ToolSelectionStrategy::All => tool_router.all_tool_ids(),
        _ => {
            tool_router.select_tools(
                &task,
                tool_config.max_tools,
                tool_config.selection_strategy,
            ).await?
        }
    };
    
    // 2. 加载工具定义
    let tools = load_tools_by_ids(&selected_tool_ids).await?;
    
    // 3. 构建 Agent（带工具）
    let agent = build_agent_with_tools(
        &config,
        tools,
        &app_handle,
    ).await?;
    
    // 4. 执行 Agent（工具调用循环）
    let result = execute_agent_loop(
        agent,
        &task,
        &config,
        &app_handle,
    ).await?;
    
    Ok(result)
}

/// Agent 执行循环（支持多轮工具调用）
async fn execute_agent_loop(
    agent: Agent,
    task: &str,
    config: &AgentExecuteConfig,
    app_handle: &AppHandle,
) -> Result<String> {
    let max_iterations = 10;
    let mut iteration = 0;
    let mut current_prompt = task.to_string();
    
    loop {
        iteration += 1;
        if iteration > max_iterations {
            return Err("Max iterations reached".into());
        }
        
        // 调用 LLM
        let response = agent.stream_chat(&current_prompt).await?;
        
        // 检查是否有工具调用
        if let Some(tool_calls) = response.tool_calls {
            // 执行工具
            let mut tool_results = Vec::new();
            for tool_call in tool_calls {
                let result = execute_tool(
                    &tool_call.name,
                    &tool_call.arguments,
                    app_handle,
                ).await?;
                
                tool_results.push(format!(
                    "Tool: {}\nResult: {}",
                    tool_call.name,
                    result
                ));
                
                // 发送工具执行事件到前端
                app_handle.emit("agent:tool_executed", &serde_json::json!({
                    "tool_name": tool_call.name,
                    "result": result,
                }));
            }
            
            // 构建下一轮 prompt
            current_prompt = format!(
                "Tool results:\n{}\n\nContinue with the task.",
                tool_results.join("\n\n")
            );
        } else {
            // 没有工具调用，返回最终结果
            return Ok(response.content);
        }
    }
}
```

## 前端界面设计

### 工具配置面板

```vue
<template>
  <div class="tool-config-panel">
    <div class="tool-toggle">
      <label>
        <input type="checkbox" v-model="toolsEnabled" />
        启用工具调用
      </label>
    </div>
    
    <div v-if="toolsEnabled" class="tool-settings">
      <!-- 工具选择策略 -->
      <div class="setting-group">
        <label>工具选择策略</label>
        <select v-model="toolStrategy">
          <option value="keyword">关键词匹配（快速）</option>
          <option value="llm">智能分析（准确）</option>
          <option value="manual">手动选择</option>
          <option value="all">全部工具</option>
        </select>
      </div>
      
      <!-- 最大工具数 -->
      <div class="setting-group">
        <label>最大工具数</label>
        <input type="number" v-model="maxTools" min="1" max="20" />
      </div>
      
      <!-- 手动选择工具 -->
      <div v-if="toolStrategy === 'manual'" class="tool-selector">
        <div v-for="tool in availableTools" :key="tool.id" class="tool-item">
          <label>
            <input type="checkbox" v-model="selectedTools" :value="tool.id" />
            {{ tool.name }} - {{ tool.description }}
          </label>
        </div>
      </div>
      
      <!-- 固定工具 -->
      <div class="setting-group">
        <label>始终启用的工具</label>
        <div class="tool-chips">
          <span v-for="tool in fixedTools" :key="tool" class="chip">
            {{ tool }}
            <button @click="removeFixedTool(tool)">×</button>
          </span>
        </div>
      </div>
    </div>
  </div>
</template>
```

## Token 成本对比

假设有 50 个工具，每个工具定义平均 200 tokens：

| 方案 | 工具数 | Token 成本 | 说明 |
|------|--------|-----------|------|
| 全部工具 | 50 | ~10,000 tokens | 每次请求都发送 |
| 关键词匹配 | 5 | ~1,000 tokens | 节省 90% |
| LLM 智能选择 | 5 | ~1,500 tokens | 包含选择成本 |
| 手动选择 | 3-5 | ~600-1,000 tokens | 最省 |

## 实施优先级

### Phase 1: 基础工具集成（1-2 天）
- [ ] 实现 ToolRouter 基础框架
- [ ] 实现关键词匹配策略
- [ ] 集成内置工具到 Agent
- [ ] 前端添加工具开关

### Phase 2: 智能选择（2-3 天）
- [ ] 实现 LLM 工具选择
- [ ] 工具元数据管理
- [ ] 前端工具配置面板

### Phase 3: 高级功能（3-5 天）
- [ ] MCP 工具集成
- [ ] 插件工具集成
- [ ] 工具执行可视化
- [ ] 工具使用统计

## 推荐配置

### 默认配置（平衡）
```rust
ToolConfig {
    selection_strategy: ToolSelectionStrategy::Keyword,
    max_tools: 5,
    fixed_tools: vec!["local_time".to_string()],
    enabled: true,
}
```

### 安全测试场景
```rust
ToolConfig {
    selection_strategy: ToolSelectionStrategy::Manual(vec![
        "port_scan".to_string(),
        "http_request".to_string(),
        "subdomain_scan".to_string(),
    ]),
    max_tools: 3,
    enabled: true,
}
```

### 数据分析场景
```rust
ToolConfig {
    selection_strategy: ToolSelectionStrategy::Keyword,
    max_tools: 8,
    fixed_tools: vec![
        "rag_query".to_string(),
        "web_search".to_string(),
    ],
    enabled: true,
}
```
