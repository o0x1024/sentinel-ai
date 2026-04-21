import type { ComposerTranslation } from 'vue-i18n'
import type { ShellPermissionHistoryEntry } from './useShellPermissionHistoryPanel'

export const decisionLabelKey = (decision: string) => {
  if (decision === 'allow_forever') {
    return 'allowForever'
  }
  return decision === 'deny' ? 'deny' : 'allow'
}

export const decisionBadgeClass = (decision: string) => {
  if (decision === 'allow_forever') {
    return 'badge-primary'
  }
  return decision === 'deny' ? 'badge-error' : 'badge-success'
}

export const semanticBadgeClass = (semanticKind: string) => {
  if (semanticKind === 'dangerous') {
    return 'badge-error'
  }
  return semanticKind === 'mutating' ? 'badge-warning' : 'badge-info'
}

export const formatTimestamp = (timestamp: string) => {
  const date = new Date(timestamp)
  if (Number.isNaN(date.getTime())) {
    return timestamp
  }
  return date.toLocaleString()
}

export const buildDetailSummary = (
  entry: ShellPermissionHistoryEntry,
  t: ComposerTranslation,
) => {
  const persistedRules = entry.persisted_allow_rules ?? []
  const suggestedRules = entry.suggested_allow_rules ?? []
  const parts: string[] = []
  if (persistedRules.length > 0) {
    parts.push(
      t('settings.agent.permissionHistory.summary.persistedRules', {
        count: persistedRules.length,
      })
    )
  }
  if (suggestedRules.length > 0) {
    parts.push(
      t('settings.agent.permissionHistory.summary.suggestedRules', {
        count: suggestedRules.length,
      })
    )
  }
  if (parts.length === 0) {
    return t('settings.agent.permissionHistory.summary.noExtraDetails')
  }
  return parts.join(' · ')
}
