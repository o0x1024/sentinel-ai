# Tenth Man 混合模式使用说明

## 概述

Tenth Man（第十人法则）现已支持**混合模式**，结合了 LLM 主动调用和系统强制审查的优点，形成双重保障机制。

> *"如果我们九个人基于相同的信息得出了完全相同的结论，那么第十个人的职责就是提出异议。无论看起来多么不可能。"* —— 《僵尸世界大战》

## 三种工作模式

### 1. Tool-Only 模式（工具专用）
- **特点**：Tenth Man 仅作为工具供 LLM 调用
- **优点**：LLM 完全自主决定何时需要审查，灵活性最高
- **缺点**：依赖 LLM 判断，可能在关键时刻忘记调用
- **适用场景**：高度信任 LLM 判断能力，或不希望增加额外开销

```rust
TenthManConfig {
    mode: InterventionMode::ToolOnly,
    auto_inject_to_context: false,
    require_user_confirmation: false,
}
```

### 2. System-Only 模式（系统专用）
- **特点**：系统在最终结果时自动强制审查
- **优点**：保证每次执行都有审查，不依赖 LLM
- **缺点**：LLM 无法主动请求审查，缺乏灵活性
- **适用场景**：关键任务，必须保证最终审查

```rust
TenthManConfig {
    mode: InterventionMode::SystemOnly,
    auto_inject_to_context: false,
    require_user_confirmation: false,
}
```

### 3. Hybrid 模式（混合模式，推荐）⭐
- **特点**：LLM 可随时调用 + 系统强制最终审查
- **优点**：兼具灵活性和保障性，最佳实践
- **缺点**：可能产生额外 LLM 调用成本
- **适用场景**：大多数生产环境

```rust
TenthManConfig {
    mode: InterventionMode::Hybrid {
        tool_available: true,          // LLM 可以调用工具
        force_final_review: true,      // 系统强制最终审查
        last_tool_call_time: None,     // (内部使用)
    },
    auto_inject_to_context: false,
    require_user_confirmation: false,
}
```

## 使用方法

### 1. 后端配置（Rust）

在创建 AgentExecuteParams 时启用 Tenth Man：

```rust
let params = AgentExecuteParams {
    execution_id: "exec-123".to_string(),
    model: "deepseek-reasoner".to_string(),
    // ...其他参数
    enable_tenth_man_rule: true,
    tenth_man_config: Some(TenthManConfig {
        mode: InterventionMode::Hybrid {
            tool_available: true,
            force_final_review: true,
            last_tool_call_time: None,
        },
        auto_inject_to_context: false,
        require_user_confirmation: false,
    }),
};
```

### 2. LLM 工具调用（自动）

当配置为 `ToolOnly` 或 `Hybrid` 模式时，LLM 可以主动调用 `tenth_man_review` 工具：

**工具定义：**
```json
{
  "name": "tenth_man_review",
  "description": "Request an adversarial review of your current plan or conclusion. The Tenth Man will challenge your assumptions, identify hidden risks, and find potential flaws. Use 'quick' review for rapid risk checks, or 'full' review for comprehensive analysis.",
  "parameters": {
    "execution_id": "当前执行 ID",
    "content_to_review": "需要审查的内容（计划、结论等）",
    "context_description": "可选：上下文描述",
    "review_type": "quick（快速）或 full（全面）"
  }
}
```

**LLM 调用示例：**
```json
{
  "tool": "tenth_man_review",
  "arguments": {
    "execution_id": "exec-123",
    "content_to_review": "我计划使用 sqlmap 对目标进行 SQL 注入测试...",
    "context_description": "SQL 注入测试计划",
    "review_type": "full"
  }
}
```

### 3. 系统强制审查（自动）

在 `SystemOnly` 或 `Hybrid` 模式下，系统会在 Agent 执行完成时自动进行最终审查。无需手动触发。

## 工作流程

### Hybrid 模式完整流程

