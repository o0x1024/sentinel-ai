# 第十人原则使用示例

## 概述

第十人原则现在支持完整的对话历史审查，而不仅仅是单条消息。这使得第十人能够：
- 发现跨多轮对话的逻辑漏洞
- 审查完整的工具调用链路
- 识别早期决策中的错误假设
- 评估整体方案的一致性

## LLM 工具调用示例

### 1. 默认模式：审查完整历史

```json
{
  "name": "tenth_man_review",
  "arguments": {
    "execution_id": "current_execution_id",
    "review_type": "full"
  }
}
```

**说明**：
- 使用默认的 `FullHistory` 模式
- 自动使用滑动窗口的智能摘要
- 包含：全局摘要 + 段落摘要 + 最近消息

**输出示例**：
```json
{
  "success": true,
  "critique": "**[Tenth Man Intervention]**\n**1. Critical Flaw**: 你在第3步假设了API总是返回JSON格式，但没有处理非JSON响应的情况...\n**2. Hidden Risk**: 如果目标服务器实施了速率限制，你的重试逻辑可能会导致IP被封禁...\n**3. Counter-Argument**: 你的错误处理只捕获了网络错误，但忽略了业务逻辑错误...",
  "risk_level": "high",
  "message": "Review completed - Risk level: high"
}
```

### 2. 快速审查模式

```json
{
  "name": "tenth_man_review",
  "arguments": {
    "execution_id": "current_execution_id",
    "review_type": "quick",
    "focus_area": "SQL injection risks"
  }
}
```

**说明**：
- 使用 `quick` 模式进行快速风险识别
- 指定关注领域：SQL注入风险
- 返回1-2句话的简短警告

**输出示例**：
```json
{
  "success": true,
  "critique": "警告：你在构建SQL查询时直接拼接了用户输入，存在严重的SQL注入风险。建议使用参数化查询。",
  "risk_level": "critical",
  "message": "Review completed - Risk level: critical"
}
```

### 3. 审查最近N条消息

```json
{
  "name": "tenth_man_review",
  "arguments": {
    "execution_id": "current_execution_id",
    "review_mode": {
      "mode": "recent_messages",
      "count": 10
    },
    "review_type": "full",
    "focus_area": "command execution safety"
  }
}
```

**说明**：
- 只审查最近10条消息
- 适用于长对话中的局部审查
- 性能更好，Token消耗更少

### 4. 审查特定内容（向后兼容）

```json
{
  "name": "tenth_man_review",
  "arguments": {
    "execution_id": "current_execution_id",
    "review_mode": {
      "mode": "specific_content",
      "content": "I plan to execute: rm -rf /tmp/* to clean up temporary files"
    },
    "review_type": "full",
    "focus_area": "command safety"
  }
}
```

**说明**：
- 审查指定的内容片段
- 向后兼容旧的使用方式
- 适用于审查特定计划或结论

## System 模式（自动触发）

### 配置

在 `AgentExecuteParams` 中配置：

```rust
AgentExecuteParams {
    enable_tenth_man_rule: true,
    tenth_man_config: Some(TenthManConfig {
        mode: InterventionMode::Hybrid {
            tool_available: true,        // LLM可以主动调用
            force_final_review: true,    // 系统强制最终审查
        },
        auto_inject_to_context: false,
        require_user_confirmation: false,
    }),
    // ... other params
}
```

### 触发时机

System 模式会在以下时机自动触发：

1. **最终响应前**（`force_final_review: true`）
   - 在返回最终结果给用户之前
   - 自动审查完整的对话历史
   - 使用 `FullHistory` 模式

2. **关键操作前**（可选，未来扩展）
   - 执行危险命令前
   - 修改关键配置前
   - 访问敏感数据前

### 审查结果处理

审查结果会：
1. 保存到数据库（role=system, kind=tenth_man_critique）
2. 发送事件到前端（`agent:tenth_man_critique`）
3. 显示在对话界面中

## 实际使用场景

### 场景1：安全审计

```typescript
// LLM 在执行敏感操作前主动调用
{
  "name": "tenth_man_review",
  "arguments": {
    "execution_id": "audit-001",
    "review_type": "full",
    "focus_area": "security implications of the planned operations"
  }
}
```

**第十人可能发现**：
- 权限提升风险
- 数据泄露可能
- 未授权访问路径
- 日志记录缺失

### 场景2：代码审查

```typescript
{
  "name": "tenth_man_review",
  "arguments": {
    "execution_id": "code-review-042",
    "review_mode": {
      "mode": "recent_messages",
      "count": 15
    },
    "review_type": "full",
    "focus_area": "code quality and potential bugs"
  }
}
```

**第十人可能发现**：
- 边界条件未处理
- 资源泄露风险
- 并发问题
- 性能瓶颈

### 场景3：渗透测试

```typescript
{
  "name": "tenth_man_review",
  "arguments": {
    "execution_id": "pentest-123",
    "review_type": "full",
    "focus_area": "attack surface and exploitation strategy"
  }
}
```

