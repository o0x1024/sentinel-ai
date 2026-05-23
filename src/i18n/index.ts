import { createI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
	
// 定义支持的语言类型
type SupportedLocale = 'en' | 'zh'
type LocaleMessages = Record<string, unknown>

const localeLoaders: Record<SupportedLocale, () => Promise<{ default: LocaleMessages }>> = {
  en: () => import('./locales/en'),
  zh: () => import('./locales/zh'),
}

const loadedLocales = new Set<SupportedLocale>()
	
// 对生产环境进行翻译对象验证
const validateMessages = (locale: SupportedLocale, messages: LocaleMessages) => {
  const messageKeys = Object.keys(messages)

  if (import.meta.env.DEV) {
    console.log(`[i18n] ${locale.toUpperCase()} keys:`, messageKeys.length, messageKeys.slice(0, 10))
	
    // 检查关键的子模块是否存在
    const criticalKeys = ['common', 'settings', 'sidebar', 'agent', 'tools']
    const missingKeys = criticalKeys.filter(k => !messageKeys.includes(k))
	
    if (missingKeys.length > 0) {
      console.warn(`[i18n] Missing ${locale.toUpperCase()} keys:`, missingKeys)
    }
  }
}

// 检测浏览器语言
const getBrowserLanguage = (): SupportedLocale => {
  const browserLang = navigator.language
  if (browserLang.startsWith('zh')) {
    return 'zh'
  }
  return 'en'
}

// 从localStorage获取保存的语言设置
const getSavedLanguage = (): SupportedLocale => {
  const saved = localStorage.getItem('sentinel-language')
  return (saved === 'zh' || saved === 'en') ? saved : getBrowserLanguage()
}

const normalizeLocale = (lang: string): SupportedLocale => lang.startsWith('zh') ? 'zh' : 'en'
	
// 创建i18n实例配置
// Note: vue-i18n v12 has different API, using inline types
const i18n = createI18n({
  legacy: false, // 使用组合式API
  locale: getSavedLanguage(),
  fallbackLocale: 'en',
  messages: {},
})

export const loadLanguageMessages = async (lang: SupportedLocale) => {
  if (loadedLocales.has(lang)) {
    return
  }

  const messages = (await localeLoaders[lang]()).default
  validateMessages(lang, messages)
  i18n.global.setLocaleMessage(lang, messages)
  loadedLocales.add(lang)
}

export const initializeI18n = async (lang: SupportedLocale = getSavedLanguage()) => {
  await loadLanguageMessages(lang)
  if (i18n.global.locale && typeof i18n.global.locale === 'object' && 'value' in i18n.global.locale) {
    (i18n.global.locale as { value: string }).value = lang
  }
  localStorage.setItem('sentinel-language', lang)
}
	
// 保存语言设置到localStorage并更新i18n实例
export const setLanguage = async (lang: SupportedLocale) => {
  const normalizedLang = normalizeLocale(lang)
  await loadLanguageMessages(normalizedLang)
  localStorage.setItem('sentinel-language', lang)
  // 使用类型断言确保类型安全
  if (i18n.global.locale && typeof i18n.global.locale === 'object' && 'value' in i18n.global.locale) {
    (i18n.global.locale as { value: string }).value = normalizedLang
  }
  void invoke('set_language', { lang: normalizedLang }).catch(error => {
    console.warn('[i18n] Failed to sync language to backend', error)
  })
}

// 导出当前语言获取函数
export const getCurrentLanguage = (): SupportedLocale => {
  if (i18n.global.locale && typeof i18n.global.locale === 'object' && 'value' in i18n.global.locale) {
    return (i18n.global.locale as { value: string }).value as SupportedLocale
  }
  return getSavedLanguage()
}

export default i18n
