/**
 * LLM Compiler消息解析 Composable
 * 用于检测和解析LLM Compiler架构的消息
 */

export interface LLMCompilerPlanningTask {
  id: string
  name: string
  description: string
  tool: string
  inputs?: any
  dependencies?: string[]
  reason?: string
}

export interface LLMCompilerPlanningData {
  summary: string
  tasks?: LLMCompilerPlanningTask[]
  strategy?: string
}

export interface LLMCompilerExecutionTask {
  task_id: string
  name?: string
  tool: string
  inputs?: any
  dependencies?: string[]
  result?: any
  error?: string
  status?: 'Pending' | 'Ready' | 'Running' | 'Completed' | 'Failed' | 'Cancelled' | 'Retrying'
  duration_ms?: number
}

export interface LLMCompilerExecutionRound {
  round: number
  tasks: LLMCompilerExecutionTask[]
  duration_ms?: number
}

export interface LLMCompilerExecutionData {
  rounds: LLMCompilerExecutionRound[]
}

export interface LLMCompilerJoinerData {
  decision?: 'complete' | 'continue'
  response?: string
  feedback?: string
  meta?: string
}

export interface LLMCompilerSummaryData {
  total_tasks: number
  successful_tasks: number
  failed_tasks: number
  total_duration_ms: number
  replanning_count: number
}

export interface LLMCompilerMessageData {
  planningData?: LLMCompilerPlanningData
  executionData?: LLMCompilerExecutionData
  joinerData?: LLMCompilerJoinerData
  summaryData?: LLMCompilerSummaryData
}

/**
 * 检测是否为LLM Compiler消息
 */
