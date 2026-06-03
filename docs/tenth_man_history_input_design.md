# 第十人原则历史消息输入方案设计

## 问题分析

当前第十人原则的输入存在以下问题：

1. **Tool模式**：LLM主动调用时，只能传入 `content_to_review`（单个内容片段）
2. **System模式**：系统自动触发时，只使用最后一条助手消息 `last_assistant_message`
3. **缺少完整上下文**：无法看到完整的对话历史、工具调用序列、推理过程

这导致第十人无法：
- 识别跨多轮对话的逻辑漏洞
- 审查工具调用的完整链路
- 发现早期决策中的错误假设
- 评估整体方案的一致性

## 核心设计目标

1. **完整历史访问**：第十人应该能看到完整的对话历史
2. **智能摘要**：对于长历史，使用滑动窗口的摘要机制
3. **灵活性**：支持全量历史、最近N条、或指定范围
4. **性能优化**：避免每次都传输完整历史（使用缓存和增量更新）

## 方案设计

### 方案1：基于 SlidingWindow 的历史访问（推荐）

**核心思路**：复用现有的 `SlidingWindowManager`，它已经实现了历史管理和摘要

#### 架构设计

```
┌─────────────────────────────────────────────────────────────┐
│                    Tenth Man 输入层                          │
├─────────────────────────────────────────────────────────────┤
│                                                               │
│  1. Global Summary (全局摘要)                                │
│     - 早期对话的高层次总结                                    │
│     - Token 占用：10%                                         │
│                                                               │
│  2. Segment Summaries (段落摘要)                             │
│     - 中间对话的分段总结                                      │
│     - Token 占用：30%                                         │
│                                                               │
│  3. Recent Messages (最近消息)                               │
│     - 最近 20 条完整消息                                      │
│     - Token 占用：60%                                         │
│     - 包含：用户消息、助手消息、工具调用、工具结果            │
│                                                               │
└─────────────────────────────────────────────────────────────┘
```

#### 实现步骤

##### 1. 扩展 `TenthManToolArgs`

```rust
// sentinel-tools/src/buildin_tools/tenth_man_tool.rs

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct TenthManToolArgs {
    pub execution_id: String,
    
    /// 审查模式
    #[serde(default = "default_review_mode")]
    pub review_mode: ReviewMode,
    
    /// 审查类型：quick 或 full
    #[serde(default = "default_review_type")]
    pub review_type: String,
    
    /// 可选：特定关注点
    pub focus_area: Option<String>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReviewMode {
    /// 审查完整历史（使用滑动窗口摘要）
    FullHistory,
    /// 审查最近N条消息
    RecentMessages { count: usize },
    /// 审查指定内容（向后兼容）
    SpecificContent { content: String },
}

fn default_review_mode() -> ReviewMode {
    ReviewMode::FullHistory
}
```

##### 2. 扩展 `TenthManExecutor` 访问历史

