import type {
  TeamOrchestrationPlan,
  TeamOrchestrationPresetId,
  TeamOrchestrationPresetMeta,
  TeamOrchestrationStep,
  TeamOrchestrationStepType,
  TeamRecoveryPreset,
  TeamStepMovePayload,
} from './teamOrchestrationTypes'

export const TEAM_ORCHESTRATION_PRESET_METAS: TeamOrchestrationPresetMeta[] = [
  {
    id: 'product_delivery_chain',
    label: '需求到交付',
    description: '产品 -> 架构 -> 研发/测试并行 -> 发布决策，适合 idea 到落地全链路。',
  },
  {
    id: 'incident_response_flow',
    label: '故障处置链路',
    description: '故障接管 -> 并行根因分析 -> 处置方案 -> 验证复盘，适合线上异常场景。',
  },
]

export const TEAM_RECOVERY_PRESETS: TeamRecoveryPreset[] = [
  {
    id: 'conservative',
    label: '保守',
    description: '低重试、长退避、较长人工等待窗口，优先稳定性与可回滚性。',
    max_attempts: 1,
    backoff_ms: 1500,
    human_intervention_timeout_secs: 900,
    max_human_interventions: 4,
    no_human_input_policy: 'conservative',
  },
  {
    id: 'balanced',
    label: '平衡',
    description: '中等重试与等待窗口，在质量、风险和进度间折中。',
    max_attempts: 2,
    backoff_ms: 800,
    human_intervention_timeout_secs: 600,
    max_human_interventions: 3,
    no_human_input_policy: 'balanced',
  },
  {
    id: 'aggressive',
    label: '激进',
    description: '高重试、短退避、短等待窗口，优先推进速度与产出。',
    max_attempts: 3,
    backoff_ms: 400,
    human_intervention_timeout_secs: 300,
    max_human_interventions: 2,
    no_human_input_policy: 'aggressive',
  },
]

export const defaultOrchestrationPlan = (): TeamOrchestrationPlan => ({
  version: 1,
  steps: [],
})

export const generateTeamStepId = (prefix = 'step') => {
  const random = Math.random().toString(36).slice(2, 7)
  return `${prefix}-${Date.now().toString(36)}-${random}`
}

const pickTeamMemberForPreset = (
  memberOptions: string[],
  keywords: string[],
  fallbackIndex = 0,
) => {
  if (memberOptions.length === 0) return ''
  const lowered = memberOptions.map((name) => name.toLowerCase())
  for (const keyword of keywords) {
    const hitIndex = lowered.findIndex((item) => item.includes(keyword.toLowerCase()))
    if (hitIndex >= 0) {
      return memberOptions[hitIndex]
    }
  }
  return memberOptions[Math.min(Math.max(fallbackIndex, 0), memberOptions.length - 1)] || memberOptions[0]
}

const createPresetAgentStep = (input: {
  prefix: string
  name: string
  phase: string
  instruction: string
  member: string
}): TeamOrchestrationStep => ({
  id: generateTeamStepId(input.prefix),
  type: 'agent',
  name: input.name,
  member: input.member,
  phase: input.phase,
  instruction: input.instruction,
  retry: { max_attempts: 1, backoff_ms: 800 },
})

