import { createI18n } from 'vue-i18n'
import zh from './locales/zh/index'
import en from './locales/en/index'

// 定义支持的语言类型
type SupportedLocale = 'en' | 'zh'

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

// 创建i18n实例
const i18n = createI18n({
  legacy: false, // 使用组合式API
  locale: getSavedLanguage(),
  fallbackLocale: 'en',
  messages: {
    en,
    zh
  }
})

// 保存语言设置到localStorage
export const setLanguage = (lang: SupportedLocale) => {
  localStorage.setItem('sentinel-language', lang)
  i18n.global.locale.value = lang
}

export default i18n
