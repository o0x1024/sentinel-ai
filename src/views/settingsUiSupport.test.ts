import { beforeEach, describe, expect, it, vi } from 'vitest'

type MatchMediaChangeListener = (event: MediaQueryListEvent) => void

const createMatchMediaStub = (initialMatches: boolean) => {
  let matches = initialMatches
  const listeners = new Set<MatchMediaChangeListener>()

  const mediaQuery = {
    media: '(prefers-color-scheme: dark)',
    matches,
    onchange: null,
    addEventListener: (_type: string, listener: MatchMediaChangeListener) => {
      listeners.add(listener)
    },
    removeEventListener: (_type: string, listener: MatchMediaChangeListener) => {
      listeners.delete(listener)
    },
    addListener: (listener: MatchMediaChangeListener) => {
      listeners.add(listener)
    },
    removeListener: (listener: MatchMediaChangeListener) => {
      listeners.delete(listener)
    },
    dispatchEvent: () => true,
  } as MediaQueryList

  return {
    mediaQuery,
    setMatches(nextMatches: boolean) {
      matches = nextMatches
      Object.defineProperty(mediaQuery, 'matches', {
        configurable: true,
        value: matches,
      })
      const event = { matches, media: mediaQuery.media } as MediaQueryListEvent
      listeners.forEach(listener => listener(event))
    },
  }
}

describe('settingsUiSupport', () => {
  beforeEach(() => {
    vi.resetModules()
    localStorage.clear()
    document.documentElement.setAttribute('data-theme', 'light')
  })

  it('syncs auto theme with system theme changes', async () => {
    const matchMediaStub = createMatchMediaStub(false)
    Object.defineProperty(window, 'matchMedia', {
      configurable: true,
      writable: true,
      value: vi.fn().mockReturnValue(matchMediaStub.mediaQuery),
    })

    const { applyTheme, resolveThemePreference } = await import('./settingsUiSupport')
    const settings = { general: { darkMode: true } }

    expect(resolveThemePreference('auto')).toBe('light')

    applyTheme('auto', settings)
    expect(document.documentElement.getAttribute('data-theme')).toBe('light')
    expect(settings.general.darkMode).toBe(false)

    matchMediaStub.setMatches(true)

    expect(document.documentElement.getAttribute('data-theme')).toBe('dark')
    expect(localStorage.getItem('theme')).toBe('dark')
    expect(settings.general.darkMode).toBe(true)
  })
})
