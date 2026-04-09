import type { GlobalSearchEntry, GlobalSearchResult } from '@/services/globalSearch'

const PINNED_SEARCH_SHORTCUTS_STORAGE_KEY = 'sentinel-pinned-search-shortcuts'
const MAX_PINNED_SEARCH_SHORTCUTS = 12

export interface PinnedSearchShortcutSnapshot {
  id: string
  title: string
  description: string
  customTitle?: string
  customDescription?: string
  customTags?: string[]
  path: string
  icon: string
  category: GlobalSearchEntry['category']
  keywords: string[]
  aliases?: string[]
  query?: Record<string, string>
}

export interface ResolvedPinnedSearchShortcutEntry extends GlobalSearchEntry {
  pinnedTags: string[]
}

export type PinnedSearchShortcutGroupKey = 'command' | 'resource'

export interface PinnedSearchShortcutGroup<T extends GlobalSearchEntry | PinnedSearchShortcutSnapshot> {
  key: PinnedSearchShortcutGroupKey
  label: string
  items: T[]
}

export function getPinnedSearchShortcutGroupKey(
  item: Pick<GlobalSearchEntry, 'category'> | Pick<PinnedSearchShortcutSnapshot, 'category'>,
): PinnedSearchShortcutGroupKey {
  return item.category === 'action' ? 'command' : 'resource'
}

function isBrowser() {
  return typeof window !== 'undefined'
}

function normalizePinnedShortcut(entry: PinnedSearchShortcutSnapshot) {
  return {
    ...entry,
    title: String(entry.title || '').trim(),
    description: String(entry.description || '').trim(),
    customTitle: String(entry.customTitle || '').trim() || undefined,
    customDescription: String(entry.customDescription || '').trim() || undefined,
    customTags: Array.isArray(entry.customTags)
      ? entry.customTags
          .map(tag => String(tag || '').trim())
          .filter(Boolean)
          .filter((tag, index, items) => items.indexOf(tag) === index)
      : undefined,
    path: String(entry.path || ''),
    icon: String(entry.icon || ''),
    keywords: Array.isArray(entry.keywords)
      ? entry.keywords.map(keyword => String(keyword || '').trim()).filter(Boolean)
      : [],
    aliases: Array.isArray(entry.aliases)
      ? entry.aliases.map(alias => String(alias || '').trim()).filter(Boolean)
      : undefined,
  }
}

function normalizeCustomTags(tags: string[]) {
  return tags
    .map(tag => String(tag || '').trim())
    .filter(Boolean)
    .filter((tag, index, items) => items.indexOf(tag) === index)
}

function toPinnedShortcutSnapshot(entry: GlobalSearchEntry | GlobalSearchResult): PinnedSearchShortcutSnapshot {
  return normalizePinnedShortcut({
    id: entry.id,
    title: entry.title,
    description: entry.description,
    path: entry.path,
    icon: entry.icon,
    category: entry.category,
    keywords: entry.keywords,
    aliases: entry.aliases,
    query: entry.query,
  })
}

export function loadPinnedSearchShortcuts() {
  if (!isBrowser()) {
    return []
  }

  try {
    const raw = window.localStorage.getItem(PINNED_SEARCH_SHORTCUTS_STORAGE_KEY)
    if (!raw) {
      return []
    }

    const parsed = JSON.parse(raw)
    if (!Array.isArray(parsed)) {
      return []
    }

    return parsed
      .map(item => item as PinnedSearchShortcutSnapshot)
      .filter(item => item && typeof item.id === 'string' && item.id.trim())
      .map(normalizePinnedShortcut)
      .slice(0, MAX_PINNED_SEARCH_SHORTCUTS)
  } catch (error) {
    console.warn('[pinnedSearchShortcuts] Failed to load pinned shortcuts:', error)
    return []
  }
}

