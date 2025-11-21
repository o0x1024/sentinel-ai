/**
 * ReWOO消息解析和处理
 */

export interface ReWOOPlanningData {
  summary: string
  steps?: Array<{
    id: string
    tool: string
    description: string
    args?: any
  }>
}

export interface ReWOOExecutionStep {
  toolName: string
  description?: string
  args?: any
  thinking?: string
  result?: any
  error?: string
  status?: 'running' | 'success' | 'failed'
}

export interface ReWOOSolvingData {
  answer: string
  meta?: string
}

export interface ReWOOMessageData {
  planningData?: ReWOOPlanningData
  executionSteps?: ReWOOExecutionStep[]
  solvingData?: ReWOOSolvingData
}

/**
 * 检测消息是否为ReWOO格式
 */
export function isReWOOMessage(content: string, chunks?: any[]): boolean {
  if (!content) return false

  // 检查chunks中是否有明确的ReWOO架构标识
  if (chunks && chunks.length > 0) {
    // 优先检查 architecture 字段
    const hasReWOOArch = chunks.some(chunk => chunk.architecture === 'ReWOO')
    if (hasReWOOArch) return true
    
    // 检查ReWOO特定的stage标识
    const hasReWOOStage = chunks.some(chunk =>
      chunk.stage && (
        chunk.stage.includes('rewoo_planning') ||
        chunk.stage.includes('rewoo_execution') ||
        chunk.stage.includes('rewoo_solving')
      )
    )
    if (hasReWOOStage) return true
  }

  // 检查内容中是否包含ReWOO特征
  const rewooPatterns = [
    /开始生成执行计划/,
    /执行步骤\s*\d+\/\d+/,
    /开始执行工具/,
    /开始生成最终答案/,
    /\"plan_summary\"\s*:/,        // 新的JSON计划格式
    /\"steps\"\s*:\s*\[/,
  ]

  return rewooPatterns.some(pattern => pattern.test(content))
}

/**
 * 解析ReWOO Planning阶段的消息
 */
function parsePlanningData(content: string, chunks: any[] = []): ReWOOPlanningData | null {
  // 1) 优先：从明确标记为 ReWOO 的 PlanInfo 块解析（支持JSON与纯文本两种内容）
  const planInfoChunks = (chunks || []).filter(c => 
    (c.chunk_type === 'PlanInfo' || c.stage === 'rewoo_planning') &&
    // 确保是 ReWOO 架构的 chunk
    (c.architecture === 'ReWOO' || c.stage?.includes('rewoo'))
  )
  console.log('[parsePlanningData] Found PlanInfo chunks:', planInfoChunks.length)
  for (const ch of planInfoChunks) {
    const text = (ch?.content ?? '').toString().trim()
    if (!text) continue
    console.log('[parsePlanningData] Processing chunk content:', text.substring(0, 200))
    // 尝试解析为JSON计划
    try {
      const json = JSON.parse(text)
      if (json && typeof json === 'object' && Array.isArray(json.steps)) {
        const steps = (json.steps as any[]).map((s, idx) => ({
          id: (s?.id ? String(s.id) : `E${idx + 1}`),
          tool: String(s?.tool ?? ''),
          description: String(s?.description ?? ''),
          args: s?.args
        })).filter(s => s.tool)
        console.log('[parsePlanningData] Parsed JSON plan with steps:', steps)
        return {
          summary: String(json.plan_summary ?? '执行计划'),
          steps: steps.length ? steps : undefined
        }
      }
    } catch (e) {
      console.log('[parsePlanningData] JSON parse failed:', e)
      // 不是JSON则继续尝试文本解析
    }
    // 文本计划："生成执行计划成功，共N个步骤：\n1. tool - desc\n2. tool - desc..."
    const lines = text.split('\n').map(l => l.trim()).filter(Boolean)
    if (lines.length) {
      const summaryLine = lines[0]
      const stepRegex = /^\s*(\d+)\.\s*([A-Za-z0-9_\-]+)\s*-\s*(.*)$/
      const steps: Array<{ id: string; tool: string; description: string }> = []
      for (let i = 1; i < lines.length; i++) {
        const m = stepRegex.exec(lines[i])
        if (m) {
          const idx = Number(m[1])
          steps.push({
            id: `E${isNaN(idx) ? (steps.length + 1) : idx}`,
            tool: m[2],
            description: (m[3] || '').trim()
          })
        }
      }
      if (summaryLine || steps.length) {
        return {
          summary: summaryLine || '执行计划',
          steps: steps.length ? steps : undefined
        }
      }
    }
  }

  // 2) 次之：尝试从消息文本中解析JSON计划
  try {
    const json = JSON.parse(content)
    if (json && typeof json === 'object' && Array.isArray(json.steps)) {
      const steps = (json.steps as any[]).map((s, idx) => ({
        id: (s?.id ? String(s.id) : `E${idx + 1}`),
        tool: String(s?.tool ?? ''),
        description: String(s?.description ?? ''),
        args: s?.args
      })).filter(s => s.tool)
      return {
        summary: String(json.plan_summary ?? '执行计划'),
        steps: steps.length ? steps : undefined
      }
    }
  } catch {
    // ignore
  }

  // 无可解析计划
  return null
}

