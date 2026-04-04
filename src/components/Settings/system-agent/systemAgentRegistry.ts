import type { SystemAgentProfilePayload, SystemAgentProfileSummary } from '../systemAgentSettingsSupport'

type AgentIdentity = Pick<SystemAgentProfilePayload, 'id' | 'description' | 'mode' | 'capability'>
  | Pick<SystemAgentProfileSummary, 'id' | 'description' | 'mode' | 'capability'>

export type SystemAgentUiKind =
  | 'manual_audit'
  | 'workflow_designer'
  | 'plugin_generator'
  | 'plugin_fixer'
  | 'traffic_triage'
  | 'traffic_verifier'
  | 'generic'

export type SystemAgentStatsMode = 'findings' | 'versions'
export type SystemAgentSafetyMode = 'triage' | 'verifier' | 'generic'
export type SystemAgentModeBadge = {
  label: string
  className: string
}

interface SystemAgentUiDefinition {
  kind: SystemAgentUiKind
  description: string
  passiveEventName?: string
  supportsFindings?: boolean
  supportsBehaviorSource?: boolean
  supportsDispatch?: boolean
  statsMode: SystemAgentStatsMode
  safetyMode: SystemAgentSafetyMode
  promptPatchPlaceholder: string[]
  promptPatchGuidance: string[]
  manualInputGuidance: string[]
  dispatchPayloadGuidance: string[]
  manualInputTemplate: Record<string, unknown>
  dispatchPayloadTemplate?: Record<string, unknown>
}

const GENERIC_DEFINITION: SystemAgentUiDefinition = {
  kind: 'generic',
  description: '系统内置 Agent',
  supportsFindings: false,
  supportsDispatch: false,
  statsMode: 'versions',
  safetyMode: 'generic',
  promptPatchPlaceholder: [
    '示例：',
    '优先生成结构稳定、可审阅、可回滚的结果。',
    '避免引入额外高风险动作或破坏现有契约。',
  ],
  promptPatchGuidance: [
    '建议写输出偏好、质量要求、项目上下文。',
    '不建议在这里堆完整 Prompt，优先补充而不是覆盖。',
  ],
  manualInputGuidance: [
    '推荐：用自然语言描述任务目标和约束。',
    '如果需要更稳定结果，再补结构化字段。',
  ],
  dispatchPayloadGuidance: ['当前 Agent 不支持事件派发。'],
  manualInputTemplate: {
    summary: '手动执行 Agent 任务',
  },
}

