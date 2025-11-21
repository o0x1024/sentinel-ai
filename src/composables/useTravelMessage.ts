import type { ChatMessage } from '../types/chat'
import type { OrderedMessageChunk } from '../types/ordered-chat'

export interface TravelMessageData {
    taskComplexity?: string
    oodaCycles?: any[]
    metrics?: any
}

export const isTravelMessage = (content: string, chunks: OrderedMessageChunk[]): boolean => {
    // Check if any chunks have architecture: 'Travel'
    const hasTravelArch = chunks.some(chunk => 
        chunk.architecture === 'Travel'
    )
    if (hasTravelArch) return true

    // Check for Travel-specific stages in chunks
    const travelStages = ['Observe', 'Orient', 'Decide', 'Act']
    const hasTravelStage = chunks.some(chunk => 
        chunk.stage && travelStages.includes(chunk.stage)
    )
    if (hasTravelStage) return true

    // Check content for Travel patterns
    const travelPatterns = [
        /OODA\s+cycle/i,
        /Observe\s+phase/i,
        /Orient\s+phase/i,
        /Decide\s+phase/i,
        /Act\s+phase/i,
    ]
    return travelPatterns.some(pattern => pattern.test(content))
}

export const parseTravelMessage = (content: string, chunks: OrderedMessageChunk[]): TravelMessageData => {
    const data: TravelMessageData = {
        oodaCycles: [],
        metrics: {
            total_cycles: 0,
            total_tool_calls: 0,
            total_duration_ms: 0,
            guardrail_checks: 0,
            guardrail_failures: 0,
            rollback_count: 0,
            threat_intel_queries: 0,
        }
    }

    // Extract data from chunks with Travel architecture or Travel-related stages
    const travelChunks = chunks.filter(c => 
        c.architecture === 'Travel' || 
        (c.stage && ['Observe', 'Orient', 'Decide', 'Act'].includes(c.stage))
    )
    
    if (travelChunks.length === 0) {
        return data
    }

    console.log('[parseTravelMessage] Processing', travelChunks.length, 'chunks')

    // Group chunks by cycle (based on stage progression)
    const cyclesMap = new Map<number, any>()
    let currentCycleNum = 1

    for (const chunk of travelChunks) {
        console.log('[parseTravelMessage] Processing chunk:', {
            type: chunk.chunk_type,
            stage: chunk.stage,
            tool_name: chunk.tool_name,
            sequence: chunk.sequence
        })
        // Get or create cycle
        if (!cyclesMap.has(currentCycleNum)) {
            cyclesMap.set(currentCycleNum, {
                cycle_number: currentCycleNum,
                phase_history: [],
                status: 'Running',
                started_at: chunk.timestamp || new Date().toISOString()
            })
        }

        const cycle = cyclesMap.get(currentCycleNum)!
        const stage = chunk.stage

        // Parse structured_data if available
        let structuredData: any = null
        if (chunk.structured_data) {
            try {
                structuredData = typeof chunk.structured_data === 'string' 
                    ? JSON.parse(chunk.structured_data) 
                    : chunk.structured_data
            } catch (e) {
                console.warn('Failed to parse structured_data:', e)
            }
        }

        // Handle phase information
        if (stage && ['Observe', 'Orient', 'Decide', 'Act'].includes(stage)) {
            let existingPhase = cycle.phase_history.find((p: any) => p.phase === stage)
            
            if (!existingPhase) {
                // New phase
                const phaseExec: any = {
                    phase: stage,
                    status: structuredData?.status === 'started' ? 'Running' : 
                            structuredData?.status === 'completed' ? 'Completed' : 
                            structuredData?.status === 'error' ? 'Failed' : 'Running',
                    started_at: chunk.timestamp || new Date().toISOString(),
                    input: {},
                    guardrail_checks: [],
                    tool_calls: [],
                    thinking: [],  // 思考过程
                    content: []    // 内容输出
                }

                if (structuredData?.output) {
                    phaseExec.output = structuredData.output
                }
                if (structuredData?.error) {
                    phaseExec.error = structuredData.error
                }

                cycle.phase_history.push(phaseExec)
                existingPhase = phaseExec
            } else {
                // Update existing phase
                if (structuredData?.status === 'completed') {
                    existingPhase.status = 'Completed'
                    existingPhase.completed_at = chunk.timestamp
                    if (structuredData.output) {
                        existingPhase.output = structuredData.output
                    }
                } else if (structuredData?.status === 'error') {
                    existingPhase.status = 'Failed'
                    existingPhase.error = structuredData.error
                }
            }

            // Handle Thinking chunks
            if (chunk.chunk_type === 'Thinking' && chunk.content) {
                existingPhase.thinking = existingPhase.thinking || []
                existingPhase.thinking.push({
                    content: chunk.content.toString(),
                    timestamp: chunk.timestamp
                })
            }

            // Handle Content chunks
            if (chunk.chunk_type === 'Content' && chunk.content) {
                existingPhase.content = existingPhase.content || []
                existingPhase.content.push({
                    content: chunk.content.toString(),
                    timestamp: chunk.timestamp
                })
            }
        }

        // Handle tool results
        if (chunk.chunk_type === 'ToolResult') {
            // 如果有 stage，添加到对应阶段；否则添加到最近的阶段
            const targetPhase = stage 
                ? cycle.phase_history.find((p: any) => p.phase === stage)
                : cycle.phase_history[cycle.phase_history.length - 1]
                
            if (targetPhase) {
                try {
                    let toolResult: any = null
                    let toolName = chunk.tool_name || structuredData?.tool_name || 'unknown'
                    let toolArgs: any = structuredData?.arguments || null
                    let toolOutput: any = structuredData?.result || null
                    let success = true

                    // 优先使用 structured_data 中的信息，其次解析 content
                    if (!toolArgs || !toolOutput) {
                        try {
                            toolResult = typeof chunk.content === 'string' 
                                ? JSON.parse(chunk.content) 
                                : chunk.content
                            
                            // 提取工具名称
                            if (toolResult.tool_name) toolName = toolResult.tool_name
                            if (toolResult.name) toolName = toolResult.name
                            
                            // 提取参数
                            if (!toolArgs) {
                                if (toolResult.args) toolArgs = toolResult.args
                                if (toolResult.arguments) toolArgs = toolResult.arguments
                                if (toolResult.input) toolArgs = toolResult.input
                            }
                            
                            // 提取结果
                            if (!toolOutput) {
                                if (toolResult.result !== undefined) toolOutput = toolResult.result
                                else if (toolResult.output !== undefined) toolOutput = toolResult.output
                                else toolOutput = toolResult
                            }
                            
                            // 检查执行状态
                            if (toolResult.success === false || toolResult.error) {
                                success = false
                            }
                        } catch (parseError) {
                            // 解析失败，使用原始内容
                            toolOutput = toolOutput || chunk.content
                        }
                    }
                    
                    targetPhase.tool_calls = targetPhase.tool_calls || []
                    targetPhase.tool_calls.push({
                        call_id: toolResult?.call_id || `tool_${Date.now()}`,
                        tool_name: toolName,
                        args: toolArgs,
                        status: success ? 'Completed' : 'Failed',
                        result: toolOutput,
                        error: toolResult?.error || structuredData?.error,
                        called_at: chunk.timestamp || new Date().toISOString()
                    })
                } catch (e) {
                    console.warn('[parseTravelMessage] Failed to parse tool result:', e, chunk)
                }
            }
        }

        // Detect cycle completion (when Act phase completes, move to next cycle)
        if (stage === 'Act' && structuredData?.status === 'completed') {
            cycle.status = 'Completed'
            cycle.completed_at = chunk.timestamp
            currentCycleNum++
        }
    }

    // Convert map to array
    data.oodaCycles = Array.from(cyclesMap.values())
    data.metrics!.total_cycles = data.oodaCycles.length

    // Calculate metrics from cycles
    for (const cycle of data.oodaCycles) {
        for (const phase of cycle.phase_history || []) {
            if (phase.tool_calls) {
                data.metrics!.total_tool_calls += phase.tool_calls.length
            }
            if (phase.guardrail_checks) {
                data.metrics!.guardrail_checks += phase.guardrail_checks.length
                data.metrics!.guardrail_failures += phase.guardrail_checks.filter(
                    (c: any) => c.result === 'Failed'
                ).length
            }
        }
    }

    return data
}


