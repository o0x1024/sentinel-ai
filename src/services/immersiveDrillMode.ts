import { readonly, ref } from 'vue'

const IMMERSIVE_DRILL_MODE_STORAGE_KEY = 'sentinel:immersive-drill-mode:v1'

function loadImmersiveDrillModePreference() {
  if (typeof window === 'undefined') {
    return false
  }

  return window.localStorage.getItem(IMMERSIVE_DRILL_MODE_STORAGE_KEY) === 'true'
}

function persistImmersiveDrillModePreference(enabled: boolean) {
  if (typeof window === 'undefined') {
    return
  }

  window.localStorage.setItem(IMMERSIVE_DRILL_MODE_STORAGE_KEY, String(enabled))
}

const immersiveDrillModeEnabledState = ref(loadImmersiveDrillModePreference())

export const immersiveDrillModeEnabled = readonly(immersiveDrillModeEnabledState)

export function setImmersiveDrillModeEnabled(enabled: boolean) {
  immersiveDrillModeEnabledState.value = enabled
  persistImmersiveDrillModePreference(enabled)
}

export function toggleImmersiveDrillMode() {
  setImmersiveDrillModeEnabled(!immersiveDrillModeEnabledState.value)
}
