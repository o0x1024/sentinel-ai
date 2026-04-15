import type { TrafficContextDictionaryCandidate, TrafficContextCandidateCategory } from './trafficContextCandidateTypes'

export const trafficContextCandidateCategoryLabels: Record<TrafficContextCandidateCategory, string> = {
  principal: '主体字段',
  resource: '资源主键',
  auth_header: '认证头',
  auth_token: 'Token 参数',
  cookie_hint: 'Cookie 提示词',
  action_alias: '动作别名',
}

export const trafficContextCandidateCategoryOrder: TrafficContextCandidateCategory[] = [
  'resource',
  'principal',
  'auth_header',
  'auth_token',
  'cookie_hint',
  'action_alias',
]

export function buildTrafficContextCandidateId(candidate: TrafficContextDictionaryCandidate) {
  return candidate.category === 'action_alias'
    ? `${candidate.category}:${candidate.suggestedCanonicalAction || ''}:${candidate.normalizedKey}`
    : `${candidate.category}:${candidate.normalizedKey}`
}

export function getTrafficContextCandidateConfidenceBadgeClass(confidence: string) {
  if (confidence === 'high') return 'badge-success'
  if (confidence === 'medium') return 'badge-warning'
  return 'badge-ghost'
}

export function sortTrafficContextCandidates(candidates: TrafficContextDictionaryCandidate[]) {
  const categoryRank = new Map(
    trafficContextCandidateCategoryOrder.map((category, index) => [category, index]),
  )
  return [...candidates].sort((left, right) =>
    (categoryRank.get(left.category) ?? 999) - (categoryRank.get(right.category) ?? 999)
    || right.score - left.score
    || left.key.localeCompare(right.key),
  )
}
