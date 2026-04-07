export interface BehaviorSummary {
  label: string
  effectiveModeLabel: string
  usedExtension: boolean
  summary: string
  steps: string[]
  intentHints: string[]
  lastPageTitle: string
  lastPageUrl: string
}

export function parseStructuredPayload(raw?: string) {
  if (!raw) return null
  try {
    return JSON.parse(raw)
  } catch {
    return null
  }
}

export function parseSystemAgentMeta(raw?: string) {
  return parseStructuredPayload(raw)
}

export function formatStructuredValue(value: unknown) {
  if (value == null) return ''
  return JSON.stringify(value, null, 2)
}

export function getContextSkillEntriesFromPayload(payload: unknown) {
  const record = asRecord(payload)
  const skills = record?.logicSkillContext
  return Array.isArray(skills) ? skills : []
}

export function getContextInvariantEntriesFromPayload(payload: unknown) {
  const record = asRecord(payload)
  const invariants = record?.logicInvariants
  return Array.isArray(invariants) ? invariants : []
}

export function getContextProcessGraphFromPayload(payload: unknown) {
  const record = asRecord(payload)
  return record?.processGraph ?? null
}

export function getContextBehaviorSummaryFromPayload(payload: unknown): BehaviorSummary | null {
  const record = asRecord(payload)
  if (!record) return null

  const behaviorSession = asRecord(record.behaviorSession)
  const browserExtensionBehavior = asRecord(record.browserExtensionBehavior)
  const behaviorSignal = asRecord(record.behaviorSignal)
  const effectiveMode =
    stringValue(behaviorSession?.effectiveMode) ||
    stringValue(behaviorSignal?.effectiveMode) ||
    'proxy_inferred'
  const selectedMode =
    stringValue(behaviorSession?.selectedMode) ||
    stringValue(behaviorSignal?.selectedMode) ||
    effectiveMode
  const steps = stringArray(behaviorSession?.behaviorSteps)
  const intentHints = stringArray(behaviorSession?.intentHints)
  const browserExtensionEvents = Array.isArray(browserExtensionBehavior?.events)
    ? browserExtensionBehavior.events
    : []

  const usedExtension =
    effectiveMode === 'browser_extension' ||
    browserExtensionBehavior?.kind === 'browser_extension' ||
    browserExtensionEvents.length > 0

  const label = usedExtension ? '浏览器行为增强' : '代理弱行为推断'
  const effectiveModeLabel = usedExtension
    ? '浏览器扩展行为采集'
    : selectedMode === 'browser_extension'
      ? '扩展未连接，已回退为代理推断'
      : '仅代理弱行为推断'

  return {
    label,
    effectiveModeLabel,
    usedExtension,
    summary:
      stringValue(behaviorSession?.summary) || '本次判定使用了行为会话来增强逻辑漏洞分析。',
    steps,
    intentHints,
    lastPageTitle: stringValue(browserExtensionBehavior?.lastPageTitle) || '',
    lastPageUrl: stringValue(browserExtensionBehavior?.lastPageUrl) || '',
  }
}

function asRecord(value: unknown): Record<string, unknown> | null {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return null
  return value as Record<string, unknown>
}

function stringArray(raw: unknown): string[] {
  if (!Array.isArray(raw)) return []
  return raw.filter((item): item is string => typeof item === 'string' && item.length > 0)
}

function stringValue(value: unknown) {
  return typeof value === 'string' ? value : ''
}
