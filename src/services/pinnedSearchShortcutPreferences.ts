import type { PinnedSearchShortcutGroupKey } from '@/services/pinnedSearchShortcuts'

const PINNED_SEARCH_SHORTCUT_PREFERENCES_STORAGE_KEY = 'sentinel-pinned-search-shortcut-preferences'

export interface PinnedSearchShortcutPreferences {
  collapsedGroups: PinnedSearchShortcutGroupKey[]
}

const DEFAULT_PINNED_SEARCH_SHORTCUT_PREFERENCES: PinnedSearchShortcutPreferences = {
  collapsedGroups: [],
}

function isBrowser() {
  return typeof window !== 'undefined'
}

function normalizeCollapsedGroups(value: unknown) {
  if (!Array.isArray(value)) {
    return []
  }

  return value
    .filter((item): item is PinnedSearchShortcutGroupKey => item === 'command' || item === 'resource')
    .filter((item, index, items) => items.indexOf(item) === index)
}

function normalizePreferences(value: unknown): PinnedSearchShortcutPreferences {
  if (!value || typeof value !== 'object') {
    return { ...DEFAULT_PINNED_SEARCH_SHORTCUT_PREFERENCES }
  }

  const candidate = value as Partial<PinnedSearchShortcutPreferences>
  return {
    collapsedGroups: normalizeCollapsedGroups(candidate.collapsedGroups),
  }
}

export function loadPinnedSearchShortcutPreferences() {
  if (!isBrowser()) {
    return { ...DEFAULT_PINNED_SEARCH_SHORTCUT_PREFERENCES }
  }

  try {
    const raw = window.localStorage.getItem(PINNED_SEARCH_SHORTCUT_PREFERENCES_STORAGE_KEY)
    if (!raw) {
      return { ...DEFAULT_PINNED_SEARCH_SHORTCUT_PREFERENCES }
    }

    return normalizePreferences(JSON.parse(raw))
  } catch (error) {
    console.warn('[pinnedSearchShortcutPreferences] Failed to load preferences:', error)
    return { ...DEFAULT_PINNED_SEARCH_SHORTCUT_PREFERENCES }
  }
}

export function savePinnedSearchShortcutPreferences(preferences: PinnedSearchShortcutPreferences) {
  if (!isBrowser()) {
    return
  }

  window.localStorage.setItem(
    PINNED_SEARCH_SHORTCUT_PREFERENCES_STORAGE_KEY,
    JSON.stringify(normalizePreferences(preferences)),
  )
}

export function togglePinnedSearchShortcutGroupCollapsed(groupKey: PinnedSearchShortcutGroupKey) {
  const preferences = loadPinnedSearchShortcutPreferences()
  const isCollapsed = preferences.collapsedGroups.includes(groupKey)
  const collapsedGroups = isCollapsed
    ? preferences.collapsedGroups.filter(item => item !== groupKey)
    : [...preferences.collapsedGroups, groupKey]

  const nextPreferences = normalizePreferences({ ...preferences, collapsedGroups })
  savePinnedSearchShortcutPreferences(nextPreferences)
  return nextPreferences
}

export function isPinnedSearchShortcutGroupCollapsed(
  preferences: PinnedSearchShortcutPreferences,
  groupKey: PinnedSearchShortcutGroupKey,
) {
  return preferences.collapsedGroups.includes(groupKey)
}
