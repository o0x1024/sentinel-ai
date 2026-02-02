<template>
  <div class="general-settings">
    <!-- 应用信息概览 -->
    <div class="grid grid-cols-1 md:grid-cols-3 gap-4 mb-6">
      <div class="stat bg-base-100 rounded-lg">
        <div class="stat-figure text-2xl">
          <i class="fas fa-info-circle"></i>
        </div>
        <div class="stat-title">{{ t('settings.general.version') }}</div>
        <div class="stat-value text-sm">{{ appInfo.version || '1.0.0' }}</div>
        <div class="stat-desc">{{ t('settings.general.buildDate') }}: {{ formatDate(appInfo.buildDate) }}</div>
      </div>
      
      <div class="stat bg-base-100 rounded-lg">
        <div class="stat-figure text-2xl">
          <i class="fas fa-globe"></i>
        </div>
        <div class="stat-title">{{ t('settings.general.language') }}</div>
        <div class="stat-value text-sm">{{ getCurrentLanguageName() }}</div>
        <div class="stat-desc">{{ t('settings.general.region') }}: {{ settings?.general?.region || 'Auto' }}</div>
      </div>
      
      <div class="stat bg-base-100 rounded-lg">
        <div class="stat-figure text-2xl">
          <i class="fas fa-palette"></i>
        </div>
        <div class="stat-title">{{ t('settings.general.theme') }}</div>
        <div class="stat-value text-sm">{{ getCurrentThemeName() }}</div>
        <div class="stat-desc">{{ t('settings.general.darkMode') }}: {{ settings?.general?.darkMode ? t('settings.enabled') : t('settings.disabled') }}</div>
      </div>
    </div>

    

    <!-- 界面设置 -->
    <div class="card bg-base-100 shadow-sm mb-6">
      <div class="card-body">
        <h3 class="card-title mb-4">
          <i class="fas fa-desktop"></i>
          {{ t('settings.general.interface') }}
        </h3>
        
        <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
          <!-- 外观设置 -->
          <div class="space-y-4">
            <h4 class="font-semibold border-b pb-2">{{ t('settings.general.appearance') }}</h4>
            
            <!-- 主题选择 -->
            <div class="form-control">
              <label class="label">
                <span class="label-text">{{ t('settings.general.theme') }}</span>
              </label>
              <select class="select select-bordered" v-model="settings.general.theme">
                <option value="auto">{{ t('settings.general.themes.auto') }}</option>
                <option value="light">{{ t('settings.general.themes.light') }}</option>
                <option value="dark">{{ t('settings.general.themes.dark') }}</option>
                <option value="cupcake">{{ t('settings.general.themes.cupcake') }}</option>
                <option value="bumblebee">{{ t('settings.general.themes.bumblebee') }}</option>
                <option value="emerald">{{ t('settings.general.themes.emerald') }}</option>
                <option value="corporate">{{ t('settings.general.themes.corporate') }}</option>
                <option value="synthwave">{{ t('settings.general.themes.synthwave') }}</option>
                <option value="retro">{{ t('settings.general.themes.retro') }}</option>
                <option value="cyberpunk">{{ t('settings.general.themes.cyberpunk') }}</option>
                <option value="valentine">{{ t('settings.general.themes.valentine') }}</option>
                <option value="halloween">{{ t('settings.general.themes.halloween') }}</option>
                <option value="garden">{{ t('settings.general.themes.garden') }}</option>
                <option value="forest">{{ t('settings.general.themes.forest') }}</option>
                <option value="aqua">{{ t('settings.general.themes.aqua') }}</option>
                <option value="lofi">{{ t('settings.general.themes.lofi') }}</option>
                <option value="pastel">{{ t('settings.general.themes.pastel') }}</option>
                <option value="fantasy">{{ t('settings.general.themes.fantasy') }}</option>
                <option value="wireframe">{{ t('settings.general.themes.wireframe') }}</option>
                <option value="black">{{ t('settings.general.themes.black') }}</option>
                <option value="luxury">{{ t('settings.general.themes.luxury') }}</option>
                <option value="dracula">{{ t('settings.general.themes.dracula') }}</option>
              </select>
            </div>
            
            <!-- 深色模式 -->
            <div class="form-control">
              <label class="label cursor-pointer">
                <span class="label-text">{{ t('settings.general.darkMode') }}</span>
                <input type="checkbox" class="toggle toggle-primary" 
                       v-model="settings.general.darkMode">
              </label>
              <label class="label">
                <span class="label-text-alt">{{ t('settings.general.darkModeHint') }}</span>
              </label>
            </div>
            
            <!-- 字体大小 -->
            <div class="form-control">
              <label class="label">
                <span class="label-text">{{ t('settings.general.fontSize') }}</span>
              </label>
              <div class="flex items-center gap-4">
                <input type="range" class="range range-primary flex-1" 
                       v-model.number="settings.general.fontSize" 
                       min="12" max="20" step="1">
                <span class="text-sm min-w-[60px]">{{ settings.general.fontSize }}px</span>
              </div>
              <div class="flex justify-between text-xs px-2">
                <span>12px</span>
                <span>16px</span>
                <span>20px</span>
              </div>
            </div>
            
            <!-- 紧凑模式 -->
            <div class="form-control">
              <label class="label cursor-pointer">
                <span class="label-text">{{ t('settings.general.compactMode') }}</span>
                <input type="checkbox" class="toggle toggle-primary" 
                       v-model="settings.general.compactMode">
              </label>
              <label class="label">
                <span class="label-text-alt">{{ t('settings.general.compactModeHint') }}</span>
              </label>
            </div>
          </div>
          
          <!-- 语言和地区 -->
          <div class="space-y-4">
            <h4 class="font-semibold border-b pb-2">{{ t('settings.general.languageRegion') }}</h4>
            
            <!-- 语言选择 -->
            <div class="form-control">
              <label class="label">
                <span class="label-text">{{ t('settings.general.language') }}</span>
              </label>
              <select class="select select-bordered" v-model="settings.general.language" @change="saveGeneralConfig">
                <option value="auto">{{ t('settings.general.languages.auto') }}</option>
                <option value="zh-CN">{{ t('settings.general.languages.zhCN') }}</option>
                <option value="en-US">{{ t('settings.general.languages.enUS') }}</option>
              </select>
            </div>
            
            <!-- 地区设置 -->
            <div class="form-control">
              <label class="label">
                <span class="label-text">{{ t('settings.general.region') }}</span>
              </label>
              <select class="select select-bordered" v-model="settings.general.region">
                <option value="auto">{{ t('settings.general.regions.auto') }}</option>
                <option value="CN">{{ t('settings.general.regions.CN') }}</option>
              </select>
            </div>
            
            <!-- 时区设置 -->
            <div class="form-control">
              <label class="label">
                <span class="label-text">{{ t('settings.general.timezone') }}</span>
              </label>
              <select class="select select-bordered" v-model="settings.general.timezone">
                <option value="auto">{{ t('settings.general.timezones.auto') }}</option>
                <option value="Asia/Shanghai">{{ t('settings.general.timezones.shanghai') }}</option>
                <option value="Asia/Taipei">{{ t('settings.general.timezones.taipei') }}</option>
                <option value="Asia/Hong_Kong">{{ t('settings.general.timezones.hongkong') }}</option>
                <option value="Asia/Tokyo">{{ t('settings.general.timezones.tokyo') }}</option>
                <option value="Asia/Seoul">{{ t('settings.general.timezones.seoul') }}</option>
                <option value="America/New_York">{{ t('settings.general.timezones.newyork') }}</option>
                <option value="America/Los_Angeles">{{ t('settings.general.timezones.losangeles') }}</option>
                <option value="Europe/London">{{ t('settings.general.timezones.london') }}</option>
                <option value="Europe/Paris">{{ t('settings.general.timezones.paris') }}</option>
                <option value="UTC">UTC</option>
              </select>
            </div>
            
            <!-- 日期格式 -->
            <div class="form-control">
              <label class="label">
                <span class="label-text">{{ t('settings.general.dateFormat') }}</span>
              </label>
              <select class="select select-bordered" v-model="settings.general.dateFormat">
                <option value="YYYY-MM-DD">YYYY-MM-DD</option>
                <option value="MM/DD/YYYY">MM/DD/YYYY</option>
                <option value="DD/MM/YYYY">DD/MM/YYYY</option>
                <option value="YYYY年MM月DD日">YYYY年MM月DD日</option>
              </select>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- 行为设置 -->
    <div class="card bg-base-100 shadow-sm mb-6">
      <div class="card-body">
        <h3 class="card-title mb-4">
          <i class="fas fa-cog"></i>
          {{ t('settings.general.behavior') }}
        </h3>
        
        <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
          <!-- 启动设置 -->
          <div class="space-y-4">
            <h4 class="font-semibold border-b pb-2">{{ t('settings.general.startup') }}</h4>
            
            <!-- 开机启动 -->
            <div class="form-control">
              <label class="label cursor-pointer">
                <span class="label-text">{{ t('settings.general.autoStart') }}</span>
                <input type="checkbox" class="toggle toggle-primary" 
                       v-model="settings.general.autoStart">
              </label>
              <label class="label">
                <span class="label-text-alt">{{ t('settings.general.autoStartHint') }}</span>
              </label>
            </div>
            
            <!-- 启动时最小化 -->
            <div class="form-control">
              <label class="label cursor-pointer">
                <span class="label-text">{{ t('settings.general.startMinimized') }}</span>
                <input type="checkbox" class="toggle toggle-primary" 
                       v-model="settings.general.startMinimized">
              </label>
            </div>
            
            <!-- 恢复上次会话 -->
            <div class="form-control">
              <label class="label cursor-pointer">
                <span class="label-text">{{ t('settings.general.restoreSession') }}</span>
                <input type="checkbox" class="toggle toggle-primary" 
                       v-model="settings.general.restoreSession">
              </label>
              <label class="label">
                <span class="label-text-alt">{{ t('settings.general.restoreSessionHint') }}</span>
              </label>
            </div>
            
            <!-- 检查更新 -->
            <div class="form-control">
              <label class="label cursor-pointer">
                <span class="label-text">{{ t('settings.general.checkUpdates') }}</span>
                <input type="checkbox" class="toggle toggle-primary" 
                       v-model="settings.general.checkUpdates">
              </label>
            </div>
            
            <!-- 日志级别 -->
            <div class="form-control">
              <label class="label">
                <span class="label-text">{{ t('settings.general.logLevel') }}</span>
              </label>
              <select class="select select-bordered" v-model="settings.general.logLevel">
                <option value="trace">{{ t('settings.general.logLevels.trace') }}</option>
                <option value="debug">{{ t('settings.general.logLevels.debug') }}</option>
                <option value="info">{{ t('settings.general.logLevels.info') }}</option>
                <option value="warn">{{ t('settings.general.logLevels.warn') }}</option>
                <option value="error">{{ t('settings.general.logLevels.error') }}</option>
              </select>
              <label class="label">
                <span class="label-text-alt">{{ t('settings.general.logLevelHint') }}</span>
              </label>
            </div>
          </div>
          
          <!-- 窗口设置 -->
          <div class="space-y-4">
            <h4 class="font-semibold border-b pb-2">{{ t('settings.general.window') }}</h4>
            
            <!-- 关闭到系统托盘 -->
            <div class="form-control">
              <label class="label cursor-pointer">
                <span class="label-text">{{ t('settings.general.closeToTray') }}</span>
                <input type="checkbox" class="toggle toggle-primary" 
                       v-model="settings.general.closeToTray">
              </label>
              <label class="label">
                <span class="label-text-alt">{{ t('settings.general.closeToTrayHint') }}</span>
              </label>
            </div>
            
            <!-- 最小化到系统托盘 -->
            <div class="form-control">
              <label class="label cursor-pointer">
                <span class="label-text">{{ t('settings.general.minimizeToTray') }}</span>
                <input type="checkbox" class="toggle toggle-primary" 
                       v-model="settings.general.minimizeToTray">
              </label>
            </div>
            
            <!-- 始终置顶 -->
            <div class="form-control">
              <label class="label cursor-pointer">
                <span class="label-text">{{ t('settings.general.alwaysOnTop') }}</span>
                <input type="checkbox" class="toggle toggle-primary" 
                       v-model="settings.general.alwaysOnTop">
              </label>
            </div>
            
            <!-- 窗口透明度 -->
            <div class="form-control">
              <label class="label">
                <span class="label-text">{{ t('settings.general.windowOpacity') }}</span>
              </label>
              <div class="flex items-center gap-4">
                <input type="range" class="range range-primary flex-1" 
                       v-model.number="settings.general.windowOpacity" 
                       min="0.5" max="1" step="0.05">
                <span class="text-sm min-w-[60px]">{{ Math.round((settings.general.windowOpacity || 1) * 100) }}%</span>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- 性能设置 -->
    <div class="card bg-base-100 shadow-sm mb-6">
      <div class="card-body">
        <h3 class="card-title mb-4">
          <i class="fas fa-tachometer-alt"></i>
          {{ t('settings.general.performance') }}
        </h3>
        
        <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
          <!-- 内存设置 -->
          <div class="space-y-4">
            <h4 class="font-semibold border-b pb-2">{{ t('settings.general.memory') }}</h4>
            
            <!-- 内存限制 -->
            <div class="form-control">
              <label class="label">
                <span class="label-text">{{ t('settings.general.memoryLimit') }}</span>
              </label>
              <div class="flex items-center gap-4">
                <input type="range" class="range range-primary flex-1" 
                       v-model.number="settings.general.memoryLimit" 
                       min="512" max="8192" step="256">
                <span class="text-sm min-w-[80px]">{{ settings.general.memoryLimit }}MB</span>
              </div>
              <label class="label">
                <span class="label-text-alt">{{ t('settings.general.memoryLimitHint') }}</span>
              </label>
            </div>
            
            <!-- 自动垃圾回收 -->
            <div class="form-control">
              <label class="label cursor-pointer">
                <span class="label-text">{{ t('settings.general.autoGC') }}</span>
                <input type="checkbox" class="toggle toggle-primary" 
                       v-model="settings.general.autoGC">
              </label>
            </div>
            
            <!-- 预加载 -->
            <div class="form-control">
              <label class="label cursor-pointer">
                <span class="label-text">{{ t('settings.general.preload') }}</span>
                <input type="checkbox" class="toggle toggle-primary" 
                       v-model="settings.general.preload">
              </label>
              <label class="label">
                <span class="label-text-alt">{{ t('settings.general.preloadHint') }}</span>
              </label>
            </div>
          </div>
          
          <!-- 网络设置 -->
          <div class="space-y-4">
            <h4 class="font-semibold border-b pb-2">{{ t('settings.general.network') }}</h4>
            
            <!-- 并发连接数 -->
            <div class="form-control">
              <label class="label">
                <span class="label-text">{{ t('settings.general.maxConnections') }}</span>
              </label>
              <input type="number" class="input input-bordered" 
                     v-model.number="settings.general.maxConnections" 
                     min="1" max="20">
              <label class="label">
                <span class="label-text-alt">{{ t('settings.general.maxConnectionsHint') }}</span>
              </label>
            </div>
            
            <!-- 请求超时 -->
            <div class="form-control">
              <label class="label">
                <span class="label-text">{{ t('settings.general.requestTimeout') }}</span>
              </label>
              <div class="flex items-center gap-4">
                <input type="range" class="range range-primary flex-1" 
                       v-model.number="settings.general.requestTimeout" 
                       min="5" max="120" step="5">
                <span class="text-sm min-w-[60px]">{{ settings.general.requestTimeout }}s</span>
              </div>
            </div>
            
            <!-- 重试次数 -->
            <div class="form-control">
              <label class="label">
                <span class="label-text">{{ t('settings.general.retryCount') }}</span>
              </label>
              <input type="number" class="input input-bordered" 
                     v-model.number="settings.general.retryCount" 
                     min="0" max="10">
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- 隐私设置 -->
    <div class="card bg-base-100 shadow-sm mb-6">
      <div class="card-body">
        <h3 class="card-title mb-4">
          <i class="fas fa-shield-alt"></i>
          {{ t('settings.general.privacy') }}
        </h3>
        
        <div class="space-y-4">
          <!-- 数据收集 -->
          <div class="form-control">
            <label class="label cursor-pointer">
              <span class="label-text">{{ t('settings.general.analytics') }}</span>
              <input type="checkbox" class="toggle toggle-primary" 
                     v-model="settings.general.analytics">
            </label>
            <label class="label">
              <span class="label-text-alt">{{ t('settings.general.analyticsHint') }}</span>
            </label>
          </div>
          
          <!-- 错误报告 -->
          <div class="form-control">
            <label class="label cursor-pointer">
              <span class="label-text">{{ t('settings.general.errorReporting') }}</span>
              <input type="checkbox" class="toggle toggle-primary" 
                     v-model="settings.general.errorReporting">
            </label>
            <label class="label">
              <span class="label-text-alt">{{ t('settings.general.errorReportingHint') }}</span>
            </label>
          </div>
          
          <!-- 使用统计 -->
          <div class="form-control">
            <label class="label cursor-pointer">
              <span class="label-text">{{ t('settings.general.usageStats') }}</span>
              <input type="checkbox" class="toggle toggle-primary" 
                     v-model="settings.general.usageStats">
            </label>
          </div>
          
          <!-- 本地存储加密 -->
          <div class="form-control">
            <label class="label cursor-pointer">
              <span class="label-text">{{ t('settings.general.encryptLocalData') }}</span>
              <input type="checkbox" class="toggle toggle-primary" 
                     v-model="settings.general.encryptLocalData">
            </label>
            <label class="label">
              <span class="label-text-alt">{{ t('settings.general.encryptLocalDataHint') }}</span>
            </label>
          </div>
        </div>
      </div>
    </div>


  </div>
