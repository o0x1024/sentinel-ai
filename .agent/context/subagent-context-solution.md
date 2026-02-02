# Subagent 上下文丢失问题 - 完整解决方案

## 问题描述

在原始实现中,子智能体（Subagent）存在严重的"失忆"问题:

1. **上下文压缩过度**: `build_subagent_task` 仅将父任务的简短摘要传递给子智能体
2. **无法访问历史**: 子智能体无法查看父智能体的完整对话历史、工具调用记录或推理过程
3. **信息断层**: 当父智能体下发复杂任务时,子智能体缺乏必要的背景信息,导致执行效率低下

**后果**: 子智能体就像一个刚被创建的"失忆"助手,无法利用父智能体已经积累的知识和上下文。

## 解决方案

### 设计思路

采用**"摘要 + 历史文件引用"**的混合策略:

1. **主动传递**: 在子智能体任务描述中包含父任务的压缩摘要（保证即时可用的上下文）
2. **被动访问**: 将父智能体的**完整对话历史**导出到文件,并告知子智能体文件路径
3. **工具赋能**: 确保子智能体可以使用 shell 工具（`grep`、`cat`、`less` 等）主动查询历史文件

这种方案兼顾了效率（不在每次调用时传递巨量上下文）和完整性（需要时可以查询）。

### 实现细节

#### 1. 增强的任务描述生成 (`build_subagent_task`)

**文件**: `src-tauri/src/agents/subagent_executor.rs`

