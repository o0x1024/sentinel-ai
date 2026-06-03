interface RecentHistoryStoreOptions {
  storageKey: string
  maxItems?: number
}

function isBrowser() {
  return typeof window !== 'undefined'
}

export function createRecentHistoryStore(options: RecentHistoryStoreOptions) {
  const maxItems = options.maxItems ?? 8

  const load = () => {
    if (!isBrowser()) {
      return []
    }

    try {
      const raw = window.localStorage.getItem(options.storageKey)
      if (!raw) {
        return []
      }

      const parsed = JSON.parse(raw)
      if (!Array.isArray(parsed)) {
        return []
      }

      return parsed
        .map(item => String(item || '').trim())
        .filter(Boolean)
        .slice(0, maxItems)
    } catch (error) {
      console.warn(`[recentHistoryStorage] Failed to load ${options.storageKey}:`, error)
      return []
    }
  }

  const save = (items: string[]) => {
    if (!isBrowser()) {
      return
    }

    window.localStorage.setItem(
      options.storageKey,
      JSON.stringify(items.slice(0, maxItems)),
    )
  }

  const add = (query: string) => {
    const normalized = String(query || '').trim()
    if (!normalized) {
      return load()
    }

    const nextItems = [
      normalized,
      ...load().filter(item => item.toLowerCase() !== normalized.toLowerCase()),
    ].slice(0, maxItems)

    save(nextItems)
    return nextItems
  }

  const clear = () => {
    save([])
    return []
  }

  return {
    load,
    save,
    add,
    clear,
  }
}
