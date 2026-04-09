import { createRecentHistoryStore } from '@/services/recentHistoryStorage'

const recentCommandHistoryStore = createRecentHistoryStore({
  storageKey: 'sentinel-command-palette-history',
  maxItems: 8,
})

export const loadRecentCommands = recentCommandHistoryStore.load
export const saveRecentCommands = recentCommandHistoryStore.save
export const addRecentCommand = recentCommandHistoryStore.add
export const clearRecentCommands = recentCommandHistoryStore.clear