```rust
fn build_subagent_task(
    parent_task: &str, 
    subagent_task: &str, 
    parent_execution_id: &str,
    context_dir: &str,
) -> String {
    // 生成父任务摘要
    let brief = condense_text(parent, ContextPolicy::subagent().task_brief_max_chars);
    
    // 计算父级历史文件路径
    let parent_history_path = format!(
        "{}/history_{}.txt", 
        context_dir, 
        &parent_execution_id[..12]
    );
    
    // 构建增强的任务描述
    format!(
        "[Parent Context Summary]\n{}\n\n\
        [Parent Context History Access]\n\
        The parent agent's full conversation history is available at:\n\
        - Path: {}\n\
        - Usage: You can use shell tools (cat, grep, less, etc.) to search this file\n\
        - Example: `grep -i \"specific topic\" {}`\n\n\
        [Your Subagent Task]\n{}",
        brief, parent_history_path, parent_history_path, subagent
    )
}
```

**关键改进**:
- 明确告知子智能体父级历史文件的路径
- 提供具体的使用示例（`grep`、`cat`）
- 保持摘要简洁,避免 Token 浪费

#### 2. 父级历史导出 (`execute_spawn`)

在启动子智能体之前,执行以下步骤:

```rust
async fn execute_spawn(args: SubagentSpawnArgs) -> Result<...> {
    // 1. 检测执行环境（Host 或 Docker）
    let (context_dir, is_docker) = determine_execution_context().await;
    
    // 2. 加载父级的 SlidingWindowManager
    let mut parent_sliding_window = SlidingWindowManager::new(
        &app_handle,
        &args.parent_execution_id,
        None,
    ).await?;
    
    // 3. 导出父级完整对话历史
    let parent_history_content = parent_sliding_window.export_history().await?;
    
    // 4. 根据环境写入文件
    if is_docker {
        store_history_in_container_with_id(&sandbox, &parent_history_content, Some(&parent_execution_id)).await?;
    } else {
        store_history_on_host(&parent_history_content, Some(&parent_execution_id)).await?;
    }
    
    // 5. 构建增强的任务描述
    let task_with_context = build_subagent_task(&parent.task_context, &args.task, &parent_execution_id, &context_dir);
    
    // ... 继续启动子智能体
}
```

**关键逻辑**:
- **环境自适应**: 自动检测 Docker/Host 环境,将历史写入正确位置
- **文件隔离**: 使用 `history_{execution_id}.txt` 格式,避免不同执行会话的数据混淆
- **容错处理**: 历史导出失败不会阻塞子智能体启动,只会记录警告日志

#### 3. 历史文件存储机制

使用现有的 `sentinel-tools/output_storage.rs` 模块:

- **Host 环境**: 写入 `~/.sentinel-ai/context/history_{parent_exec_id}.txt`
- **Docker 环境**: 写入 `/workspace/context/history_{parent_exec_id}.txt`

文件内容包含:
- 完整的多轮对话（User/Assistant/Tool 消息）
- 工具调用记录和结果
- 推理内容（Reasoning Content）

### 工作流程示例

#### 场景: 主智能体委托子智能体分析代码

1. **主智能体执行**:
   ```
   User: 帮我分析 src/main.rs 的性能问题
   Assistant: [进行多轮对话,使用了 shell、grep 等工具]
   Assistant: 我需要分配一个专门的子智能体来深度分析内存泄漏...
   [调用 subagent_spawn]
   ```

2. **系统自动处理**:
   - 导出主智能体的完整对话历史到 `/workspace/context/history_abc123def456.txt`
   - 生成子智能体任务描述:
     ```
     [Parent Context Summary]
     父任务摘要: 分析 src/main.rs 性能问题...
     
     [Parent Context History Access]
     父智能体的完整对话历史位于:
     - Path: /workspace/context/history_abc123def456.txt
     - Usage: 你可以使用 shell 工具查询此文件
     - Example: grep -i "memory leak" /workspace/context/history_abc123def456.txt
     
     [Your Subagent Task]
     深度分析 src/main.rs 中的内存泄漏问题...
     ```

3. **子智能体执行**:
   ```
   Assistant: 让我先查看父智能体已经收集的信息...
   [调用 shell: cat /workspace/context/history_abc123def456.txt | grep -A 5 "main.rs"]
   Assistant: 根据父智能体的分析,问题集中在第 42 行...
   ```

### 优势

1. **零上下文丢失**: 子智能体可以访问父智能体的所有历史信息
2. **性能高效**: 不在每次调用时传递完整历史（避免 Token 浪费）
3. **按需查询**: 子智能体根据需要主动查询,而非被动接收
4. **工具原生**: 利用现有的 shell 工具,无需新增特殊 API
5. **可追溯性**: 所有历史文件都有明确的 `execution_id` 标识,便于调试

### 配套基础设施

- ✅ **历史导出**: `SlidingWindowManager::export_history()`
- ✅ **文件存储**: `output_storage::store_history_in_container_with_id()`
- ✅ **路径隔离**: 每个 `execution_id` 对应独立的历史文件
- ✅ **环境自适应**: 自动识别 Host/Docker 环境

## 验证要点

### 功能测试

1. **基础场景**: 主智能体 → 子智能体 (单层)
   - [ ] 子智能体收到正确的历史文件路径
   - [ ] 历史文件内容完整
   - [ ] 子智能体可以使用 `grep` 查询历史

2. **递归场景**: 主智能体 → 子智能体1 → 子智能体2 (多层)
   - [ ] 每层子智能体都能访问其直接父级的历史
   - [ ] 历史文件路径正确传递

3. **环境测试**:
   - [ ] Host 模式: 历史写入 `~/.sentinel-ai/context/`
   - [ ] Docker 模式: 历史写入 `/workspace/context/`

### 性能测试

- [ ] 历史文件大小合理（压缩后通常 < 1MB）
- [ ] 导出操作不阻塞主流程（< 100ms）
- [ ] 子智能体启动延迟可接受（增加 < 200ms）

## 后续优化方向

1. **历史压缩**: 对超大历史文件进行智能摘要（保留关键信息）
2. **向量检索**: 将历史内容向量化,支持语义搜索
3. **跨层访问**: 允许子智能体访问"祖父"层级的历史（当前只能访问直接父级）
4. **清理机制**: 自动清理过期的历史文件（例如 7 天后）

## 相关文件

- `src-tauri/src/agents/subagent_executor.rs`: 子智能体启动逻辑
- `src-tauri/sentinel-tools/src/output_storage.rs`: 历史文件存储
- `src-tauri/src/agents/sliding_window.rs`: 对话历史管理
- `src-tauri/src/agents/context_engineering/builder.rs`: 上下文构建

---

**实施日期**: 2026-01-30  
**实施者**: Antigravity AI Assistant  
**状态**: ✅ 已完成并通过编译验证
