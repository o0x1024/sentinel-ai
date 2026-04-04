export interface SystemAgentProfileSummary {
  id: string
  name: string
  description: string
  mode: 'active' | 'passive' | string
  capability: string
  enabled: boolean
  triggerMode: string
  riskLevel: string
  visibility: string
  updatedAt: string
}

export interface SystemAgentBindingPayload {
  id?: string
  profileId?: string
  eventName: string
  filter?: Record<string, unknown> | null
  filterText?: string
  priority: number
  enabled: boolean
  createdAt?: string
  updatedAt?: string
}

export interface SystemAgentProfilePayload {
  id: string
  name: string
  description: string
  mode: 'active' | 'passive' | string
  capability: string
  enabled: boolean
  triggerMode: string
  basePromptId?: string | null
  promptPatch?: string | null
  inputSchema?: Record<string, unknown> | null
  outputSchema?: Record<string, unknown> | null
  requiredTools: string[]
  optionalTools: string[]
  forbiddenTools: string[]
  triggerEvents: string[]
  budget?: Record<string, unknown> | null
  safetyPolicy?: Record<string, unknown> | null
  cooldownSecs: number
  maxConcurrency: number
  riskLevel: string
  visibility: string
  bindings: SystemAgentBindingPayload[]
  createdAt?: string
  updatedAt?: string
}

export interface SystemAgentRunPayload {
  id: string
  profileId: string
  triggerEvent?: string | null
  status: string
  inputSummary?: Record<string, unknown> | null
  output?: Record<string, unknown> | null
  errorMessage?: string | null
  startedAt: string
  finishedAt?: string | null
  createdAt: string
  updatedAt: string
}

export interface SystemAgentProfileVersionPayload {
  id: string
  profileId: string
  snapshot?: Record<string, unknown> | null
  createdAt: string
}

export interface SystemAgentSafetyPolicyForm {
  allowActiveReplay: boolean
  autoMode: boolean
  shadowMode: boolean
  scopeHostsText: string
}

export interface SystemAgentFindingSummary {
  id: string
  plugin_id: string
  vuln_type: string
  severity: string
  confidence: string
  title: string
  status: string
  hit_count: number
  last_seen_at: string
  url?: string | null
  evidence?: Array<{
    id: string
    location: string
    request_body?: string | null
    response_body?: string | null
  }>
}

export interface SystemAgentToolMetadata {
  id: string
  name: string
  description: string
  category: string
  virtual?: boolean
}

export interface SystemAgentListItem {
  id: string
  name: string
  description: string
  enabled: boolean
  modeBadge: {
    label: string
    className: string
  }
}

export interface CommandResponse<T> {
  success: boolean
  data?: T
  error?: string | null
}

export const SYSTEM_AGENT_VIRTUAL_TOOLS: SystemAgentToolMetadata[] = [
  {
    id: 'traffic_history_reader',
    name: 'Traffic History Reader',
    description: '读取历史流量摘要和上下文，供主动流量审计 Agent 使用。',
    category: 'security',
    virtual: true,
  },
  {
    id: 'repeater_launcher',
    name: 'Repeater Launcher',
    description: '触发重放器相关流程或生成重放建议。',
    category: 'network',
    virtual: true,
  },
  {
    id: 'workflow_catalog_reader',
    name: 'Workflow Catalog Reader',
    description: '读取现有工作流模板和节点能力目录。',
    category: 'workflow',
    virtual: true,
  },
  {
    id: 'tool_catalog_reader',
    name: 'Tool Catalog Reader',
    description: '读取系统工具目录，供工作流设计 Agent 参考。',
    category: 'system',
    virtual: true,
  },
  {
    id: 'plugin_prompt_reader',
    name: 'Plugin Prompt Reader',
    description: '读取插件生成相关提示词模板。',
    category: 'plugin',
    virtual: true,
  },
  {
    id: 'plugin_example_reader',
    name: 'Plugin Example Reader',
    description: '读取插件示例代码和 few-shot 参考。',
    category: 'plugin',
    virtual: true,
  },
  {
    id: 'plugin_validator',
    name: 'Plugin Validator',
    description: '执行插件静态校验和结构合法性检查。',
    category: 'plugin',
    virtual: true,
  },
  {
    id: 'plugin_test_result_reader',
    name: 'Plugin Test Result Reader',
    description: '读取插件执行测试结果，辅助修复 Agent 分析失败原因。',
    category: 'plugin',
    virtual: true,
  },
  {
    id: 'traffic_cluster_reader',
    name: 'Traffic Cluster Reader',
    description: '读取流量聚类摘要、同类请求和上下文簇信息。',
    category: 'security',
    virtual: true,
  },
  {
    id: 'auth_context_diff',
    name: 'Auth Context Diff',
    description: '比较不同身份、租户或会话上下文的流量差异。',
    category: 'security',
    virtual: true,
  },
  {
    id: 'workflow_state_reader',
    name: 'Workflow State Reader',
    description: '读取流程状态线索和业务状态迁移摘要。',
    category: 'workflow',
    virtual: true,
  },
  {
    id: 'traffic_sequence_reader',
    name: 'Traffic Sequence Reader',
    description: '读取近期请求序列和动作顺序上下文。',
    category: 'security',
    virtual: true,
  },
  {
    id: 'active_replay',
    name: 'Active Replay',
    description: '执行最小化安全重放验证。属于高风险系统工具。',
    category: 'security',
    virtual: true,
  },
]

export const createEmptySystemAgentProfile = (): SystemAgentProfilePayload => ({
  id: '',
  name: '',
  description: '',
  mode: 'active',
  capability: 'designer',
  enabled: true,
  triggerMode: 'manual',
  basePromptId: null,
  promptPatch: '',
  inputSchema: {},
  outputSchema: {},
  requiredTools: [],
  optionalTools: [],
  forbiddenTools: [],
  triggerEvents: [],
  budget: {},
  safetyPolicy: {},
  cooldownSecs: 0,
  maxConcurrency: 1,
  riskLevel: 'medium',
  visibility: 'system',
  bindings: [],
})

export const splitCommaLines = (value: string): string[] =>
  value
    .split(/[\n,]/)
    .map(item => item.trim())
    .filter(Boolean)

export const joinCommaLines = (items: string[] | undefined): string =>
  (items ?? []).join('\n')

export const prettyJson = (value: unknown): string => {
  try {
    return JSON.stringify(value ?? {}, null, 2)
  } catch {
    return '{}'
  }
}

export const parseJsonText = (value: string, fallback: Record<string, unknown> | null = {}): Record<string, unknown> | null => {
  const trimmed = value.trim()
  if (!trimmed) return fallback
  return JSON.parse(trimmed)
}

export const cloneProfile = (profile: SystemAgentProfilePayload): SystemAgentProfilePayload =>
  JSON.parse(JSON.stringify(profile))
