# Travel Architecture Messaging Flow

## 概述

Travel架构基于OODA循环(Observe-Orient-Decide-Act)，通过结构化消息在前后端之间传递执行状态和结果。

## 后端消息发送

### 消息结构

Travel引擎通过 `emit_message_chunk_arc` 函数发送消息，消息包含以下字段：

```rust
emit_message_chunk_arc(
    app_handle,           // Tauri应用句柄
    execution_id,         // 执行ID
    message_id,           // 消息ID
    conversation_id,      // 会话ID(可选)
    chunk_type,           // 块类型: Thinking, Content, ToolResult, Error
    content,              // 文本内容
    is_final,             // 是否为最终块
    stage,                // OODA阶段: "Observe", "Orient", "Decide", "Act"
    tool_name,            // 工具名称(可选)
    architecture,         // 架构类型: ArchitectureType::Travel
    structured_data       // 结构化数据(JSON)
)
```

### OODA阶段消息

#### 1. Observe阶段 (观察)

**阶段开始:**
```rust
stage: "Observe"
chunk_type: ChunkType::Thinking
content: "🔄 Starting Observe phase..."
structured_data: {
    "phase": "Observe",
    "status": "started"
}
```

**阶段完成:**
```rust
stage: "Observe"
chunk_type: ChunkType::Thinking
content: "✅ Observe phase completed"
structured_data: {
    "phase": "Observe",
    "status": "completed",
    "output": {
        "observations": {...}
    }
}
```

#### 2. Orient阶段 (分析定位)

**阶段开始:**
```rust
stage: "Orient"
chunk_type: ChunkType::Thinking
content: "🔄 Starting Orient phase..."
structured_data: {
    "phase": "Orient",
    "status": "started"
}
```

**阶段完成:**
```rust
stage: "Orient"
chunk_type: ChunkType::Thinking
content: "✅ Orient phase completed"
structured_data: {
    "phase": "Orient",
    "status": "completed",
    "output": {
        "threat_level": "Medium",
        "threats": [...],
        "vulnerabilities": [...]
    }
}
```

#### 3. Decide阶段 (决策)

**阶段开始:**
```rust
stage: "Decide"
chunk_type: ChunkType::Thinking
content: "🔄 Starting Decide phase..."
structured_data: {
    "phase": "Decide",
    "status": "started"
}
```

**阶段完成:**
```rust
stage: "Decide"
chunk_type: ChunkType::Thinking
content: "✅ Decide phase completed"
structured_data: {
    "phase": "Decide",
    "status": "completed",
    "output": {
        "name": "Action Plan",
        "steps": [...],
        "estimated_duration": 300
    }
}
```

#### 4. Act阶段 (执行)

**阶段开始:**
```rust
stage: "Act"
chunk_type: ChunkType::Content
content: "📊 Dispatching with complexity: Medium"
```

**工具调用结果:**
```rust
stage: "Act"
chunk_type: ChunkType::ToolResult
tool_name: "tool_name"
content: JSON.stringify({
    "call_id": "...",
    "tool_name": "...",
    "result": {...}
})
```

**阶段完成:**
```rust
stage: "Act"
chunk_type: ChunkType::Thinking
content: "✅ Act phase completed"
structured_data: {
    "phase": "Act",
    "status": "completed",
    "output": {
        "execution_result": {...}
    }
}
```

### 错误处理

当任何阶段发生错误时：

```rust
stage: "阶段名称"
chunk_type: ChunkType::Error
content: "❌ 阶段名称 phase error: 错误信息"
structured_data: {
    "phase": "阶段名称",
    "status": "error",
    "error": "详细错误信息"
}
```

## 前端消息解析

### useTravelMessage.ts

#### isTravelMessage

检测消息是否为Travel架构消息：

```typescript
export const isTravelMessage = (content: string, chunks: OrderedMessageChunk[]): boolean => {
    // 检查chunks是否有architecture: 'Travel'
    const hasTravelArch = chunks.some(chunk => chunk.architecture === 'Travel')
    if (hasTravelArch) return true

    // 检查是否有Travel特定的stage
    const travelStages = ['Observe', 'Orient', 'Decide', 'Act']
    const hasTravelStage = chunks.some(chunk => 
        chunk.stage && travelStages.includes(chunk.stage)
    )
    if (hasTravelStage) return true

    // 检查内容模式
    const travelPatterns = [
        /OODA\\s+cycle/i,
        /Observe\\s+phase/i,
        /Orient\\s+phase/i,
        /Decide\\s+phase/i,
        /Act\\s+phase/i,
    ]
    return travelPatterns.some(pattern => pattern.test(content))
}
```

