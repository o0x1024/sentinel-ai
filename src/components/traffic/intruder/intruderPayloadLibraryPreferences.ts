const STORAGE_KEY = 'trafficAnalysis.intruder.payloadLibrary.v1'
const MAX_RECENT_ITEMS = 10

export interface IntruderPayloadLibraryPreferences {
  favorites: string[]
  recent: string[]
}

function loadPreferences(): IntruderPayloadLibraryPreferences {
  try {
    const raw = localStorage.getItem(STORAGE_KEY)
    if (!raw) {
      return {
        favorites: [],
        recent: [],
      }
    }

    const parsed = JSON.parse(raw) as Partial<IntruderPayloadLibraryPreferences>
    return {
      favorites: Array.isArray(parsed.favorites) ? parsed.favorites.filter((item) => typeof item === 'string' && item.trim()) : [],
      recent: Array.isArray(parsed.recent) ? parsed.recent.filter((item) => typeof item === 'string' && item.trim()) : [],
    }
  } catch {
    return {
      favorites: [],
      recent: [],
    }
  }
}

function persistPreferences(value: IntruderPayloadLibraryPreferences) {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(value))
}

export function getIntruderPayloadLibraryPreferences(): IntruderPayloadLibraryPreferences {
  return loadPreferences()
}

export function toggleIntruderPayloadLibraryFavorite(sourceRef: string): IntruderPayloadLibraryPreferences {
  const preferences = loadPreferences()
  const favorites = preferences.favorites.includes(sourceRef)
    ? preferences.favorites.filter((item) => item !== sourceRef)
    : [sourceRef, ...preferences.favorites]

  const next = {
    ...preferences,
    favorites,
  }
  persistPreferences(next)
  return next
}

export function markIntruderPayloadLibraryRecent(sourceRef: string): IntruderPayloadLibraryPreferences {
  const preferences = loadPreferences()
  const recent = [sourceRef, ...preferences.recent.filter((item) => item !== sourceRef)].slice(0, MAX_RECENT_ITEMS)

  const next = {
    ...preferences,
    recent,
  }
  persistPreferences(next)
  return next
}