```rust
// src-tauri/src/agents/tenth_man_executor.rs

use crate::agents::sliding_window::SlidingWindowManager;
use tauri::Manager;

/// 全局 AppHandle 存储（用于访问数据库）
static APP_HANDLE: once_cell::sync::OnceCell<tauri::AppHandle> = 
    once_cell::sync::OnceCell::new();

pub fn set_app_handle(handle: tauri::AppHandle) {
    let _ = APP_HANDLE.set(handle);
}

/// 构建历史上下文
async fn build_history_context(
    execution_id: &str,
    review_mode: &ReviewMode,
) -> Result<String, TenthManToolError> {
    let app_handle = APP_HANDLE.get()
        .ok_or_else(|| TenthManToolError::InternalError("AppHandle not initialized".to_string()))?;
    
    match review_mode {
        ReviewMode::FullHistory => {
            // 使用 SlidingWindow 获取完整上下文
            let sw = SlidingWindowManager::new(app_handle, execution_id, None)
                .await
                .map_err(|e| TenthManToolError::InternalError(e.to_string()))?;
            
            // 构建上下文（包含全局摘要、段落摘要、最近消息）
            let context_messages = sw.build_context("");
            
            // 格式化为文本
            let mut history = String::new();
            
            // 提取全局摘要
            if let Some(first) = context_messages.first() {
                if first.role == "system" {
                    history.push_str("=== 全局上下文摘要 ===\n");
                    history.push_str(&first.content);
                    history.push_str("\n\n");
                }
            }
            
            // 格式化对话历史
            history.push_str("=== 对话历史 ===\n");
            for (idx, msg) in context_messages.iter().enumerate().skip(1) {
                history.push_str(&format!("\n[{}] {}:\n", idx, msg.role.to_uppercase()));
                history.push_str(&msg.content);
                
                if let Some(ref tool_calls) = msg.tool_calls {
                    history.push_str(&format!("\n[工具调用]: {}", tool_calls));
                }
                
                if let Some(ref reasoning) = msg.reasoning_content {
                    history.push_str(&format!("\n[推理过程]: {}", reasoning));
                }
                
                history.push_str("\n");
            }
            
            Ok(history)
        }
        
        ReviewMode::RecentMessages { count } => {
            // 获取最近N条消息
            let db = app_handle.state::<Arc<sentinel_db::DatabaseService>>();
            let messages = db.get_ai_messages_by_conversation(execution_id)
                .await
                .map_err(|e| TenthManToolError::InternalError(e.to_string()))?;
            
            let recent = messages.iter()
                .rev()
                .take(*count)
                .rev()
                .collect::<Vec<_>>();
            
            let mut history = String::new();
            history.push_str(&format!("=== 最近 {} 条消息 ===\n", count));
            
            for (idx, msg) in recent.iter().enumerate() {
                history.push_str(&format!("\n[{}] {}:\n", idx + 1, msg.role.to_uppercase()));
                history.push_str(&msg.content);
                
                if let Some(ref tool_calls) = msg.tool_calls {
                    history.push_str(&format!("\n[工具调用]: {}", tool_calls));
                }
                
                history.push_str("\n");
            }
            
            Ok(history)
        }
        
        ReviewMode::SpecificContent { content } => {
            // 向后兼容：直接返回指定内容
            Ok(content.clone())
        }
    }
}

/// 执行第十人审查（更新版）
pub async fn execute_tenth_man_review(
    args: TenthManToolArgs
) -> Result<TenthManToolOutput, TenthManToolError> {
    // 获取 LLM 配置
    let config = {
        let configs = TENTH_MAN_CONFIGS.read().await;
        configs.get(&args.execution_id).cloned()
    };
    
    let Some(config) = config else {
        return Err(TenthManToolError::ConfigNotFound(args.execution_id.clone()));
    };
    
    // 获取任务上下文
    let task_context = {
        let contexts = TASK_CONTEXTS.read().await;
        contexts.get(&args.execution_id)
            .cloned()
            .unwrap_or_else(|| "Unknown task".to_string())
    };
    
    // 构建历史上下文
    let history_context = build_history_context(&args.execution_id, &args.review_mode).await?;
    
    // 构建审查提示词
    let focus_area = args.focus_area
        .as_deref()
        .unwrap_or("整体方案和执行过程");
    
    let review_prompt = match args.review_type.as_str() {
        "quick" => {
            format!(
                "### 原始任务:\n{}\n\n### 关注领域:\n{}\n\n### 历史上下文:\n{}\n\n---\n\n快速风险评估:",
                task_context, focus_area, history_context
            )
        }
        "full" | _ => {
            format!(
                "### 原始任务:\n{}\n\n### 关注领域:\n{}\n\n### 完整历史上下文:\n{}\n\n---\n\n执行完整的第十人审查。挑战当前的结论和执行过程。",
                task_context, focus_area, history_context
            )
        }
    };
    
    let system_prompt = match args.review_type.as_str() {
        "quick" => TENTH_MAN_QUICK_REVIEW_PROMPT,
        "full" | _ => TENTH_MAN_FULL_REVIEW_PROMPT,
    };
    
    // 执行审查
    let client = LlmClient::new(config);
    let critique = client
        .completion(Some(system_prompt), &review_prompt)
        .await
        .map_err(|e| TenthManToolError::ReviewFailed(e.to_string()))?;
    
    // 评估风险等级
    let risk_level = assess_risk_level(&critique);
    
    let success = !critique.trim().is_empty();
    let message = if risk_level == "none" {
        "未发现重大风险".to_string()
    } else {
        format!("审查完成 - 风险等级: {}", risk_level)
    };
    
    tracing::info!(
        "Tenth Man review completed - execution_id: {}, risk_level: {}, history_length: {}",
        args.execution_id,
        risk_level,
        history_context.len()
    );
    
    Ok(TenthManToolOutput {
        success,
        critique: Some(critique),
        risk_level,
        message,
    })
}
```

