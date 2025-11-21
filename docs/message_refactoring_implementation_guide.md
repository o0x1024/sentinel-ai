# 消息传递机制重构实施指南

本文档提供详细的实施步骤，用于完成剩余架构的适配工作。

## 已完成的基础设施

### 后端
1. ✅ `ArchitectureType` 枚举和 `StreamComplete` ChunkType
2. ✅ `OrderedMessageChunk` 扩展（architecture, structured_data字段）
3. ✅ `StandardMessageEmitter` 工具类
4. ✅ ReAct架构完整适配

### 前端
1. ✅ TypeScript类型定义更新
2. ✅ `MessageChunkProcessor` 架构元数据持久化
3. ✅ `StreamComplete` 完成判断逻辑
4. ✅ 架构数据保存到message对象

## 剩余架构适配模板

### Plan-and-Execute架构适配

**文件**: `src-tauri/src/engines/plan_and_execute/executor.rs`

#### 步骤1：添加imports
```rust
use crate::utils::message_emitter::StandardMessageEmitter;
use crate::utils::ordered_message::ArchitectureType as ArchType;
```

#### 步骤2：在Executor结构体中添加emitter字段
```rust
pub struct Executor {
    // ... 现有字段
    emitter: Option<StandardMessageEmitter>,
}
```

#### 步骤3：在execute方法开始处创建emitter
```rust
pub async fn execute(&self, plan: ExecutionPlan, context: &mut ExecutionContext) -> Result<ExecutionResult> {
    // 创建标准消息发送器
    let emitter = if let Some(app_handle) = &self.app_handle {
        let execution_id = self.resolve_execution_id(context).await;
        let (message_id, conversation_id) = self.resolve_message_and_conversation_ids(context).await;
        
        Some(StandardMessageEmitter::new(
            Arc::new(app_handle.clone()),
            execution_id,
            message_id,
            conversation_id,
            ArchType::PlanAndExecute,
        ))
    } else {
        None
    };

    // 发送架构开始信号
    if let Some(ref emitter) = emitter {
        emitter.emit_start(Some(serde_json::json!({
            "plan_name": plan.name,
            "total_steps": plan.steps.len(),
        })));
    }
    
    // ... 执行逻辑
}
```

#### 步骤4：替换emit_message_chunk调用
查找所有 `emit_message_chunk` 或 `emit_message_chunk_arc` 调用，替换为：

**思考内容**:
```rust
// 原代码
emit_message_chunk_arc(app, execution_id, message_id, conversation_id, ChunkType::Thinking, content, false, Some("planner"), None);

// 新代码
if let Some(ref emitter) = emitter {
    emitter.emit_thinking(content);
}
```

**工具结果**:
```rust
// 原代码
emit_message_chunk_arc(app, execution_id, message_id, conversation_id, ChunkType::ToolResult, result_json, false, Some("executor"), Some(tool_name));

// 新代码
if let Some(ref emitter) = emitter {
    let result_value = serde_json::from_str(result_json).unwrap_or(serde_json::json!({"result": result_json}));
    emitter.emit_tool_result(tool_name, &result_value);
}
```

**计划信息**:
```rust
// 原代码
emit_message_chunk_arc(app, execution_id, message_id, conversation_id, ChunkType::PlanInfo, plan_json, false, Some("planner"), None);

// 新代码
if let Some(ref emitter) = emitter {
    emitter.emit_plan(plan_json);
}
```

**步骤更新**:
```rust
// 原代码 - Meta事件
let meta = serde_json::json!({"type": "step_started", "step_index": index, "step_name": step.name});
emit_message_chunk_arc(app, execution_id, message_id, conversation_id, ChunkType::Meta, &meta.to_string(), false, Some("executor"), None);

// 新代码
if let Some(ref emitter) = emitter {
    emitter.emit_step_update(index, &step.name, "started");
}
```

#### 步骤5：在执行结束时发送完成信号
```rust
// 在返回ExecutionResult之前
if let Some(ref emitter) = emitter {
    emitter.emit_complete(Some(serde_json::json!({
        "total_steps": plan.steps.len(),
        "completed_steps": completed_count,
        "failed_steps": failed_count,
        "total_duration_ms": total_duration,
    })));
}

Ok(result)
```

### LLMCompiler架构适配

**文件**: `src-tauri/src/engines/llm_compiler/engine_adapter.rs`

遵循相同的模式：

1. 添加imports
2. 在LlmCompilerEngine中添加emitter字段
3. 在execute方法中创建emitter并发送start信号
4. Planning阶段使用 `emitter.emit_plan()`
5. Execution阶段使用 `emitter.emit_tool_result()`
6. Joiner阶段使用 `emitter.emit_thinking()`
7. 最后发送 `emitter.emit_complete()`

