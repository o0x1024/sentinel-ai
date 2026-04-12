export type AgentListBadge = {
  label: string
  className: string
}

export type AgentListFilterOption = {
  key: string
  label: string
}

export interface AgentListItemViewModel {
  id: string
  title: string
  description: string
  metaLine?: string
  badges: AgentListBadge[]
  searchText?: string
  filterKeys?: string[]
}

export const DEFAULT_AGENT_LIST_SORT_OPTIONS = [
  { key: 'default', label: '默认顺序' },
  { key: 'title-asc', label: '名称 A-Z' },
  { key: 'title-desc', label: '名称 Z-A' },
] as const