export const buildOrchestrationPresetPlan = (params: {
  memberOptions: string[]
  presetId: TeamOrchestrationPresetId
  version: number
}): TeamOrchestrationPlan => {
  const version = Math.max(1, Number(params.version || 1))
  const pm = pickTeamMemberForPreset(params.memberOptions, ['产品', 'product', 'pm'], 0)
  const architect = pickTeamMemberForPreset(params.memberOptions, ['架构', 'architect'], 1)
  const engineer = pickTeamMemberForPreset(params.memberOptions, ['研发', '开发', 'engineer', 'dev'], 2)
  const qa = pickTeamMemberForPreset(params.memberOptions, ['测试', 'qa', 'quality'], 3)
  const security = pickTeamMemberForPreset(params.memberOptions, ['安全', 'security'], 2)
  const sre = pickTeamMemberForPreset(params.memberOptions, ['sre', '运维', 'ops'], 0)

  if (params.presetId === 'product_delivery_chain') {
    return {
      version,
      steps: [
        createPresetAgentStep({
          prefix: 'prd',
          name: '需求澄清',
          phase: 'requirements',
          member: pm,
          instruction: '明确目标用户、核心场景、验收标准与边界条件，沉淀为可执行需求。',
        }),
        createPresetAgentStep({
          prefix: 'arch',
          name: '架构设计',
          phase: 'design',
          member: architect || engineer || pm,
          instruction: '输出系统架构、模块边界、关键技术选型与风险控制点。',
        }),
        {
          id: generateTeamStepId('delivery'),
          type: 'parallel',
          name: '并行交付',
          phase: 'implementation_and_validation',
          children: [
            createPresetAgentStep({
              prefix: 'impl',
              name: '研发实现方案',
              phase: 'implementation',
              member: engineer || architect || pm,
              instruction: '拆解实现任务、接口契约与交付顺序，明确依赖与里程碑。',
            }),
            createPresetAgentStep({
              prefix: 'qa',
              name: '测试验证策略',
              phase: 'validation',
              member: qa || engineer || architect,
              instruction: '设计测试范围、关键用例、回归策略与上线前验证清单。',
            }),
          ],
        },
        createPresetAgentStep({
          prefix: 'gate',
          name: '发布决策',
          phase: 'release_gate',
          member: architect || pm || engineer,
          instruction: '综合风险、质量与收益做最终发布建议，并给出发布后观测指标。',
        }),
      ],
    }
  }

  return {
    version,
    steps: [
      createPresetAgentStep({
        prefix: 'takeover',
        name: '故障接管',
        phase: 'incident_intake',
        member: sre || architect || engineer,
        instruction: '明确故障范围、影响用户、时间线与当前止损动作。',
      }),
      {
        id: generateTeamStepId('analysis'),
        type: 'parallel',
        name: '并行根因分析',
        phase: 'incident_analysis',
        children: [
          createPresetAgentStep({
            prefix: 'rootcause',
            name: '技术根因分析',
            phase: 'root_cause',
            member: engineer || architect || sre,
            instruction: '定位技术根因，给出可验证证据与复现路径。',
          }),
          createPresetAgentStep({
            prefix: 'secimpact',
            name: '安全影响评估',
            phase: 'security_impact',
            member: security || engineer || architect,
            instruction: '评估是否存在安全风险扩散、数据泄露或权限滥用。',
          }),
        ],
      },
      createPresetAgentStep({
        prefix: 'plan',
        name: '处置与恢复方案',
        phase: 'mitigation_plan',
        member: architect || engineer || sre,
        instruction: '制定短期止血与长期修复方案，明确执行步骤与责任人。',
      }),
      createPresetAgentStep({
        prefix: 'postmortem',
        name: '验证与复盘',
        phase: 'verification_postmortem',
        member: qa || pm || architect,
        instruction: '验证恢复效果并输出复盘结论、预防措施与后续追踪指标。',
      }),
    ],
  }
}

export const getAllTeamAgentSteps = (steps: TeamOrchestrationStep[]): TeamOrchestrationStep[] => {
  const result: TeamOrchestrationStep[] = []
  const walk = (nodes: TeamOrchestrationStep[]) => {
    for (const node of nodes) {
      if (node.type === 'agent') {
        result.push(node)
      } else if (Array.isArray(node.children) && node.children.length > 0) {
        walk(node.children)
      }
    }
  }
  walk(steps)
  return result
}

export const normalizeTeamOrchestrationStep = (raw: any): TeamOrchestrationStep => {
  const typeRaw = typeof raw?.type === 'string' ? raw.type : 'agent'
  const type: TeamOrchestrationStepType = typeRaw === 'parallel' || typeRaw === 'serial' ? typeRaw : 'agent'
  const step: TeamOrchestrationStep = {
    id: typeof raw?.id === 'string' && raw.id.trim()
      ? raw.id.trim()
      : generateTeamStepId('step'),
    type,
    name: typeof raw?.name === 'string' ? raw.name : '',
    phase: typeof raw?.phase === 'string' ? raw.phase : '',
    instruction: typeof raw?.instruction === 'string'
      ? raw.instruction
      : (typeof raw?.prompt === 'string' ? raw.prompt : ''),
  }

  if (type === 'agent') {
    step.member = typeof raw?.member === 'string' ? raw.member : ''
    const retryMaxAttempts = Number(raw?.retry?.max_attempts ?? raw?.retry_max_attempts ?? 1)
    const retryBackoffMs = Number(raw?.retry?.backoff_ms ?? raw?.retry_backoff_ms ?? 800)
    step.retry = {
      max_attempts: Number.isFinite(retryMaxAttempts) ? Math.max(1, Math.floor(retryMaxAttempts)) : 1,
      backoff_ms: Number.isFinite(retryBackoffMs) ? Math.max(100, Math.floor(retryBackoffMs)) : 800,
    }
  } else {
    const rawChildren = Array.isArray(raw?.children) ? raw.children : []
    step.children = rawChildren.map((child: any) => normalizeTeamOrchestrationStep(child))
  }

  return step
}

