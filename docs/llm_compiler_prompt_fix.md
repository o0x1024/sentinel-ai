# LLMCompiler Prompt累积问题修复

## 问题描述

在LLMCompiler架构下，从第二次Joiner调用开始，User input消息会包含之前的完整Joiner prompt模板，导致消息越来越长，prompt重复累积。

### 问题表现

**日志症状**:
```
[第一次Joiner调用]
system prompt: You are a helpful AI assistant.
User input: 你是一个善于分析和总结的AI助手... (完整的Joiner prompt模板)

[第二次Joiner调用]  
system prompt: You are a helpful AI assistant.
User input: 
  你是一个善于分析和总结的AI助手... (第一次的Joiner prompt)
  [THINKING]... [DECISION]... (第一次的响应)
  你是一个善于分析和总结的AI助手... (第二次的Joiner prompt - 重复!)
```

### 影响

1. **Token浪费**: 每次调用都携带完整的历史prompt，导致token数量激增
2. **性能下降**: 消息越来越长，处理时间增加
3. **成本增加**: 更多的token消耗意味着更高的API成本
4. **可能的语义混淆**: LLM看到重复的prompt可能会困惑

## 根本原因

### 代码位置
`src-tauri/src/services/ai.rs` 第1500-1505行（修复前）

### 问题代码
```rust
let mut history_buf = String::new();
let take = messages.len().min(12);
for m in messages.iter().rev().take(take).rev() {
    history_buf.push_str(&format!("{}\n", m.content));
}
let user_input = history_buf.trim();
```

### 问题原因

1. **消息累积**: 即使Joiner调用时设置了`user_prompt_to_save=None`，代码仍然会把`user_prompt_for_llm`添加到messages列表中（第1438-1451行）

2. **历史合并**: 上述代码会把messages列表中的所有消息合并到user_input中，包括：
   - 第一次Joiner的user消息（包含完整Joiner prompt模板）
   - 第一次Joiner的assistant响应
   - 第二次Joiner的user消息（又是完整Joiner prompt模板）

3. **重复累积**: 每次Joiner调用都会添加新的消息到messages，然后所有历史都会被合并，导致prompt不断重复累积

## 解决方案

### 修复代码

```rust
// ✅ 对于LLMCompiler等架构，只使用最后一条user消息
// 不合并历史消息，避免prompt重复累积
let user_input = messages.last()
    .map(|m| m.content.as_str())
    .unwrap_or("");
```

### 修复原理

1. **只取最后一条**: 只使用messages列表中的最后一条消息作为user_input
2. **避免累积**: 不再合并历史消息，每次调用都是独立的
3. **适用场景**: 对于LLMCompiler这种内部多次调用的架构，每次调用都应该是独立的，不需要之前的内部调用历史

### 兼容性考虑

这个修改主要影响LLMCompiler架构：
- ✅ **Planner**: 第一次调用，只有当前的规划请求
- ✅ **Joiner**: 每次决策都是独立的，不需要之前Joiner调用的历史
- ✅ **普通对话**: 对于用户的正常对话，每次也只关注最新的用户输入

### 潜在影响

**如果有些场景需要完整的对话历史**（例如：多轮对话需要理解上下文），可以考虑：

1. **方案A**: 根据`chunk_type`区分处理
   ```rust
   let user_input = if chunk_type == Some(ChunkType::Thinking) {
       // 内部架构调用，只用最后一条
       messages.last().map(|m| m.content.as_str()).unwrap_or("")
   } else {
       // 正常对话，合并最近N条历史
       // ... 合并逻辑
   };
   ```

2. **方案B**: 在conversation_id为None时（临时调用）只用最后一条

3. **方案C**: 添加配置参数控制是否需要历史上下文

**当前修复采用最简单方案**，因为：
- LLMCompiler架构的内部调用不需要累积历史
- 用户的正常对话也主要关注最新输入
- 如果需要历史上下文，应该在更高层（如前端）显式管理

## 数据结构兼容性修复

除了上述问题，还修复了另一个关键问题：

### Prompt格式不匹配

**问题**: Planner的拟人化prompt使用`tasks`字段，但代码期望`nodes`字段

**修复**: 在`planner.rs`中同时支持两种格式

```rust
// ✅ 同时支持 "nodes" 和 "tasks" 字段（拟人化prompt使用tasks）
let nodes = parsed["nodes"]
    .as_array()
    .or_else(|| parsed["tasks"].as_array())
    .ok_or_else(|| anyhow::anyhow!("缺少nodes或tasks字段"))?

// ✅ 同时支持 "tool_name" 和 "tool" 字段（拟人化prompt使用tool）
tool_name: node["tool_name"]
    .as_str()
    .or_else(|| node["tool"].as_str())
    .ok_or_else(|| anyhow::anyhow!("任务缺少tool_name或tool字段"))?
```

## 测试验证

### 测试步骤

1. 编译项目
   ```bash
   cd src-tauri && cargo check
   ```

2. 运行应用，使用LLMCompiler引擎执行任务

3. 查看日志文件
   ```bash
   tail -f src-tauri/logs/llm-http-requests-*.log
   ```

### 验证要点

- ✅ 第二次及后续Joiner调用的User input应该只包含当前的分析请求
- ✅ 不应该出现重复的Joiner prompt模板
- ✅ system prompt应该正确显示拟人化的Joiner prompt（如果从数据库读取）
- ✅ Token数量应该保持在合理范围，不会随调用次数增长

## 总结

### 修复的文件

1. ✅ `src-tauri/src/services/ai.rs` - 修复消息累积问题
2. ✅ `src-tauri/src/engines/llm_compiler/planner.rs` - 支持tasks/tool字段
3. ✅ `src-tauri/src/engines/llm_compiler/joiner.rs` - 添加debug导入

### 效果

- **Token减少**: 每次Joiner调用减少数千tokens
- **性能提升**: 处理速度更快
- **成本降低**: API调用成本显著减少
- **语义清晰**: LLM不再看到重复的prompt

### 建议

如果发现某些场景确实需要对话历史，可以：
1. 在前端层面管理对话上下文
2. 通过system_prompt注入必要的上下文信息
3. 或者在特定场景下恢复历史合并逻辑（通过配置控制）

## 相关文件

- [拟人化Prompt SQL](./llm_compiler_humanized_prompts.sql)
- [拟人化Prompt指南](./llm_compiler_humanized_guide.md)
- [快速开始](./llm_compiler_humanized_quickstart.md)

