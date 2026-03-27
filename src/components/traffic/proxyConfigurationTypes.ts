export interface UpstreamProxyConfig {
  enabled: boolean
  destination_host: string
  proxy_host: string
  proxy_port: number
  auth_type: string
  username?: string
  password?: string
}

export interface ProxyConfigState {
  start_port: number
  max_port_attempts: number
  mitm_enabled: boolean
  max_request_body_size: number
  max_response_body_size: number
  upstream_proxy: UpstreamProxyConfig | null
  exclude_self_traffic: boolean
}

export interface ProxyListener {
  running: boolean
  interface: string
  invisible: boolean
  redirect: boolean
  certificate: string
  tlsProtocols: string
  supportHTTP2: boolean
}

export interface InterceptionRule {
  enabled: boolean
  operator: string
  matchType: string
  relationship: string
  condition: string
}

export interface MatchReplaceRule {
  enabled: boolean
  type: string
  match: string
  replace: string
  scope: string
  item: string
  comment: string
}

export interface MatchReplaceEditorState {
  enabled: boolean
  type: string
  match: string
  replace: string
  comment: string
}

export interface TlsPassThroughRule {
  enabled: boolean
  destination: string
  protocol: string
  host: string
  port: string
}

export interface TlsPassThroughEditorState {
  enabled: boolean
  host: string
  port: string
}

export interface EditingListenerState {
  host: string
  port: number
  certificate: string
  tlsProtocols: string
  supportHTTP2: boolean
  invisible: boolean
  redirect: boolean
}

export interface FilterRulePayload {
  ruleType: 'request' | 'response'
  rule: InterceptionRule
}

export const requestMatchTypeValues = [
  'domain_name',
  'ip_address',
  'protocol',
  'http_method',
  'url',
  'file_extension',
  'request',
  'cookie_name',
  'cookie_value',
  'any_header',
  'body',
  'param_name',
  'param_value',
  'listener_port',
]

export const responseMatchTypeValues = [
  'domain_name',
  'ip_address',
  'protocol',
  'http_method',
  'url',
  'file_extension',
  'request',
  'cookie_name',
  'cookie_value',
  'any_header',
  'body',
  'param_name',
  'param_value',
  'status_code',
  'content_type_header',
]

export const relationshipValues = [
  'matches',
  'does_not_match',
  'contains_parameters',
  'is_in_target_scope',
  'was_modified',
  'was_intercepted',
]

export function createDefaultProxyConfig(): ProxyConfigState {
  return {
    start_port: 8080,
    max_port_attempts: 10,
    mitm_enabled: true,
    max_request_body_size: 2 * 1024 * 1024,
    max_response_body_size: 2 * 1024 * 1024,
    upstream_proxy: null,
    exclude_self_traffic: true,
  }
}

export function createDefaultProxyListener(port = 8080): ProxyListener {
  return {
    running: true,
    interface: `127.0.0.1:${port}`,
    invisible: false,
    redirect: false,
    certificate: 'Per-host',
    tlsProtocols: 'Default',
    supportHTTP2: true,
  }
}

export function createDefaultRequestRules(): InterceptionRule[] {
  return [
    {
      enabled: true,
      operator: '',
      matchType: 'file_extension',
      relationship: 'does_not_match',
      condition: '(^gif$|^jpg$|^png$|^css$|^js$|^ico$|^woff$)',
    },
    {
      enabled: false,
      operator: 'Or',
      matchType: 'request',
      relationship: 'contains_parameters',
      condition: '',
    },
    {
      enabled: false,
      operator: 'Or',
      matchType: 'http_method',
      relationship: 'does_not_match',
      condition: '(get|post)',
    },
    {
      enabled: false,
      operator: 'And',
      matchType: 'url',
      relationship: 'is_in_target_scope',
      condition: '',
    },
  ]
}

export function createDefaultResponseRules(): InterceptionRule[] {
  return [
    {
      enabled: true,
      operator: '',
      matchType: 'content_type_header',
      relationship: 'matches',
      condition: 'text',
    },
    {
      enabled: false,
      operator: 'Or',
      matchType: 'request',
      relationship: 'was_modified',
      condition: '',
    },
    {
      enabled: false,
      operator: 'Or',
      matchType: 'request',
      relationship: 'was_intercepted',
      condition: '',
    },
    {
      enabled: false,
      operator: 'And',
      matchType: 'status_code',
      relationship: 'does_not_match',
      condition: '^304$',
    },
    {
      enabled: false,
      operator: 'And',
      matchType: 'url',
      relationship: 'is_in_target_scope',
      condition: '',
    },
  ]
}

export function createDefaultEditingRule(): InterceptionRule {
  return {
    enabled: true,
    operator: '',
    matchType: 'domain_name',
    relationship: 'matches',
    condition: '',
  }
}

export function createDefaultEditingUpstream(): UpstreamProxyConfig {
  return {
    enabled: false,
    destination_host: '*',
    proxy_host: '',
    proxy_port: 8080,
    auth_type: '',
    username: '',
    password: '',
  }
}

export function createDefaultEditingMatchReplace(): MatchReplaceEditorState {
  return {
    enabled: true,
    type: 'Request header',
    match: '',
    replace: '',
    comment: '',
  }
}

export function createDefaultEditingTlsPassThrough(): TlsPassThroughEditorState {
  return {
    enabled: true,
    host: '',
    port: '443',
  }
}

export function createDefaultEditingListener(): EditingListenerState {
  return {
    host: '127.0.0.1',
    port: 8080,
    certificate: 'Per-host',
    tlsProtocols: 'Default',
    supportHTTP2: true,
    invisible: false,
    redirect: false,
  }
}

export function createDefaultMatchReplaceRules(): MatchReplaceRule[] {
  return [
    {
      enabled: true,
      type: 'Request header',
      match: 'User-Agent:.*',
      replace: 'User-Agent: Mozilla/5.0',
      scope: 'In scope',
      item: '1',
      comment: '修改 User-Agent',
    },
  ]
}

export function createDefaultTlsPassThroughRules(): TlsPassThroughRule[] {
  return [
    {
      enabled: true,
      destination: '*',
      protocol: 'TLS',
      host: '*.example.com',
      port: '443',
    },
  ]
}
