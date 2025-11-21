# Travel架构快速参考

## 核心概念

Travel架构基于**OODA循环**(Observe-Orient-Decide-Act)，是一个智能任务执行框架。

### OODA四阶段

| 阶段 | 英文 | 中文 | 职责 |
|------|------|------|------|
| O | Observe | 观察 | 收集信息、查询Memory、护栏检查 |
| O | Orient | 定位 | 威胁分析、漏洞识别、情报查询 |
| D | Decide | 决策 | 制定行动计划、评估风险 |
| A | Act | 执行 | 根据复杂度调用工具或引擎 |

## 文件结构

### 后端 (Rust)

```
src-tauri/src/engines/travel/
├── types.rs                 # 核心类型定义
├── engine_adapter.rs        # 引擎适配器
├── ooda_executor.rs         # OODA执行器
├── engine_dispatcher.rs     # 引擎调度器
├── complexity_analyzer.rs   # 复杂度分析
├── guardrails.rs           # 护栏检查
├── threat_intel.rs         # 威胁情报
├── memory_integration.rs   # Memory集成
└── react_executor.rs       # ReAct执行器
```

### 前端 (TypeScript/Vue)

```
src/
├── composables/
│   └── useTravelMessage.ts              # 消息解析逻辑
├── components/MessageParts/
│   └── TravelStepDisplay.vue            # 展示组件
└── types/
    └── ordered-chat.ts                  # 类型定义
```

## 关键类型

### TravelMessageData

```typescript
interface TravelMessageData {
    taskComplexity?: string          // Simple | Medium | Complex
    oodaCycles?: OodaCycle[]        // OODA循环列表
    metrics?: TravelMetrics         // 执行指标
}
```

### OodaCycle

```typescript
interface OodaCycle {
    cycle_number: number            // 循环序号
    phase_history: PhaseExecution[] // 阶段执行历史
    status: string                  // Running | Completed | Failed
    started_at: string              // 开始时间
    completed_at?: string           // 完成时间
    result?: any                    // 循环结果
    error?: string                  // 错误信息
}
```

### PhaseExecution

```typescript
interface PhaseExecution {
    phase: string                   // Observe | Orient | Decide | Act
    status: string                  // Pending | Running | Completed | Failed
    started_at: string              // 开始时间
    completed_at?: string           // 完成时间
    input: any                      // 阶段输入
    output?: any                    // 阶段输出
    guardrail_checks: GuardrailCheck[]  // 护栏检查结果
    tool_calls: ToolCall[]          // 工具调用记录
    error?: string                  // 错误信息
}
```

## 快速开始

### 后端发送消息

```rust
use crate::utils::ordered_message::{emit_message_chunk_arc, ChunkType, ArchitectureType};

// 发送阶段开始消息
emit_message_chunk_arc(
    &app_handle,
    &execution_id,
    &message_id,
    conversation_id.as_deref(),
    ChunkType::Thinking,
    "🔄 Starting Observe phase...",
    false,
    Some("Observe"),
    None,
    Some(ArchitectureType::Travel),
    Some(serde_json::json!({
        "phase": "Observe",
        "status": "started"
    }))
);

// 发送阶段完成消息
emit_message_chunk_arc(
    &app_handle,
    &execution_id,
    &message_id,
    conversation_id.as_deref(),
    ChunkType::Thinking,
    "✅ Observe phase completed",
    false,
    Some("Observe"),
    None,
    Some(ArchitectureType::Travel),
    Some(serde_json::json!({
        "phase": "Observe",
        "status": "completed",
        "output": {
            "observations": {...}
        }
    }))
);
```

### 前端检测和解析

```typescript
import { isTravelMessage, parseTravelMessage } from '@/composables/useTravelMessage'

// 检测是否为Travel消息
const isTravelMsg = isTravelMessage(message.content, message.orderedChunks)

// 解析Travel消息
if (isTravelMsg) {
  const travelData = parseTravelMessage(message.content, message.orderedChunks)
  console.log('OODA Cycles:', travelData.oodaCycles)
  console.log('Metrics:', travelData.metrics)
}
```

### 前端展示

```vue
<template>
  <TravelStepDisplay 
    :message="message"
    :stepData="parseTravelMessageData(message)"
  />
</template>

<script setup>
import TravelStepDisplay from '@/components/MessageParts/TravelStepDisplay.vue'
import { parseTravelMessage } from '@/composables/useTravelMessage'

const parseTravelMessageData = (message) => {
  const chunks = message.orderedChunks || []
  return parseTravelMessage(message.content, chunks)
}
</script>
```

## 常见任务

### 添加新的阶段状态

1. **后端**: 在 `structured_data` 中添加新字段
2. **前端**: 在 `parseTravelMessage` 中处理新字段
3. **组件**: 在 `TravelStepDisplay.vue` 中展示

### 添加自定义指标

1. **后端**: 在 `TravelMetrics` 类型中添加字段
2. **前端**: 在 `parseTravelMessage` 中计算指标
3. **组件**: 在指标摘要部分展示

### 调试消息流

```typescript
// 在 parseTravelMessage 中添加日志
console.log('[Travel] Parsing chunks:', chunks.length)
console.log('[Travel] Chunks:', chunks.map(c => ({
  stage: c.stage,
  type: c.chunk_type,
  arch: c.architecture,
  data: c.structured_data
})))
```

## 最佳实践

1. **消息发送**: 
   - 总是设置 `architecture: ArchitectureType::Travel`
   - 在 `structured_data` 中包含阶段状态信息
   - 使用标准的阶段名称 (Observe, Orient, Decide, Act)

2. **状态管理**:
   - 阶段状态: Pending → Running → Completed/Failed
   - 循环状态: Running → Completed/Failed
   - 使用时间戳记录阶段的开始和结束

3. **错误处理**:
   - 使用 `ChunkType::Error` 发送错误消息
   - 在 `structured_data` 中包含详细错误信息
   - 记录错误发生的阶段

4. **工具调用**:
   - 使用 `ChunkType::ToolResult` 发送工具结果
   - 包含 `tool_name` 字段
   - 在Act阶段记录所有工具调用

## 故障排查

### 消息未显示

1. 检查 `architecture` 字段是否为 "Travel"
2. 检查 `stage` 字段是否为有效的OODA阶段
3. 检查 `structured_data` 格式是否正确

### 阶段状态不更新

1. 确保发送了阶段开始和完成消息
2. 检查 `structured_data.status` 值
3. 验证时间戳是否正确

### 工具调用未显示

1. 确保使用了 `ChunkType::ToolResult`
2. 检查 `tool_name` 字段
3. 验证在Act阶段发送

## 相关文档

- [完整架构文档](./travel_messaging_architecture.md)
- [Travel引擎实现](../src-tauri/src/engines/travel/)
- [前端消息处理](../src/composables/useTravelMessage.ts)
- [展示组件](../src/components/MessageParts/TravelStepDisplay.vue)