```
1. Agent 开始执行任务
   ↓
2. [可选] LLM 在感觉不确定时主动调用 tenth_man_review 工具
   - 快速审查：检查当前步骤是否有严重风险
   - 全面审查：深入分析计划的漏洞和假设
   ↓
3. Agent 继续执行或调整计划
   ↓
4. Agent 完成任务并生成最终结果
   ↓
5. [强制] 系统自动触发 Tenth Man 最终审查
   - 对整个执行结果进行全面审查
   - 识别潜在的风险和遗漏
   ↓
6. 审查结果作为系统消息插入对话历史
   ↓
7. 前端显示审查结果（带风险等级标识）
```

## 审查输出

### 风险等级

- `none` - 无风险：未发现显著问题
- `low` - 低风险：有轻微改进空间
- `medium` - 中风险：存在潜在问题
- `high` - 高风险：存在重大缺陷
- `critical` - 严重风险：可能导致严重后果

### 输出格式

```json
{
  "success": true,
  "critique": "**[Tenth Man Intervention]**\n**1. Critical Flaw**: ...\n**2. Hidden Risk**: ...\n**3. Counter-Argument**: ...",
  "risk_level": "medium",
  "message": "Review completed - Risk level: medium"
}
```

## 前端集成

### 工具列表

Tenth Man 工具已注册到工具系统，可以通过以下 Tauri 命令获取：

```typescript
// 获取所有内置工具（包括 Tenth Man）
const tools = await invoke('get_builtin_tools_with_status');

// 查找 Tenth Man 工具
const tenthManTool = tools.find(t => t.id === 'tenth_man_review');
console.log(tenthManTool);
// {
//   id: "tenth_man_review",
//   name: "tenth_man_review",
//   description: "Request an adversarial review...",
//   category: "ai",
//   version: "1.0.0",
//   enabled: true,
//   input_schema: { ... }
// }
```

### 工具元数据

通过工具路由器获取元数据：

```typescript
// 获取所有工具元数据
const allTools = await invoke('get_all_tool_metadata');

// 按分类获取工具
const aiTools = await invoke('get_tools_by_category', { category: 'ai' });

// 搜索工具
const searchResults = await invoke('search_tools', { query: 'tenth man' });

// 获取特定工具元数据
const metadata = await invoke('get_tool_metadata', { tool_id: 'tenth_man_review' });
```

### 手动执行工具

前端可以直接调用 Tenth Man 工具（用于测试或手动触发）：

```typescript
import { invoke } from '@tauri-apps/api/tauri';

// 执行 Tenth Man 审查
const result = await invoke('unified_execute_tool', {
  tool_name: 'tenth_man_review',
  inputs: {
    execution_id: 'current-execution-id',
    content_to_review: '我计划使用 sqlmap 进行 SQL 注入测试...',
    context_description: 'SQL 注入测试计划',
    review_type: 'full'
  }
});

console.log(result);
// {
//   success: true,
//   output: {
//     success: true,
//     critique: "**[Tenth Man Intervention]**\n...",
//     risk_level: "medium",
//     message: "Review completed - Risk level: medium"
//   },
//   error: null,
//   execution_time_ms: 1234
// }
```

### 前端事件

#### agent:tenth_man_critique
系统强制审查完成时触发：
```typescript
{
  execution_id: string,
  critique: string,
  message_id: string,
  trigger: "final_review",
  mode: "system_enforced"
}
```

#### agent:tenth_man_warning
LLM 主动调用审查时触发（工具执行结果）：
```typescript
{
  execution_id: string,
  tool_call_id: string,
  result: string  // JSON 格式的 TenthManToolOutput
}
```

#### agent:tool_result
工具执行结果事件（包括 Tenth Man）：
```typescript
{
  execution_id: string,
  tool_call_id: string,
  result: string  // 包含 critique、risk_level 等信息
}
```

### 工具启用/禁用

前端可以控制工具的启用状态：

```typescript
// 禁用 Tenth Man 工具
await invoke('toggle_builtin_tool', {
  tool_name: 'tenth_man_review',
  enabled: false
});

// 启用 Tenth Man 工具
await invoke('toggle_builtin_tool', {
  tool_name: 'tenth_man_review',
  enabled: true
});
```

## i18n 支持

已添加以下翻译键：

