import type { EvaluationComparisonSummary } from './vulnerabilitiesEvaluationSupport'

export function mergeEvaluationHistory(
  existing: EvaluationComparisonSummary[],
  incoming: EvaluationComparisonSummary,
  limit = 10,
): EvaluationComparisonSummary[] {
  const deduped = [incoming, ...existing.filter(item => item.comparedAt !== incoming.comparedAt)]
  deduped.sort((left, right) => right.comparedAt.localeCompare(left.comparedAt))
  return deduped.slice(0, limit)
}

export function findEvaluationHistoryEntry(
  history: EvaluationComparisonSummary[],
  comparedAt: string,
): EvaluationComparisonSummary | null {
  return history.find(item => item.comparedAt === comparedAt) || null
}