#### parseTravelMessage

解析Travel消息数据：

```typescript
export const parseTravelMessage = (content: string, chunks: OrderedMessageChunk[]): TravelMessageData => {
    const data: TravelMessageData = {
        oodaCycles: [],
        metrics: { ... }
    }

    // 1. 过滤Travel架构的chunks
    const travelChunks = chunks.filter(c => c.architecture === 'Travel')
    
    // 2. 按cycle分组处理
    const cyclesMap = new Map<number, any>()
    let currentCycleNum = 1

    for (const chunk of travelChunks) {
        // 获取或创建cycle
        if (!cyclesMap.has(currentCycleNum)) {
            cyclesMap.set(currentCycleNum, {
                cycle_number: currentCycleNum,
                phase_history: [],
                status: 'Running',
                started_at: chunk.timestamp
            })
        }

        const cycle = cyclesMap.get(currentCycleNum)!
        const stage = chunk.stage // "Observe", "Orient", "Decide", "Act"

        // 3. 解析structured_data
        let structuredData = null
        if (chunk.structured_data) {
            structuredData = typeof chunk.structured_data === 'string' 
                ? JSON.parse(chunk.structured_data) 
                : chunk.structured_data
        }

        // 4. 处理阶段信息
        if (stage && ['Observe', 'Orient', 'Decide', 'Act'].includes(stage)) {
            const existingPhase = cycle.phase_history.find(p => p.phase === stage)
            
            if (!existingPhase) {
                // 创建新阶段
                const phaseExec = {
                    phase: stage,
                    status: structuredData?.status === 'started' ? 'Running' : 
                            structuredData?.status === 'completed' ? 'Completed' : 
                            structuredData?.status === 'error' ? 'Failed' : 'Pending',
                    started_at: chunk.timestamp,
                    input: {},
                    output: structuredData?.output,
                    error: structuredData?.error,
                    guardrail_checks: [],
                    tool_calls: []
                }
                cycle.phase_history.push(phaseExec)
            } else {
                // 更新现有阶段
                if (structuredData?.status === 'completed') {
                    existingPhase.status = 'Completed'
                    existingPhase.completed_at = chunk.timestamp
                    if (structuredData.output) {
                        existingPhase.output = structuredData.output
                    }
                }
            }
        }

        // 5. 处理工具调用结果
        if (chunk.chunk_type === 'ToolResult' && stage) {
            const phaseExec = cycle.phase_history.find(p => p.phase === stage)
            if (phaseExec) {
                const toolResult = typeof chunk.content === 'string' 
                    ? JSON.parse(chunk.content) 
                    : chunk.content
                
                phaseExec.tool_calls.push({
                    call_id: toolResult.call_id || Date.now().toString(),
                    tool_name: chunk.tool_name || toolResult.tool_name,
                    status: 'Completed',
                    result: toolResult.result || toolResult,
                    called_at: chunk.timestamp
                })
            }
        }

        // 6. 检测cycle完成 (Act阶段完成后进入下一个cycle)
        if (stage === 'Act' && structuredData?.status === 'completed') {
            cycle.status = 'Completed'
            cycle.completed_at = chunk.timestamp
            currentCycleNum++
        }
    }

    // 7. 转换为数组并计算指标
    data.oodaCycles = Array.from(cyclesMap.values())
    data.metrics.total_cycles = data.oodaCycles.length
    
    // 统计工具调用和护栏检查
    for (const cycle of data.oodaCycles) {
        for (const phase of cycle.phase_history || []) {
            if (phase.tool_calls) {
                data.metrics.total_tool_calls += phase.tool_calls.length
            }
            if (phase.guardrail_checks) {
                data.metrics.guardrail_checks += phase.guardrail_checks.length
                data.metrics.guardrail_failures += phase.guardrail_checks.filter(
                    c => c.result === 'Failed'
                ).length
            }
        }
    }

    return data
}
```

### TravelStepDisplay.vue

组件接收解析后的数据并展示：

