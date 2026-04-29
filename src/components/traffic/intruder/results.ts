import type {
  IntruderAttackResult,
  IntruderResultColumnFilter,
  IntruderResultFilter,
  IntruderResultSort,
  IntruderResultsWindowState,
} from './types'
import { getIntruderResultColumnValue } from './analysis'
import { setLocalStorageItem } from '@/utils/browserStorage'

const STORAGE_KEY_PREFIX = 'intruder-results-window'

export function createDefaultResultFilter(): IntruderResultFilter {
  return {
    enabled: false,
    query: '',
    invert: false,
    statusCode: '',
    onlyErrors: false,
    hideBaseline: false,
    grepMatchRuleIds: [],
    columnFilters: [],
  }
}

export function normalizeIntruderResultFilter(filter?: Partial<IntruderResultFilter> | null): IntruderResultFilter {
  return {
    ...createDefaultResultFilter(),
    ...(filter || {}),
    grepMatchRuleIds: [...(filter?.grepMatchRuleIds || [])],
    columnFilters: (filter?.columnFilters || []).map((columnFilter) => ({ ...columnFilter })),
  }
}

export function createDefaultResultSort(): IntruderResultSort {
  return {
    key: 'index',
    direction: 'asc',
  }
}

export function matchesIntruderResultFilter(
  result: IntruderAttackResult,
  filter: IntruderResultFilter,
): boolean {
  if (!filter.enabled) {
    return true
  }

  if (!filter.query.trim()) {
    return matchesStructuredResultFilter(result, filter, '')
  }

  const query = filter.query.trim().toLowerCase()
  const haystack = [
    result.index,
    result.payloadSummary,
    result.payloadReflectionCount,
    result.redirectCount,
    result.finalUrl,
    result.statusCode ?? '',
    result.responseLength,
    result.responseTimeMs ?? '',
    result.error ?? '',
    result.rawRequest,
    result.rawResponse,
  ]
    .join(' ')
    .toLowerCase()

  const matched = haystack.includes(query)
  const queryMatched = filter.invert ? !matched : matched
  return matchesStructuredResultFilter(result, filter, queryMatched)
}

export function summarizeIntruderResultFilter(filter: IntruderResultFilter, kind: 'capture' | 'view'): string {
  if (!filter.enabled) {
    return kind === 'capture' ? 'Capturing all items' : 'Showing all items'
  }

  const clauses: string[] = []

  if (filter.enabled && filter.query.trim()) {
    clauses.push(`query "${filter.query}"`)
  }
  if (filter.statusCode.trim()) {
    clauses.push(`status ${filter.statusCode}`)
  }
  if (filter.onlyErrors) {
    clauses.push('errors only')
  }
  if (filter.hideBaseline) {
    clauses.push('hide baseline')
  }
  if (filter.grepMatchRuleIds.length > 0) {
    clauses.push(`${filter.grepMatchRuleIds.length} grep matches`)
  }
  if (filter.columnFilters.length > 0) {
    clauses.push(`${filter.columnFilters.length} column filters`)
  }

  if (!clauses.length) {
    return kind === 'capture' ? 'Capturing all items' : 'Showing all items'
  }

  return kind === 'capture'
    ? `Capturing ${clauses.join(', ')}`
    : `Showing ${clauses.join(', ')}`
}

export function sortIntruderResults(results: IntruderAttackResult[], sort: IntruderResultSort): IntruderAttackResult[] {
  return [...results].sort((left, right) => {
    const leftValue = getSortValue(left, sort.key)
    const rightValue = getSortValue(right, sort.key)

    if (leftValue === rightValue) return left.index - right.index
    if (leftValue == null) return 1
    if (rightValue == null) return -1

    const direction = sort.direction === 'asc' ? 1 : -1
    if (typeof leftValue === 'number' && typeof rightValue === 'number') {
      return (leftValue - rightValue) * direction
    }

    return String(leftValue).localeCompare(String(rightValue)) * direction
  })
}