export function isLLMCompilerMessage(content: string, chunks: any[] = []): boolean {
  // 检查architecture字段（最可靠的方式）
  const hasLLMCompilerArch = chunks.some(c => c.architecture === 'LLMCompiler')
  if (hasLLMCompilerArch) return true

  // 检查stage或tool_name标记（后端meta消息用tool_name传递stage信息）
  const llmCompilerStages = [
    'llm_compiler_planning',
    'llm_compiler_execution',
    'llm_compiler_joiner',
    'llm_compiler_summary',
    'llm_compiler'
  ]
  const hasLLMCompilerStage = chunks.some(c =>
    llmCompilerStages.includes(c.stage) || llmCompilerStages.includes(c.tool_name)
  )

  if (hasLLMCompilerStage) return true

  // 检查内容特征
  const llmCompilerPatterns = [
    /DAG规划成功/,
    /DAG执行计划/,
    /任务节点/,
    /并行执行/,
    /轮次.*完成/,
    /"tasks"\s*:\s*\[/,
    /task_id/,
    /execution_strategy/,
    /plan_summary/,
    /\[THINKING\][\s\S]*\[DECISION\]/, // LLM Compiler的Joiner输出格式
    /"decision"\s*:\s*"COMPLETE"/i,     // Joiner决策JSON格式
  ]

  return llmCompilerPatterns.some(pattern => pattern.test(content))
}

/**
 * 解析Planning阶段的消息
 */
function parsePlanningData(content: string, chunks: any[] = []): LLMCompilerPlanningData | null {
  // 从PlanInfo块解析
  const planInfoChunks = chunks.filter(c => 
    c.chunk_type === 'PlanInfo' || 
    c.stage === 'llm_compiler_planning'
  )
  
  for (const chunk of planInfoChunks) {
    // 优先使用structured_data（后端直接传递的JSON对象）
    let json: any = null
    if (chunk.structured_data && typeof chunk.structured_data === 'object') {
      json = chunk.structured_data
    } else {
      // 回退到解析content字符串
      const text = (chunk?.content ?? '').toString().trim()
      if (!text) continue
      try {
        json = JSON.parse(text)
      } catch {
        // 文本格式解析："DAG规划成功，共N个任务节点"
        const taskCountMatch = text.match(/共(\d+)个任务节点/)
        if (taskCountMatch) {
          return {
            summary: text,
            tasks: undefined,
            strategy: undefined
          }
        }
        continue
      }
    }
    
    if (json && typeof json === 'object') {
      // 检查是否包含tasks数组
      if (Array.isArray(json.tasks)) {
        const tasks = json.tasks.map((t: any) => ({
          id: String(t?.id ?? ''),
          name: String(t?.name ?? ''),
          description: String(t?.description ?? ''),
          tool: String(t?.tool ?? ''),
          inputs: t?.inputs,
          dependencies: Array.isArray(t?.dependencies) ? t.dependencies : [],
          reason: t?.reason ? String(t.reason) : undefined
        })).filter((t: any) => t.id && t.tool)
        
        return {
          summary: String(json.plan_summary ?? json.summary ?? `DAG规划成功，共${tasks.length}个任务节点`),
          tasks: tasks.length ? tasks : undefined,
          strategy: json.execution_strategy ? String(json.execution_strategy) : undefined
        }
      }
    }
  }
  
  // 从content中解析
  try {
    const json = JSON.parse(content)
    if (json && typeof json === 'object' && Array.isArray(json.tasks)) {
      const tasks = json.tasks.map((t: any) => ({
        id: String(t?.id ?? ''),
        name: String(t?.name ?? ''),
        description: String(t?.description ?? ''),
        tool: String(t?.tool ?? ''),
        inputs: t?.inputs,
        dependencies: Array.isArray(t?.dependencies) ? t.dependencies : [],
        reason: t?.reason ? String(t.reason) : undefined
      })).filter((t: any) => t.id && t.tool)
      
      return {
        summary: String(json.plan_summary ?? json.summary ?? `DAG规划成功，共${tasks.length}个任务节点`),
        tasks: tasks.length ? tasks : undefined,
        strategy: json.execution_strategy ? String(json.execution_strategy) : undefined
      }
    }
  } catch {
    // ignore
  }
  
  return null
}

/**
 * 解析Execution阶段的消息
 */
function parseExecutionData(content: string, chunks: any[] = [], planningData?: LLMCompilerPlanningData | null): LLMCompilerExecutionData | null {
  // Filter execution-related chunks
  const executionChunks = chunks.filter(c =>
    c.stage === 'llm_compiler_execution' ||
    (c.stage === 'llm_compiler' && c.chunk_type === 'ToolResult') ||
    c.chunk_type === 'ToolResult'
  )

  if (executionChunks.length === 0) return null

  // 按轮次组织任务
  const roundsMap = new Map<number, Map<string, LLMCompilerExecutionTask>>()

  for (const chunk of executionChunks) {
    const chunkType = chunk.chunk_type

    // 从ToolResult块中提取任务信息
    if (chunkType === 'ToolResult') {
      try {
        // 优先使用structured_data，否则解析content
        let resultData: any
        if (chunk.structured_data && typeof chunk.structured_data === 'object') {
          resultData = chunk.structured_data
        } else {
          resultData = typeof chunk.content === 'string'
            ? JSON.parse(chunk.content)
            : chunk.content
        }

        if (resultData && resultData.task_id) {
          const taskId = resultData.task_id
          const round = resultData.round ?? 1

          if (!roundsMap.has(round)) {
            roundsMap.set(round, new Map())
          }

          const roundTasks = roundsMap.get(round)!

          if (!roundTasks.has(taskId)) {
            // 从计划数据中获取任务信息
            const planTask = planningData?.tasks?.find(t => t.id === taskId)

            roundTasks.set(taskId, {
              task_id: taskId,
              name: planTask?.name,
              tool: resultData.tool_name ?? planTask?.tool ?? 'unknown',
              inputs: planTask?.inputs ?? resultData.inputs,
              dependencies: planTask?.dependencies,
              status: 'Completed',
              duration_ms: resultData.duration_ms
            })
          }

          const task = roundTasks.get(taskId)!
          task.result = resultData.result ?? resultData.output ?? resultData
          task.error = resultData.error
          // Parse status: handle both string "Completed" and "Failed" formats
          const status = resultData.status?.toString() || ''
          if (status.includes('Failed') || status.includes('failed')) {
            task.status = 'Failed'
          } else if (status.includes('Completed') || status.includes('completed')) {
            task.status = 'Completed'
          } else if (resultData.error) {
            task.status = 'Failed'
          } else {
            task.status = 'Completed'
          }
        }
      } catch (e) {
        console.warn('Failed to parse ToolResult chunk:', e)
      }
    }

    // 从Error块中提取错误信息
    if (chunkType === 'Error') {
      const errorText = chunk.content?.toString() || ''
      // 尝试从错误信息中提取task_id
      const taskIdMatch = errorText.match(/task[_\s]?id[:\s]+["']?(\w+)["']?/i)
      if (taskIdMatch) {
        const taskId = taskIdMatch[1]
        // 假设错误发生在第1轮
        const round = 1

        if (!roundsMap.has(round)) {
          roundsMap.set(round, new Map())
        }

        const roundTasks = roundsMap.get(round)!

        if (!roundTasks.has(taskId)) {
          const planTask = planningData?.tasks?.find(t => t.id === taskId)
          roundTasks.set(taskId, {
            task_id: taskId,
            name: planTask?.name,
            tool: planTask?.tool ?? 'unknown',
            inputs: planTask?.inputs,
            dependencies: planTask?.dependencies,
            status: 'Failed'
          })
        }

        const task = roundTasks.get(taskId)!
        task.error = errorText
        task.status = 'Failed'
      }
    }
  }

  // 转换为数组格式
  if (roundsMap.size === 0) return null

  const rounds: LLMCompilerExecutionRound[] = []
  const sortedRounds = Array.from(roundsMap.keys()).sort((a, b) => a - b)

  for (const roundNum of sortedRounds) {
    const tasks = Array.from(roundsMap.get(roundNum)!.values())
    if (tasks.length > 0) {
      rounds.push({
        round: roundNum,
        tasks,
        duration_ms: tasks.reduce((sum, t) => sum + (t.duration_ms ?? 0), 0)
      })
    }
  }

  return rounds.length > 0 ? { rounds } : null
}

/**
 * 解析Joiner阶段的消息
 */
function parseJoinerData(content: string, chunks: any[] = []): LLMCompilerJoinerData | null {
  // 同时检查stage和tool_name字段（后端meta消息用tool_name传递stage信息）
  const joinerChunks = chunks.filter(c =>
    c.stage === 'llm_compiler_joiner' || c.tool_name === 'llm_compiler_joiner'
  )

  if (joinerChunks.length === 0) {
    // 从content中检测
    if (content.includes('决策:') || content.includes('Joiner')) {
      const isComplete = content.includes('完成执行') || content.includes('complete')
      const isContinue = content.includes('继续执行') || content.includes('continue')

      return {
        decision: isComplete ? 'complete' : isContinue ? 'continue' : undefined,
        response: isComplete ? content : undefined,
        feedback: isContinue ? content : undefined
      }
    }
    return null
  }

  const joinerData: LLMCompilerJoinerData = {}

  for (const chunk of joinerChunks) {
    const text = chunk.content?.toString() || ''
    
    // 优先使用structured_data
    let json: any = null
    if (chunk.structured_data && typeof chunk.structured_data === 'object') {
      json = chunk.structured_data
    } else if (text) {
      try {
        json = JSON.parse(text)
      } catch {
        // Not JSON, will try text parsing later
      }
    }

    if (json && typeof json === 'object') {
      if (json.decision === 'complete') {
        joinerData.decision = 'complete'
        joinerData.response = json.response
        joinerData.meta = json.meta
      } else if (json.decision === 'continue') {
        joinerData.decision = 'continue'
        joinerData.feedback = json.feedback
        joinerData.meta = json.meta
      }
      continue
    }

    if (chunk.chunk_type === 'Meta') {
      joinerData.meta = text

      // 从meta中提取决策
      if (text.includes('完成执行')) {
        joinerData.decision = 'complete'
        joinerData.response = text
      } else if (text.includes('继续执行')) {
        joinerData.decision = 'continue'
        joinerData.feedback = text
      }
    } else if (chunk.chunk_type === 'Thinking') {
      if (!joinerData.meta) {
        joinerData.meta = text
      }
    }
  }

  return Object.keys(joinerData).length > 0 ? joinerData : null
}

/**
 * 解析Summary数据
 */
function parseSummaryData(content: string, chunks: any[] = []): LLMCompilerSummaryData | null {
  // First, try to find summary chunk from llm_compiler_summary stage (check both stage and tool_name)
  const summaryChunks = chunks.filter(c =>
    c.stage === 'llm_compiler_summary' || c.tool_name === 'llm_compiler_summary'
  )
  for (const chunk of summaryChunks) {
    // 优先使用structured_data
    let json: any = null
    if (chunk.structured_data && typeof chunk.structured_data === 'object') {
      json = chunk.structured_data
    } else {
      try {
        const text = chunk.content?.toString() || ''
        json = JSON.parse(text)
      } catch {
        continue
      }
    }
    
    if (json && typeof json === 'object' && 'total_tasks' in json) {
      return {
        total_tasks: Number(json.total_tasks ?? 0),
        successful_tasks: Number(json.successful_tasks ?? 0),
        failed_tasks: Number(json.failed_tasks ?? 0),
        total_duration_ms: Number(json.total_duration_ms ?? 0),
        replanning_count: Number(json.replanning_count ?? 0)
      }
    }
  }

  // 尝试从content中解析JSON格式的摘要
  try {
    const json = JSON.parse(content)
    if (json && typeof json === 'object' && 'total_tasks' in json) {
      return {
        total_tasks: Number(json.total_tasks ?? 0),
        successful_tasks: Number(json.successful_tasks ?? 0),
        failed_tasks: Number(json.failed_tasks ?? 0),
        total_duration_ms: Number(json.total_duration_ms ?? 0),
        replanning_count: Number(json.replanning_count ?? 0)
      }
    }
  } catch {
    // ignore
  }

  // 从文本中提取摘要信息
  const totalMatch = content.match(/总.*?(\d+).*?任务/)
  const successMatch = content.match(/成功.*?(\d+)/)
  const failedMatch = content.match(/失败.*?(\d+)/)
  const durationMatch = content.match(/耗时.*?(\d+)\s*ms/)
  const replanMatch = content.match(/重规划.*?(\d+)/)

  if (totalMatch || successMatch || failedMatch) {
    return {
      total_tasks: totalMatch ? Number(totalMatch[1]) : 0,
      successful_tasks: successMatch ? Number(successMatch[1]) : 0,
      failed_tasks: failedMatch ? Number(failedMatch[1]) : 0,
      total_duration_ms: durationMatch ? Number(durationMatch[1]) : 0,
      replanning_count: replanMatch ? Number(replanMatch[1]) : 0
    }
  }

  return null
}

/**
 * 解析完整的LLM Compiler消息
 */
export function parseLLMCompilerMessage(content: string, chunks: any[] = []): LLMCompilerMessageData {
  const result: LLMCompilerMessageData = {}

  // 检查各个阶段（同时检查stage和tool_name，后端meta消息用tool_name传递stage信息）
  const hasPlanningStage = chunks.some(c =>
    c.stage === 'llm_compiler_planning' ||
    c.tool_name === 'llm_compiler_planning' ||
    c.chunk_type === 'PlanInfo'
  ) || content.includes('DAG规划') || content.includes('任务节点') || content.includes('plan_summary')

  const hasExecutionStage = chunks.some(c =>
    c.stage === 'llm_compiler_execution' ||
    c.tool_name === 'llm_compiler_execution' ||
    (c.stage === 'llm_compiler' && c.chunk_type === 'ToolResult')
  ) || content.includes('并行执行') || content.includes('轮次')

  const hasJoinerStage = chunks.some(c =>
    c.stage === 'llm_compiler_joiner' || c.tool_name === 'llm_compiler_joiner'
  ) || content.includes('Joiner') || content.includes('决策')

  const hasSummaryStage = chunks.some(c =>
    c.stage === 'llm_compiler_summary' || c.tool_name === 'llm_compiler_summary'
  )

  // 解析Planning阶段
  let planningData: LLMCompilerPlanningData | null = null
  if (hasPlanningStage) {
    planningData = parsePlanningData(content, chunks)
    if (planningData) {
      result.planningData = planningData
    }
  }

  // 解析Execution阶段
  if (hasExecutionStage) {
    const executionData = parseExecutionData(content, chunks, planningData)
    if (executionData) {
      result.executionData = executionData
    }
  }

  // 解析Joiner阶段
  if (hasJoinerStage) {
    const joinerData = parseJoinerData(content, chunks)
    if (joinerData) {
      result.joinerData = joinerData
    }
  }

  // 解析Summary数据
  if (hasSummaryStage || content.includes('total_tasks')) {
    const summaryData = parseSummaryData(content, chunks)
    if (summaryData) {
      result.summaryData = summaryData
    }
  }

  return result
}

/**
 * Composable函数
 */
export function useLLMCompilerMessage() {
  return {
    isLLMCompilerMessage,
    parseLLMCompilerMessage
  }
}

