import {
  OUTPUT_STORAGE_THRESHOLD_DEFAULT,
  OUTPUT_STORAGE_THRESHOLD_MIN,
  OUTPUT_STORAGE_THRESHOLD_RECOMMENDED_MAX,
} from './settingsDefinitions'
import { setLanguage as applyI18nLanguage } from '@/i18n'

const DARK_THEMES = new Set(['dark', 'synthwave', 'halloween', 'forest', 'black', 'luxury', 'dracula'])
const THEME_MEDIA_QUERY = typeof window !== 'undefined'
  ? window.matchMedia('(prefers-color-scheme: dark)')
  : null

let trackedThemePreference = 'light'
let trackedThemeSettings: any = null
let autoThemeListenerBound = false

export const resolveThemePreference = (theme: string): string => {
  if (theme !== 'auto') {
    return theme
  }

  return THEME_MEDIA_QUERY?.matches ? 'dark' : 'light'
}

const syncResolvedTheme = () => {
  if (typeof document === 'undefined') {
    return
  }

  const finalTheme = resolveThemePreference(trackedThemePreference)
  document.documentElement.setAttribute('data-theme', finalTheme)
  localStorage.setItem('theme', finalTheme)

  if (trackedThemeSettings?.general) {
    trackedThemeSettings.general.darkMode = DARK_THEMES.has(finalTheme)
  }
}

const ensureAutoThemeListener = () => {
  if (!THEME_MEDIA_QUERY || autoThemeListenerBound) {
    return
  }

  const handleThemeChange = () => {
    if (trackedThemePreference === 'auto') {
      syncResolvedTheme()
    }
  }

  if (typeof THEME_MEDIA_QUERY.addEventListener === 'function') {
    THEME_MEDIA_QUERY.addEventListener('change', handleThemeChange)
  } else {
    THEME_MEDIA_QUERY.addListener(handleThemeChange)
  }

  autoThemeListenerBound = true
}

export const clampOutputStorageThreshold = (value: number): number => {
  if (!Number.isFinite(value)) return OUTPUT_STORAGE_THRESHOLD_DEFAULT
  return Math.min(
    OUTPUT_STORAGE_THRESHOLD_RECOMMENDED_MAX,
    Math.max(OUTPUT_STORAGE_THRESHOLD_MIN, Math.round(value)),
  )
}

export const normalizeCloseAction = (value: unknown): 'hide' | 'minimize' | 'exit' => {
  const normalized = String(value ?? '').trim().toLowerCase()
  if (normalized === 'hide' || normalized === 'tray' || normalized === 'close_to_tray') {
    return 'hide'
  }
  if (normalized === 'exit' || normalized === 'quit' || normalized === 'close') {
    return 'exit'
  }
  return 'minimize'
}
export const applyTheme = (theme: string, settings: any) => {
  trackedThemePreference = theme
  trackedThemeSettings = settings
  ensureAutoThemeListener()
  syncResolvedTheme()
}

export const applyFontSize = (fontSize: number) => {
  const rootElement = document.documentElement
  rootElement.style.fontSize = `${fontSize}px`
  rootElement.style.setProperty('--font-size-base', `${fontSize}px`)

  if (window.updateFontSize) {
    const sizeMap: Record<number, string> = {
      12: 'small', 14: 'normal', 16: 'normal', 18: 'large', 20: 'large',
    }
    window.updateFontSize(sizeMap[fontSize] || 'normal')
  }
}

export const applyLanguage = (language: string, locale: { value: string }) => {
  let finalLang = language
  if (language === 'auto') {
    const browserLang = navigator.language.toLowerCase()
    if (browserLang.startsWith('zh')) {
      finalLang = browserLang.includes('tw') || browserLang.includes('hk') ? 'zh-TW' : 'zh-CN'
    } else if (browserLang.startsWith('en')) {
      finalLang = 'en-US'
    } else {
      finalLang = 'zh-CN'
    }
  }

  let langCode = finalLang.split('-')[0]
  if (langCode !== 'zh') {
    langCode = 'en'
  }

  locale.value = langCode
  applyI18nLanguage(langCode as 'zh' | 'en')
}

export const applyUIScale = (scale: number) => {
  const rootElement = document.documentElement
  const scaleValue = scale / 100

  rootElement.style.setProperty('--ui-scale', scaleValue.toString())
  rootElement.style.transform = `scale(${scaleValue})`
  rootElement.style.transformOrigin = 'top left'

  if (scale !== 100) {
    rootElement.style.width = `${10000 / scale}%`
    rootElement.style.height = `${10000 / scale}%`
  } else {
    rootElement.style.width = '100%'
    rootElement.style.height = '100%'
  }

  if (window.updateUIScale) {
    window.updateUIScale(scale)
  }
}