```vue
<template>
  <div class="travel-step-display">
    <!-- 任务复杂度标识 -->
    <div v-if="taskComplexity" class="complexity-badge">
      <div class="badge" :class="getComplexityClass(taskComplexity)">
        <i :class="getComplexityIcon(taskComplexity)"></i>
        {{ getComplexityText(taskComplexity) }}
      </div>
    </div>

    <!-- OODA 循环列表 -->
    <div v-for="cycle in oodaCycles" :key="cycle.id || cycleIndex" class="ooda-cycle">
      <!-- 循环标题 -->
      <div class="cycle-header">
        <i class="fas fa-sync-alt"></i>
        <span>OODA 循环 #{{ cycle.cycle_number }}</span>
        <span class="badge" :class="getCycleStatusClass(cycle.status)">
          {{ getCycleStatusText(cycle.status) }}
        </span>
      </div>

      <!-- OODA 阶段 -->
      <div v-for="phaseExec in cycle.phase_history" :key="phaseIndex">
        <details class="collapse">
          <summary>
            <!-- 阶段图标和名称 -->
            <i :class="getPhaseIcon(phaseExec.phase)"></i>
            {{ getPhaseText(phaseExec.phase) }}
            <span class="badge" :class="getPhaseStatusClass(phaseExec.status)">
              {{ getPhaseStatusText(phaseExec.status) }}
            </span>
          </summary>
          
          <div class="collapse-content">
            <!-- 护栏检查 -->
            <div v-if="phaseExec.guardrail_checks?.length">
              ...
            </div>

            <!-- 工具调用 -->
            <div v-if="phaseExec.tool_calls?.length">
              ...
            </div>

            <!-- 阶段输出 -->
            <div v-if="phaseExec.output">
              <pre>{{ formatOutput(phaseExec.output, phaseExec.phase) }}</pre>
            </div>

            <!-- 错误信息 -->
            <div v-if="phaseExec.error">
              <div class="alert alert-error">{{ phaseExec.error }}</div>
            </div>
          </div>
        </details>
      </div>
    </div>

    <!-- 执行指标 -->
    <div v-if="metrics" class="metrics-summary">
      <div>循环次数: {{ metrics.total_cycles }}</div>
      <div>工具调用: {{ metrics.total_tool_calls }}</div>
      <div>护栏检查: {{ metrics.guardrail_checks }}</div>
      ...
    </div>
  </div>
</template>
```

## 数据流图

```
Backend (Rust)                          Frontend (TypeScript)
━━━━━━━━━━━━━━                          ━━━━━━━━━━━━━━━━━━━━━

TravelEngine
    ↓
OodaExecutor
    ↓
emit_message_chunk_arc()                OrderedMessageChunk[]
    ├─ architecture: "Travel"               ↓
    ├─ stage: "Observe|Orient|..."      isTravelMessage()
    ├─ chunk_type: Thinking|Content         ↓
    ├─ structured_data: {               parseTravelMessage()
    │    phase: "...",                      ↓
    │    status: "started|completed",   TravelMessageData {
    │    output: {...}                      oodaCycles: [{
    └─}                                         cycle_number: 1,
                                                phase_history: [{
EngineDispatcher                                    phase: "Observe",
    ↓                                               status: "Completed",
emit_message_chunk_arc()                            output: {...},
    ├─ stage: "Act"                                 tool_calls: [...],
    ├─ chunk_type: ToolResult                       guardrail_checks: [...]
    └─ tool_name: "..."                         }],
                                                    status: "Completed"
                                                }],
                                                metrics: {...}
                                            }
                                                ↓
                                        TravelStepDisplay.vue
                                                ↓
                                        用户界面显示
```

## 关键点

1. **架构标识**: 所有Travel消息都带有 `architecture: "Travel"` 标识
2. **阶段标识**: 使用 `stage` 字段标识OODA四个阶段
3. **结构化数据**: `structured_data` 包含阶段状态和输出信息
4. **循环检测**: 当Act阶段完成时，表示一个OODA循环结束
5. **增量更新**: 前端通过解析chunks增量构建OODA循环状态
6. **工具调用**: 在Act阶段通过ToolResult类型的chunk传递工具执行结果

## 扩展建议

1. **添加循环间依赖**: 在 `structured_data` 中记录前一个循环的结果
2. **性能指标**: 增加阶段耗时、LLM调用次数等指标
3. **可视化增强**: 添加OODA循环的流程图可视化
4. **实时进度**: 在每个阶段内部添加更细粒度的进度信息
5. **错误恢复**: 记录回退(rollback)操作的详细信息
