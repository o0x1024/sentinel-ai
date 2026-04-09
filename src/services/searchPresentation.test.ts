import { describe, expect, it } from 'vitest'
import {
  collectSearchCategoryFilters,
  filterSearchResultsByCategory,
  formatSearchKeywords,
  getNextSearchCategoryFilter,
  getSearchActionLabel,
  getSearchCategoryFilterLabel,
  groupSearchResults,
} from '@/services/searchPresentation'
import type { GlobalSearchResult } from '@/services/globalSearch'

const results: GlobalSearchResult[] = [
  {
    id: 'action-1',
    title: '查看严重漏洞',
    description: '打开严重漏洞列表',
    path: '/security-center',
    icon: 'fas fa-radiation',
    category: 'action',
    keywords: ['critical'],
    score: 110,
  },
  {
    id: 'page-1',
    title: '仪表盘',
    description: '总览页面',
    path: '/dashboard',
    icon: 'fas fa-home',
    category: 'page',
    keywords: ['dashboard'],
    score: 100,
  },
  {
    id: 'finding-1',
    title: 'Reflected XSS',
    description: 'search 参数反射',
    path: '/security-center',
    icon: 'fas fa-bug',
    category: 'finding',
    keywords: ['xss'],
    score: 90,
    query: { tab: 'vulnerabilities', findingId: 'finding-1' },
  },
  {
    id: 'notification-1',
    title: 'AI 助手完成',
    description: '有新的分析结果',
    path: '/ai-assistant',
    icon: 'fas fa-robot',
    category: 'notification',
    keywords: ['message'],
    score: 80,
  },
]

describe('searchPresentation', () => {
  it('groups results by configured category order', () => {
    const groups = groupSearchResults(results)
    expect(groups.map(group => group.category)).toEqual(['action', 'page', 'finding', 'notification'])
  })

  it('returns action labels by category', () => {
    expect(getSearchActionLabel(results[0])).toBe('执行操作')
    expect(getSearchActionLabel(results[1])).toBe('打开页面')
    expect(getSearchActionLabel(results[2])).toBe('查看漏洞详情')
    expect(getSearchActionLabel(results[3])).toBe('打开通知目标')
  })

  it('builds category filters and cycles across them', () => {
    const filters = collectSearchCategoryFilters(results)
    expect(filters).toEqual(['all', 'action', 'page', 'finding', 'notification'])
    expect(getSearchCategoryFilterLabel(filters[0])).toBe('全部')
    expect(getNextSearchCategoryFilter('all', filters, 1)).toBe('action')
    expect(getNextSearchCategoryFilter('action', filters, -1)).toBe('all')
  })

  it('filters results by active category', () => {
    expect(filterSearchResultsByCategory(results, 'page').map(result => result.id)).toEqual(['page-1'])
    expect(filterSearchResultsByCategory(results, 'all')).toHaveLength(results.length)
  })

  it('formats keywords with a limit', () => {
    expect(formatSearchKeywords(['a', 'b', 'c', 'd', 'e'], 3)).toBe('a / b / c')
  })
})
