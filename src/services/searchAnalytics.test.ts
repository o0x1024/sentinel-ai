import { beforeEach, describe, expect, it } from 'vitest'
import {
  buildSearchAnalyticsSummary,
  clearSearchAnalyticsEvents,
  recordSearchAnalyticsEvent,
  useSearchAnalytics,
} from '@/services/searchAnalytics'

describe('searchAnalytics', () => {
  beforeEach(() => {
    window.localStorage.clear()
    clearSearchAnalyticsEvents()
  })

  it('records and summarizes search analytics events', () => {
    recordSearchAnalyticsEvent('search-submit', { query: '漏洞' })
    recordSearchAnalyticsEvent('search-submit', { query: '漏洞' })
    recordSearchAnalyticsEvent('command-execute', { query: 'theme set dark' })
    recordSearchAnalyticsEvent('result-open', {
      query: '漏洞',
      entryId: 'finding-1',
      entryTitle: 'Reflected XSS',
      entryCategory: 'finding',
    })

    const summary = buildSearchAnalyticsSummary(useSearchAnalytics().events.value)
    expect(summary.totalSearches).toBe(2)
    expect(summary.totalCommands).toBe(1)
    expect(summary.totalOpens).toBe(1)
    expect(summary.topQueries[0]).toEqual({ value: '漏洞', count: 2 })
    expect(summary.topCommands[0]).toEqual({ value: 'theme set dark', count: 1 })
    expect(summary.topOpenedEntries[0]).toEqual({ value: 'Reflected XSS', count: 1 })
  })
})
