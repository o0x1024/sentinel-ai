# 消息块顺序显示问题修复

## 问题描述

在 `useOrderedMessages.ts` 中，消息内容并非按照服务端返回的顺序显示。具体表现为：
- 所有 ToolResult 类型的 chunk 都显示在最后
- 违反了增量附加显示的原则
- 破坏了消息的时间顺序和逻辑连贯性

## 根本原因

在 `renderChunksInSequenceOrder` 方法中，代码实现了"智能分组"策略：

```typescript
// 错误的实现：将chunks分为三组，按固定顺序渲染
const contentChunks: OrderedMessageChunk[] = []      // Content + Thinking
const toolResultChunks: OrderedMessageChunk[] = []  // ToolResult
const otherChunks: OrderedMessageChunk[] = []       // 其他

// 渲染顺序：Content/Thinking → Other → ToolResult
// 这导致所有 ToolResult 都显示在最后
```

这种"优化"破坏了服务端通过 `sequence` 字段建立的原始顺序。

## 修复方案

### 核心原则
**完全尊重服务端的 sequence 顺序，不做任何重排**

### 关键修改

#### 1. 简化排序逻辑
```typescript
// 严格按 sequence 顺序排序
const sortedChunks = chunks.slice().sort((a, b) => {
  // 首先按 sequence 排序（这是服务端定义的顺序）
  if (a.sequence !== b.sequence) {
    return a.sequence - b.sequence
  }
  // sequence 相同时，使用到达顺序作为稳定排序的辅助
  const messageId = a.message_id
  const orderMap = this.chunkArrivalOrder.get(messageId)
  const ka = orderMap?.get(`${a.execution_id}#${a.sequence}`) || 0
  const kb = orderMap?.get(`${b.execution_id}#${b.sequence}`) || 0
  return ka - kb
})
```

#### 2. 移除智能分组
删除了将 chunks 分为 Content/Thinking、ToolResult、Other 三组的逻辑。

#### 3. 按序渲染
```typescript
// 按顺序渲染，使用文本缓冲区优化连续的 Content chunks
let textBuffer = ''

for (const chunk of sortedChunks) {
  usedChunks.add(chunk.sequence)
  
  if (chunk.chunk_type === 'Content') {
    // Content 类型：累积到缓冲区
    textBuffer += chunk.content?.toString() || ''
  } else {
    // 非 Content 类型：先输出缓冲区，再渲染当前 chunk
    if (textBuffer.trim()) {
      parts.push(textBuffer)
      textBuffer = ''
    }
    const formatted = this.formatChunkWithSpecialHandling(chunk)
    if (formatted.trim()) {
      parts.push(formatted)
    }
  }
}

// 输出剩余的缓冲文本
if (textBuffer.trim()) {
  parts.push(textBuffer)
}
```

### 附加优化

#### 1. 改进 Action 声明过滤
更精确地过滤 ReAct 架构中的 Action 声明，避免误过滤正常内容：

```typescript
// 按行过滤，只移除明确的格式化 Action 声明
const lines = contentStr.split('\n')
const filtered = lines.filter(line => {
  const trimmed = line.trim()
  // 过滤掉单独的 "Action: xxx" 或 "Action Input: {...}" 行
  if (/^Action:\s*[\w-]+\s*$/i.test(trimmed)) return false
  if (/^Action Input:\s*\{[\s\S]*\}\s*$/i.test(trimmed)) return false
  return true
}).join('\n')
```

#### 2. 调试日志优化
- 将 `debugMode` 改为公开属性，便于外部控制
- 所有调试日志受 `debugMode` 控制，减少生产环境的日志噪音

```typescript
// MessageChunkProcessorImpl 类
debugMode: boolean = false  // 改为公开

// handleMessageChunk 中
if (processor.debugMode) {
  console.log('📥 chunk received:', chunk)
}
```

## 预期效果

修复后，消息显示将完全按照服务端返回的 sequence 顺序：

```
序列示例：
sequence=1  [Content]    "让我帮你查询天气..."
sequence=2  [Thinking]   "需要调用天气API"
sequence=3  [ToolResult] "天气API返回：晴天，25°C"  ← 不再延后显示
sequence=4  [Content]    "根据查询结果，今天天气晴朗..."
sequence=5  [ToolResult] "获取详细预报..."         ← 按顺序显示
sequence=6  [Content]    "完整的天气预报是..."
```

## 测试建议

1. **基本顺序测试**：验证 Content 和 ToolResult 交替出现时的显示顺序
2. **ReAct 架构测试**：验证多轮 Thought-Action-Observation 的显示
3. **步骤视图测试**：验证步骤分组时内部顺序是否正确
4. **边界情况**：相同 sequence 的 chunks（依赖到达顺序）

## 回归风险

- **低风险**：修改仅影响渲染顺序逻辑，不改变数据结构
- **兼容性**：完全向后兼容，只是修正了错误的排序行为
- **性能**：移除了分组逻辑，理论上性能略有提升

## 相关文件

- `src/composables/useOrderedMessages.ts` - 主要修改文件
- `src/types/ordered-chat.ts` - 类型定义（未修改）