**中文 (zh):**
- `tenthManCritique`: '第十人审查'
- `tenthManReview`: '第十人评审'
- `tenthManWarning`: '第十人警告'
- `riskLevelNone`: '无风险'
- `riskLevelLow`: '低风险'
- `riskLevelMedium`: '中风险'
- `riskLevelHigh`: '高风险'
- `riskLevelCritical`: '严重风险'
- `tenthManToolCalled`: 'Agent 主动请求第十人审查'
- `tenthManSystemEnforced`: '系统强制第十人审查'

**英文 (en):**
- `tenthManCritique`: 'Tenth Man Critique'
- `tenthManReview`: 'Tenth Man Review'
- `riskLevelNone`: 'No Risk'
- ...等

## 最佳实践

### 1. 选择合适的模式
- **开发/测试**：使用 `ToolOnly` 模式观察 LLM 行为
- **生产环境**：使用 `Hybrid` 模式确保双重保障
- **关键任务**：使用 `SystemOnly` 模式保证强制审查

### 2. 合理设置审查类型
- **quick 审查**：用于频繁的轻量级风险检查
- **full 审查**：用于最终决策前的全面分析

### 3. 监控审查结果
- 在前端显示审查结果和风险等级
- 对 `high` 和 `critical` 级别的警告特别关注
- 将审查结果作为改进计划的参考

### 4. 成本优化
如果 LLM 频繁调用工具导致成本过高，可以：
- 切换到 `SystemOnly` 模式
- 在 System Prompt 中引导 LLM 少调用工具
- 调整 `force_final_review` 为 false（仅在已调用过工具时）

## 技术架构

```
┌─────────────────────────────────────────────────────────┐
│                     Agent Executor                       │
│  - 初始化 Tenth Man 配置和上下文                         │
│  - 根据模式决定是否注册工具                              │
│  - 执行最终审查（SystemOnly/Hybrid）                     │
└────────────────────┬────────────────────────────────────┘
                     │
        ┌────────────┴────────────┐
        │                         │
        ▼                         ▼
┌──────────────┐          ┌──────────────────┐
│  Tool Server │          │ Tenth Man Review │
│              │          │   (final check)  │
│ - 注册工具   │          │                  │
│ - 执行调用   │          │ - 全面审查       │
└──────┬───────┘          └──────────────────┘
       │
       ▼
┌──────────────────────┐
│ Tenth Man Executor   │
│                      │
│ - 存储 LLM 配置      │
│ - 执行审查逻辑       │
│ - 评估风险等级       │
└──────────────────────┘
```

## 常见问题

### Q: Hybrid 模式会增加多少成本？
A: 取决于 LLM 调用频率。通常情况下，系统强制审查只执行一次，LLM 主动调用次数取决于任务复杂度（通常 0-3 次）。

### Q: 如何知道审查是 LLM 主动调用还是系统强制的？
A: 查看事件的 `trigger` 字段：
- `final_review` + `mode: system_enforced` = 系统强制
- 作为工具调用结果 = LLM 主动

### Q: 可以禁用最终强制审查吗？
A: 可以，在 Hybrid 模式中设置 `force_final_review: false`。但不推荐，除非 LLM 已经频繁调用审查工具。

### Q: 审查使用的是什么模型？
A: 使用与主 Agent 相同的模型和配置（model、api_key、api_base 等）。

## 更新日志

### v1.0.0 (2026-01-12)
- ✅ 实现 Tenth Man 混合模式
- ✅ 添加 `tenth_man_review` 工具供 LLM 调用
- ✅ 支持三种工作模式：ToolOnly、SystemOnly、Hybrid
- ✅ 添加风险等级评估
- ✅ 添加前端事件和 i18n 支持
- ✅ 完善文档和使用说明

## 参考

- 源码：`src-tauri/src/agents/tenth_man.rs`
- 执行器：`src-tauri/src/agents/tenth_man_executor.rs`
- 工具定义：`src-tauri/sentinel-tools/src/buildin_tools/tenth_man_tool.rs`
- 工具注册：`src-tauri/sentinel-tools/src/tool_server.rs`
