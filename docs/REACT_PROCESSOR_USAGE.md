# ReAct 消息处理器使用指南

本文档说明如何在项目中使用重构后的 ReAct 消息处理系统。

---

## 📖 快速开始

### 在 Vue 组件中使用 ReActStepDisplay

```vue
<template>
  <ReActStepDisplay :message="message" />
</template>

<script setup lang="ts">
import ReActStepDisplay from '@/components/MessageParts/ReActStepDisplay.vue'
import type { ChatMessage } from '@/types/chat'

defineProps<{
  message: ChatMessage
}>()
</script>
```

### 在其他地方使用 ReActMessageProcessor

```typescript
import { ReActMessageProcessor } from '@/composables/processors/ReActMessageProcessor'
import type { ChatMessage } from '@/types/chat'

// 从消息构建步骤数据
const message: ChatMessage = { /* ... */ }
const steps = ReActMessageProcessor.buildReActStepsFromMessage(message)

// 格式化数据
const formatted = ReActMessageProcessor.formatJson(someObject)
const label = ReActMessageProcessor.getStatusLabel('success')
```

---

## 🔧 核心 API

### ReActMessageProcessor

#### 主要方法

##### `buildReActStepsFromMessage(message: ChatMessage): ReActStepDisplay[]`
从完整的 ChatMessage 对象构建步骤显示数据。

**参数**:
- `message` - ChatMessage 对象，包含 `architectureMeta` 或 `reactSteps` 字段

**返回值**:
- `ReActStepDisplay[]` - 步骤显示数组

**示例**:
```typescript
const steps = ReActMessageProcessor.buildReActStepsFromMessage(message)
console.log(steps[0].thought) // 输出第一步的思考内容
console.log(steps[0].action)  // 输出第一步的行动信息
```

---

##### `extractStepsFromChunks(chunks: OrderedMessageChunk[]): ReActStepDisplay[]`
从原始的消息块数组中提取步骤信息，用于重建未完成的流。

**参数**:
- `chunks` - OrderedMessageChunk 数组

**返回值**:
- `ReActStepDisplay[]` - 提取的步骤数组

**示例**:
```typescript
const chunks = processor.chunks.get(messageId) || []
const steps = ReActMessageProcessor.extractStepsFromChunks(chunks)
```

---

#### 工具方法

##### `shouldCollapseToolCall(action: ReActStepDisplay['action']): boolean`
判断工具调用详情是否应该折叠（不展开）。

**逻辑**:
- 运行中或待处理时 → 返回 `false`（保持展开）
- 已完成、成功、失败或错误 → 返回 `true`（折叠）

---

##### `hasObservationError(observation: any): boolean`
检测观察数据中是否包含错误信息。

**检查项**:
- 字符串中包含 'error'、'failed'、'失败'
- JSON 中 `success === false`
- 对象中有 `error` 字段

---

##### `formatObservation(observation: any): string`
将观察数据格式化为可读的字符串。

**处理**:
- 字符串 → 直接返回
- 对象 → JSON 序列化，缩进 2 空格
- 其他 → 调用 `String()`

---

##### `formatParams(args: any): Record<string, any>`
格式化参数对象。

**处理**:
- 对象 → 直接返回
- JSON 字符串 → 解析并返回
- 其他 → 包装为 `{ value: args }`

---

##### `formatJson(obj: any): string`
格式化任意对象为美化的 JSON 字符串。

**示例**:
```typescript
const json = ReActMessageProcessor.formatJson({ name: 'test', value: 123 })
// 返回：
// {
//   "name": "test",
//   "value": 123
// }
```

---

##### `getStepIcon(stepType: ReActStepType | string): string`
获取步骤类型对应的图标。

**映射**:
- `Thought` → `🤔`
- `Action` → `🔧`
- `Observation` → `👁️`
- `Final` → `🏁`
- `Error` → `❌`
- 其他 → `⚙️`

---

##### `getStatusLabel(status?: string): string`
获取执行状态的中文标签。