/**
 * 解析ReWOO Execution阶段的消息
 */
function parseExecutionSteps(content: string, chunks: any[], planningData?: ReWOOPlanningData | null): ReWOOExecutionStep[] {
  const steps: ReWOOExecutionStep[] = []

  // 从chunks中提取执行步骤信息
  const executionChunks = chunks.filter(chunk =>
    chunk.stage === 'rewoo_execution'
  )

  // 用于记录每个工具的信息（按出现顺序）
  const toolMap = new Map<string, ReWOOExecutionStep>()
  const toolOrder: string[] = []

  for (const chunk of executionChunks) {
    const chunkType = chunk.chunk_type
    const toolName = chunk.tool_name || 'unknown'

    // 确保工具存在于map中
    if (!toolMap.has(toolName)) {
      // 尝试从计划阶段获取该工具的参数信息
      const planStep = planningData?.steps?.find(s => s.tool === toolName)
      toolMap.set(toolName, {
        toolName,
        description: planStep?.description, // 从计划阶段获取描述
        args: planStep?.args // 从计划阶段获取参数
      })
      toolOrder.push(toolName)
    }

    const step = toolMap.get(toolName)!

    // 根据chunk_type处理不同内容
    if (chunkType === 'Thinking') {
      // "执行步骤 X/Y: tool - description" 或其他思考内容
      const thinkingText = chunk.content?.toString().trim() || ''
      if (!step.thinking) {
        step.thinking = thinkingText
      } else {
        step.thinking += '\n' + thinkingText
      }

      step.status = 'running'
    } else if (chunkType === 'ToolResult') {
      // 工具执行结果
      try {
        const resultStr = chunk.content?.toString().trim() || ''
        // 尝试解析为JSON
        step.result = JSON.parse(resultStr)
      } catch {
        step.result = chunk.content
      }
      step.status = 'success'
    } else if (chunkType === 'Error') {
      // 错误信息
      step.error = chunk.content?.toString() || 'Unknown error'
      step.status = 'failed'
    }
  }

  // 按顺序添加步骤
  for (const toolName of toolOrder) {
    const step = toolMap.get(toolName)!

    // 如果没有明确的状态，根据结果设置
    if (!step.status) {
      step.status = step.result ? 'success' : 'running'
    }

    steps.push(step)
  }

  // 如果chunks中没有提取到步骤，尝试从content文本中解析
  if (steps.length === 0) {
    const executionPattern = /执行步骤\s*(\d+)\/\d+:\s*(\w+)\s*-\s*(.+?)(?=\n|$)/gi
    let match
    while ((match = executionPattern.exec(content)) !== null) {
      const toolName = match[2]
      const planStep = planningData?.steps?.find(s => s.tool === toolName)
      steps.push({
        toolName,
        description: planStep?.description,
        thinking: match[3],
        args: planStep?.args,
        status: 'running'
      })
    }
  }

  return steps
}

/**
 * 解析ReWOO Solving阶段的消息
 */