</template>

<script setup lang="ts">
import { computed, watch, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { getCurrentWindow } from '@tauri-apps/api/window'
import i18n, { setLanguage } from '@/i18n' // Import i18n instance and setLanguage for direct access

const { t, locale } = useI18n({ useScope: 'global' })

// Props
interface Props {
  appInfo: any
  settings: any
  saving: boolean
}

const props = defineProps<Props>()

// Emits
interface Emits {
  'update:settings': [value: any]
  'saveGeneralConfig': []
}

const emit = defineEmits<Emits>()

// Computed
const settings = computed({
  get: () => {
    // 确保 settings.general 存在
    if (!props.settings) {
      return { general: {} }
    }
    if (!props.settings.general) {
      return { ...props.settings, general: {} }
    }
    // 默认化 Tavily 字段
    const s: any = props.settings
    if (s.general && typeof s.general === 'object') {
      if (typeof s.general.tavilyApiKey === 'undefined') s.general.tavilyApiKey = ''
      if (typeof s.general.tavilyMaxResults === 'undefined') s.general.tavilyMaxResults = 5
    }
    return s
  },
  set: (value: any) => {
    console.log('GeneralSettings: Emitting settings update:', value)
    emit('update:settings', value)
  }
})

// Methods
const formatDate = (date: string | null) => {
  if (!date) return t('settings.general.unknown')
  return new Date(date).toLocaleDateString()
}

const getCurrentLanguageName = () => {
  const languages: Record<string, string> = {
    'auto': t('settings.general.languages.auto'),
    'zh-CN': t('settings.general.languages.zhCN'),
    'zh-TW': t('settings.general.languages.zhTW'),
    'en-US': t('settings.general.languages.enUS'),
    'ja-JP': t('settings.general.languages.jaJP'),
    'ko-KR': t('settings.general.languages.koKR'),
    'fr-FR': t('settings.general.languages.frFR'),
    'de-DE': t('settings.general.languages.deDE'),
    'es-ES': t('settings.general.languages.esES'),
    'pt-BR': t('settings.general.languages.ptBR'),
    'ru-RU': t('settings.general.languages.ruRU')
  }
  return languages[props.settings?.general?.language] || t('settings.general.languages.auto')
}

const getCurrentThemeName = () => {
  const themes: Record<string, string> = {
    'auto': t('settings.general.themes.auto'),
    'light': t('settings.general.themes.light'),
    'dark': t('settings.general.themes.dark'),
    'cupcake': t('settings.general.themes.cupcake'),
    'bumblebee': t('settings.general.themes.bumblebee'),
    'emerald': t('settings.general.themes.emerald'),
    'corporate': t('settings.general.themes.corporate'),
    'synthwave': t('settings.general.themes.synthwave'),
    'retro': t('settings.general.themes.retro'),
    'cyberpunk': t('settings.general.themes.cyberpunk'),
    'valentine': t('settings.general.themes.valentine'),
    'halloween': t('settings.general.themes.halloween'),
    'garden': t('settings.general.themes.garden'),
    'forest': t('settings.general.themes.forest'),
    'aqua': t('settings.general.themes.aqua'),
    'lofi': t('settings.general.themes.lofi'),
    'pastel': t('settings.general.themes.pastel'),
    'fantasy': t('settings.general.themes.fantasy'),
    'wireframe': t('settings.general.themes.wireframe'),
    'black': t('settings.general.themes.black'),
    'luxury': t('settings.general.themes.luxury'),
    'dracula': t('settings.general.themes.dracula')
  }
  return themes[props.settings?.general?.theme] || t('settings.general.themes.auto')
}

const saveGeneralConfig = () => {
  emit('saveGeneralConfig')
}

// 实时预览设置变化
watch(() => props.settings?.general?.theme, (newTheme) => {
  if (newTheme) {
    applyThemePreview(newTheme)
  }
})

watch(() => props.settings?.general?.fontSize, (newSize) => {
  if (newSize) {
    applyFontSizePreview(newSize)
  }
})

watch(() => props.settings?.general?.language, (newLang) => {
  if (newLang) {
    applyLanguagePreview(newLang)
  }
})

const applyThemePreview = (theme: string) => {
  let finalTheme = theme
  if (theme === 'auto') {
    finalTheme = window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'
  }
  document.documentElement.setAttribute('data-theme', finalTheme)
}

const applyFontSizePreview = (fontSize: number) => {
  document.documentElement.style.fontSize = `${fontSize}px`
  document.documentElement.style.setProperty('--font-size-base', `${fontSize}px`)
}

const applyLanguagePreview = (language: string) => {
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
  
  // Map to supported locales (zh, en) and apply using the exported setLanguage function
  const targetLocale = (finalLang.startsWith('zh') ? 'zh' : 'en') as 'zh' | 'en'
  
  // Use the setLanguage function for reliable locale switching in production
  setLanguage(targetLocale)
}

// 监听窗口置顶设置
watch(() => props.settings?.general?.alwaysOnTop, async (val) => {
  try {
    await getCurrentWindow().setAlwaysOnTop(!!val)
  } catch (e) {
    console.error('Failed to set always on top:', e)
  }
})

// 监听紧凑模式设置
watch(() => props.settings?.general?.compactMode, (val) => {
  if (val) {
    document.documentElement.classList.add('compact-mode')
  } else {
    document.documentElement.classList.remove('compact-mode')
  }
})

// 监听深色模式设置 - 与主题联动
watch(() => props.settings?.general?.darkMode, (val) => {
  if (val) {
    // 如果开启深色模式且当前是浅色主题，自动切换到深色
    if (props.settings.general.theme === 'light' || props.settings.general.theme === 'corporate' || props.settings.general.theme === 'cupcake') {
      emit('update:settings', {
        ...props.settings,
        general: {
          ...props.settings.general,
          theme: 'dark'
        }
      })
    }
  } else {
    // 如果关闭深色模式且当前是深色主题，自动切换到浅色
    if (props.settings.general.theme === 'dark' || props.settings.general.theme === 'business' || props.settings.general.theme === 'dracula') {
      emit('update:settings', {
        ...props.settings,
        general: {
          ...props.settings.general,
          theme: 'light'
        }
      })
    }
  }
})

// 监听窗口透明度 (仅作为视觉效果，实际窗口透明度需要后端支持)
watch(() => props.settings?.general?.windowOpacity, (val) => {
  if (val) {
    document.documentElement.style.opacity = `${val}`
  }
})

// 自动保存
let autoSaveTimer: ReturnType<typeof setTimeout> | null = null
watch(() => props.settings?.general, () => {
  if (autoSaveTimer) clearTimeout(autoSaveTimer)
  autoSaveTimer = setTimeout(() => {
    saveGeneralConfig()
  }, 1000)
}, { deep: true })

// 初始化时应用设置
onMounted(async () => {
  if (props.settings?.general) {
    // 应用紧凑模式
    if (props.settings.general.compactMode) {
      document.documentElement.classList.add('compact-mode')
    }
    
    // 应用置顶
    if (props.settings.general.alwaysOnTop) {
      try {
        await getCurrentWindow().setAlwaysOnTop(true)
      } catch (e) {
        // 忽略错误
      }
    }

    // 应用透明度
    if (props.settings.general.windowOpacity) {
      document.documentElement.style.opacity = `${props.settings.general.windowOpacity}`
    }
  }
})
</script>

<style scoped>
.general-settings {
  @apply space-y-6;
}

.card {
  @apply transition-all duration-200 hover:shadow-md;
}

.stat {
  @apply transition-all duration-200 hover:scale-105;
}

.form-control {
  @apply transition-all duration-200;
}

.toggle:checked {
  @apply bg-primary border-primary;
}

.range {
  @apply transition-all duration-200;
}

/* 紧凑模式样式 */
:global(.compact-mode) .card-body {
  @apply p-4;
}

:global(.compact-mode) .form-control {
  @apply min-h-0;
}

:global(.compact-mode) .label {
  @apply py-1;
}

:global(.compact-mode) .space-y-6 > :not([hidden]) ~ :not([hidden]) {
  @apply mt-4;
}

:global(.compact-mode) .space-y-4 > :not([hidden]) ~ :not([hidden]) {
  @apply mt-2;
}

:global(.compact-mode) .stat {
  @apply py-2;
}
</style>
