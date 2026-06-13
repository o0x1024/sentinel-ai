import {
  OUTPUT_STORAGE_THRESHOLD_DEFAULT,
  OUTPUT_STORAGE_THRESHOLD_MIN,
  OUTPUT_STORAGE_THRESHOLD_MAX,
} from './settingsDefinitions'
import { setLanguage as applyI18nLanguage } from '@/i18n'

const DARK_THEMES = new Set(['dark', 'synthwave', 'halloween', 'forest', 'black', 'luxury', 'dracula'])
const DEFAULT_FONT_SIZE = 16
const DEFAULT_UI_SCALE = 100
const LEGACY_FONT_SIZE_MAP: Record<string, number> = {
  small: 12,
  normal: 16,
  large: 18,
  xlarge: 20,
}
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
    OUTPUT_STORAGE_THRESHOLD_MAX,
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

const normalizeFontSize = (fontSize: unknown): number => {
  if (typeof fontSize === 'number' && Number.isFinite(fontSize)) {
    return Math.min(20, Math.max(12, Math.round(fontSize)))
  }
  if (typeof fontSize === 'string') {
    const mapped = LEGACY_FONT_SIZE_MAP[fontSize.trim().toLowerCase()]
    if (mapped) {
      return mapped
    }
  }
  return DEFAULT_FONT_SIZE
}

const normalizeUiScale = (scale: unknown): number => {
  if (typeof scale === 'number' && Number.isFinite(scale)) {
    return Math.min(200, Math.max(50, Math.round(scale)))
  }
  return DEFAULT_UI_SCALE
}

const syncRootTypography = () => {
  const rootElement = document.documentElement
  const baseFontSize = rootElement.style.getPropertyValue('--font-size-base') || `${DEFAULT_FONT_SIZE}px`
  const uiScale = rootElement.style.getPropertyValue('--ui-scale') || '1'

  rootElement.style.setProperty('--font-size-base', baseFontSize)
  rootElement.style.setProperty('--ui-scale', uiScale)
  rootElement.style.fontSize = 'calc(var(--font-size-base, 16px) * var(--ui-scale, 1))'
}

export const migrateLegacyAppearanceSettings = (settings: any): boolean => {
  if (!settings || typeof settings !== 'object') {
    return false
  }

  let changed = false

  if (!settings.general || typeof settings.general !== 'object') {
    settings.general = {}
    changed = true
  }

  const resolvedFontSize = normalizeFontSize(settings.general.fontSize ?? settings.system?.fontSize)
  const resolvedUiScale = normalizeUiScale(settings.general.uiScale ?? settings.system?.uiScale)

  if (settings.general.fontSize !== resolvedFontSize) {
    settings.general.fontSize = resolvedFontSize
    changed = true
  }

  if (settings.general.uiScale !== resolvedUiScale) {
    settings.general.uiScale = resolvedUiScale
    changed = true
  }

  if (settings.system && typeof settings.system === 'object') {
    if ('fontSize' in settings.system) {
      delete settings.system.fontSize
      changed = true
    }
    if ('uiScale' in settings.system) {
      delete settings.system.uiScale
      changed = true
    }
  }

  return changed
}

export const applyFontSize = (fontSize: number) => {
  const rootElement = document.documentElement
  rootElement.style.setProperty('--font-size-base', `${normalizeFontSize(fontSize)}px`)
  syncRootTypography()
}

export const applyLanguage = async (language: string, locale: { value: string }) => {
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

  await applyI18nLanguage(langCode as 'zh' | 'en')
}

export const applyUIScale = (scale: number) => {
  const rootElement = document.documentElement
  const scaleValue = normalizeUiScale(scale) / 100

  rootElement.style.setProperty('--ui-scale', scaleValue.toString())
  rootElement.style.removeProperty('transform')
  rootElement.style.removeProperty('transform-origin')
  rootElement.style.removeProperty('width')
  rootElement.style.removeProperty('height')
  syncRootTypography()
}
