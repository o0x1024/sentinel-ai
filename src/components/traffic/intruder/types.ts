export type IntruderAttackType = 'sniper' | 'batteringRam' | 'pitchfork' | 'clusterBomb'

export interface IntruderSourceRequest {
  method: string
  url: string
  headers: Record<string, string>
  body?: string
}

export interface IntruderTarget {
  host: string
  port: number
  useTls: boolean
}

export interface IntruderPosition {
  index: number
  start: number
  end: number
  value: string
  preview: string
}

export type IntruderPayloadType =
  | 'simpleList'
  | 'numbers'
  | 'dates'
  | 'runtimeFile'
  | 'characterList'
  | 'nullPayloads'
  | 'characterSubstitution'
  | 'usernameGenerator'

export interface IntruderPayloadSet {
  id: string
  name: string
  payloadType: IntruderPayloadType
  payloadsText: string
  urlEncode: boolean
  filePath: string
  characterList: string
  substitutionSource: string
  substitutionRules: string
  numberFrom: number
  numberTo: number
  numberStep: number
  numberPadWidth: number
  dateFrom: string
  dateTo: string
  dateStepDays: number
  dateFormat: 'yyyy-MM-dd' | 'yyyyMMdd' | 'MM/dd/yyyy'
  nullCount: number
  nullValue: string
  usernameFirstNames: string
  usernameLastNames: string
  usernameFormats: string
}

export type IntruderPayloadProcessingRuleType =
  | 'prefix'
  | 'suffix'
  | 'replace'
  | 'replaceRegex'
  | 'lowercase'
  | 'uppercase'
  | 'trim'
  | 'base64'
  | 'urlEncode'
  | 'reverse'
  | 'removeWhitespace'
  | 'repeat'
  | 'hexEncode'

export type IntruderPayloadProcessingConditionType = 'always' | 'contains' | 'notContains' | 'regex'

export interface IntruderPayloadProcessingRule {
  id: string
  enabled: boolean
  type: IntruderPayloadProcessingRuleType
  matchValue: string
  replaceValue: string
  caseSensitive?: boolean
  conditionType?: IntruderPayloadProcessingConditionType
  conditionValue?: string
}

export interface IntruderGrepMatchRule {
  id: string
  name: string
  enabled: boolean
  pattern: string
  caseSensitive: boolean
  invert: boolean
}

export interface IntruderGrepExtractRule {
  id: string
  name: string
  enabled: boolean
  pattern: string
  groupIndex: number
  caseSensitive: boolean
}

export type IntruderResultColumnFilterOperator =
  | 'contains'
  | 'equals'
  | 'notEquals'
  | 'startsWith'
  | 'greaterThan'
  | 'lessThan'

export interface IntruderResultColumnFilter {
  id: string
  key: string
  operator: IntruderResultColumnFilterOperator
  value: string
}

export interface IntruderResultFilter {
  enabled: boolean
  query: string
  invert: boolean
  statusCode: string
  onlyErrors: boolean
  hideBaseline: boolean
  grepMatchRuleIds: string[]
  columnFilters: IntruderResultColumnFilter[]
}

export interface IntruderResultSort {
  key: string
  direction: 'asc' | 'desc'
}

export interface IntruderResourcePool {
  id: string
  name: string
  concurrency: number
  delayMs: number
  randomDelayMs: number
  builtIn?: boolean
}

export interface IntruderAttackOptions {
  concurrency: number
  delayMs: number
  randomDelayMs: number
  timeoutSecs: number
  maxRequests: number
  updateHostHeader: boolean
  updateContentLength: boolean
  setConnectionClose: boolean
  retryCount: number
  retryPauseMs: number
  storeRequests: boolean
  storeResponses: boolean
  storeFullPayloads: boolean
  denialOfServiceMode: boolean
  makeUnmodifiedBaseline: boolean
  autoPauseEnabled: boolean
  autoPauseMode: 'contains' | 'missing'
  autoPauseExpression: string
  autoPauseExpressions: string[]
}

export interface ParsedHttpRequest {
  method: string
  path: string
  protocol: string
  headers: Record<string, string>
  body: string
}

export interface ParsedHttpResponse {
  statusCode: number
  headers: Record<string, string>
  body: string
  responseTimeMs: number
}

export interface IntruderAttackCandidate {
  requestText: string
  payloadValues: string[]
  payloadSummary: string
  isBaseline?: boolean
}

export interface IntruderAttackPlan {
  requests: IntruderAttackCandidate[]
  totalGenerated: number
  truncated: boolean
}

export interface IntruderAttackProgress {
  total: number
  completed: number
  failed: number
  active: number
  truncated: boolean
}

export interface IntruderAttackResult {
  id: string
  index: number
  payloadSummary: string
  payloadValues: string[]
  statusCode: number | null
  responseLength: number
  wordCount: number
  lineCount: number
  responseTimeMs: number | null
  rawRequest: string
  rawResponse: string
  isBaseline?: boolean
  grepMatches: Record<string, boolean>
  grepExtracts: Record<string, string>
  error?: string
}

export interface IntruderResultsWindowState {
  workspaceId: string
  workspaceName: string
  target: IntruderTarget
  requestText: string
  positions: IntruderPosition[]
  results: IntruderAttackResult[]
  selectedResultId: string | null
  progress: IntruderAttackProgress
  isRunning: boolean
  captureFilter: IntruderResultFilter
  viewFilter: IntruderResultFilter
  sort: IntruderResultSort
  grepMatchRules: IntruderGrepMatchRule[]
  grepExtractRules: IntruderGrepExtractRule[]
  visibleColumns: string[]
}
