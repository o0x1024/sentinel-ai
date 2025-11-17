# ReWOO消息显示优化

## 问题描述

ReWOO架构的消息显示存在以下问题：
1. **计划未显示**：JSON格式的计划没有被正确解析和渲染
2. **Unknown折叠面板**：执行阶段出现tool_name为"unknown"的折叠面板，信息混乱
3. **缺少参数和响应**：工具调用没有显示参数（PARAMETERS）和响应（RESPONSE）信息

## 解决方案

### 1. 移除旧Plan格式兼容

**文件**: `src/composables/useReWOOMessage.ts`

**修改内容**:
- 移除对旧式 `Plan: ... #E1 = tool[...]` 文本格式的解析
- 仅保留JSON计划格式的解析
- 在 `isReWOOMessage` 中移除旧式检测正则
- 在 `parsePlanningData` 中移除旧式回退解析
- 在 `extractReWOOSummary` 中优先读取JSON的 `plan_summary`

### 2. 优化执行步骤解析

**文件**: `src/composables/useReWOOMessage.ts`

**修改内容**:
- 改进 `parseExecutionSteps` 函数，按工具名称分组并保持顺序
- 从 `rewoo_execution` stage的chunks中提取工具执行信息
- 正确处理 `Thinking`、`ToolResult`、`Error` 等不同chunk类型
- 添加 `args` 字段到 `ReWOOExecutionStep` 接口

### 3. 关联计划参数到执行步骤

**文件**: `src/composables/useReWOOMessage.ts`

**修改内容**:
- 在 `parsePlanningData` 中提取JSON计划的 `args` 字段
- 在 `parseReWOOMessage` 中创建工具名称到计划步骤的映射
- 将计划中的参数关联到对应的执行步骤

**实现逻辑**:
```typescript
// 创建工具名称到计划步骤的映射
const planStepMap = new Map<string, any>()
planningData.steps.forEach(step => {
  planStepMap.set(step.tool, step)
})

// 为每个执行步骤关联参数
executionSteps.forEach(execStep => {
  const planStep = planStepMap.get(execStep.toolName)
  if (planStep && planStep.args && !execStep.args) {
    execStep.args = planStep.args
  }
})
```

### 4. 重构显示组件

**文件**: `src/components/MessageParts/ReWOOStepDisplay.vue`

**修改内容**:
- 参考 `ReActStepDisplay.vue` 的样式和结构
- 使用 `<details>` 标签替代自定义折叠逻辑
- 添加参数（PARAMETERS）部分显示
- 添加响应（RESPONSE）部分显示
- 添加错误（ERROR）部分显示
- 统一图标、状态徽章和边框颜色

**主要改进**:
- **参数显示**: 使用 `formatParams` 解析并显示工具参数
- **响应显示**: 使用 `formatObservation` 格式化工具执行结果
- **状态管理**: 根据status显示不同颜色和图标
- **折叠控制**: 运行中的工具默认展开，完成的默认折叠

## 数据流程

### 后端数据发送

1. **Planning阶段**:
   ```rust
   // 发送JSON格式的计划
   emit_plan_info_chunk(
       app,
       &execution_id,
       message_id,
       conversation_id,
       &plan_info,  // 包含JSON格式的plan_summary和steps
       Some("rewoo_planning"),
       None,
   );
   ```

2. **Execution阶段**:
   ```rust
   // 发送工具执行开始
   emit_thinking_chunk(
       app,
       &execution_id,
       message_id,
       conversation_id,
       "执行步骤 X/Y: tool_name - description",
       Some("rewoo_execution"),
   );
   
   // 发送工具执行结果
   emit_tool_result_chunk(
       app,
       &execution_id,
       message_id,
       conversation_id,
       &result_str,  // JSON格式的执行结果
       Some("rewoo_execution"),
       Some(&plan_step.tool),
   );
   ```

3. **Solving阶段**:
   ```rust
   // Solver内部通过AI服务流式发送内容
   // engine_adapter发送元数据
   emit_meta_chunk(
       app,
       &execution_id,
       message_id,
       conversation_id,
       &meta_info,
       None,
   );
   ```

### 前端数据解析

1. **Planning解析**:
   - 从 `PlanInfo` chunks中提取JSON计划
   - 解析 `plan_summary` 和 `steps`
   - 提取每个步骤的 `id`、`tool`、`description`、`args`

2. **Execution解析**:
   - 从 `rewoo_execution` stage的chunks中提取
   - 按 `tool_name` 分组
   - 合并 `Thinking`、`ToolResult`、`Error` 类型的chunks
   - 从Planning数据中关联参数

3. **Solving解析**:
   - 从 `rewoo_solving` stage的chunks中提取
   - 合并所有内容生成最终答案

## 显示效果

### 规划阶段
- 显示计划摘要（plan_summary）
- 列出所有执行步骤，包括步骤ID、工具名称、描述

### 执行阶段
- 每个工具调用显示为一个折叠面板
- **状态指示**: 图标和边框颜色反映执行状态
  - 🟢 成功: 绿色边框 + ✓ 图标
  - 🔴 失败: 红色边框 + ✗ 图标
  - 🟡 运行中: 黄色边框 + 旋转图标
- **参数部分**: 显示工具的所有输入参数
- **响应部分**: 显示工具的执行结果
  - 成功: 绿色背景
  - 错误: 红色背景
- **错误部分**: 如果有错误，单独显示

### 求解阶段
- 使用Markdown渲染最终答案
- 显示元数据信息（执行时间、步骤数等）

## 类型定义

```typescript
// 计划数据
export interface ReWOOPlanningData {
  summary: string
  steps?: Array<{
    id: string
    tool: string
    description: string
    args?: any
  }>
}

// 执行步骤
export interface ReWOOExecutionStep {
  toolName: string
  args?: any
  thinking?: string
  result?: any
  error?: string
  status?: 'running' | 'success' | 'failed'
}

// 求解数据
export interface ReWOOSolvingData {
  answer: string
  meta?: string
}

// 完整消息
export interface ReWOOMessageData {
  planningData?: ReWOOPlanningData
  executionSteps?: ReWOOExecutionStep[]
  solvingData?: ReWOOSolvingData
}
```

## 测试要点

1. **计划显示**: 验证JSON计划的summary和steps正确显示
2. **参数显示**: 验证每个工具的参数正确显示（包括复杂对象）
3. **结果显示**: 验证工具执行结果正确显示和格式化
4. **状态显示**: 验证不同状态（running/success/failed）的视觉反馈
5. **折叠交互**: 验证折叠面板的展开/折叠交互
6. **错误处理**: 验证工具执行失败时的错误信息显示
7. **Markdown渲染**: 验证最终答案的Markdown格式正确渲染

## 注意事项

1. **不再兼容旧格式**: 移除了对旧式 `Plan: ... #E1 = tool[...]` 格式的支持
2. **参数来源**: 工具参数从计划阶段获取，而不是执行阶段
3. **工具匹配**: 通过工具名称（toolName）将计划和执行关联
4. **JSON解析**: 计划和结果都使用JSON格式，确保数据结构的一致性

