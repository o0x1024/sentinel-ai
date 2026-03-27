import { createIntruderId } from './http'
import type {
  IntruderAttackResult,
  IntruderGrepExtractRule,
  IntruderGrepMatchRule,
} from './types'

export interface IntruderResultColumn {
  key: string
  label: string
  kind: 'base' | 'match' | 'extract'
}

export const INTRUDER_BASE_RESULT_COLUMNS: IntruderResultColumn[] = [
  { key: 'index', label: 'Request', kind: 'base' },
  { key: 'payloadSummary', label: 'Payload', kind: 'base' },
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
    caseSensitive: false,
    invert: false,
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
): Record<string, boolean> {
  return Object.fromEntries(
    rules.map((rule) => {
      const matched = evaluateMatchRule(rawResponse, rule)
      return [rule.id, matched]
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
): IntruderResultColumn[] {
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
  return [...INTRUDER_BASE_RESULT_COLUMNS, ...matchColumns, ...extractColumns]
}

export function createDefaultVisibleColumns(
  grepMatchRules: IntruderGrepMatchRule[],
  grepExtractRules: IntruderGrepExtractRule[],
): string[] {
  return getIntruderResultColumns(grepMatchRules, grepExtractRules).map((column) => column.key)
}

export function getIntruderResultColumnValue(result: IntruderAttackResult, key: string): string | number | boolean | null {
  switch (key) {
    case 'index':
      return result.index - 1
    case 'payloadSummary':
      return result.payloadSummary
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

function evaluateMatchRule(rawResponse: string, rule: IntruderGrepMatchRule): boolean {
  if (!rule.enabled || !rule.pattern.trim()) return false

  const matched = buildSafeRegex(rule.pattern, rule.caseSensitive).test(rawResponse)
  return rule.invert ? !matched : matched
}

function evaluateExtractRule(rawResponse: string, rule: IntruderGrepExtractRule): string {
  if (!rule.enabled || !rule.pattern.trim()) return ''

  const match = rawResponse.match(buildSafeRegex(rule.pattern, rule.caseSensitive))
  if (!match) return ''
  return match[rule.groupIndex] ?? match[0] ?? ''
}

function buildSafeRegex(pattern: string, caseSensitive: boolean): RegExp {
  try {
    return new RegExp(pattern, caseSensitive ? '' : 'i')
  } catch {
    const escaped = pattern.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
    return new RegExp(escaped, caseSensitive ? '' : 'i')
  }
}