**映射示例**:
```typescript
getStatusLabel('running')    // → '运行中'
getStatusLabel('success')    // → '成功'
getStatusLabel('failed')     // → '失败'
getStatusLabel('error')      // → '错误'
getStatusLabel(undefined)    // → '待处理'
```

---

### ArchitectureProcessorFactory

#### `getProcessor(architectureType?: ArchitectureType): IArchitectureMessageProcessor | null`
根据架构类型获取对应的处理器。

**参数**:
- `architectureType` - 架构类型（'ReAct'、'ReWOO'、'LLMCompiler' 等）

**返回值**:
- 对应的处理器实例，或 `null`（如果架构不支持）

**支持的架构**:
- ✅ `'ReAct'` - 返回 `ReActProcessorAdapter`
- ⏳ `'ReWOO'` - 待实现
- ⏳ `'LLMCompiler'` - 待实现
- ⏳ `'PlanAndExecute'` - 待实现
- ⏳ `'Travel'` - 待实现

**示例**:
```typescript
const processor = ArchitectureProcessorFactory.getProcessor('ReAct')
if (processor) {
  const data = processor.buildDisplayData(message)
  console.log(data)
}
```

---

#### `hasArchitecture(message: ChatMessage): boolean`
检查消息是否具有特定的架构类型。

**示例**:
```typescript
if (ArchitectureProcessorFactory.hasArchitecture(message)) {
  const processor = ArchitectureProcessorFactory.getProcessor(message.architectureType)
  // ...
}
```

---

## 📊 数据结构

### ReActStepDisplay

```typescript
interface ReActStepDisplay {
  // 步骤索引
  index: number
  
  // 思考内容（可选）
  thought?: string
  
  // 行动信息（可选）
  action?: {
    tool: string                    // 工具名称
    args: Record<string, any>       // 工具参数
    status?: ActionStatus           // 执行状态
  }
  
  // 观察信息（可选）
  observation?: any
  
  // 错误信息（可选）
  error?: string
  
  // 最终答案（可选）
  finalAnswer?: string
  
  // 时间戳（可选）
  timestamp?: string
  
  // 步骤 ID（可选）
  id?: string
}
```

---

### ReActArchitectureMeta

```typescript
interface ReActArchitectureMeta {
  type: 'ReAct'
  
  statistics?: {
    total_iterations: number
    tool_calls_count: number
    successful_tool_calls: number
    failed_tool_calls: number
    total_duration_ms: number
    status: string
  }
  
  steps?: Array<{
    thought?: string
    action?: {
      tool: string
      args: Record<string, any>
      status: string
    }
    observation?: any
    finalAnswer?: string
    citations?: string[]
    error?: {
      type: string
      message: string
      retryable: boolean
    }
  }>
}
```

---

## 🎯 常见使用场景

### 场景 1: 在消息列表中显示 ReAct 步骤

```vue
<template>
  <div v-for="message in messages" :key="message.id">
    <div class="message-content">
      {{ message.content }}
    </div>
    
    <!-- 如果是 ReAct 消息，显示步骤详情 -->
    <ReActStepDisplay
      v-if="message.architectureType === 'ReAct'"
      :message="message"
    />
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import ReActStepDisplay from '@/components/MessageParts/ReActStepDisplay.vue'

const props = defineProps<{
  messages: ChatMessage[]
}>()
</script>
```

---

### 场景 2: 提取并分析 ReAct 步骤

```typescript
import { ReActMessageProcessor } from '@/composables/processors/ReActMessageProcessor'

function analyzeReActMessage(message: ChatMessage) {
  const steps = ReActMessageProcessor.buildReActStepsFromMessage(message)
  
  // 统计信息
  const thoughtCount = steps.filter(s => s.thought).length
  const actionCount = steps.filter(s => s.action).length
  const hasError = steps.some(s => s.error)
  
  return {
    totalSteps: steps.length,
    thoughtCount,
    actionCount,
    hasError,
    steps
  }
}
```

