import { beforeEach, describe, expect, it } from 'vitest'
import { addRecentSearch, clearRecentSearches, loadRecentSearches, saveRecentSearches } from '@/services/searchHistory'

describe('searchHistory', () => {
  beforeEach(() => {
    window.localStorage.clear()
  })

  it('stores newest queries first', () => {
    addRecentSearch('dashboard')
    addRecentSearch('消息中心')
    expect(loadRecentSearches()).toEqual(['消息中心', 'dashboard'])
  })

  it('deduplicates queries case-insensitively', () => {
    saveRecentSearches(['dashboard', 'settings'])
    expect(addRecentSearch('Dashboard')).toEqual(['Dashboard', 'settings'])
  })

  it('clears saved searches', () => {
    saveRecentSearches(['dashboard', 'settings'])
    expect(clearRecentSearches()).toEqual([])
    expect(loadRecentSearches()).toEqual([])
  })
})