export const normalizeTeamOrchestrationPlan = (raw: any): TeamOrchestrationPlan => {
  const versionRaw = Number(raw?.version ?? 1)
  const version = Number.isFinite(versionRaw) ? Math.max(1, Math.floor(versionRaw)) : 1
  const steps = Array.isArray(raw?.steps)
    ? raw.steps.map((step: any) => normalizeTeamOrchestrationStep(step))
    : []
  return { version, steps }
}

export const teamOrchestrationPlanToJson = (plan: TeamOrchestrationPlan): any => {
  const mapStep = (step: TeamOrchestrationStep): any => {
    const base: Record<string, any> = {
      id: step.id || generateTeamStepId('step'),
      type: step.type,
    }
    if (step.name && step.name.trim()) base.name = step.name.trim()
    if (step.phase && step.phase.trim()) base.phase = step.phase.trim()
    if (step.instruction && step.instruction.trim()) base.instruction = step.instruction.trim()
    if (step.type === 'agent') {
      base.member = (step.member || '').trim()
      const retryMaxAttempts = Number(step.retry?.max_attempts ?? 1)
      const retryBackoffMs = Number(step.retry?.backoff_ms ?? 800)
      base.retry = {
        max_attempts: Number.isFinite(retryMaxAttempts) ? Math.max(1, Math.floor(retryMaxAttempts)) : 1,
        backoff_ms: Number.isFinite(retryBackoffMs) ? Math.max(100, Math.floor(retryBackoffMs)) : 800,
      }
    } else {
      base.children = Array.isArray(step.children) ? step.children.map(mapStep) : []
    }
    return base
  }

  return {
    version: Number.isFinite(Number(plan.version)) ? Math.max(1, Math.floor(Number(plan.version))) : 1,
    steps: Array.isArray(plan.steps) ? plan.steps.map(mapStep) : [],
  }
}

export const serializeTeamOrchestrationPlan = (plan: TeamOrchestrationPlan): string => (
  JSON.stringify(teamOrchestrationPlanToJson(plan), null, 2)
)

export const parseTeamOrchestrationPlanInput = (raw: string): {
  jsonValue: any
  normalized: TeamOrchestrationPlan
} => {
  const trimmed = raw.trim()
  if (!trimmed) {
    throw new Error('编排计划不能为空。')
  }
  const parsed = JSON.parse(trimmed)
  if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) {
    throw new Error('编排计划必须是 JSON 对象。')
  }
  const normalized = normalizeTeamOrchestrationPlan(parsed)
  return {
    jsonValue: teamOrchestrationPlanToJson(normalized),
    normalized,
  }
}

const ensureTeamContainerChildren = (step: TeamOrchestrationStep): TeamOrchestrationStep[] => {
  if (step.type === 'agent') {
    step.type = 'serial'
    step.member = ''
    step.retry = undefined
  }
  if (!Array.isArray(step.children)) {
    step.children = []
  }
  return step.children
}

const getTeamStepArrayByContainerPath = (
  steps: TeamOrchestrationStep[],
  containerPath: number[],
): TeamOrchestrationStep[] | null => {
  let current = steps
  if (containerPath.length === 0) {
    return current
  }
  for (const idx of containerPath) {
    if (!Array.isArray(current) || idx < 0 || idx >= current.length) {
      return null
    }
    const step = current[idx]
    current = ensureTeamContainerChildren(step)
  }
  return current
}

