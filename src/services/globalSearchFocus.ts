export const GLOBAL_SEARCH_OPEN_EVENT = 'sentinel:open-global-search'
export const GLOBAL_SEARCH_CLOSE_EVENT = 'sentinel:close-global-search'
export const GLOBAL_SEARCH_FOCUS_EVENT = 'sentinel:focus-global-search'

export function requestGlobalSearchOpen() {
  if (typeof window === 'undefined') {
    return
  }

  window.dispatchEvent(new CustomEvent(GLOBAL_SEARCH_OPEN_EVENT))
}

export function requestGlobalSearchClose() {
  if (typeof window === 'undefined') {
    return
  }

  window.dispatchEvent(new CustomEvent(GLOBAL_SEARCH_CLOSE_EVENT))
}

export function requestGlobalSearchFocus() {
  if (typeof window === 'undefined') {
    return
  }

  window.dispatchEvent(new CustomEvent(GLOBAL_SEARCH_FOCUS_EVENT))
}

export function isGlobalSearchShortcut(event: Pick<KeyboardEvent, 'key' | 'metaKey' | 'ctrlKey' | 'altKey' | 'shiftKey'>) {
  const usesPrimaryModifier = event.metaKey || event.ctrlKey
  if (!usesPrimaryModifier || event.altKey || event.shiftKey) {
    return false
  }

  return event.key.toLowerCase() === 'k'
}
