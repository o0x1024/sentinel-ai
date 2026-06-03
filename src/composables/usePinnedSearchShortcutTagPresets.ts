import { computed, ref } from 'vue'
import {
  addCustomPinnedSearchShortcutTagPreset,
  loadCustomPinnedSearchShortcutTagPresets,
  renameCustomPinnedSearchShortcutTagPreset,
  reorderCustomPinnedSearchShortcutTagPresets,
  removeCustomPinnedSearchShortcutTagPreset,
} from '@/services/pinnedSearchShortcutTagPresetStorage'

const customTagPresetsState = ref<string[]>(loadCustomPinnedSearchShortcutTagPresets())

export function usePinnedSearchShortcutTagPresets() {
  const customTagPresets = computed(() => customTagPresetsState.value)

  const addCustomTagPreset = (tag: string) => {
    customTagPresetsState.value = addCustomPinnedSearchShortcutTagPreset(tag)
  }

  const removeCustomTagPreset = (tag: string) => {
    customTagPresetsState.value = removeCustomPinnedSearchShortcutTagPreset(tag)
  }

  const renameCustomTagPreset = (currentTag: string, nextTag: string) => {
    customTagPresetsState.value = renameCustomPinnedSearchShortcutTagPreset(currentTag, nextTag)
  }

  const reorderCustomTagPresets = (activeTag: string, targetTag: string) => {
    customTagPresetsState.value = reorderCustomPinnedSearchShortcutTagPresets(activeTag, targetTag)
  }

  return {
    customTagPresets,
    addCustomTagPreset,
    removeCustomTagPreset,
    renameCustomTagPreset,
    reorderCustomTagPresets,
  }
}
