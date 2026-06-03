import { ref } from 'vue'
import type { PinnedSearchShortcutGroupKey } from '@/services/pinnedSearchShortcuts'
import {
  isPinnedSearchShortcutGroupCollapsed,
  loadPinnedSearchShortcutPreferences,
  togglePinnedSearchShortcutGroupCollapsed,
  type PinnedSearchShortcutPreferences,
} from '@/services/pinnedSearchShortcutPreferences'

const preferencesState = ref<PinnedSearchShortcutPreferences>(loadPinnedSearchShortcutPreferences())

export function usePinnedSearchShortcutPreferences() {
  const toggleGroupCollapsed = (groupKey: PinnedSearchShortcutGroupKey) => {
    preferencesState.value = togglePinnedSearchShortcutGroupCollapsed(groupKey)
  }

  const isGroupCollapsed = (groupKey: PinnedSearchShortcutGroupKey) =>
    isPinnedSearchShortcutGroupCollapsed(preferencesState.value, groupKey)

  return {
    preferences: preferencesState,
    isGroupCollapsed,
    toggleGroupCollapsed,
  }
}