export function savePinnedSearchShortcuts(items: PinnedSearchShortcutSnapshot[]) {
  if (!isBrowser()) {
    return
  }

  window.localStorage.setItem(
    PINNED_SEARCH_SHORTCUTS_STORAGE_KEY,
    JSON.stringify(items.slice(0, MAX_PINNED_SEARCH_SHORTCUTS)),
  )
}

export function clonePinnedSearchShortcutSnapshots(items: PinnedSearchShortcutSnapshot[]) {
  return items.map(item => ({
    ...item,
    keywords: [...item.keywords],
    aliases: item.aliases ? [...item.aliases] : undefined,
    customTags: item.customTags ? [...item.customTags] : undefined,
    query: item.query ? { ...item.query } : undefined,
  }))
}

export function addPinnedSearchShortcut(entry: GlobalSearchEntry | GlobalSearchResult) {
  const normalizedEntry = toPinnedShortcutSnapshot(entry)
  const nextItems = [
    normalizedEntry,
    ...loadPinnedSearchShortcuts().filter(item => item.id !== normalizedEntry.id),
  ].slice(0, MAX_PINNED_SEARCH_SHORTCUTS)

  savePinnedSearchShortcuts(nextItems)
  return nextItems
}

export function removePinnedSearchShortcut(entryId: string) {
  const nextItems = loadPinnedSearchShortcuts().filter(item => item.id !== entryId)
  savePinnedSearchShortcuts(nextItems)
  return nextItems
}

export function removePinnedSearchShortcuts(entryIds: string[]) {
  const validEntryIds = entryIds
    .map(entryId => String(entryId || '').trim())
    .filter(Boolean)

  if (validEntryIds.length === 0) {
    return loadPinnedSearchShortcuts()
  }

  const entryIdSet = new Set(validEntryIds)
  const nextItems = loadPinnedSearchShortcuts().filter(item => !entryIdSet.has(item.id))
  savePinnedSearchShortcuts(nextItems)
  return nextItems
}

export function updatePinnedSearchShortcutMeta(
  entryId: string,
  patch: {
    customTitle?: string
    customDescription?: string
    customTags?: string[]
  },
) {
  const nextItems = loadPinnedSearchShortcuts().map(item =>
    item.id === entryId
      ? normalizePinnedShortcut({
          ...item,
          customTitle: patch.customTitle,
          customDescription: patch.customDescription,
          customTags: patch.customTags,
        })
      : item,
  )

  savePinnedSearchShortcuts(nextItems)
  return nextItems
}

export function addPinnedSearchShortcutTagToMany(entryIds: string[], tag: string) {
  const normalizedTag = String(tag || '').trim()
  if (!normalizedTag) {
    return loadPinnedSearchShortcuts()
  }

  const entryIdSet = new Set(entryIds.map(entryId => String(entryId || '').trim()).filter(Boolean))
  if (entryIdSet.size === 0) {
    return loadPinnedSearchShortcuts()
  }

  const nextItems = loadPinnedSearchShortcuts().map(item =>
    entryIdSet.has(item.id)
      ? normalizePinnedShortcut({
          ...item,
          customTags: normalizeCustomTags([...(item.customTags || []), normalizedTag]),
        })
      : item,
  )

  savePinnedSearchShortcuts(nextItems)
  return nextItems
}

export function removePinnedSearchShortcutTagFromMany(entryIds: string[], tag: string) {
  const normalizedTag = String(tag || '').trim()
  if (!normalizedTag) {
    return loadPinnedSearchShortcuts()
  }

  const entryIdSet = new Set(entryIds.map(entryId => String(entryId || '').trim()).filter(Boolean))
  if (entryIdSet.size === 0) {
    return loadPinnedSearchShortcuts()
  }

  const nextItems = loadPinnedSearchShortcuts().map(item =>
    entryIdSet.has(item.id)
      ? normalizePinnedShortcut({
          ...item,
          customTags: (item.customTags || []).filter(currentTag => currentTag !== normalizedTag),
        })
      : item,
  )

  savePinnedSearchShortcuts(nextItems)
  return nextItems
}

