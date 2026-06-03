import { createRecentHistoryStore } from '@/services/recentHistoryStorage'

const recentSearchHistoryStore = createRecentHistoryStore({
  storageKey: 'sentinel-global-search-history',
  maxItems: 8,
})

export const loadRecentSearches = recentSearchHistoryStore.load
export const saveRecentSearches = recentSearchHistoryStore.save
export const addRecentSearch = recentSearchHistoryStore.add
export const clearRecentSearches = recentSearchHistoryStore.clear
