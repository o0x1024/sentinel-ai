import { createIntruderId } from './http'
import type {
  IntruderAttackResult,
  IntruderGrepExtractRule,
  IntruderGrepMatchRule,
  IntruderGrepPayloadSettings,
  IntruderPatternType,
} from './types'

export interface IntruderResultColumn {
  key: string
  label: string
  kind: 'base' | 'match' | 'extract'
}

export const INTRUDER_BASE_RESULT_COLUMNS: IntruderResultColumn[] = [
  { key: 'index', label: 'Request', kind: 'base' },
  { key: 'payloadSummary', label: 'Payload', kind: 'base' },
  { key: 'payloadReflectionCount', label: 'Reflections', kind: 'base' },
  { key: 'redirectCount', label: 'Redirects', kind: 'base' },
  { key: 'statusCode', label: 'Status', kind: 'base' },
  { key: 'responseTimeMs', label: 'Response time', kind: 'base' },
  { key: 'error', label: 'Error', kind: 'base' },
  { key: 'responseLength', label: 'Length', kind: 'base' },
]

export function createDefaultGrepMatchRule(): IntruderGrepMatchRule {
  return {
    id: createIntruderId('grep-match'),
    name: 'Match',
    enabled: true,
    pattern: '',
    patternType: 'literal',
    caseSensitive: false,
    excludeHeaders: false,
    invert: false,
  }
}

export function normalizeGrepMatchRule(
  value?: Partial<IntruderGrepMatchRule> | null,
): IntruderGrepMatchRule {
  return {
    ...createDefaultGrepMatchRule(),
    ...(value || {}),
  }
}

export function createDefaultGrepPayloadSettings(): IntruderGrepPayloadSettings {
  return {
    enabled: false,
    caseSensitive: false,
    excludeHeaders: false,
    matchUrlEncoded: true,
  }
}

export function normalizeGrepPayloadSettings(
  value?: Partial<IntruderGrepPayloadSettings> | null,
): IntruderGrepPayloadSettings {
  return {
    ...createDefaultGrepPayloadSettings(),
    ...(value || {}),
  }
}

export function createDefaultGrepExtractRule(): IntruderGrepExtractRule {
  return {
    id: createIntruderId('grep-extract'),
    name: 'Extract',
    enabled: true,
    pattern: '',
    groupIndex: 1,
    caseSensitive: false,
  }
}

export function evaluateIntruderGrepMatches(
  rawResponse: string,
  rules: IntruderGrepMatchRule[],
): Record<string, number> {
  return Object.fromEntries(
    rules.map((rule) => {
      const count = evaluateMatchRule(rawResponse, rule)
      return [rule.id, count]
    }),
  )
}

export function evaluateIntruderGrepExtracts(
  rawResponse: string,
  rules: IntruderGrepExtractRule[],
): Record<string, string> {
  return Object.fromEntries(
    rules.map((rule) => {
      const extracted = evaluateExtractRule(rawResponse, rule)
      return [rule.id, extracted]
    }),
  )
}

export function getIntruderResultColumns(
  grepMatchRules: IntruderGrepMatchRule[],
  grepExtractRules: IntruderGrepExtractRule[],
  grepPayloadSettings: IntruderGrepPayloadSettings = createDefaultGrepPayloadSettings(),
): IntruderResultColumn[] {
  const baseColumns = grepPayloadSettings.enabled
    ? INTRUDER_BASE_RESULT_COLUMNS
    : INTRUDER_BASE_RESULT_COLUMNS.filter((column) => column.key !== 'payloadReflectionCount')
  const matchColumns = grepMatchRules.map((rule) => ({
    key: `match:${rule.id}`,
    label: rule.name || 'Match',
    kind: 'match' as const,
  }))
  const extractColumns = grepExtractRules.map((rule) => ({
    key: `extract:${rule.id}`,
    label: rule.name || 'Extract',
    kind: 'extract' as const,
  }))
  return [...baseColumns, ...matchColumns, ...extractColumns]
}

export function createDefaultVisibleColumns(
  grepMatchRules: IntruderGrepMatchRule[],
  grepExtractRules: IntruderGrepExtractRule[],
  grepPayloadSettings: IntruderGrepPayloadSettings = createDefaultGrepPayloadSettings(),
): string[] {
  return getIntruderResultColumns(grepMatchRules, grepExtractRules, grepPayloadSettings).map((column) => column.key)
}