**第十人可能发现**：
- 遗漏的攻击向量
- 误报的漏洞
- 检测规避不足
- 法律合规问题

### 场景4：数据分析

```typescript
{
  "name": "tenth_man_review",
  "arguments": {
    "execution_id": "analysis-789",
    "review_type": "quick",
    "focus_area": "statistical validity and data interpretation"
  }
}
```

**第十人可能发现**：
- 样本偏差
- 因果关系误判
- 统计显著性问题
- 数据清洗不当

## 最佳实践

### 1. 何时使用完整历史审查

✅ **推荐使用场景**：
- 最终决策前
- 复杂多步骤操作
- 涉及多个工具调用的链路
- 需要评估整体一致性

❌ **不推荐场景**：
- 简单的单步操作
- 对话刚开始（历史很少）
- 频繁的中间检查（使用 quick 模式）

### 2. 何时使用最近消息审查

✅ **推荐使用场景**：
- 长对话中的局部审查
- 性能敏感的场景
- 只关注最近的操作
- Token 预算有限

❌ **不推荐场景**：
- 需要全局视角
- 问题可能源于早期决策
- 需要检查整体逻辑

### 3. 何时使用快速审查

✅ **推荐使用场景**：
- 频繁的安全检查
- 实时风险监控
- 简单的是/否判断
- 高频调用场景

❌ **不推荐场景**：
- 需要详细分析
- 复杂的决策评估
- 最终审查

### 4. Focus Area 的使用

**好的 focus_area 示例**：
- "SQL injection vulnerabilities"
- "command execution safety"
- "authentication and authorization logic"
- "data validation and sanitization"
- "error handling completeness"

**不好的 focus_area 示例**：
- "everything" (太宽泛)
- "bugs" (不够具体)
- "check it" (无意义)

## 性能考虑

### Token 消耗

| 模式 | 预估 Token 消耗 | 适用场景 |
|------|----------------|----------|
| FullHistory | 5K-20K | 完整审查 |
| RecentMessages(10) | 2K-5K | 局部审查 |
| RecentMessages(5) | 1K-3K | 快速检查 |
| SpecificContent | 500-2K | 单点审查 |

### 响应时间

| 审查类型 | 预估时间 | 说明 |
|---------|---------|------|
| quick | 2-5秒 | 快速风险识别 |
| full | 5-15秒 | 完整分析 |

### 缓存优化

系统会自动缓存：
- 已构建的历史上下文（60秒TTL）
- 滑动窗口的摘要
- 数据库查询结果

## 前端集成

### 监听审查事件

```typescript
import { listen } from '@tauri-apps/api/event'

// 监听第十人审查结果
listen('agent:tenth_man_critique', (event) => {
  const { execution_id, critique, risk_level, trigger, mode } = event.payload
  
  console.log(`Tenth Man Review (${trigger}, ${mode}):`)
  console.log(`Risk Level: ${risk_level}`)
  console.log(`Critique: ${critique}`)
  
  // 根据风险等级显示不同的UI
  if (risk_level === 'critical' || risk_level === 'high') {
    showWarningDialog(critique)
  } else {
    showInfoNotification(critique)
  }
})
```

### 显示审查历史

```typescript
// 获取对话中的所有第十人审查
const messages = await getConversationMessages(conversationId)
const tenthManReviews = messages.filter(msg => 
  msg.role === 'system' && 
  msg.metadata?.kind === 'tenth_man_critique'
)

// 按风险等级分类
const criticalReviews = tenthManReviews.filter(r => 
  r.metadata?.risk_level === 'critical'
)
```

## 故障排查

### 问题1：审查返回空结果

**可能原因**：
- AppHandle 未初始化
- 数据库连接失败
- 对话历史为空

**解决方法**：
```rust
// 检查日志
tracing::info!("Tenth Man review completed - history_length: {}", history_length);
```

### 问题2：Token 超限

**可能原因**：
- 对话历史过长
- 未启用滑动窗口压缩

**解决方法**：
- 使用 `RecentMessages` 模式限制历史长度
- 启用滑动窗口自动压缩
- 调整 `max_context_tokens` 配置

### 问题3：审查速度慢

**可能原因**：
- 使用 `full` 审查类型
- 历史过长

**解决方法**：
- 使用 `quick` 模式进行快速检查
- 减少审查的消息数量
- 启用缓存机制

## 总结

第十人原则的历史输入功能提供了：

✅ **完整的上下文感知**：能看到完整的对话历史和工具调用链  
✅ **智能摘要**：自动压缩长历史，避免 Token 浪费  
✅ **灵活的模式**：支持完整历史、最近消息、特定内容三种模式  
✅ **性能优化**：缓存机制、智能截断、增量更新  
✅ **向后兼容**：保持旧 API 兼容性  

通过合理使用这些功能，可以显著提升 AI Agent 的安全性和可靠性。
