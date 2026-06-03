export type FindingFilterPresetId =
  | 'high_value_candidate'
  | 'object_boundary_candidate'
  | 'ai_augmented_only'

export type VulnerabilityLifecycleView =
  'formal'
  | 'candidate'
  | 'verified'
  | 'false_positive'
  | 'all'

export interface VulnerabilityFilterState {
  severity: string
  status: string
  lifecycleView: VulnerabilityLifecycleView
  search: string
  semanticSource: string
  hypothesisRiskType: string
  hypothesisRiskTypes: string[]
}

export const filterPresets: Array<{
  id: FindingFilterPresetId
  label: string
  description: string
}> = [
  {
    id: 'high_value_candidate',
    label: '高价值候选',
    description: '候选待验证 + AI 增强 + workflow/race，优先看流程绕过和竞态类结果。',
  },
  {
    id: 'object_boundary_candidate',
    label: '对象边界',
    description: '候选待验证 + AI 增强 + IDOR/BOLA/BFLA，优先看对象越权和功能越权。',
  },
  {
    id: 'ai_augmented_only',
    label: '仅 AI 增强',
    description: '只看命中 AI 语义增强的 finding，不限制生命周期和假设类型。',
  },
]

export function detectActiveFilterPreset(
  filters: VulnerabilityFilterState
): FindingFilterPresetId | null {
  if (
    filters.lifecycleView === 'candidate' &&
    !filters.status &&
    !filters.severity &&
    !filters.search &&
    filters.semanticSource === 'ai_augmented' &&
    sameStringSet(filters.hypothesisRiskTypes, ['workflow', 'race'])
  ) {
    return 'high_value_candidate'
  }
  if (
    filters.lifecycleView === 'candidate' &&
    !filters.status &&
    !filters.severity &&
    !filters.search &&
    filters.semanticSource === 'ai_augmented' &&
    sameStringSet(filters.hypothesisRiskTypes, ['idor', 'bola', 'bfla'])
  ) {
    return 'object_boundary_candidate'
  }
  if (
    filters.lifecycleView === 'all' &&
    !filters.status &&
    !filters.severity &&
    !filters.search &&
    filters.semanticSource === 'ai_augmented' &&
    filters.hypothesisRiskTypes.length === 0
  ) {
    return 'ai_augmented_only'
  }
  return null
}

function sameStringSet(left: string[], right: string[]) {
  if (left.length !== right.length) return false
  const leftSorted = [...left].sort()
  const rightSorted = [...right].sort()
  return leftSorted.every((value, index) => value === rightSorted[index])
}