export function getIntruderResultColumnValue(result: IntruderAttackResult, key: string): string | number | boolean | null {
  switch (key) {
    case 'index':
      return result.index - 1
    case 'payloadSummary':
      return result.payloadSummary
    case 'payloadReflectionCount':
      return result.payloadReflectionCount
    case 'redirectCount':
      return result.redirectCount
    case 'statusCode':
      return result.statusCode
    case 'responseTimeMs':
      return result.responseTimeMs
    case 'error':
      return result.error ?? ''
    case 'responseLength':
      return result.responseLength
    default:
      break
  }

  if (key.startsWith('match:')) {
    return result.grepMatches[key.slice('match:'.length)] ?? false
  }

  if (key.startsWith('extract:')) {
    return result.grepExtracts[key.slice('extract:'.length)] ?? ''
  }

  return null
}

export function evaluateIntruderPayloadReflections(
  rawResponse: string,
  payloadValues: string[],
  settings: IntruderGrepPayloadSettings,
): number {
  if (!settings.enabled) return 0

  const searchTarget = getSearchTarget(rawResponse, settings.excludeHeaders)
  const uniquePayloads = Array.from(
    new Set(
      payloadValues
        .map((value) => value.trim())
        .filter((value) => value.length > 0),
    ),
  )

  if (!uniquePayloads.length) return 0

  return uniquePayloads.reduce((total, payload) => {
    const variants = new Set([payload])
    if (settings.matchUrlEncoded) {
      variants.add(encodeURIComponent(payload))
    }

    for (const variant of variants) {
      const count = countLiteralMatches(searchTarget, variant, settings.caseSensitive)
      if (count > 0) {
        return total + count
      }
    }

    return total
  }, 0)
}

function evaluateMatchRule(rawResponse: string, rule: IntruderGrepMatchRule): number {
  if (!rule.enabled || !rule.pattern.trim()) return 0

  const searchTarget = getSearchTarget(rawResponse, rule.excludeHeaders)
  const matchCount = countPatternMatches(searchTarget, rule.pattern, rule.patternType, rule.caseSensitive)
  if (rule.invert) {
    return matchCount === 0 ? 1 : 0
  }

  return matchCount
}

function getSearchTarget(rawResponse: string, excludeHeaders: boolean): string {
  if (!excludeHeaders) return rawResponse

  const headerSeparator = rawResponse.indexOf('\r\n\r\n')
  if (headerSeparator !== -1) {
    return rawResponse.slice(headerSeparator + 4)
  }

  const normalizedSeparator = rawResponse.indexOf('\n\n')
  if (normalizedSeparator !== -1) {
    return rawResponse.slice(normalizedSeparator + 2)
  }

  return rawResponse
}

function countPatternMatches(
  source: string,
  pattern: string,
  patternType: IntruderPatternType,
  caseSensitive: boolean,
): number {
  if (!pattern.trim()) return 0

  if (patternType === 'literal') {
    return countLiteralMatches(source, pattern, caseSensitive)
  }

  return countRegexMatches(source, pattern, caseSensitive)
}

function countLiteralMatches(source: string, pattern: string, caseSensitive: boolean): number {
  if (!pattern) return 0

  const haystack = caseSensitive ? source : source.toLowerCase()
  const needle = caseSensitive ? pattern : pattern.toLowerCase()
  let count = 0
  let offset = 0

  while (offset <= haystack.length - needle.length) {
    const nextIndex = haystack.indexOf(needle, offset)
    if (nextIndex === -1) break
    count += 1
    offset = nextIndex + Math.max(needle.length, 1)
  }

  return count
}

function countRegexMatches(source: string, pattern: string, caseSensitive: boolean): number {
  const regex = buildSafeRegex(pattern, caseSensitive, true)
  let count = 0

  while (true) {
    const match = regex.exec(source)
    if (!match) break

    count += 1
    if (match[0].length === 0) {
      regex.lastIndex += 1
    }
  }

  return count
}

function evaluateExtractRule(rawResponse: string, rule: IntruderGrepExtractRule): string {
  if (!rule.enabled || !rule.pattern.trim()) return ''

  const match = rawResponse.match(buildSafeRegex(rule.pattern, rule.caseSensitive))
  if (!match) return ''
  return match[rule.groupIndex] ?? match[0] ?? ''
}

function buildSafeRegex(pattern: string, caseSensitive: boolean, global = false): RegExp {
  const flags = `${caseSensitive ? '' : 'i'}${global ? 'g' : ''}`

  try {
    return new RegExp(pattern, flags)
  } catch {
    const escaped = pattern.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
    return new RegExp(escaped, flags)
  }
}
