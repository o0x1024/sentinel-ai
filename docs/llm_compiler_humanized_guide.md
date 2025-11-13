# LLMCompiler 拟人化Prompt使用指南

## 概述

本指南介绍如何使用LLMCompiler架构的拟人化Prompt系统,让AI助手的响应更加自然和人性化,而不是冰冷的任务执行机器人。

## 主要特性

### 1. **思考过程可见化**
- AI助手会展示它的思考过程,就像和朋友讨论问题一样
- 用户可以看到AI是如何分析问题、制定计划和做出决策的
- 使用流式输出实时展示思考过程

### 2. **拟人化的响应格式**
- **Planner阶段**: `[THINKING]` + `[PLAN]`
- **Joiner阶段**: `[THINKING]` + `[DECISION]`

### 3. **自然语言交互**
- AI使用口语化的表达方式
- 解释每个决策的原因
- 保持友好、专业的对话风格

## 架构组件

### Planner（规划器）

**职责**: 理解用户需求,分解任务,生成DAG执行计划

**响应格式**:
```
[THINKING]
好的,用户想要扫描example.com的安全情况。让我想想...

首先,我需要知道这个域名对应的IP地址,这样才能进行后续的扫描。然后,我应该扫描一下开放的端口,了解有哪些服务在运行。根据开放的端口,我可以进一步分析可能存在的漏洞。

[PLAN]
```json
{
  "tasks": [
    {
      "id": "task_1",
      "name": "DNS解析",
      "description": "获取example.com的IP地址",
      "tool": "dns_scanner",
      "inputs": {
        "domain": "example.com"
      },
      "dependencies": [],
      "reason": "需要知道目标IP才能进行后续扫描"
    }
  ],
  "execution_strategy": "先获取IP地址,然后扫描端口"
}
```
```

**关键点**:
- `[THINKING]` 部分用自然语言表达思考过程
- `[PLAN]` 部分包含结构化的JSON计划
- 每个任务都有明确的`reason`字段解释原因

### Joiner（决策器）

**职责**: 分析执行结果,决定是继续执行还是给出最终答案

**响应格式**:
```
[THINKING]
让我看看这一轮执行的结果...

DNS解析成功,得到了IP 192.168.1.1。端口扫描显示开放了80、443、22这三个端口。漏洞扫描的结果显示80端口的Apache版本过低,存在已知的CVE-2021-41773漏洞,风险等级为高危。

用户问的是"检查example.com的安全情况",现在我已经有了明确的发现:发现了一个高危漏洞。这应该足够回答用户的问题了。

[DECISION]
```json
{
  "decision": "COMPLETE",
  "response": "我完成了对example.com的安全扫描,发现了一些重要的安全问题:\n\n🔴 **高危漏洞**\n- Apache HTTP Server存在路径穿越漏洞...",
  "confidence": 0.95,
  "summary": {...}
}
```
```

**决策类型**:
- **COMPLETE**: 已有足够信息,给出最终答案
- **CONTINUE**: 需要更多信息,建议继续执行

## 数据库配置

### 安装Prompt到数据库

执行SQL文件安装拟人化prompt:

```bash
sqlite3 your-database.db < docs/llm_compiler_humanized_prompts.sql
```

### Prompt模板

系统会自动从数据库读取以下模板:

1. **LLMCompiler Planner** (Planning阶段)
   - ID: `llmcompiler_planner_v1`
   - 定义Planner的行为和响应格式

2. **LLMCompiler Joiner** (Execution阶段)
   - ID: `llmcompiler_joiner_v1`
   - 定义Joiner的分析和决策流程

## 代码集成

### 1. Planner集成

Planner会自动:
- 从数据库读取prompt模板
- 构建`system_prompt`(角色定义)和`user_prompt`(具体任务)
- 使用流式输出,标记为`ChunkType::Thinking`
- 解析响应中的`[PLAN]`部分

关键代码位置: `src-tauri/src/engines/llm_compiler/planner.rs`

### 2. Joiner集成

Joiner会自动:
- 从数据库读取prompt模板
- 构建分析上下文
- 使用流式输出展示思考过程
- 解析`[DECISION]`部分并执行相应行动

关键代码位置: `src-tauri/src/engines/llm_compiler/joiner.rs`

## 前端显示

### ChunkType类型

系统使用不同的ChunkType来区分消息类型:

- `ChunkType::Thinking`: 思考过程（显示在思考区域）
- `ChunkType::Content`: 最终内容（显示在主对话区）
- `ChunkType::ToolCall`: 工具调用信息
- `ChunkType::ToolResult`: 工具执行结果

### AIChat组件

`AIChat.vue`组件会:
- 接收流式消息
- 根据ChunkType分类显示
- 在UI上展示思考过程和最终答案

## 使用示例

### 场景1: 安全扫描

**用户输入**: "扫描example.com的安全漏洞"

**Planner思考**:
```
[THINKING]
用户想要进行安全扫描...我需要先了解目标的基本信息,
然后进行针对性的漏洞检测。让我设计一个合理的扫描流程...

[PLAN]
(生成包含DNS解析、端口扫描、漏洞扫描的DAG计划)
```

