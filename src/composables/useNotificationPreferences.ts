import { computed, ref } from 'vue'
import type {
  NotificationDesktopMode,
  NotificationPreferences,
  NotificationSource,
} from '@/types/notification'

const STORAGE_KEY = 'sentinel-notification-center-preferences'

const defaultPreferences: NotificationPreferences = {
  desktopEnabled: true,
  desktopMode: 'background',
  soundEnabled: false,
  sources: {
    ai_assistant: true,
    workflow: true,
    monitor: true,
    bug_bounty_workflow: true,
  },
}

const preferences = ref<NotificationPreferences>(loadStoredPreferences())

function loadStoredPreferences(): NotificationPreferences {
  if (typeof window === 'undefined') {
    return structuredClone(defaultPreferences)
  }

  try {
    const raw = window.localStorage.getItem(STORAGE_KEY)
    if (!raw) {
      return structuredClone(defaultPreferences)
    }

    const parsed = JSON.parse(raw) as Partial<NotificationPreferences> | null
    return {
      desktopEnabled: parsed?.desktopEnabled ?? defaultPreferences.desktopEnabled,
      desktopMode: parsed?.desktopMode === 'always' ? 'always' : defaultPreferences.desktopMode,
      soundEnabled: parsed?.soundEnabled ?? defaultPreferences.soundEnabled,
      sources: {
        ai_assistant: parsed?.sources?.ai_assistant ?? defaultPreferences.sources.ai_assistant,
        workflow: parsed?.sources?.workflow ?? defaultPreferences.sources.workflow,
        monitor: parsed?.sources?.monitor ?? defaultPreferences.sources.monitor,
        bug_bounty_workflow: parsed?.sources?.bug_bounty_workflow ?? defaultPreferences.sources.bug_bounty_workflow,
      },
    }
  } catch (error) {
    console.warn('[useNotificationPreferences] Failed to load stored preferences:', error)
    return structuredClone(defaultPreferences)
  }
}

function persistPreferences() {
  if (typeof window === 'undefined') return
  window.localStorage.setItem(STORAGE_KEY, JSON.stringify(preferences.value))
}

function updatePreferences(next: Partial<NotificationPreferences>) {
  preferences.value = {
    ...preferences.value,
    ...next,
    sources: {
      ...preferences.value.sources,
      ...(next.sources || {}),
    },
  }
  persistPreferences()
}

function setDesktopEnabled(enabled: boolean) {
  updatePreferences({ desktopEnabled: enabled })
}

function setDesktopMode(mode: NotificationDesktopMode) {
  updatePreferences({ desktopMode: mode })
}

function setSoundEnabled(enabled: boolean) {
  updatePreferences({ soundEnabled: enabled })
}

function setSourceEnabled(source: NotificationSource, enabled: boolean) {
  updatePreferences({
    sources: {
      [source]: enabled,
    } as Partial<Record<NotificationSource, boolean>> as Record<NotificationSource, boolean>,
  })
}

function isSourceEnabled(source: NotificationSource) {
  return preferences.value.sources[source] !== false
}

const enabledSourceCount = computed(() => {
  return Object.values(preferences.value.sources).filter(Boolean).length
})

export function useNotificationPreferences() {
  return {
    preferences,
    enabledSourceCount,
    updatePreferences,
    setDesktopEnabled,
    setDesktopMode,
    setSoundEnabled,
    setSourceEnabled,
    isSourceEnabled,
  }
}
