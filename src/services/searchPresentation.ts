import { getSearchCategoryLabel, type GlobalSearchEntry, type GlobalSearchResult } from '@/services/globalSearch'

type SearchPresentationEntry = GlobalSearchEntry | GlobalSearchResult
export type SearchCategoryFilter = 'all' | GlobalSearchEntry['category']

export const SEARCH_RESULT_CATEGORY_ORDER: GlobalSearchEntry['category'][] = [
  'action',
  'page',
  'shortcut',
  'task',
  'workflow',
  'document',
  'plugin',
  'finding',
  'notification',
]

export function groupSearchResults<T extends SearchPresentationEntry>(results: T[]) {
  return SEARCH_RESULT_CATEGORY_ORDER
    .map(category => ({
      category,
      label: getSearchCategoryLabel(category),
      items: results.filter(result => result.category === category),
    }))
    .filter(group => group.items.length > 0)
}

export function getSearchCategoryFilterLabel(category: SearchCategoryFilter) {
  return category === 'all' ? '全部' : getSearchCategoryLabel(category)
}

export function collectSearchCategoryFilters<T extends SearchPresentationEntry>(results: T[]): SearchCategoryFilter[] {
  return [
    'all',
    ...SEARCH_RESULT_CATEGORY_ORDER.filter(category => results.some(result => result.category === category)),
  ]
}

export function filterSearchResultsByCategory<T extends SearchPresentationEntry>(
  results: T[],
  category: SearchCategoryFilter,
) {
  if (category === 'all') {
    return results
  }

  return results.filter(result => result.category === category)
}

export function getNextSearchCategoryFilter(
  current: SearchCategoryFilter,
  availableFilters: SearchCategoryFilter[],
  direction: 1 | -1,
) {
  if (availableFilters.length === 0) {
    return 'all'
  }

  const currentIndex = availableFilters.indexOf(current)
  if (currentIndex < 0) {
    return availableFilters[0]
  }

  const nextIndex = (currentIndex + direction + availableFilters.length) % availableFilters.length
  return availableFilters[nextIndex]
}

export function getSearchActionLabel(result: SearchPresentationEntry) {
  switch (result.category) {
    case 'action':
      return '执行操作'
    case 'document':
      return '打开知识库'
    case 'finding':
      return '查看漏洞详情'
    case 'notification':
      return '打开通知目标'
    case 'plugin':
      return '打开插件管理'
    case 'shortcut':
      return '前往快捷入口'
    case 'task':
      return '查看任务'
    case 'workflow':
      return '打开工作流'
    default:
      return '打开页面'
  }
}

export function formatSearchKeywords(keywords: string[], limit = 4) {
  return keywords.slice(0, limit).join(' / ')
}