const SYSTEM_AGENT_UI_DEFINITIONS: Record<string, SystemAgentUiDefinition> = {
  manual_traffic_audit_agent: {
    kind: 'manual_audit',
    description: '手动审查一批流量，帮助定位值得继续验证的风险点。',
    supportsFindings: false,
    supportsDispatch: false,
    statsMode: 'versions',
    safetyMode: 'generic',
    promptPatchPlaceholder: [
      '示例：',
      '优先标记对象边界、权限差异和业务流程异常。',
      '忽略静态资源、登录页和明显无风险接口。',
    ],
    promptPatchGuidance: [
      '建议补充关注资产范围、风险偏好和忽略路径。',
      '不建议重写 Agent 角色本身，尽量只做补充约束。',
    ],
    manualInputGuidance: [
      '推荐：提供本次要审查的流量范围或目标摘要。',
      '适合描述“请帮我优先找越权或流程异常线索”。',
    ],
    dispatchPayloadGuidance: ['当前 Agent 不支持事件派发。'],
    manualInputTemplate: {
      summary: '手动审计目标',
    },
  },
  workflow_designer_agent: {
    kind: 'workflow_designer',
    description: '根据自然语言需求生成工作流草稿，帮助快速搭建自动化流程。',
    supportsFindings: false,
    supportsDispatch: false,
    statsMode: 'versions',
    safetyMode: 'generic',
    promptPatchPlaceholder: [
      '示例：',
      '优先输出结构稳定、可审阅、可回滚的工作流草稿。',
      '避免引入高风险节点或破坏现有契约。',
    ],
    promptPatchGuidance: [
      '建议写输出偏好、节点约束、项目上下文。',
      '不建议在这里重写完整工作流设计 Prompt。',
    ],
    manualInputGuidance: [
      '推荐：直接描述你希望生成的工作流目标。',
      '例如：每天 8 点抓取公告页面并生成摘要后发送通知。',
    ],
    dispatchPayloadGuidance: ['当前 Agent 不支持事件派发。'],
    manualInputTemplate: {
      description: '每天早上 8 点抓取目标公告页面，提取新增内容并生成摘要，最后发送通知。',
    },
  },
  traffic_plugin_generator_agent: {
    kind: 'plugin_generator',
    description: '根据漏洞目标生成流量分析插件初稿，便于后续校验和完善。',
    supportsFindings: false,
    supportsDispatch: false,
    statsMode: 'versions',
    safetyMode: 'generic',
    promptPatchPlaceholder: [
      '示例：',
      '优先生成低误报、易维护、可读性好的规则型插件。',
      '避免输出依赖外部不稳定上下文的检测逻辑。',
    ],
    promptPatchGuidance: [
      '建议写漏洞目标、输出风格和质量要求。',
      '不建议在这里直接堆完整插件实现。',
    ],
    manualInputGuidance: [
      '推荐：描述漏洞类型、检测目标和期望输出。',
      '例如：生成一个检测未授权对象读取的流量分析插件。',
    ],
    dispatchPayloadGuidance: ['当前 Agent 不支持事件派发。'],
    manualInputTemplate: {
      vulnType: 'idor',
      goal: '生成一个检测对象边界越权风险的流量分析插件',
      constraints: ['输出可维护 TypeScript', '优先低误报规则'],
    },
  },
  plugin_fix_agent: {
    kind: 'plugin_fixer',
    description: '结合报错和测试结果修复插件，尽量保持原有契约不变。',
    supportsFindings: false,
    supportsDispatch: false,
    statsMode: 'versions',
    safetyMode: 'generic',
    promptPatchPlaceholder: [
      '示例：',
      '优先最小修改修复问题，并保持原始输出结构不变。',
      '避免引入新的依赖或额外副作用。',
    ],
    promptPatchGuidance: [
      '建议补充修复偏好和保持不变的契约。',
      '不建议把完整错误日志和代码都塞进这里。',
    ],
    manualInputGuidance: [
      '推荐：提供失败原因、原始代码和修复目标。',
      '优先描述“哪里坏了”和“修完后应保持什么不变”。',
    ],
    dispatchPayloadGuidance: ['当前 Agent 不支持事件派发。'],
    manualInputTemplate: {
      errorMessage: '插件执行测试失败',
      fixGoal: '修复错误但保持原始输出结构不变',
    },
  },
  traffic_logic_triage: {
    kind: 'traffic_triage',
    description: '在后台持续分析流量，发现越权、流程异常和业务逻辑风险。',
    passiveEventName: 'traffic.cluster.ready',
    supportsFindings: true,
    supportsBehaviorSource: true,
    supportsDispatch: true,
    statsMode: 'findings',
    safetyMode: 'triage',
    promptPatchPlaceholder: [
      '示例：',
      '优先关注订单、用户、组织、项目等对象边界字段。',
      '忽略公共搜索、静态资源和健康检查接口。',
    ],
    promptPatchGuidance: [
      '建议写关注点、忽略范围、保守策略。',
      '不建议重写 Agent 角色本身，尽量只做补充约束。',
    ],
    manualInputGuidance: [
      '推荐：仅在需要手动回放 triage 逻辑时使用。',
      '通常不需要手动构造完整 JSON，直接使用推荐模板即可。',
    ],
    dispatchPayloadGuidance: [
      '推荐：提供 host、pathTemplate、method 和 clusterKey。',
      '如需更稳定 triage，可额外补 requestId、actionKind、resourceKeys。',
    ],
    manualInputTemplate: {
      summary: '手动审计目标',
    },
    dispatchPayloadTemplate: {
      clusterKey: 'demo-cluster',
      host: 'example.com',
      pathTemplate: '/api/demo/{id}',
      method: 'GET',
      actionKind: 'read',
    },
  },
  traffic_active_verifier: {
    kind: 'traffic_verifier',
    description: '对已发现的风险做最小化安全验证，帮助确认是否值得人工跟进。',
    passiveEventName: 'traffic.hypothesis.ready',
    supportsFindings: false,
    supportsDispatch: true,
    statsMode: 'versions',
    safetyMode: 'verifier',
    promptPatchPlaceholder: [
      '示例：',
      '仅在状态码和响应结构高度相似时判定为验证成功。',
      '优先保守，避免对不稳定接口触发误报。',
    ],
    promptPatchGuidance: [
      '建议写验证阈值、保守偏好和作用域约束。',
      '不建议把完整验证逻辑改写到这里。',
    ],
    manualInputGuidance: [
      '推荐：提供要复验的 finding 或核心证据摘要。',
      '适合描述“请用最小化验证判断该风险是否值得人工继续跟进”。',
    ],
    dispatchPayloadGuidance: [
      '推荐：只提供 findingId，让验证链路基于已有证据执行。',
      '适合复验某个已落库的风险项。',
    ],
    manualInputTemplate: {
      summary: '请对该风险执行最小化安全验证',
    },
    dispatchPayloadTemplate: {
      findingId: 'finding-demo-id',
    },
  },
}