export function replacePinnedSearchShortcutTagAcrossAll(currentTag: string, nextTag: string) {
  const normalizedCurrentTag = String(currentTag || '').trim()
  const normalizedNextTag = String(nextTag || '').trim()
  if (!normalizedCurrentTag || !normalizedNextTag) {
    return loadPinnedSearchShortcuts()
  }

  const nextItems = loadPinnedSearchShortcuts().map(item =>
    normalizePinnedShortcut({
      ...item,
      customTags: normalizeCustomTags(
        (item.customTags || []).map(tag => (tag === normalizedCurrentTag ? normalizedNextTag : tag)),
      ),
    }),
  )

  savePinnedSearchShortcuts(nextItems)
  return nextItems
}

export function removePinnedSearchShortcutTagAcrossAll(tag: string, replacementTag?: string) {
  const normalizedTag = String(tag || '').trim()
  const normalizedReplacementTag = String(replacementTag || '').trim()
  if (!normalizedTag) {
    return loadPinnedSearchShortcuts()
  }

  const nextItems = loadPinnedSearchShortcuts().map(item => {
    const currentTags = (item.customTags || []).filter(currentTag => currentTag !== normalizedTag)
    const nextTags = normalizedReplacementTag
      ? normalizeCustomTags([...currentTags, normalizedReplacementTag])
      : currentTags

    return normalizePinnedShortcut({
      ...item,
      customTags: nextTags,
    })
  })

  savePinnedSearchShortcuts(nextItems)
  return nextItems
}

export function reorderPinnedSearchShortcuts(activeId: string, targetId: string) {
  if (!activeId || !targetId || activeId === targetId) {
    return loadPinnedSearchShortcuts()
  }

  const currentItems = loadPinnedSearchShortcuts()
  const activeIndex = currentItems.findIndex(item => item.id === activeId)
  const targetIndex = currentItems.findIndex(item => item.id === targetId)

  if (activeIndex < 0 || targetIndex < 0) {
    return currentItems
  }

  const activeItem = currentItems[activeIndex]
  const targetItem = currentItems[targetIndex]

  if (getPinnedSearchShortcutGroupKey(activeItem) !== getPinnedSearchShortcutGroupKey(targetItem)) {
    return currentItems
  }

  const nextItems = [...currentItems]
  const [movedItem] = nextItems.splice(activeIndex, 1)
  nextItems.splice(targetIndex, 0, movedItem)

  savePinnedSearchShortcuts(nextItems)
  return nextItems
}

export function togglePinnedSearchShortcut(entry: GlobalSearchEntry | GlobalSearchResult) {
  const existingItems = loadPinnedSearchShortcuts()
  const isPinned = existingItems.some(item => item.id === entry.id)

  if (isPinned) {
    return removePinnedSearchShortcut(entry.id)
  }

  return addPinnedSearchShortcut(entry)
}

export function resolvePinnedSearchShortcutEntries(
  snapshots: PinnedSearchShortcutSnapshot[],
  entriesById: Map<string, GlobalSearchEntry>,
): ResolvedPinnedSearchShortcutEntry[] {
  return snapshots.map(snapshot => {
    const entry = entriesById.get(snapshot.id) || snapshot

    return {
      ...entry,
      title: snapshot.customTitle || entry.title,
      description: snapshot.customDescription || entry.description,
      pinnedTags: snapshot.customTags || [],
    }
  })
}

export function groupPinnedSearchShortcuts<T extends GlobalSearchEntry | PinnedSearchShortcutSnapshot>(items: T[]) {
  const groups: PinnedSearchShortcutGroup<T>[] = [
    {
      key: 'command',
      label: '命令',
      items: items.filter(item => getPinnedSearchShortcutGroupKey(item) === 'command'),
    },
    {
      key: 'resource',
      label: '页面与结果',
      items: items.filter(item => getPinnedSearchShortcutGroupKey(item) === 'resource'),
    },
  ]

  return groups.filter(group => group.items.length > 0)
}