const getTeamStepByPath = (steps: TeamOrchestrationStep[], path: number[]): TeamOrchestrationStep | null => {
  if (path.length === 0) return null
  const container = getTeamStepArrayByContainerPath(steps, path.slice(0, -1))
  const index = path[path.length - 1]
  if (!container || index < 0 || index >= container.length) return null
  return container[index]
}

const pathsEqual = (a: number[], b: number[]): boolean =>
  a.length === b.length && a.every((v, idx) => v === b[idx])

const isPathPrefix = (prefix: number[], path: number[]): boolean =>
  prefix.length <= path.length && prefix.every((v, idx) => path[idx] === v)

const removeTeamStepAtPath = (steps: TeamOrchestrationStep[], path: number[]): TeamOrchestrationStep | null => {
  if (path.length === 0) return null
  const container = getTeamStepArrayByContainerPath(steps, path.slice(0, -1))
  const index = path[path.length - 1]
  if (!container || index < 0 || index >= container.length) return null
  const [removed] = container.splice(index, 1)
  return removed || null
}

const adjustPathAfterRemoval = (path: number[], removedPath: number[]): number[] | null => {
  if (isPathPrefix(removedPath, path)) {
    return null
  }
  const next = [...path]
  if (
    path.length === removedPath.length &&
    pathsEqual(path.slice(0, -1), removedPath.slice(0, -1)) &&
    removedPath[removedPath.length - 1] < path[path.length - 1]
  ) {
    next[next.length - 1] -= 1
  }
  return next
}

export const moveTeamStepByPath = (
  steps: TeamOrchestrationStep[],
  payload: TeamStepMovePayload,
): boolean => {
  const sourcePath = payload.sourcePath || []
  const targetPath = payload.targetPath || []
  if (sourcePath.length === 0 || targetPath.length === 0) return false
  if (pathsEqual(sourcePath, targetPath) && payload.mode === 'before') return false
  if (payload.mode === 'inside' && isPathPrefix(sourcePath, targetPath)) return false

  const moved = removeTeamStepAtPath(steps, sourcePath)
  if (!moved) return false
  const adjustedTargetPath = adjustPathAfterRemoval(targetPath, sourcePath)
  if (!adjustedTargetPath) return true

  if (payload.mode === 'before') {
    const targetContainer = getTeamStepArrayByContainerPath(steps, adjustedTargetPath.slice(0, -1))
    const targetIndex = adjustedTargetPath[adjustedTargetPath.length - 1]
    if (!targetContainer) return false
    const safeIndex = Math.max(0, Math.min(targetIndex, targetContainer.length))
    targetContainer.splice(safeIndex, 0, moved)
    return true
  }

  const targetStep = getTeamStepByPath(steps, adjustedTargetPath)
  if (!targetStep) return false
  const children = ensureTeamContainerChildren(targetStep)
  children.push(moved)
  return true
}

export const promoteTeamStep = (steps: TeamOrchestrationStep[], path: number[]): boolean => {
  if (path.length < 2) return false
  const moveIndex = path[path.length - 1]
  const parentPath = path.slice(0, -1)
  const grandParentPath = path.slice(0, -2)
  const parentIndex = path[path.length - 2]
  const sourceArray = getTeamStepArrayByContainerPath(steps, parentPath)
  const targetArray = getTeamStepArrayByContainerPath(steps, grandParentPath)
  if (!sourceArray || !targetArray) return false
  if (moveIndex < 0 || moveIndex >= sourceArray.length) return false
  const [moved] = sourceArray.splice(moveIndex, 1)
  targetArray.splice(parentIndex + 1, 0, moved)
  return true
}

export const nestTeamStep = (steps: TeamOrchestrationStep[], path: number[]): boolean => {
  if (path.length < 1) return false
  const moveIndex = path[path.length - 1]
  if (moveIndex <= 0) return false
  const containerPath = path.slice(0, -1)
  const siblingArray = getTeamStepArrayByContainerPath(steps, containerPath)
  if (!siblingArray) return false
  if (moveIndex < 0 || moveIndex >= siblingArray.length) return false
  const prevSibling = siblingArray[moveIndex - 1]
  const [moved] = siblingArray.splice(moveIndex, 1)
  const children = ensureTeamContainerChildren(prevSibling)
  children.push(moved)
  return true
}
