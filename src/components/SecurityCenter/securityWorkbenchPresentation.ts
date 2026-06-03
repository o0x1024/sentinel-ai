import type {
  WorkbenchActivityKind,
  WorkbenchActivityStateSnapshot,
  WorkbenchAssessmentSuggestion,
  WorkbenchCase,
  WorkbenchCaseStatus,
  WorkbenchConfidence,
  WorkbenchNoteKind,
} from './securityWorkbenchTypes'
import { getWorkbenchLocale, wb } from './securityWorkbenchLocale'

export const getWorkbenchStatusLabel = (status: WorkbenchCaseStatus) => {
  return wb(`status.${status}`)
}

export const getWorkbenchStatusBadgeClass = (status: WorkbenchCaseStatus) => {
  switch (status) {
    case 'new':
      return 'badge-ghost'
    case 'investigating':
      return 'badge-info'
    case 'awaiting_verification':
      return 'badge-warning'
    case 'verified':
      return 'badge-success'
    case 'false_positive':
      return 'badge-error'
    case 'archived':
      return 'badge-outline'
    default:
      return 'badge-ghost'
  }
}

export const getWorkbenchNoteKindLabel = (kind: WorkbenchNoteKind) => {
  switch (kind) {
    case 'observation':
      return wb('review.noteTitle.observation')
    case 'conclusion':
      return wb('review.noteTitle.conclusion')
    case 'false_positive_reason':
      return wb('review.noteTitle.false_positive_reason')
    case 'remediation_note':
      return wb('review.noteTitle.remediation_note')
    case 'replay_note':
      return wb('review.noteTitle.replay_note')
    default:
      return kind
  }
}

export const getWorkbenchActivityKindLabel = (kind: WorkbenchActivityKind) => {
  switch (kind) {
    case 'finding_sync':
      return wb('review.activityKind.finding_sync')
    case 'suggestion_sync':
      return wb('review.activityKind.suggestion_sync')
    case 'draft_execution':
      return wb('review.activityKind.draft_execution')
    default:
      return kind
  }
}

export const getWorkbenchCaseSubtitle = (item: WorkbenchCase) =>
  `${item.finding.vulnType} · ${item.finding.pluginId}`

export const formatWorkbenchTime = (timestamp: string) => {
  if (!timestamp) return '-'
  return new Intl.DateTimeFormat(getWorkbenchLocale() === 'zh' ? 'zh-CN' : 'en-US', {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
    hour12: false,
  }).format(new Date(timestamp))
}

export const getWorkbenchConfidenceLabel = (confidence: WorkbenchConfidence) => {
  return wb(confidence === 'high' ? 'common.high' : confidence === 'medium' ? 'common.medium' : 'common.low')
}

export const getWorkbenchConfidenceBadgeClass = (confidence: WorkbenchConfidence) => {
  switch (confidence) {
    case 'high':
      return 'badge-success'
    case 'medium':
      return 'badge-warning'
    default:
      return 'badge-ghost'
  }
}

export const hasWorkbenchAssessmentSuggestion = (
  suggestion: WorkbenchAssessmentSuggestion | null,
): suggestion is WorkbenchAssessmentSuggestion => Boolean(suggestion)

const formatWorkbenchSnapshotValue = (value: unknown): string => {
  if (value == null) return '-'
  if (typeof value === 'string') return value
  if (typeof value === 'number' || typeof value === 'boolean') return String(value)
  try {
    return JSON.stringify(value)
  } catch {
    return String(value)
  }
}

export const getWorkbenchSnapshotEntries = (snapshot?: WorkbenchActivityStateSnapshot | null) => {
  if (!snapshot) return []
  return Object.entries(snapshot).map(([key, value]) => ({
    key,
    value: formatWorkbenchSnapshotValue(value),
  }))
}
