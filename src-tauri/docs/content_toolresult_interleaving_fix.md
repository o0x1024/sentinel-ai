# Content 和 ToolResult 穿插显示问题修复

## 问题描述

在前端显示中，ToolResult 和 Content 内容出现穿插显示的问题：

```
步骤 2: 获取登录页面特征
分析                          <- Content chunk
🔧 http_request 成功          <- ToolResult chunk  
: **                          <- Content chunk (continued)
```

预期应该是：
```
步骤 2: 获取登录页面特征
分析: **完整的文本内容**      <- 所有 Content chunks 合并
🔧 http_request 成功          <- ToolResult chunk
```

## 问题根源h

### 后端发送机制

1. **AI 流式响应**（Content chunks）
   - 通过 `AiService::send_message_stream` 异步发送
   - 每个文本片段作为一个 Content chunk 发送
   - 使用全局 sequence 计数器分配序号

2. **工具执行结果**（ToolResult chunks）
   - 在步骤执行完成后同步发送
   - 通过 `emit_tool_result` 发送
   - 使用同一个全局 sequence 计数器

3. **Sequence 交错**
   - 由于异步执行，chunks 的 sequence 可能是：
     - Content(seq=1) "分析"
     - Content(seq=2) ": "
     - ToolResult(seq=3) "http_request result"
     - Content(seq=4) "**"

### 前端原有逻辑

之前的 `renderChunksInSequenceOrder` 方法严格按 sequence 顺序渲染，导致：
- 遇到 Content chunk → 追加到文本缓冲区
- 遇到 ToolResult chunk → 输出缓冲区，然后输出 ToolResult
- 再遇到 Content chunk → 重新开始新的文本缓冲区

这就造成了穿插显示。

## 解决方案

### 核心策略

在步骤内智能重排序 chunks，确保逻辑顺序正确：

1. **Content/Thinking 优先**：先渲染所有 Content 和 Thinking chunks
2. **ToolResult 延后**：最后渲染所有 ToolResult chunks
3. **保持内部顺序**：每组内部仍按 sequence 排序

### 实现细节

```typescript
private renderChunksInSequenceOrder(
  chunks: OrderedMessageChunk[], 
  parts: string[], 
  usedChunks: Set<number>
): void {
  // 1. 按类型分组
  const contentChunks: OrderedMessageChunk[] = []
  const toolResultChunks: OrderedMessageChunk[] = []
  const otherChunks: OrderedMessageChunk[] = []
  
  for (const chunk of sortedChunks) {
    if (chunk.chunk_type === 'Content' || chunk.chunk_type === 'Thinking') {
      contentChunks.push(chunk)
    } else if (chunk.chunk_type === 'ToolResult') {
      toolResultChunks.push(chunk)
    } else {
      otherChunks.push(chunk)
    }
  }
  
  // 2. 按组渲染：Content → Other → ToolResult
  // 先渲染所有 Content 和 Thinking
  // 再渲染其他类型（Meta, Error等）
  // 最后渲染所有 ToolResult
}
```

### 渲染顺序

```
步骤开始
  ↓
1. 渲染所有 Content chunks（合并为连续文本）
  ↓
2. 渲染所有 Thinking chunks
  ↓
3. 渲染其他类型 chunks（Meta, Error等）
  ↓
4. 渲染所有 ToolResult chunks
  ↓
步骤结束
```

## 效果对比

### 修复前
```
步骤 2: 获取登录页面特征
分析
🔧 http_request 成功
: **响应头包含...
```

### 修复后
```
步骤 2: 获取登录页面特征
分析: **响应头包含...
🔧 http_request 成功
```

## 调试功能

可以通过以下方式开启调试模式查看 chunks 的分组和渲染顺序：

```typescript
const { setDebugMode } = useOrderedMessages(messages)
setDebugMode(true)
```

调试信息会在浏览器控制台输出：
```
📊 Rendering chunks - Original sequence order: [...]
📊 After grouping: { content: 3, toolResult: 1, other: 0 }
```

## 相关文件

- 前端渲染逻辑：`src/composables/useOrderedMessages.ts`
- 后端 sequence 分配：`src-tauri/src/utils/ordered_message.rs`
- AI 服务流式发送：`src-tauri/src/services/ai.rs`
- 执行器工具结果发送：`src-tauri/src/engines/plan_and_execute/executor.rs`

## 注意事项

1. **不改变 sequence 语义**：sequence 仍然表示发送顺序，只是前端渲染时智能重排
2. **保持步骤隔离**：重排序只在步骤内部进行，不跨步骤
3. **向后兼容**：对于没有步骤信息的消息，回退到时间线视图（严格按 sequence）