### ReWOO架构适配

**文件**: `src-tauri/src/engines/rewoo/engine_adapter.rs`

1. 添加imports和emitter字段
2. Planning阶段：`emitter.emit_plan()`
3. Solving阶段：`emitter.emit_tool_result()` for each tool
4. Answering阶段：`emitter.emit_content()`
5. 完成时：`emitter.emit_complete()`

### Travel架构适配

**文件**: `src-tauri/src/engines/travel/ooda_executor.rs`

1. 添加imports和emitter字段
2. OODA循环各阶段使用相应的emit方法
3. Observe: `emitter.emit_thinking()`
4. Orient: `emitter.emit_thinking()`
5. Decide: `emitter.emit_plan()`
6. Act: `emitter.emit_tool_result()`
7. 完成时：`emitter.emit_complete()`

## 前端组件重构指南

### StepDisplay组件通用改造模式

以 `ReActStepDisplay.vue` 为例：

#### 原代码
```vue
<script setup lang="ts">
const props = defineProps<{
  stepData: ReActStepData
}>()
</script>
```

#### 新代码
```vue
<script setup lang="ts">
import type { ChatMessage } from '../../types/chat'

const props = defineProps<{
  message: ChatMessage
}>()

// 从message对象读取数据
const steps = computed(() => {
  return props.message.reactSteps || []
})

// 降级显示
const hasFallback = computed(() => {
  return !props.message.reactSteps || props.message.reactSteps.length === 0
})
</script>

<template>
  <div v-if="hasFallback" class="prose prose-sm" v-html="renderMarkdown(message.content)"></div>
  <div v-else>
    <!-- 正常的步骤显示 -->
    <div v-for="(step, index) in steps" :key="index">
      <!-- ... -->
    </div>
  </div>
</template>
```

### AIChat.vue架构判断改进

```typescript
// 新增：从架构元数据判断
const getMessageArchitecture = (message: ChatMessage) => {
  // 优先使用message对象中的architectureType
  if (message.architectureType) {
    return message.architectureType
  }
  
  // 回退到processor（仅用于streaming消息）
  if (message.isStreaming) {
    const archInfo = orderedMessages.processor.getArchitectureInfo(message.id)
    return archInfo?.type || 'Unknown'
  }
  
  return 'Unknown'
}

// 更新架构判断函数
const isReActMessage = (message: ChatMessage) => {
  return getMessageArchitecture(message) === 'ReAct'
}

const isLLMCompilerMessage = (message: ChatMessage) => {
  return getMessageArchitecture(message) === 'LLMCompiler'
}

const isPlanAndExecuteMessage = (message: ChatMessage) => {
  return getMessageArchitecture(message) === 'PlanAndExecute'
}

const isReWOOMessage = (message: ChatMessage) => {
  return getMessageArchitecture(message) === 'ReWOO'
}

const isTravelMessage = (message: ChatMessage) => {
  return getMessageArchitecture(message) === 'Travel'
}
```

### AIChat.vue组件传递改进

```vue
<!-- 原代码 -->
<ReActStepDisplay
  v-for="(step, index) in parseReActSteps(message.content, message.id)"
  :key="`react-step-${index}`"
  :step-data="step"
/>

<!-- 新代码 -->
<ReActStepDisplay
  v-if="isReActMessage(message)"
  :message="message"
/>
```

## 数据库迁移

### 创建迁移脚本

**文件**: `src-tauri/migrations/add_architecture_fields.sql`

```sql
-- 添加架构相关字段
ALTER TABLE messages ADD COLUMN IF NOT EXISTS architecture_type TEXT;
ALTER TABLE messages ADD COLUMN IF NOT EXISTS architecture_meta TEXT;
ALTER TABLE messages ADD COLUMN IF NOT EXISTS structured_data TEXT;

-- 创建索引以提升查询性能
CREATE INDEX IF NOT EXISTS idx_messages_architecture_type ON messages(architecture_type);
```

### 更新消息保存逻辑

**文件**: `src/composables/useConversation.ts`

```typescript
const saveMessagesToConversation = async (messages: ChatMessage[]) => {
  const messagesToSave = messages.filter(m => !m.isStreaming)
  
  for (const msg of messagesToSave) {
    await invoke('save_message_to_conversation', {
      conversationId: currentConversationId.value,
      message: {
        id: msg.id,
        role: msg.role,
        content: msg.content,
        timestamp: msg.timestamp,
        // 新增：架构元数据
        architecture_type: msg.architectureType,
        architecture_meta: JSON.stringify(msg.architectureMeta || {}),
        structured_data: JSON.stringify(extractStructuredData(msg)),
      }
    })
  }
}

// 提取结构化数据
const extractStructuredData = (msg: ChatMessage) => {
  const archType = msg.architectureType
  
  switch (archType) {
    case 'ReAct':
      return { reactSteps: msg.reactSteps }
    case 'LLMCompiler':
      return { llmCompilerData: msg.llmCompilerData }
    case 'PlanAndExecute':
      return { planAndExecuteData: msg.planAndExecuteData }
    case 'ReWOO':
      return { rewooData: msg.rewooData }
    case 'Travel':
      return { travelData: msg.travelData }
    default:
      return {}
  }
}
```

