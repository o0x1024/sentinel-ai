import { getSemanticAbstractionSummaryFromPayload } from './systemAgentTrafficSummaries'

export interface FindingSignalBadge {
  label: string
  className: string
  title?: string
}

export function getSemanticBadgeFromPayload(payload: unknown): FindingSignalBadge | null {
  const summary = getSemanticAbstractionSummaryFromPayload(payload)
  if (!summary) return null

  const primaryAction = pickPrimaryAction(summary.actionCandidates)
  if (!primaryAction) return null

  return {
    label: `语义 ${primaryAction.kind}`,
    className: summary.source === 'ai_augmented' ? 'badge-secondary' : 'badge-ghost',
    title: primaryAction.reason || `来源: ${primaryAction.source}`,
  }
}

export function getLogicHypothesisBadgeFromPayload(payload: unknown): FindingSignalBadge | null {
  const items = parseLogicHypotheses(payload)
  if (!items.length) return null

  const primary = items.sort(compareHypotheses)[0]
  const riskType = typeof primary.riskType === 'string' ? primary.riskType : ''
  if (!riskType) return null
  const strategy =
    typeof primary.recommendedVerification?.preferredStrategy === 'string'
      ? primary.recommendedVerification.preferredStrategy
      : ''

  return {
    label: `假设 ${formatRiskTypeLabel(riskType)}`,
    className: logicRiskBadgeClass(riskType),
    title: [primary.summary, strategy ? `验证: ${strategy}` : ''].filter(Boolean).join('\n'),
  }
}

function parseLogicHypotheses(payload: unknown): Record<string, any>[] {
  const record = asRecord(payload)
  const items = record?.logicHypotheses
  if (!Array.isArray(items)) return []
  return items.filter((item): item is Record<string, any> => !!asRecord(item))
}

function pickPrimaryAction(
  items: Array<{ kind: string; source: string; confidence: string; reason: string }>
) {
  return [...items].sort((left, right) => compareConfidence(right.confidence, left.confidence))[0]
}

function compareHypotheses(left: Record<string, any>, right: Record<string, any>) {
  return (
    compareConfidence(
      typeof right.confidence === 'string' ? right.confidence : '',
      typeof left.confidence === 'string' ? left.confidence : ''
    ) ||
    String(left.riskType || '').localeCompare(String(right.riskType || ''))
  )
}

function compareConfidence(left: string, right: string) {
  return confidenceRank(left) - confidenceRank(right)
}

function confidenceRank(value: string) {
  if (value === 'high') return 3
  if (value === 'medium') return 2
  if (value === 'low') return 1
  return 0
}

function formatRiskTypeLabel(riskType: string) {
  if (riskType === 'idor') return 'IDOR'
  if (riskType === 'bola') return 'BOLA'
  if (riskType === 'bfla') return 'BFLA'
  if (riskType === 'workflow') return '流程'
  if (riskType === 'race') return '竞态'
  if (riskType === 'logic') return '逻辑'
  return riskType
}

function logicRiskBadgeClass(riskType: string) {
  if (riskType === 'idor' || riskType === 'bola' || riskType === 'bfla') return 'badge-error'
  if (riskType === 'workflow') return 'badge-warning'
  if (riskType === 'race') return 'badge-secondary'
  if (riskType === 'logic') return 'badge-info'
  return 'badge-outline'
}

function asRecord(value: unknown): Record<string, unknown> | null {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return null
  return value as Record<string, unknown>
}
