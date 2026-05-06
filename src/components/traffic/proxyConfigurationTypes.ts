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
  scope_include_rules: ProxyScopeRule[]
  scope_exclude_rules: ProxyScopeRule[]
  match_replace_rules: MatchReplaceRule[]
}

export interface ProxyScopeRule {
  enabled: boolean
  protocol: string
  host_or_ip_range: string
  port: string
  file: string
}

export type TrafficBehaviorSignalMode = 'proxy_inferred' | 'browser_extension'

export interface TrafficBehaviorSignalSettings {
  mode: TrafficBehaviorSignalMode
  browserExtensionConnected: boolean
  browserExtensionLastSeenAt: string | null
}

export interface TrafficOastConfig {
  enabled: boolean
  serverBaseUrl: string
  apiKey: string
  pollIntervalSecs: number
  requestTimeoutSecs: number
}

export interface TrafficPluginActiveProbeSettings {
  maxQueueDepth: number
  maxPendingPerRun: number
  maxPendingPerPlugin: number
  maxGlobalConcurrent: number
  jitterRange: [number, number]
  minHostCooldownMs: number
  maxConcurrentPerHost: number
  maxConcurrentPerRun: number
  maxConcurrentPerPlugin: number
  timeoutMs: number
}

export interface TrafficPluginFetchPolicySettings {
  maxQueueDepth: number
  maxPendingPerRun: number
  maxPendingPerPlugin: number
  maxGlobalConcurrent: number
  maxConcurrentPerHost: number
  maxConcurrentPerRun: number
  maxConcurrentPerPlugin: number
  minHostDelayMs: number
  jitterRange: [number, number]
  timeoutMs: number
}

export interface TrafficPluginRuntimeSettings {
  activeProbe: TrafficPluginActiveProbeSettings
  bountyFetch: TrafficPluginFetchPolicySettings
  monitorFetch: TrafficPluginFetchPolicySettings
  agentFetch: TrafficPluginFetchPolicySettings
  pluginTestFetch: TrafficPluginFetchPolicySettings
}

export interface TrafficOastTestResult {
  reachable: boolean
  message: string
  generatedToken: string | null
  generatedFqdn: string | null
}

export interface TrafficOastEvent {
  time: string
  host: string
  method: string
  url: string
  path: string
  query: unknown
  userAgent: string
  referer: string
  ip: string
  ray: string
  colo: string
  country: string
  asn: number | null
}

export interface TrafficOastEventKey {
  time: string
  method: string
  url: string
  ip: string
}

export interface TrafficOastRecord {
  token: string
  fqdn: string
  httpUrl: string
  httpsUrl: string
  createdAt: string
  label: string
  sourceTool: string
  sourceRequestId: number | null
  hitCount: number
  lastHitAt: string | null
  lastSyncAt: string | null
  events: TrafficOastEvent[]
}

export interface DeleteTrafficOastRecordResult {
  token: string
  remoteDeletedAll: boolean
  localRemoved: boolean
}

export interface HideTrafficOastEventsResult {
  token: string
  hiddenCount: number
  visibleEventCount: number
  record: TrafficOastRecord
}

export interface TrafficContextExtractionSettings {
  principalKeys: string[]
  resourceKeyHints: string[]
  authHeaderKeys: string[]
  authTokenKeys: string[]
  cookieHintKeys: string[]
  actionAliases: Record<string, string[]>
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
    scope_include_rules: [],
    scope_exclude_rules: [],
    match_replace_rules: [],
  }
}

export function createDefaultProxyScopeRule(): ProxyScopeRule {
  return {
    enabled: true,
    protocol: 'any',
    host_or_ip_range: '',
    port: '',
    file: '',
  }
}

export function createDefaultTrafficBehaviorSignalSettings(): TrafficBehaviorSignalSettings {
  return {
    mode: 'proxy_inferred',
    browserExtensionConnected: false,
    browserExtensionLastSeenAt: null,
  }
}

export function createDefaultTrafficOastConfig(): TrafficOastConfig {
  return {
    enabled: false,
    serverBaseUrl: '',
    apiKey: '',
    pollIntervalSecs: 15,
    requestTimeoutSecs: 10,
  }
}

export function createDefaultTrafficPluginRuntimeSettings(): TrafficPluginRuntimeSettings {
  return {
    activeProbe: {
      maxQueueDepth: 1000,
      maxPendingPerRun: 250,
      maxPendingPerPlugin: 500,
      maxGlobalConcurrent: 20,
      jitterRange: [300, 1000],
      minHostCooldownMs: 1000,
      maxConcurrentPerHost: 2,
      maxConcurrentPerRun: 6,
      maxConcurrentPerPlugin: 10,
      timeoutMs: 8000,
    },
    bountyFetch: {
      maxQueueDepth: 1000,
      maxPendingPerRun: 250,
      maxPendingPerPlugin: 500,
      maxGlobalConcurrent: 20,
      maxConcurrentPerHost: 2,
      maxConcurrentPerRun: 6,
      maxConcurrentPerPlugin: 10,
      minHostDelayMs: 1000,
      jitterRange: [300, 1000],
      timeoutMs: 8000,
    },
    monitorFetch: {
      maxQueueDepth: 500,
      maxPendingPerRun: 100,
      maxPendingPerPlugin: 250,
      maxGlobalConcurrent: 8,
      maxConcurrentPerHost: 1,
      maxConcurrentPerRun: 3,
      maxConcurrentPerPlugin: 4,
      minHostDelayMs: 2000,
      jitterRange: [500, 2000],
      timeoutMs: 15000,
    },
    agentFetch: {
      maxQueueDepth: 300,
      maxPendingPerRun: 75,
      maxPendingPerPlugin: 150,
      maxGlobalConcurrent: 10,
      maxConcurrentPerHost: 2,
      maxConcurrentPerRun: 4,
      maxConcurrentPerPlugin: 6,
      minHostDelayMs: 500,
      jitterRange: [100, 500],
      timeoutMs: 15000,
    },
    pluginTestFetch: {
      maxQueueDepth: 50,
      maxPendingPerRun: 20,
      maxPendingPerPlugin: 30,
      maxGlobalConcurrent: 2,
      maxConcurrentPerHost: 1,
      maxConcurrentPerRun: 2,
      maxConcurrentPerPlugin: 2,
      minHostDelayMs: 100,
      jitterRange: [0, 100],
      timeoutMs: 5000,
    },
  }
}

export function createDefaultTrafficContextExtractionSettings(): TrafficContextExtractionSettings {
  return {
    principalKeys: [],
    resourceKeyHints: [],
    authHeaderKeys: [],
    authTokenKeys: [],
    cookieHintKeys: [],
    actionAliases: {},
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
  return []
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