### 更新消息加载逻辑

```typescript
const loadConversationMessages = async (conversationId: string) => {
  const rawMessages = await invoke('get_conversation_messages', { conversationId })
  
  return rawMessages.map(msg => {
    const base = {
      ...msg,
      architectureType: msg.architecture_type,
      architectureMeta: safeJsonParse(msg.architecture_meta),
    }
    
    // 恢复架构特定数据
    const structuredData = safeJsonParse(msg.structured_data)
    if (structuredData) {
      Object.assign(base, structuredData)
    }
    
    return base
  })
}

const safeJsonParse = (json: string) => {
  try {
    return json ? JSON.parse(json) : {}
  } catch (e) {
    console.warn('Failed to parse JSON:', e)
    return {}
  }
}
```

## 测试清单

### 功能测试

#### ReAct架构
- [ ] 发送消息后正确显示Thought
- [ ] 工具调用显示完整（tool_name, args, result）
- [ ] Final Answer正确显示
- [ ] 刷新页面后消息完整显示
- [ ] 多轮对话不互相覆盖

#### Plan-and-Execute架构
- [ ] 计划正确显示
- [ ] 步骤执行状态实时更新
- [ ] 工具调用结果正确显示
- [ ] 重新规划正确显示
- [ ] 刷新后数据完整

#### LLMCompiler架构
- [ ] Planning阶段DAG显示正确
- [ ] Execution阶段并行任务显示正确
- [ ] Joiner决策显示正确
- [ ] Summary统计正确
- [ ] 刷新后数据完整

#### ReWOO架构
- [ ] Planning阶段显示正确
- [ ] Solving阶段工具调用显示正确
- [ ] Answering阶段答案显示正确
- [ ] 刷新后数据完整

#### Travel架构
- [ ] OODA循环各阶段显示正确
- [ ] 子任务调度显示正确
- [ ] Orchestrator事件显示正确
- [ ] 刷新后数据完整

### 边界情况测试
- [ ] 取消执行后状态正确
- [ ] 网络中断后恢复正常
- [ ] 工具执行失败正确显示
- [ ] 超时情况正确处理
- [ ] 并发多个对话不冲突

### 性能测试
- [ ] 长对话（100+消息）加载流畅
- [ ] 大量工具调用（50+）显示正常
- [ ] 消息保存不阻塞UI
- [ ] 刷新页面加载速度可接受

## 常见问题排查

### 问题1：消息显示不全
**原因**: 未发送StreamComplete信号
**解决**: 确保每个架构在执行结束时调用 `emitter.emit_complete()`

### 问题2：消息覆盖
**原因**: message_id冲突或未正确使用
**解决**: 确保每次执行使用唯一的message_id和execution_id

### 问题3：刷新后数据丢失
**原因**: 架构数据未保存到数据库
**解决**: 检查 `extractStructuredData()` 函数是否正确提取数据

### 问题4：架构判断错误
**原因**: 未发送架构标识或前端未正确读取
**解决**: 
1. 后端确保在start信号中包含architecture字段
2. 前端确保使用 `getMessageArchitecture()` 判断

### 问题5：工具调用不显示
**原因**: 未使用 `emit_tool_result()` 或tool_name缺失
**解决**: 使用StandardMessageEmitter的 `emit_tool_result()` 方法，必须提供tool_name

## 验收标准

完成所有工作后，应满足以下标准：

1. ✅ 所有5个架构都使用StandardMessageEmitter
2. ✅ 所有架构都发送StreamComplete信号
3. ✅ 前端所有StepDisplay组件从message对象读取数据
4. ✅ 数据库包含架构字段并正确保存/加载
5. ✅ 所有功能测试通过
6. ✅ 刷新页面后消息显示正常
7. ✅ 无消息覆盖或丢失问题
8. ✅ 取消执行后状态正确

## 预计工作量

- Plan-and-Execute适配: 45分钟
- LLMCompiler适配: 45分钟
- ReWOO适配: 30分钟
- Travel适配: 45分钟
- 前端组件重构: 90分钟
- 数据库迁移: 30分钟
- 测试验证: 60分钟

**总计: 约6小时**

