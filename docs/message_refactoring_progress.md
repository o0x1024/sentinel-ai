# 消息传递机制重构进度报告

## 已完成的工作

### 后端改造

#### 1. 核心协议扩展 ✅
- **文件**: `src-tauri/src/utils/ordered_message.rs`
- 新增 `ArchitectureType` 枚举：ReAct, ReWOO, LLMCompiler, PlanAndExecute, Travel, Unknown
- 扩展 `ChunkType` 枚举：新增 `StreamComplete` 类型
- 扩展 `OrderedMessageChunk` 结构：
  - 新增 `architecture: Option<ArchitectureType>` 字段
  - 新增 `structured_data: Option<serde_json::Value>` 字段
- 新增 `emit_message_chunk_with_arch` 函数支持架构信息

#### 2. 标准消息发送器 ✅
- **文件**: `src-tauri/src/utils/message_emitter.rs` (新建)
- 创建 `StandardMessageEmitter` 结构体
- 实现统一的消息发送接口：
  - `emit_start()` - 发送架构开始信号
  - `emit_thinking()` - 发送思考内容
  - `emit_content()` - 发送内容块
  - `emit_tool_result()` - 发送工具结果（强制要求tool_name）
  - `emit_plan()` - 发送计划信息
  - `emit_step_update()` - 发送步骤更新
  - `emit_error()` - 发送错误信息
  - `emit_complete()` - 发送完成信号（必须调用）

#### 3. ReAct架构适配 ✅
- **文件**: `src-tauri/src/engines/react/executor.rs`
- 集成 `StandardMessageEmitter`
- 在执行开始时发送架构开始信号
- 使用 `emit_thinking()` 替代原有的思考消息发送
- 使用 `emit_tool_result()` 替代原有的工具结果发送
- 使用 `emit_step_update()` 发送工具执行状态
- 在执行完成时发送 `StreamComplete` 信号

### 前端改造

#### 1. 类型定义扩展 ✅
- **文件**: `src/types/ordered-chat.ts`
  - 扩展 `ChunkType`：新增 `'StreamComplete'`
  - 新增 `ArchitectureType` 类型
  - 扩展 `OrderedMessageChunk`：新增 `architecture` 和 `structured_data` 字段

- **文件**: `src/types/chat.ts`
  - 扩展 `ChatMessage` 接口：
    - 新增 `architectureType?: string`
    - 新增 `architectureMeta?: any`
    - 新增各架构特定数据字段：`llmCompilerData`, `planAndExecuteData`, `rewooData`, `travelData`

#### 2. 消息处理器增强 ✅
- **文件**: `src/composables/useOrderedMessages.ts`
- `MessageChunkProcessorImpl` 新增功能：
  - `architectureInfo` Map：持久化架构元数据（不随cleanup清除）
  - `streamCompleteFlags` Map：跟踪流完成状态
  - `addChunk()` 方法增强：
    - 记录架构类型信息
    - 检测 `StreamComplete` chunk
    - 更新架构统计信息
  - `isComplete()` 方法改进：优先检查 `StreamComplete` 标志
  - 新增 `getArchitectureInfo()` 方法：获取持久化的架构信息
  - `cleanup()` 方法保留架构信息不清理
- `handleMessageChunk()` 函数增强：
  - 消息完成时保存架构元数据到message对象
  - 根据架构类型解析并保存特定数据
  - 支持 ReAct 和 Travel/Orchestrator 架构的数据持久化

## 待完成的工作

### 后端架构适配

#### 1. Plan-and-Execute架构 ⏳
- **文件**: `src-tauri/src/engines/plan_and_execute/executor.rs`
- 需要集成 `StandardMessageEmitter`
- 规范化步骤开始/完成的Meta事件格式
- 添加 `StreamComplete` 信号

#### 2. LLMCompiler架构 ⏳
- **文件**: `src-tauri/src/engines/llm_compiler/engine_adapter.rs`
- 需要集成 `StandardMessageEmitter`
- 规范化Planning/Execution/Joiner各阶段消息
- 确保并行任务结果按序发送
- 添加 `StreamComplete` 信号