---

### 场景 3: 格式化并导出步骤数据

```typescript
function exportReActSteps(message: ChatMessage): string {
  const steps = ReActMessageProcessor.buildReActStepsFromMessage(message)
  
  return steps.map((step, index) => {
    let output = `## 步骤 ${index + 1}\n\n`
    
    if (step.thought) {
      output += `**思考**: ${step.thought}\n\n`
    }
    
    if (step.action) {
      output += `**行动**: ${step.action.tool}\n`
      output += `参数: ${ReActMessageProcessor.formatJson(step.action.args)}\n\n`
    }
    
    if (step.observation) {
      output += `**观察**: ${ReActMessageProcessor.formatObservation(step.observation)}\n\n`
    }
    
    if (step.finalAnswer) {
      output += `**答案**: ${step.finalAnswer}\n\n`
    }
    
    return output
  }).join('\n---\n')
}
```

---

### 场景 4: 实时更新流式消息中的步骤

```typescript
import { ReActMessageProcessor } from '@/composables/processors/ReActMessageProcessor'

const processor = useOrderedMessages(messages)

// 在消息块到达时
processor.addChunk(chunk)

// 实时获取当前步骤
const currentMessage = messages.value[messages.value.length - 1]
if (currentMessage.architectureType === 'ReAct') {
  const steps = ReActMessageProcessor.buildReActStepsFromMessage(currentMessage)
  const currentStep = steps[steps.length - 1]
  
  console.log('当前步骤:', currentStep.thought || currentStep.action)
}
```

---

## 🔄 向后兼容性

### 保留的接口

ReActStepDisplay 组件仍支持旧的 `stepData` prop，用于向后兼容：

```vue
<!-- 新方式：使用完整的消息对象 -->
<ReActStepDisplay :message="message" />

<!-- 旧方式：仍然支持 -->
<ReActStepDisplay :stepData="stepData" />
```

### 支持的遗留字段

- `message.reactSteps` - 如果存在，处理器会自动处理
- `stepData` prop - 组件自动转换为 `ReActStepDisplay` 格式

---

## 🚨 常见问题

### Q: 如何扩展处理器以支持新的数据格式？

A: 创建 ReActMessageProcessor 的子类或修改相应的解析方法：

```typescript
class CustomReActProcessor extends ReActMessageProcessor {
  static buildCustomFormat(message: ChatMessage) {
    // 自定义逻辑
  }
}
```

---

### Q: 如何为新的架构创建处理器？

A: 实现 `IArchitectureMessageProcessor` 接口，并在工厂中注册：

```typescript
class MyArchitectureProcessor implements IArchitectureMessageProcessor {
  buildDisplayData(message: ChatMessage): any { /* ... */ }
  extractStepsFromChunks(chunks: OrderedMessageChunk[]): any { /* ... */ }
  shouldCollapse(data: any): boolean { /* ... */ }
  formatData(data: any): string { /* ... */ }
}

// 在工厂中注册
export class ArchitectureProcessorFactory {
  static getProcessor(architectureType?: ArchitectureType) {
    switch (architectureType) {
      case 'MyArchitecture':
        return new MyArchitectureProcessor()
      // ...
    }
  }
}
```

---

### Q: 为什么 ReActStepDisplay 显示不正确？

A: 检查以下几点：

1. ✅ 消息是否有 `architectureType === 'ReAct'`
2. ✅ 消息是否包含 `architectureMeta` 或 `reactSteps` 数据
3. ✅ 检查浏览器控制台是否有错误信息
4. ✅ 尝试使用旧的 `stepData` prop 进行测试

---

## 📚 相关资源

- 类型定义: `src/types/react.ts`
- 处理器实现: `src/composables/processors/ReActMessageProcessor.ts`
- 组件实现: `src/components/MessageParts/ReActStepDisplay.vue`
- 重构总结: `docs/REACT_MESSAGE_REFACTORING.md`

---

**版本**: 1.0.0  
**最后更新**: 2025-11-21