##### 3. 更新工具描述

```rust
// sentinel-tools/src/buildin_tools/tenth_man_tool.rs

impl TenthManTool {
    pub const DESCRIPTION: &'static str = 
        "Request adversarial review of your work from the Tenth Man. \
        \
        The Tenth Man reviews your COMPLETE conversation history (not just current message) to find:\
        - Logic flaws across multiple steps\
        - Dangerous assumptions in your reasoning\
        - Overlooked risks and edge cases\
        - Inconsistencies in your approach\
        \
        Review modes:\
        - 'full_history' (default): Reviews entire conversation with smart summarization\
        - 'recent_messages': Reviews last N messages only\
        - 'specific_content': Reviews a specific piece of content\
        \
        Review types:\
        - 'quick': Fast risk identification (1-2 sentences)\
        - 'full': Comprehensive analysis with detailed critique\
        \
        Use this tool when:\
        - Before executing critical operations\
        - After making important decisions\
        - When you want to validate your approach\
        - To catch mistakes before they cause problems";
}
```

##### 4. 更新 System 模式的最终审查

```rust
// src-tauri/src/agents/executor.rs

// 在 execute_agent_with_tools 函数中的最终审查部分
if params.enable_tenth_man_rule {
    let tenth_man = TenthMan::new(&params);
    
    if should_run_final {
        tracing::info!("Running Tenth Man final review for execution_id: {}", params.execution_id);
        
        // 使用完整历史进行审查
        match tenth_man.review_with_history(&params.execution_id).await {
            Ok(critique) => {
                // 保存审查结果...
            }
            Err(e) => {
                tracing::warn!("Tenth Man Review failed: {}", e);
            }
        }
    }
}
```

##### 5. 更新 `TenthMan` 结构

```rust
// src-tauri/src/agents/tenth_man.rs

impl TenthMan {
    /// 使用完整历史进行审查（System 模式使用）
    pub async fn review_with_history(&self, execution_id: &str) -> Result<String> {
        use crate::agents::tenth_man_executor::{execute_tenth_man_review, ReviewMode};
        use sentinel_tools::buildin_tools::tenth_man_tool::TenthManToolArgs;
        
        let args = TenthManToolArgs {
            execution_id: execution_id.to_string(),
            review_mode: ReviewMode::FullHistory,
            review_type: "full".to_string(),
            focus_area: Some("最终方案和完整执行过程".to_string()),
        };
        
        let output = execute_tenth_man_review(args).await
            .map_err(|e| anyhow::anyhow!("Review failed: {}", e))?;
        
        output.critique.ok_or_else(|| anyhow::anyhow!("No critique generated"))
    }
}
```

### 方案2：容器历史文件访问（轻量级方案）

**核心思路**：复用现有的 `history.txt` 文件，第十人直接读取

#### 优点
- 实现简单，复用现有基础设施
- 不需要访问数据库
- 性能开销小

#### 缺点
- 历史文件可能很大（无摘要）
- 格式化不够结构化
- 无法灵活选择历史范围

#### 实现示例

```rust
async fn build_history_from_container(execution_id: &str) -> Result<String> {
    use sentinel_tools::shell::get_shell_config;
    
    let shell_config = get_shell_config().await;
    if let Some(docker_config) = shell_config.docker_config {
        let sandbox = sentinel_tools::DockerSandbox::new(docker_config);
        
        // 读取历史文件
        let history_path = format!(
            "{}/history.txt",
            sentinel_tools::output_storage::CONTAINER_CONTEXT_DIR
        );
        
        let history = sandbox.read_file(&history_path).await?;
        Ok(history)
    } else {
        Err(anyhow::anyhow!("Docker sandbox not configured"))
    }
}
```

### 方案3：混合方案（最佳实践）

结合方案1和方案2的优点：

1. **默认使用 SlidingWindow**（方案1）
   - 提供智能摘要
   - 结构化历史
   - 灵活的范围选择

2. **Fallback 到容器文件**（方案2）
   - 当数据库不可用时
   - 作为备份机制

3. **缓存机制**
   - 缓存已构建的历史上下文
   - 增量更新（只添加新消息）

## 工具调用示例

### LLM 主动调用示例

