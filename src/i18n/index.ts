import { createI18n } from 'vue-i18n'
import zh from './locales/zh'
import en from './locales/en'

// 定义支持的语言类型
type SupportedLocale = 'en' | 'zh'

// 对生产环境进行翻译对象验证
const validateMessages = () => {
  // 验证 en 和 zh 对象是否包含预期的子模块
  const enKeys = Object.keys(en)
  const zhKeys = Object.keys(zh)

  if (import.meta.env.DEV) {
    console.log('[i18n] EN keys:', enKeys.length, enKeys.slice(0, 10))
    console.log('[i18n] ZH keys:', zhKeys.length, zhKeys.slice(0, 10))

    // 检查关键的子模块是否存在
    const criticalKeys = ['common', 'settings', 'sidebar', 'agent', 'tools']
    const missingEnKeys = criticalKeys.filter(k => !enKeys.includes(k))
    const missingZhKeys = criticalKeys.filter(k => !zhKeys.includes(k))

    if (missingEnKeys.length > 0) {
      console.warn('[i18n] Missing EN keys:', missingEnKeys)
    }
    if (missingZhKeys.length > 0) {
      console.warn('[i18n] Missing ZH keys:', missingZhKeys)
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

// 验证翻译对象
validateMessages()

// 创建i18n实例配置
// Note: vue-i18n v12 has different API, using inline types
const i18n = createI18n({
  legacy: false, // 使用组合式API
  locale: getSavedLanguage(),
  fallbackLocale: 'en',
  // 确保消息对象被正确处理
  messages: {
    en: en as Record<string, unknown>,
    zh: zh as Record<string, unknown>
  }
})

// 保存语言设置到localStorage并更新i18n实例
export const setLanguage = (lang: SupportedLocale) => {
  localStorage.setItem('sentinel-language', lang)
  // 使用类型断言确保类型安全
  if (i18n.global.locale && typeof i18n.global.locale === 'object' && 'value' in i18n.global.locale) {
    (i18n.global.locale as { value: string }).value = lang
  }
}

// 导出当前语言获取函数
export const getCurrentLanguage = (): SupportedLocale => {
  if (i18n.global.locale && typeof i18n.global.locale === 'object' && 'value' in i18n.global.locale) {
    return (i18n.global.locale as { value: string }).value as SupportedLocale
  }
  return getSavedLanguage()
}

export default i18n