#### 3. ReWOO架构 ⏳
- **文件**: `src-tauri/src/engines/rewoo/engine_adapter.rs`
- 需要集成 `StandardMessageEmitter`
- 统一Planning/Solving/Answering阶段消息格式
- 添加架构标识和 `StreamComplete`

#### 4. Travel架构 ⏳
- **文件**: `src-tauri/src/engines/travel/ooda_executor.rs`
- 需要集成 `StandardMessageEmitter`
- 规范化OODA循环各阶段消息
- 统一子任务调度的消息格式
- 添加 `StreamComplete` 信号

### 前端组件重构

#### 1. StepDisplay组件改造 ⏳
需要修改以下组件，从message对象读取数据而非processor：
- `src/components/MessageParts/ReActStepDisplay.vue`
- `src/components/MessageParts/LLMCompilerStepDisplay.vue`
- `src/components/MessageParts/PlanAndExecuteStepDisplay.vue`
- `src/components/MessageParts/ReWOOStepDisplay.vue`
- `src/components/MessageParts/OrchestratorStepDisplay.vue`

改造要点：
- Props改为接收完整的 `ChatMessage` 对象
- 从 `message.reactSteps` / `message.llmCompilerData` 等读取数据
- 添加降级显示逻辑（数据缺失时显示原始content）

#### 2. AIChat.vue架构判断优化 ⏳
- **文件**: `src/components/AIChat.vue`
- 改进架构判断逻辑：
  - 新增 `getMessageArchitecture()` 函数，从架构元数据判断
  - 更新 `isReActMessage()`, `isLLMCompilerMessage()` 等函数使用元数据
  - 传递完整message对象给StepDisplay组件

### 数据库持久化

#### 1. 数据库表结构扩展 ⏳
需要添加迁移脚本：
```sql
ALTER TABLE messages ADD COLUMN IF NOT EXISTS architecture_type TEXT;
ALTER TABLE messages ADD COLUMN IF NOT EXISTS architecture_meta TEXT;
ALTER TABLE messages ADD COLUMN IF NOT EXISTS structured_data TEXT;
```

#### 2. 消息保存和加载 ⏳
- **文件**: `src/composables/useConversation.ts`
- 更新 `saveMessagesToConversation()` 函数：
  - 包含 `architecture_type`, `architecture_meta`, `structured_data` 字段
  - 提取并序列化架构特定数据
- 更新消息加载逻辑：
  - 反序列化架构元数据
  - 恢复架构特定数据到message对象

### 测试验证

#### 测试项清单 ⏳
- [ ] ReAct架构消息显示完整无覆盖
- [ ] Plan-and-Execute架构消息显示正确
- [ ] LLMCompiler架构消息显示正确
- [ ] ReWOO架构消息显示正确
- [ ] Travel架构消息显示正确
- [ ] 工具调用正确显示
- [ ] 流结束后状态正确更新
- [ ] 刷新页面后消息显示正常
- [ ] 消息保存无重复无遗漏
- [ ] 多轮对话message_id不冲突
- [ ] 取消执行后状态正确

## 关键改进点

1. **统一协议** - 所有架构遵循相同的消息发送规范
2. **明确信号** - StreamComplete专用于标记流结束
3. **元数据驱动** - 架构判断基于元数据而非内容匹配
4. **数据持久化** - 架构特定数据保存到message对象和数据库
5. **组件解耦** - Display组件从message读取，不依赖processor
6. **容错降级** - 数据缺失时降级到普通显示
7. **保存规范** - 只在流完成时由前端触发保存一次

## 下一步行动

1. 完成剩余4个架构的后端适配（Plan-and-Execute, LLMCompiler, ReWOO, Travel）
2. 重构前端StepDisplay组件
3. 更新AIChat.vue的架构判断逻辑
4. 扩展数据库表结构
5. 更新消息保存和加载逻辑
6. 进行全面测试

## 预计剩余工作量

- 后端架构适配：约2-3小时（4个架构 × 30-45分钟）
- 前端组件重构：约1-2小时（5个组件 + AIChat.vue）
- 数据库和持久化：约30分钟
- 测试验证：约1小时

**总计：约5-7小时**

