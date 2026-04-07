export type IntruderAttackType = 'sniper' | 'batteringRam' | 'pitchfork' | 'clusterBomb'
export type IntruderPatternType = 'literal' | 'regex'

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
  | 'extensionGenerated'
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
  urlEncodeCharacters: string
  pluginId: string
  pluginPresetName: string
  pluginConfig: string
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

export type IntruderPluginProcessorCategory = 'payload_processor' | 'request_processor'

export interface IntruderPluginProcessorBinding {
  id: string
  pluginId: string
  presetName: string
  enabled: boolean
  config: string
}

export type IntruderPayloadProcessingRuleType =
  | 'prefix'
  | 'suffix'
  | 'replace'
  | 'replaceRegex'
  | 'substring'
  | 'reverseSubstring'
  | 'lowercase'
  | 'uppercase'
  | 'trim'
  | 'base64'
  | 'urlEncode'
  | 'decode'
  | 'hash'
  | 'addRawPayload'
  | 'replaceBaseValue'
  | 'reverse'
  | 'removeWhitespace'
  | 'repeat'
  | 'hexEncode'
  | 'skipRegex'

export type IntruderPayloadProcessingCodec = 'url' | 'base64' | 'hex'
export type IntruderPayloadProcessingHashAlgorithm = 'sha1' | 'sha256'
export type IntruderPayloadProcessingRawPayloadPlacement = 'before' | 'after'

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
  substringStart?: number
  substringLength?: number | null
  codecType?: IntruderPayloadProcessingCodec
  hashAlgorithm?: IntruderPayloadProcessingHashAlgorithm
  rawPayloadPlacement?: IntruderPayloadProcessingRawPayloadPlacement
}

export interface IntruderGrepMatchRule {
  id: string
  name: string
  enabled: boolean
  pattern: string
  patternType: IntruderPatternType
  caseSensitive: boolean
  excludeHeaders: boolean
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

export interface IntruderGrepPayloadSettings {
  enabled: boolean
  caseSensitive: boolean
  excludeHeaders: boolean
  matchUrlEncoded: boolean
}

export interface IntruderRedirectHop {
  url: string
  statusCode: number
  location: string | null
  setCookieCount: number
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
  concurrencyEnabled: boolean
  concurrency: number
  delayEnabled: boolean
  delayMs: number
  randomDelayEnabled: boolean
  randomDelayMs: number
  delayIncrementEnabled: boolean
  delayIncrementMs: number
  autoThrottleEnabled: boolean
  autoThrottleStatusCodes: number[]
  builtIn?: boolean
}

export interface IntruderAttackOptions {
  concurrency: number
  delayMs: number
  randomDelayMs: number
  delayIncrementMs: number
  autoThrottleEnabled: boolean
  autoThrottleStatusCodes: number[]
  timeoutSecs: number
  maxRequests: number
  updateHostHeader: boolean
  updateContentLength: boolean
  setConnectionClose: boolean
  followRedirects: boolean
  maxRedirects: number
  processCookiesInRedirects: boolean
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
  redirectCount: number
  finalUrl: string
  redirectChain: IntruderRedirectHop[]
  isBaseline?: boolean
  payloadReflectionCount: number
  grepMatches: Record<string, number>
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
  grepPayloadSettings: IntruderGrepPayloadSettings
  visibleColumns: string[]
}