function parseSolvingData(content: string, chunks: any[]): ReWOOSolvingData | null {
  // 查找solving阶段的chunks
  const solvingChunks = chunks.filter(chunk => chunk.stage === 'rewoo_solving')

  if (solvingChunks.length === 0) {
    // 如果没有solving chunks，检查content中是否包含最终答案
    // 增加对Markdown标题的检测，因为报告通常以标题开始
    if (content.includes('开始生成最终答案') ||
      content.includes('执行完成') ||
      /^##\s+/.test(content) ||
      content.includes('## B站热门视频搜索结果分析报告')) {

      // 尝试提取报告部分（如果content包含前面的JSON计划等）
      // 简单的启发式：如果包含 "## "，则从那里开始截取
      const reportMatch = content.match(/(##\s+.*$)/s)
      if (reportMatch) {
        return {
          answer: reportMatch[1]
        }
      }

      return {
        answer: content
      }
    }
    return null
  }

  // 合并所有solving阶段的内容
  let answer = ''
  let meta = ''

  for (const chunk of solvingChunks) {
    if (chunk.chunk_type === 'Content') {
      answer += chunk.content
    } else if (chunk.chunk_type === 'Meta') {
      meta += chunk.content
    }
  }

  if (!answer && !meta) return null

  return {
    answer: answer || '正在生成最终答案...',
    meta: meta || undefined
  }
}

/**
 * 解析完整的ReWOO消息
 */
export function parseReWOOMessage(content: string, chunks: any[] = []): ReWOOMessageData {
  const result: ReWOOMessageData = {}

  // 检查各个阶段
  const hasPlanningStage = chunks.some(c => c.stage === 'rewoo_planning' || c.chunk_type === 'PlanInfo') ||
    /\"plan_summary\"\s*:/.test(content)
  const hasExecutionStage = chunks.some(c => c.stage === 'rewoo_execution') ||
    content.includes('执行步骤') || content.includes('开始执行工具')
  const hasSolvingStage = chunks.some(c => c.stage === 'rewoo_solving') ||
    content.includes('开始生成最终答案') || content.includes('执行完成') || /^##\s+/.test(content)

  // 解析Planning阶段
  let planningData: ReWOOPlanningData | null = null
  if (hasPlanningStage) {
    planningData = parsePlanningData(content, chunks)
    if (planningData) {
      result.planningData = planningData
    }
  }

  // 解析Execution阶段
  if (hasExecutionStage) {
    const executionSteps = parseExecutionSteps(content, chunks, planningData)
    if (executionSteps.length > 0) {
      // 如果有计划数据，将参数关联到执行步骤（已在parseExecutionSteps中处理）
      if (planningData && planningData.steps) {
        // 创建工具名称到计划步骤的映射
        const planStepMap = new Map<string, any>()
        planningData.steps.forEach(step => {
          planStepMap.set(step.tool, step)
        })

        // 为每个执行步骤关联参数（补充处理，以防parseExecutionSteps中未获取到）
        executionSteps.forEach(execStep => {
          const planStep = planStepMap.get(execStep.toolName)
          if (planStep) {
            if (planStep.args && !execStep.args) {
              execStep.args = planStep.args
            }
            if (planStep.description && !execStep.description) {
              execStep.description = planStep.description
            }
          }
        })
      }
      result.executionSteps = executionSteps
    }
  }

  // 解析Solving阶段
  if (hasSolvingStage) {
    const solvingData = parseSolvingData(content, chunks)
    if (solvingData) {
      result.solvingData = solvingData
    }
  }

  return result
}

/**
 * 提取ReWOO消息的纯文本摘要（用于会话列表等）
 */
export function extractReWOOSummary(content: string): string {
  // 优先JSON计划的 plan_summary
  try {
    const json = JSON.parse(content)
    if (json && typeof json === 'object' && typeof json.plan_summary === 'string') {
      const s = json.plan_summary.trim()
      return s.substring(0, 100) + (s.length > 100 ? '...' : '')
    }
  } catch { /* ignore */ }

  // 尝试提取第一行有意义的内容
  const lines = content.split('\n').filter(line => line.trim().length > 0)
  if (lines.length > 0) {
    return lines[0].substring(0, 100) + (lines[0].length > 100 ? '...' : '')
  }

  return 'ReWOO执行'
}

