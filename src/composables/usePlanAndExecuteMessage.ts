/**
 * Plan-and-Execute消息解析 Composable
 * 用于检测和解析Plan-and-Execute架构的消息
 */

export interface PlanAndExecuteToolConfig {
  tool_name: string
  parameters?: any
}

export interface PlanAndExecutePlanningStep {
  name: string
  description: string
  tool?: string
  tool_config?: PlanAndExecuteToolConfig
  step_type?: string
}

export interface PlanAndExecuteRiskItem {
  description: string
  level: string
  impact?: string
  probability?: number
}

export interface PlanAndExecuteRiskAssessment {
  overall_risk: string
  risk_items?: PlanAndExecuteRiskItem[]
  mitigation_strategies?: string[]
}

export interface PlanAndExecuteResourceRequirements {
  estimated_time?: number
  required_tools?: string[]
  memory_mb?: number
  cpu_cores?: number
}

export interface PlanAndExecutePlanningData {
  summary: string
  steps?: PlanAndExecutePlanningStep[]
  risk_assessment?: PlanAndExecuteRiskAssessment
  resource_requirements?: PlanAndExecuteResourceRequirements
  confidence?: number
}

export interface PlanAndExecuteExecutionStep {
  name: string
  description?: string
  step_type?: string
  tool_config?: PlanAndExecuteToolConfig
  result?: any
  error?: string
  status?: 'Pending' | 'Running' | 'Completed' | 'Failed' | 'Skipped' | 'Blocked'
  duration_ms?: number
  retry_count?: number
}

export interface PlanAndExecuteExecutionData {
  steps: PlanAndExecuteExecutionStep[]
  current_step?: number
}

export interface PlanAndExecuteReplanningData {
  trigger?: string
  new_plan?: string
  reason?: string
}

export interface PlanAndExecuteSummaryData {
  response?: string
  total_steps: number
  completed_steps: number
  failed_steps: number
  total_duration_ms: number
}

export interface PlanAndExecuteMessageData {
  planningData?: PlanAndExecutePlanningData
  executionData?: PlanAndExecuteExecutionData
  replanningData?: PlanAndExecuteReplanningData
  summaryData?: PlanAndExecuteSummaryData
}

/**
 * 检测是否为Plan-and-Execute消息
 */