export function getSystemAgentUiDefinition(profile: AgentIdentity): SystemAgentUiDefinition {
  return SYSTEM_AGENT_UI_DEFINITIONS[profile.id] || GENERIC_DEFINITION
}

export function getSystemAgentDescription(profile: AgentIdentity): string {
  return getSystemAgentUiDefinition(profile).description || profile.description || '系统内置 Agent'
}

export function getSystemAgentPromptPatchPlaceholder(profile: AgentIdentity): string {
  return getSystemAgentUiDefinition(profile).promptPatchPlaceholder.join('\n')
}

export function getSystemAgentPromptPatchGuidance(profile: AgentIdentity): string {
  return getSystemAgentUiDefinition(profile).promptPatchGuidance.join('\n')
}

export function getSystemAgentPassiveEventName(profile: AgentIdentity): string {
  return getSystemAgentUiDefinition(profile).passiveEventName || 'traffic.cluster.ready'
}

export function getSystemAgentModeBadge(profile: AgentIdentity): SystemAgentModeBadge {
  if (profile.mode === 'passive') {
    return {
      label: '被动类',
      className: 'badge-warning',
    }
  }

  return {
    label: '主动类',
    className: 'badge-info',
  }
}

export function supportsSystemAgentFindings(profile: AgentIdentity): boolean {
  return getSystemAgentUiDefinition(profile).supportsFindings === true
}

export function supportsSystemAgentBehaviorSource(profile: AgentIdentity): boolean {
  return getSystemAgentUiDefinition(profile).supportsBehaviorSource === true
}

export function supportsSystemAgentDispatch(profile: AgentIdentity): boolean {
  return getSystemAgentUiDefinition(profile).supportsDispatch === true && profile.mode === 'passive'
}

export function isSystemAgentTriage(profile: AgentIdentity): boolean {
  return getSystemAgentUiDefinition(profile).kind === 'traffic_triage'
}

export function isSystemAgentVerifier(profile: AgentIdentity): boolean {
  return getSystemAgentUiDefinition(profile).kind === 'traffic_verifier'
}

export function getSystemAgentStatsMode(profile: AgentIdentity): SystemAgentStatsMode {
  return getSystemAgentUiDefinition(profile).statsMode
}

export function getSystemAgentSafetyMode(profile: AgentIdentity): SystemAgentSafetyMode {
  return getSystemAgentUiDefinition(profile).safetyMode
}

export function getSystemAgentManualInputGuidance(profile: AgentIdentity): string {
  return getSystemAgentUiDefinition(profile).manualInputGuidance.join('\n')
}

export function getSystemAgentDispatchPayloadGuidance(profile: AgentIdentity): string {
  if (profile.mode !== 'passive') {
    return '当前 Agent 为主动类，不需要事件载荷。'
  }
  return getSystemAgentUiDefinition(profile).dispatchPayloadGuidance.join('\n')
}

export function getSystemAgentManualInputTemplate(profile: AgentIdentity): string {
  return JSON.stringify(getSystemAgentUiDefinition(profile).manualInputTemplate, null, 2)
}

export function getSystemAgentDispatchPayloadTemplate(profile: AgentIdentity): string {
  const template = getSystemAgentUiDefinition(profile).dispatchPayloadTemplate
  if (!template) {
    return JSON.stringify(
      {
        clusterKey: 'demo-cluster',
        host: 'example.com',
        pathTemplate: '/api/demo',
        method: 'GET',
      },
      null,
      2,
    )
  }
  return JSON.stringify(template, null, 2)
}
