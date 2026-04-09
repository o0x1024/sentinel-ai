import type { SystemAgentProfilePayload, SystemAgentProfileSummary } from '../systemAgentSettingsSupport'

type AgentIdentity = Pick<SystemAgentProfilePayload, 'id' | 'description' | 'mode' | 'capability'>
  | Pick<SystemAgentProfileSummary, 'id' | 'description' | 'mode' | 'capability'>

export type SystemAgentUiKind =
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
  dispatchPayloadGuidance: string[]
  dispatchPayloadTemplate?: Record<string, unknown>
}

const GENERIC_DEFINITION: SystemAgentUiDefinition = {
  kind: 'generic',
  description: '系统智能体',
  supportsFindings: false,
  supportsDispatch: false,
  statsMode: 'versions',
  safetyMode: 'generic',
  promptPatchPlaceholder: [
    '示例：',
    '优先输出保守、可审阅、可追踪的分析结论。',
    '避免引入额外高风险动作或偏离既有事件契约。',
  ],
  promptPatchGuidance: [
    '建议补充分析重点、保守策略和作用域边界。',
    '不建议在这里堆完整 Prompt，优先补充而不是覆盖。',
  ],
  dispatchPayloadGuidance: ['当前系统智能体未声明测试事件模板。'],
}

const SYSTEM_AGENT_UI_DEFINITIONS: Record<string, SystemAgentUiDefinition> = {
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
    dispatchPayloadGuidance: [
      '推荐：提供 host、pathTemplate、method 和 clusterKey。',
      '如需更稳定 triage，可额外补 requestId、actionKind、resourceKeys。',
    ],
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
    dispatchPayloadGuidance: [
      '推荐：只提供 findingId，让验证链路基于已有证据执行。',
      '适合复验某个已落库的风险项。',
    ],
    dispatchPayloadTemplate: {
      findingId: 'finding-demo-id',
    },
  },
}

export function getSystemAgentUiDefinition(profile: AgentIdentity): SystemAgentUiDefinition {
  return SYSTEM_AGENT_UI_DEFINITIONS[profile.id] || GENERIC_DEFINITION
}

export function getSystemAgentDescription(profile: AgentIdentity): string {
  return getSystemAgentUiDefinition(profile).description || profile.description || '系统智能体'
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

export function getSystemAgentModeBadge(_profile: AgentIdentity): SystemAgentModeBadge {
  return {
    label: '事件驱动',
    className: 'badge-warning',
  }
}

export function supportsSystemAgentFindings(profile: AgentIdentity): boolean {
  return getSystemAgentUiDefinition(profile).supportsFindings === true
}

export function supportsSystemAgentBehaviorSource(profile: AgentIdentity): boolean {
  return getSystemAgentUiDefinition(profile).supportsBehaviorSource === true
}

export function supportsSystemAgentDispatch(profile: AgentIdentity): boolean {
  return getSystemAgentUiDefinition(profile).supportsDispatch === true
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

export function getSystemAgentDispatchPayloadGuidance(profile: AgentIdentity): string {
  return getSystemAgentUiDefinition(profile).dispatchPayloadGuidance.join('\n')
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