export function buildIntruderDiffSummary(baseline: IntruderAttackResult | null, result: IntruderAttackResult | null) {
  if (!baseline || !result || baseline.id === result.id) {
    return null
  }

  const baselineLines = baseline.rawResponse.split(/\r\n|\r|\n/)
  const resultLines = result.rawResponse.split(/\r\n|\r|\n/)
  const changedLines = Math.abs(resultLines.length - baselineLines.length) + countLineMismatches(baselineLines, resultLines)

  return {
    statusDelta: subtractNullable(result.statusCode, baseline.statusCode),
    lengthDelta: result.responseLength - baseline.responseLength,
    wordDelta: result.wordCount - baseline.wordCount,
    lineDelta: result.lineCount - baseline.lineCount,
    timeDelta: subtractNullable(result.responseTimeMs, baseline.responseTimeMs),
    changedLines,
  }
}

export function getIntruderResultsStorageKey(workspaceId: string): string {
  return `${STORAGE_KEY_PREFIX}:${workspaceId}`
}

export function saveIntruderResultsWindowState(state: IntruderResultsWindowState): void {
  setLocalStorageItem(getIntruderResultsStorageKey(state.workspaceId), JSON.stringify(state))
}

export function loadIntruderResultsWindowState(workspaceId: string): IntruderResultsWindowState | null {
  const raw = localStorage.getItem(getIntruderResultsStorageKey(workspaceId))
  if (!raw) return null

  try {
    return JSON.parse(raw) as IntruderResultsWindowState
  } catch {
    return null
  }
}

function matchesStructuredResultFilter(
  result: IntruderAttackResult,
  filter: IntruderResultFilter,
  queryMatched: boolean | string,
): boolean {
  if (typeof queryMatched === 'boolean' && !queryMatched) {
    return false
  }

  if (filter.statusCode.trim()) {
    const status = String(result.statusCode ?? '')
    if (!status.startsWith(filter.statusCode.trim())) {
      return false
    }
  }

  if (filter.onlyErrors && !result.error) {
    return false
  }

  if (filter.hideBaseline && result.isBaseline) {
    return false
  }

  if (filter.grepMatchRuleIds.length > 0) {
    const matchedAllRules = filter.grepMatchRuleIds.every((ruleId) => result.grepMatches[ruleId])
    if (!matchedAllRules) {
      return false
    }
  }

  if (filter.columnFilters.length > 0) {
    const matchedAllColumnFilters = filter.columnFilters.every((columnFilter) => matchesColumnFilter(result, columnFilter))
    if (!matchedAllColumnFilters) {
      return false
    }
  }

  return true
}

function getSortValue(result: IntruderAttackResult, key: string): string | number | boolean | null {
  switch (key) {
    case 'index':
      return result.index
    case 'statusCode':
      return result.statusCode
    case 'payloadReflectionCount':
      return result.payloadReflectionCount
    case 'redirectCount':
      return result.redirectCount
    case 'responseTimeMs':
      return result.responseTimeMs
    case 'responseLength':
      return result.responseLength
    case 'wordCount':
      return result.wordCount
    case 'lineCount':
      return result.lineCount
    case 'payloadSummary':
      return result.payloadSummary
    default:
      return getIntruderResultColumnValue(result, key)
  }
}

function subtractNullable(left: number | null, right: number | null): number | null {
  if (left == null || right == null) return null
  return left - right
}

function countLineMismatches(left: string[], right: string[]): number {
  const max = Math.max(left.length, right.length)
  let changed = 0

  for (let index = 0; index < max; index += 1) {
    if ((left[index] ?? '') !== (right[index] ?? '')) {
      changed += 1
    }
  }

  return changed
}

function matchesColumnFilter(result: IntruderAttackResult, columnFilter: IntruderResultColumnFilter): boolean {
  const rawValue = getIntruderResultColumnValue(result, columnFilter.key)
  const sourceValue = String(rawValue ?? '')
  const compareValue = columnFilter.value.trim()

  switch (columnFilter.operator) {
    case 'contains':
      return sourceValue.toLowerCase().includes(compareValue.toLowerCase())
    case 'equals':
      return sourceValue.toLowerCase() === compareValue.toLowerCase()
    case 'notEquals':
      return sourceValue.toLowerCase() !== compareValue.toLowerCase()
    case 'startsWith':
      return sourceValue.toLowerCase().startsWith(compareValue.toLowerCase())
    case 'greaterThan':
      return Number(rawValue ?? Number.NaN) > Number(compareValue)
    case 'lessThan':
      return Number(rawValue ?? Number.NaN) < Number(compareValue)
    default:
      return true
  }
}