```typescript
// LLM 可以这样调用工具

// 1. 审查完整历史（默认）
tenth_man_review({
  execution_id: "current_execution_id",
  review_type: "full"
})

// 2. 审查最近10条消息
tenth_man_review({
  execution_id: "current_execution_id",
  review_mode: {
    recent_messages: { count: 10 }
  },
  review_type: "quick"
})

// 3. 审查特定内容（向后兼容）
tenth_man_review({
  execution_id: "current_execution_id",
  review_mode: {
    specific_content: {
      content: "我计划执行 rm -rf / 命令来清理系统"
    }
  },
  review_type: "full",
  focus_area: "命令安全性"
})
```

## Token 优化策略

### 1. 智能截断

```rust
fn truncate_history_if_needed(history: String, max_tokens: usize) -> String {
    let estimated_tokens = history.len() / 4; // 粗略估算
    
    if estimated_tokens <= max_tokens {
        return history;
    }
    
    // 保留：全局摘要 + 段落摘要 + 最近5条消息
    // 截断：中间的详细历史
}
```

### 2. 分层审查

```rust
pub enum ReviewDepth {
    Surface,    // 只看最近消息
    Medium,     // 最近消息 + 段落摘要
    Deep,       // 完整历史（全局摘要 + 段落摘要 + 最近消息）
}
```

### 3. 增量审查

```rust
// 只审查自上次审查以来的新消息
pub struct IncrementalReview {
    last_reviewed_message_index: usize,
    accumulated_risks: Vec<Risk>,
}
```

## 性能考虑

### 1. 缓存策略

```rust
// 缓存已构建的历史上下文
static HISTORY_CACHE: Lazy<Arc<RwLock<HashMap<String, (String, Instant)>>>> = 
    Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));

const CACHE_TTL: Duration = Duration::from_secs(60);
```

### 2. 异步加载

```rust
// 在后台预加载历史，不阻塞工具调用
async fn preload_history(execution_id: &str) {
    tokio::spawn(async move {
        let _ = build_history_context(execution_id, &ReviewMode::FullHistory).await;
    });
}
```

### 3. 分页加载

```rust
// 对于超长历史，分页加载
pub struct PaginatedHistory {
    page_size: usize,
    current_page: usize,
    total_pages: usize,
}
```

## 测试计划

### 1. 单元测试

```rust
#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_full_history_review() {
        // 测试完整历史审查
    }
    
    #[tokio::test]
    async fn test_recent_messages_review() {
        // 测试最近消息审查
    }
    
    #[tokio::test]
    async fn test_history_truncation() {
        // 测试历史截断
    }
}
```

### 2. 集成测试

```rust
#[tokio::test]
async fn test_tenth_man_with_long_conversation() {
    // 创建100条消息的对话
    // 触发第十人审查
    // 验证能正确处理长历史
}
```

### 3. 性能测试

```rust
#[tokio::test]
async fn test_review_performance() {
    // 测试不同历史长度下的性能
    // 1000条消息、10000条消息
}
```

## 迁移计划

### Phase 1: 基础实现（1-2天）
- [ ] 扩展 `TenthManToolArgs` 添加 `ReviewMode`
- [ ] 实现 `build_history_context` 函数
- [ ] 更新 `execute_tenth_man_review` 使用新输入

### Phase 2: 集成测试（1天）
- [ ] 编写单元测试
- [ ] 编写集成测试
- [ ] 性能测试和优化

### Phase 3: 优化和文档（1天）
- [ ] 实现缓存机制
- [ ] 实现智能截断
- [ ] 更新用户文档

### Phase 4: 向后兼容（0.5天）
- [ ] 保持旧 API 兼容
- [ ] 添加弃用警告
- [ ] 迁移指南

## 总结

**推荐方案**：方案1（基于 SlidingWindow）+ 缓存优化

**理由**：
1. ✅ 复用现有基础设施（SlidingWindow）
2. ✅ 智能摘要，避免 Token 浪费
3. ✅ 结构化历史，便于审查
4. ✅ 灵活的范围选择
5. ✅ 性能可控（通过缓存和截断）

**关键优势**：
- 第十人能看到完整的对话上下文
- 自动处理长历史（通过摘要）
- LLM 可以灵活选择审查范围
- System 模式自动使用完整历史

**实现复杂度**：中等
**性能影响**：低（通过缓存优化）
**用户体验**：显著提升