export function isPlanAndExecuteMessage(content: string, chunks: any[] = []): boolean {
  // 检查stage标记
  const hasPlanAndExecuteStage = chunks.some(c =>
    c.stage === 'planner' ||
    c.stage === 'executor' ||
    c.stage === 'replanner' ||
    c.stage === 'plan_and_execute'
  )

  if (hasPlanAndExecuteStage) return true

  // 检查内容特征
  const planAndExecutePatterns = [
    /执行计划生成/,
    /步骤.*执行/,
    /重新规划/,
    /风险评估/,
    /资源需求/,
    /"plan"\s*:\s*\[/,
    /"past_steps"\s*:\s*\[/,
    /执行步骤\s*\d+/,
  ]

  return planAndExecutePatterns.some(pattern => pattern.test(content))
}

/**
 * 解析Planning阶段的消息
 */
function parsePlanningData(content: string, chunks: any[] = []): PlanAndExecutePlanningData | null {
  // 从PlanInfo块解析
  const planInfoChunks = chunks.filter(c =>
    c.chunk_type === 'PlanInfo' ||
    c.stage === 'planner'
  )

  for (const chunk of planInfoChunks) {
    const text = (chunk?.content ?? '').toString().trim()
    if (!text) continue

    // 尝试解析JSON格式的计划
    try {
      const json = JSON.parse(text)
      if (json && typeof json === 'object') {
        // 检查是否包含plan数组
        if (Array.isArray(json.plan)) {
          const steps = json.plan.map((stepText: string, idx: number) => ({
            name: `步骤 ${idx + 1}`,
            description: stepText,
            step_type: 'action'
          }))

          return {
            summary: String(json.summary ?? json.reasoning ?? `执行计划生成成功，共${steps.length}个步骤`),
            steps: steps.length ? steps : undefined,
            risk_assessment: json.risk_assessment ? {
              overall_risk: String(json.risk_assessment.overall_risk ?? 'Medium'),
              risk_items: Array.isArray(json.risk_assessment.risk_items)
                ? json.risk_assessment.risk_items.map((item: any) => ({
                  description: String(item.description ?? ''),
                  level: String(item.level ?? 'Medium'),
                  impact: item.impact ? String(item.impact) : undefined,
                  probability: typeof item.probability === 'number' ? item.probability : undefined
                }))
                : undefined,
              mitigation_strategies: Array.isArray(json.risk_assessment.mitigation_strategies)
                ? json.risk_assessment.mitigation_strategies.map((s: any) => String(s))
                : undefined
            } : undefined,
            resource_requirements: json.resource_requirements ? {
              estimated_time: typeof json.resource_requirements.estimated_time === 'number'
                ? json.resource_requirements.estimated_time
                : undefined,
              required_tools: Array.isArray(json.resource_requirements.required_tools)
                ? json.resource_requirements.required_tools.map((t: any) => String(t))
                : undefined,
              memory_mb: typeof json.resource_requirements.memory_mb === 'number'
                ? json.resource_requirements.memory_mb
                : undefined,
              cpu_cores: typeof json.resource_requirements.cpu_cores === 'number'
                ? json.resource_requirements.cpu_cores
                : undefined
            } : undefined,
            confidence: typeof json.confidence === 'number' ? json.confidence : undefined
          }
        }

        // 检查是否包含steps数组
        if (Array.isArray(json.steps)) {
          const steps = json.steps.map((s: any) => {
            // 提取工具配置
            let toolConfig: PlanAndExecuteToolConfig | undefined = undefined;

            if (s.tool_config) {
              toolConfig = {
                tool_name: s.tool_config.tool_name,
                parameters: s.tool_config.tool_args || s.tool_config.parameters
              }
            } else if (s.parameters && s.tool) {
              // 兼容旧格式或扁平格式
              toolConfig = {
                tool_name: s.tool,
                parameters: s.parameters
              }
            }

            return {
              name: String(s?.name ?? s?.description ?? ''),
              description: String(s?.description ?? ''),
              tool: s?.tool ? String(s.tool) : undefined,
              tool_config: toolConfig,
              step_type: s?.step_type ? String(s.step_type) : undefined
            }
          }).filter((s: any) => s.name)

          return {
            summary: String(json.summary ?? json.description ?? `执行计划生成成功，共${steps.length}个步骤`),
            steps: steps.length ? steps : undefined,
            risk_assessment: json.risk_assessment,
            resource_requirements: json.resource_requirements,
            confidence: typeof json.confidence === 'number' ? json.confidence : undefined
          }
        }
      }
    } catch {
      // 不是JSON，尝试文本解析
    }

    // 文本格式解析
    const lines = text.split('\n').map(l => l.trim()).filter(Boolean)
    if (lines.length > 0) {
      const steps: PlanAndExecutePlanningStep[] = []
      const summaryLine = lines[0]

      // 提取步骤
      const stepRegex = /^(?:步骤\s*)?(\d+)[.、:：]\s*(.+)$/
      for (let i = 0; i < lines.length; i++) {
        const match = stepRegex.exec(lines[i])
        if (match) {
          steps.push({
            name: `步骤 ${match[1]}`,
            description: match[2].trim(),
            step_type: 'action'
          })
        }
      }

      if (steps.length > 0 || summaryLine) {
        return {
          summary: summaryLine || `执行计划生成成功，共${steps.length}个步骤`,
          steps: steps.length ? steps : undefined
        }
      }
    }
  }

  return null
}

/**
 * 解析Execution阶段的消息
 */
function parseExecutionData(content: string, chunks: any[] = [], planningData?: PlanAndExecutePlanningData | null): PlanAndExecuteExecutionData | null {
  const executionChunks = chunks.filter(c =>
    c.stage === 'executor' ||
    c.stage === 'plan_and_execute' ||
    c.chunk_type === 'ToolResult' ||
    c.chunk_type === 'Meta' ||
    c.chunk_type === 'Error' ||
    c.chunk_type === 'Content'
  )

  // 如果没有执行块且没有规划数据，则无法构建执行数据
  if (executionChunks.length === 0 && (!planningData?.steps || planningData.steps.length === 0)) return null

  const stepsMap = new Map<string, PlanAndExecuteExecutionStep>()
  const stepsOrder: string[] = []
  let currentRunningStepName: string | null = null

  // 1. 优先从规划数据初始化步骤
  if (planningData?.steps) {
    for (const planStep of planningData.steps) {
      stepsMap.set(planStep.name, {
        name: planStep.name,
        description: planStep.description,
        step_type: planStep.step_type,
        tool_config: planStep.tool_config ? planStep.tool_config : (planStep.tool ? {
          tool_name: planStep.tool,
          parameters: {}
        } : undefined),
        status: 'Pending'
      })
      stepsOrder.push(planStep.name)
    }
  }

  // 2. 处理执行块更新步骤
  for (const chunk of executionChunks) {
    const chunkType = chunk.chunk_type

    // 从Meta块中提取步骤信息
    if (chunkType === 'Meta') {
      try {
        const metaText = chunk.content?.toString() || ''
        const metaJson = JSON.parse(metaText)

        if (metaJson.step_name) {
          const stepName = metaJson.step_name

          if (!stepsMap.has(stepName)) {
            stepsMap.set(stepName, {
              name: stepName,
              description: metaJson.step_description,
              step_type: metaJson.step_type,
              status: 'Pending'
            })
            stepsOrder.push(stepName)
          }

          const step = stepsMap.get(stepName)!

          // 更新状态
          if (metaJson.status) {
            step.status = metaJson.status as any
          }

          // 更新耗时
          if (metaJson.duration_ms) {
            step.duration_ms = Number(metaJson.duration_ms)
          }

          // 维护当前运行步骤
          if (metaJson.type === 'step_started') {
            currentRunningStepName = stepName
          } else if (metaJson.type === 'step_completed' || metaJson.type === 'step_failed') {
            if (currentRunningStepName === stepName) {
              currentRunningStepName = null
            }
          }
        }
      } catch {
        // 不是JSON格式的meta
      }
    }

    // 处理Content块 (主要用于AiReasoning步骤的流式输出)
    if (chunkType === 'Content') {
      if (currentRunningStepName) {
        const step = stepsMap.get(currentRunningStepName)
        if (step) {
          const content = chunk.content?.toString() || ''
          // 如果结果是对象(ToolCall)，通常不会有Content块混入，除非是混合输出
          // 对于AiReasoning，结果通常是字符串
          if (!step.result || typeof step.result === 'string') {
            step.result = (step.result || '') + content
          }
        }
      }
    }

    // 从ToolResult块中提取结果
    if (chunkType === 'ToolResult') {
      try {
        const resultData = typeof chunk.content === 'string'
          ? JSON.parse(chunk.content)
          : chunk.content

        // 尝试确定所属步骤
        let stepName = resultData?.step_name

        // 如果数据中没有step_name，尝试使用chunk.tool_name匹配
        if (!stepName && chunk.tool_name) {
          // 1. 尝试直接匹配步骤名（不太可能，除非步骤名就是工具名）
          if (stepsMap.has(chunk.tool_name)) {
            stepName = chunk.tool_name
          }
          // 2. 尝试匹配步骤配置的工具名
          else {
            const step = Array.from(stepsMap.values()).find(s => s.tool_config?.tool_name === chunk.tool_name)
            if (step) {
              stepName = step.name
            }
          }
        }

        // 如果还是没找到，且当前有正在运行的步骤，归属给它
        if (!stepName && currentRunningStepName) {
          stepName = currentRunningStepName
        }

        if (stepName && stepsMap.has(stepName)) {
          const step = stepsMap.get(stepName)!

          // 如果resultData是标准包装格式
          if (resultData && (resultData.tool_name || resultData.step_name || resultData.output !== undefined)) {
            step.result = resultData.result ?? resultData.output

            if (resultData.status) {
              step.status = resultData.status
            }

            // 只有当resultData包含参数时才更新tool_config
            if (resultData.parameters) {
              step.tool_config = {
                tool_name: resultData.tool_name || step.tool_config?.tool_name || 'unknown',
                parameters: resultData.parameters
              }
            }
          } else {
            // 否则假设整个resultData就是结果 (Raw Tool Output)
            step.result = resultData
            // ToolResult通常意味着步骤完成（除非是流式中间结果，但这里简化处理）
            if (step.status === 'Running' || step.status === 'Pending') {
              step.status = 'Completed'
            }
          }
        }
      } catch (e) {
        console.warn('Failed to parse ToolResult chunk:', e)
        // 解析失败，如果当前有运行步骤，尝试作为纯文本结果
        if (currentRunningStepName) {
          const step = stepsMap.get(currentRunningStepName)!
          step.result = chunk.content
        }
      }
    }

    // 从Error块中提取错误信息
    if (chunkType === 'Error') {
      const errorText = chunk.content?.toString() || ''
      // 尝试从错误信息中提取步骤名称
      const stepMatch = errorText.match(/步骤[:\s]+([^\s:：]+)/)
      if (stepMatch) {
        const stepName = stepMatch[1]

        if (!stepsMap.has(stepName)) {
          stepsMap.set(stepName, {
            name: stepName,
            status: 'Failed'
          })
          stepsOrder.push(stepName)
        }

        const step = stepsMap.get(stepName)!
        step.error = errorText
        step.status = 'Failed'
      } else {
        // 如果无法匹配到特定步骤，关联到当前正在运行的步骤
        const runningStepName = currentRunningStepName || Array.from(stepsMap.values()).find(s => s.status === 'Running')?.name;
        if (runningStepName) {
          const step = stepsMap.get(runningStepName)!
          step.error = errorText;
          step.status = 'Failed';
        }
      }
    }
  }

  if (stepsMap.size === 0) return null

  const steps = stepsOrder.map(name => stepsMap.get(name)!)

  return {
    steps,
    current_step: steps.findIndex(s => s.status === 'Running')
  }
}

/**
 * 解析Replanning阶段的消息
 */
function parseReplanningData(content: string, chunks: any[] = []): PlanAndExecuteReplanningData | null {
  const replanChunks = chunks.filter(c => c.stage === 'replanner')

  if (replanChunks.length === 0) {
    // 从content中检测
    if (content.includes('重新规划') || content.includes('replan')) {
      return {
        trigger: '需要重新规划',
        new_plan: content
      }
    }
    return null
  }

  const replanData: PlanAndExecuteReplanningData = {}

  for (const chunk of replanChunks) {
    const text = chunk.content?.toString() || ''

    if (chunk.chunk_type === 'PlanInfo') {
      replanData.new_plan = text
    } else if (chunk.chunk_type === 'Thinking') {
      replanData.trigger = text
    } else if (chunk.chunk_type === 'Meta') {
      try {
        const meta = JSON.parse(text)
        if (meta.trigger) {
          replanData.trigger = meta.trigger
        }
        if (meta.reason) {
          replanData.reason = meta.reason
        }
      } catch {
        replanData.reason = text
      }
    }
  }

  return Object.keys(replanData).length > 0 ? replanData : null
}

/**
 * 解析Summary数据
 */
function parseSummaryData(content: string, chunks: any[] = [], executionData?: PlanAndExecuteExecutionData | null): PlanAndExecuteSummaryData | null {
  // 尝试从content中解析JSON格式的摘要
  try {
    const json = JSON.parse(content)
    if (json && typeof json === 'object' && 'response' in json) {
      return {
        response: String(json.response ?? ''),
        total_steps: Number(json.total_steps ?? 0),
        completed_steps: Number(json.completed_steps ?? 0),
        failed_steps: Number(json.failed_steps ?? 0),
        total_duration_ms: Number(json.total_duration_ms ?? 0)
      }
    }
  } catch {
    // ignore
  }

  // 从执行数据中计算摘要
  if (executionData?.steps) {
    const totalSteps = executionData.steps.length
    const completedSteps = executionData.steps.filter(s => s.status === 'Completed').length
    const failedSteps = executionData.steps.filter(s => s.status === 'Failed').length
    const totalDuration = executionData.steps.reduce((sum, s) => sum + (s.duration_ms ?? 0), 0)

    // 检查是否有最终响应
    const responseChunks = chunks.filter(c =>
      c.chunk_type === 'Content' ||
      (c.chunk_type === 'Meta' && c.content?.includes('完成'))
    )

    let response = ''
    if (responseChunks.length > 0) {
      response = responseChunks[responseChunks.length - 1].content?.toString() || ''
    }

    if (totalSteps > 0) {
      return {
        response: response || content,
        total_steps: totalSteps,
        completed_steps: completedSteps,
        failed_steps: failedSteps,
        total_duration_ms: totalDuration
      }
    }
  }

  return null
}

/**
 * 解析完整的Plan-and-Execute消息
 */
export function parsePlanAndExecuteMessage(content: string, chunks: any[] = []): PlanAndExecuteMessageData {
  const result: PlanAndExecuteMessageData = {}

  // 检查各个阶段
  const hasPlanningStage = chunks.some(c =>
    c.stage === 'planner' ||
    c.chunk_type === 'PlanInfo'
  ) || content.includes('执行计划') || content.includes('步骤')

  const hasExecutionStage = chunks.some(c =>
    c.stage === 'executor' ||
    c.chunk_type === 'ToolResult'
  ) || content.includes('执行步骤') || content.includes('工具执行')

  const hasReplanningStage = chunks.some(c => c.stage === 'replanner') ||
    content.includes('重新规划') || content.includes('replan')

  // 解析Planning阶段
  let planningData: PlanAndExecutePlanningData | null = null
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

  // 解析Replanning阶段
  if (hasReplanningStage) {
    const replanningData = parseReplanningData(content, chunks)
    if (replanningData) {
      result.replanningData = replanningData
    }
  }

  // 解析Summary数据
  const summaryData = parseSummaryData(content, chunks, result.executionData)
  if (summaryData) {
    result.summaryData = summaryData
  }

  return result
}

/**
 * Composable函数
 */
export function usePlanAndExecuteMessage() {
  return {
    isPlanAndExecuteMessage,
    parsePlanAndExecuteMessage
  }
}

