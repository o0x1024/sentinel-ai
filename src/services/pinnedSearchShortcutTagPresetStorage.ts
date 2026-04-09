import { normalizePinnedSearchShortcutTags } from '@/services/pinnedSearchShortcutTags'

const PINNED_SEARCH_SHORTCUT_TAG_PRESETS_STORAGE_KEY = 'sentinel-pinned-search-shortcut-tag-presets'

function isBrowser() {
  return typeof window !== 'undefined'
}

export function loadCustomPinnedSearchShortcutTagPresets() {
  if (!isBrowser()) {
    return []
  }

  try {
    const raw = window.localStorage.getItem(PINNED_SEARCH_SHORTCUT_TAG_PRESETS_STORAGE_KEY)
    if (!raw) {
      return []
    }

    const parsed = JSON.parse(raw)
    return Array.isArray(parsed) ? normalizePinnedSearchShortcutTags(parsed) : []
  } catch (error) {
    console.warn('[pinnedSearchShortcutTagPresetStorage] Failed to load custom tag presets:', error)
    return []
  }
}

export function saveCustomPinnedSearchShortcutTagPresets(tags: string[]) {
  if (!isBrowser()) {
    return
  }

  window.localStorage.setItem(
    PINNED_SEARCH_SHORTCUT_TAG_PRESETS_STORAGE_KEY,
    JSON.stringify(normalizePinnedSearchShortcutTags(tags)),
  )
}

export function addCustomPinnedSearchShortcutTagPreset(tag: string) {
  const normalizedTag = normalizePinnedSearchShortcutTags([tag])[0]
  if (!normalizedTag) {
    return loadCustomPinnedSearchShortcutTagPresets()
  }

  const nextTags = normalizePinnedSearchShortcutTags([
    ...loadCustomPinnedSearchShortcutTagPresets(),
    normalizedTag,
  ])

  saveCustomPinnedSearchShortcutTagPresets(nextTags)
  return nextTags
}

export function removeCustomPinnedSearchShortcutTagPreset(tag: string) {
  const normalizedTag = String(tag || '').trim()
  if (!normalizedTag) {
    return loadCustomPinnedSearchShortcutTagPresets()
  }

  const nextTags = loadCustomPinnedSearchShortcutTagPresets().filter(item => item !== normalizedTag)
  saveCustomPinnedSearchShortcutTagPresets(nextTags)
  return nextTags
}

export function renameCustomPinnedSearchShortcutTagPreset(currentTag: string, nextTag: string) {
  const normalizedCurrentTag = normalizePinnedSearchShortcutTags([currentTag])[0]
  const normalizedNextTag = normalizePinnedSearchShortcutTags([nextTag])[0]

  if (!normalizedCurrentTag || !normalizedNextTag) {
    return loadCustomPinnedSearchShortcutTagPresets()
  }

  const nextTags = normalizePinnedSearchShortcutTags(
    loadCustomPinnedSearchShortcutTagPresets().map(tag =>
      tag === normalizedCurrentTag ? normalizedNextTag : tag,
    ),
  )

  saveCustomPinnedSearchShortcutTagPresets(nextTags)
  return nextTags
}

export function reorderCustomPinnedSearchShortcutTagPresets(activeTag: string, targetTag: string) {
  const normalizedActiveTag = String(activeTag || '').trim()
  const normalizedTargetTag = String(targetTag || '').trim()
  if (!normalizedActiveTag || !normalizedTargetTag || normalizedActiveTag === normalizedTargetTag) {
    return loadCustomPinnedSearchShortcutTagPresets()
  }

  const items = [...loadCustomPinnedSearchShortcutTagPresets()]
  const activeIndex = items.indexOf(normalizedActiveTag)
  const targetIndex = items.indexOf(normalizedTargetTag)
  if (activeIndex < 0 || targetIndex < 0) {
    return items
  }

  const [activeItem] = items.splice(activeIndex, 1)
  items.splice(targetIndex, 0, activeItem)
  saveCustomPinnedSearchShortcutTagPresets(items)
  return items
}