**Joiner分析**:
```
[THINKING]
所有扫描都完成了,发现了一个高危的Apache漏洞...
这已经足够回答用户的问题了。

[DECISION]
{
  "decision": "COMPLETE",
  "response": "扫描完成,发现高危漏洞...",
  "confidence": 0.95
}
```

### 场景2: 信息不足需要继续

**Joiner分析**:
```
[THINKING]
端口扫描完成了,但是用户问的是具体的漏洞,
光知道开放端口还不够,需要进行深度扫描...

[DECISION]
{
  "decision": "CONTINUE",
  "feedback": "需要对发现的服务进行漏洞扫描",
  "suggested_tasks": [
    {
      "id": "task_3",
      "name": "Web漏洞扫描",
      "tool": "web_vulnerability_scanner",
      ...
    }
  ],
  "confidence": 0.6
}
```

## 自定义Prompt

### 修改现有Prompt

1. 直接在数据库中修改对应的prompt记录
2. 或者在`docs/llm_compiler_humanized_prompts.sql`中修改并重新导入

### 创建新版本Prompt

```sql
INSERT INTO prompt_templates (
    id, 
    architecture, 
    stage, 
    title, 
    content, 
    enabled
) VALUES (
    'llmcompiler_planner_v2',  -- 新版本ID
    'LLMCompiler',
    'Planning',
    'LLMCompiler Planner V2 - 更友好的版本',
    '你是一个更加友好的AI助手...',
    1
);
```

## 最佳实践

### 1. Prompt设计原则

- **清晰的角色定义**: 让AI知道自己的职责
- **明确的格式要求**: 使用标记如`[THINKING]`和`[PLAN]`
- **示例驱动**: 在prompt中提供优质的示例
- **自然表达**: 鼓励AI用口语化的方式思考

### 2. 思考过程设计

- **展现推理链**: 一步步展示思考过程
- **解释决策依据**: 说明为什么做某个选择
- **识别问题**: 指出潜在的问题和注意事项
- **考虑替代方案**: 提及考虑过的其他选项

### 3. 响应格式

- **结构化数据**: 使用JSON确保可解析性
- **人性化文本**: 思考过程用自然语言
- **清晰分隔**: 使用明确的标记分隔不同部分

## 故障排除

### 问题1: AI没有按格式响应

**原因**: Prompt可能不够清晰或LLM不遵循格式

**解决方案**:
- 在prompt中添加更多示例
- 强调格式的重要性
- 使用更强大的LLM模型

### 问题2: 思考过程不显示

**原因**: ChunkType可能设置错误

**解决方案**:
- 检查`send_message_stream_with_save_control`调用
- 确保使用`ChunkType::Thinking`
- 检查前端是否正确处理Thinking类型消息

### 问题3: JSON解析失败

**原因**: AI生成的JSON格式不正确

**解决方案**:
- 代码中已包含多种JSON提取策略
- 检查`extract_plan_from_humanized_response`方法
- 查看日志中的原始响应内容

## 技术细节

### 流式输出

```rust
self.ai_service.send_message_stream_with_save_control(
    Some(&user_prompt),
    None,  // 不重复保存user消息
    Some(&system_prompt),
    conversation_id,
    message_id,
    true,  // 启用流式输出
    false,
    Some(ChunkType::Thinking)  // 标记为思考类型
).await
```

### 响应解析

系统会按以下优先级解析响应:
1. 尝试提取`[THINKING]`和`[PLAN]`/`[DECISION]`标记
2. 尝试从代码块中提取JSON
3. 尝试通过大括号匹配提取JSON
4. 尝试修复常见的JSON错误
5. 降级到文本模式解析

## 性能优化

### 缓存策略

- RAG查询结果会缓存
- 工具调用结果可以缓存(如DNS解析)
- Prompt模板会缓存在内存中

### 并行执行

- DAG任务会自动并行执行
- 无依赖的任务同时运行
- 减少总体执行时间

## 未来改进

1. **更丰富的情感表达**: 添加更多人性化的表达方式
2. **上下文记忆**: 记住之前的对话内容
3. **个性化**: 根据用户偏好调整助手风格
4. **多语言支持**: 支持不同语言的拟人化表达

## 参考资料

- [LLMCompiler架构文档](./IMPLEMENTATION_PLAN_FINAL.md)
- [Prompt系统架构](./prompt_architecture_update.md)
- [ReAct架构实现](../src-tauri/src/engines/react/)

## 总结

LLMCompiler的拟人化Prompt系统通过以下方式提升用户体验:

1. ✅ **透明的思考过程**: 用户可以看到AI如何分析问题
2. ✅ **自然的语言表达**: 像和朋友对话一样自然
3. ✅ **清晰的决策逻辑**: 明确说明为什么做某个选择
4. ✅ **流式实时反馈**: 不用等待,实时看到进展
5. ✅ **可定制的风格**: 通过修改prompt调整助手性格

让AI助手不再是冰冷的工具,而是一个善于思考、乐于助人的智能伙伴！

