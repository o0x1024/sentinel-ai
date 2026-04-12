import type { SystemAgentFindingSummary } from '../systemAgentSettingsSupport'

export type SystemAgentFindingLifecycle =
  | 'hypothesis'
  | 'verified'
  | 'false_positive'
  | 'fixed'
  | 'formal_open'

export function getSystemAgentFindingLifecycle(
  finding: SystemAgentFindingSummary,
): SystemAgentFindingLifecycle {
  switch ((finding.analysisStage || '').trim()) {
    case 'hypothesis':
      return 'hypothesis'
    case 'verified':
      return 'verified'
    case 'false_positive':
      return 'false_positive'
    case 'fixed':
      return 'fixed'
    case 'formal_open':
      return 'formal_open'
    default:
      break
  }

  switch ((finding.status || '').trim()) {
    case 'candidate':
    case 'triaging':
      return 'hypothesis'
    case 'reviewed':
      return 'verified'
    case 'false_positive':
      return 'false_positive'
    case 'fixed':
      return 'fixed'
    default:
      return 'formal_open'
  }
}

export function getSystemAgentFindingStageLabel(
  finding: SystemAgentFindingSummary,
) {
  switch (getSystemAgentFindingLifecycle(finding)) {
    case 'hypothesis':
      return '待验证'
    case 'verified':
      return '已验证'
    case 'false_positive':
      return '误报'
    case 'fixed':
      return '已修复'
    case 'formal_open':
      return '正式漏洞'
  }
}

export function getSystemAgentFindingStageBadgeClass(
  finding: SystemAgentFindingSummary,
) {
  switch (getSystemAgentFindingLifecycle(finding)) {
    case 'hypothesis':
      return 'badge-warning'
    case 'verified':
      return 'badge-success'
    case 'false_positive':
      return 'badge-neutral'
    case 'fixed':
      return 'badge-info'
    case 'formal_open':
      return 'badge-secondary'
  }
}

export function isSystemAgentFindingVerified(
  finding: SystemAgentFindingSummary,
) {
  return getSystemAgentFindingLifecycle(finding) === 'verified'
}
