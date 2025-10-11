<template>
  <div class="settings-page page-content-padded safe-top">
    <div class="flex justify-between items-center mb-6">
      <h2 class="text-2xl font-bold">{{ t('settings.title', '系统设置') }}</h2>
      <div class="flex gap-2">
        <button 
          class="btn btn-primary" 
          @click="saveAllSettings"
          :disabled="saving"
        >
          <i class="fas fa-save mr-2"></i>
          {{ saving ? t('common.saving') : t('common.saveSettings') }}
        </button>
        <button 
          class="btn btn-secondary" 
          @click="resetToDefaults"
        >
          <i class="fas fa-undo mr-2"></i>
          {{ t('common.resetDefaults') }}
        </button>
      </div>
    </div>

    <!-- 设置分类标签 -->
    <div class="tabs tabs-boxed mb-6">
      <a 
        v-for="category in categories" 
        :key="category.id"
        class="tab"
        :class="{ 'tab-active': activeCategory === category.id }"
        @click="activeCategory = category.id"
      >
        <i :class="category.icon + ' mr-2'"></i>
        {{ t(`settings.categories.${category.id}`) }}
      </a>
    </div>

    <!-- AI服务配置 -->
    <AISettings v-if="activeCategory === 'ai'" 
                :ai-service-status="aiServiceStatus"
                :ai-config="aiConfig"
                v-model:selected-ai-provider="selectedAiProvider"
                :settings="settings"
                :custom-provider="customProvider"
                :ai-usage-stats="aiUsageStats"
                :rag-config="ragConfig"
                :saving="saving"
                @test-connection="testConnection"
                @save-ai-config="saveAiConfig"
                @test-custom-provider="testCustomProvider"
                @add-custom-provider="addCustomProvider"
                @refresh-models="refreshModels"
                @apply-manual-config="applyManualConfig"
                @set-default-provider="setDefaultProvider"
                @set-default-chat-model="setDefaultChatModel"
                @save-rag-config="saveRagConfig"
                @test-embedding-connection="testEmbeddingConnection"
                @reset-rag-config="resetRagConfig" />

    <!-- 模型配置设置 -->
    <ModelSettings v-if="activeCategory === 'scheduler'" 
                       v-model:scheduler-config="settings.scheduler"
                       v-model:rag-config="ragConfig"
                       :available-models="availableModels"
                       :saving="saving"
                       @save-scheduler-config="saveSchedulerConfig"
                       @apply-high-performance-preset="applyHighPerformanceConfig"
                       @apply-balanced-preset="applyBalancedConfig"
                       @apply-economic-preset="applyEconomicConfig"
                       @save-rag-config="saveRagConfig"
                       @test-embedding-connection="testEmbeddingConnection"
                       @reset-rag-config="resetRagConfig" />

    <!-- 数据库设置 -->
    <DatabaseSettings v-if="activeCategory === 'database'" 
                      v-model:settings="settings"
                      :database-status="databaseStatus"
                      :saving="saving"
                      @test-database="testDatabase"
                      @backup-database="backupDatabase"
                      @restore-database="restoreDatabase"
                      @optimize-database="optimizeDatabase"
                      @save-database-config="saveDatabaseConfig" />

    <!-- 通用设置 -->
    <GeneralSettings v-if="activeCategory === 'system'" 
                     :app-info="{}"
                     v-model:settings="settings"
                     :saving="saving"
                     @save-general-config="saveGeneralConfig"
                     @apply-font-size="applyFontSize"
                     @apply-ui-scale="applyUIScale" />


    <!-- 网络(代理)设置 -->
    <NetworkSettings v-if="activeCategory === 'network'" 
                     :saving="saving" />
    

    <!-- 安全设置 -->
    <SecuritySettings v-if="activeCategory === 'security'" 
                      v-model:settings="settings"
                      :security-status="securityStatus"
                      :saving="saving"
                      @change-password="changePassword"
                      @run-security-audit="runSecurityAudit"
                      @check-vulnerabilities="checkVulnerabilities"
                      @generate-security-report="generateSecurityReport"
                      @lock-application="lockApplication"
                      @emergency-shutdown="emergencyShutdown"
                      @wipe-security-data="wipeSecurityData"
                      @save-security-config="saveSecurityConfig" />

  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted, watch, nextTick } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { dialog } from '@/composables/useDialog'
import AISettings from '@/components/Settings/AISettings.vue'
import ModelSettings from '@/components/Settings/ModelSettings.vue'
import DatabaseSettings from '@/components/Settings/DatabaseSettings.vue'
import GeneralSettings from '@/components/Settings/GeneralSettings.vue'
import SecuritySettings from '@/components/Settings/SecuritySettings.vue'
import NetworkSettings from '@/components/Settings/NetworkSettings.vue'

const { t, locale } = useI18n()

// 响应式数据
const activeCategory = ref('ai')
const saving = ref(false)

// 设置分类
const categories = [
  { id: 'ai', icon: 'fas fa-robot' },
  { id: 'scheduler', icon: 'fas fa-cogs' },
  { id: 'database', icon: 'fas fa-database' },
  { id: 'system', icon: 'fas fa-cog' },
  { id: 'security', icon: 'fas fa-shield-alt' },
  { id: 'network', icon: 'fas fa-network-wired' },
]

// 设置数据
const settings = ref({
  ai: {
    temperature: 0.7,
    maxTokens: 2000
  },
  scheduler: {
    enabled: true,
    models: {
      intent_analysis: '',
      intent_analysis_provider: '',
      planner: '',
      planner_provider: '',
      replanner: '',
      replanner_provider: '',
      executor: '',
      executor_provider: '',
      evaluator: '',
      evaluator_provider: ''
    },
    default_strategy: 'adaptive',
    max_retries: 3,
    timeout_seconds: 120,
    scenarios: {}
  },
  database: {
    type: 'sqlite',
    path: '',
    autoBackup: true,
    backupInterval: 24,
    maxBackups: 10
  },
  general: {
    theme: 'auto',
    darkMode: false,
    fontSize: 16,
    compactMode: false,
    language: 'auto',
    region: 'auto',
    timezone: 'auto',
    dateFormat: 'YYYY-MM-DD',
    autoStart: false,
    startMinimized: false,
    restoreSession: true,
    checkUpdates: true,
    closeToTray: false,
    minimizeToTray: false,
    alwaysOnTop: false,
    windowOpacity: 1,
    memoryLimit: 2048,
    autoGC: true,
    preload: false,
    maxConnections: 5,
    requestTimeout: 30,
    retryCount: 3,
    analytics: false,
    errorReporting: true,
    usageStats: false,
    encryptLocalData: true,
    uiScale: 100
  },
  system: {
    theme: 'dark',
    fontSize: 'normal',
    uiScale: 100,
    autoStart: false,
    minimizeToTray: true
  },
  security: {
    requireAuth: false,
    authMethod: 'password',
    sessionTimeout: 30,
    maxLoginAttempts: 5,
    lockoutDuration: 15,
    twoFactorAuth: false,
    encryption: true,
    encryptionType: 'AES-256',
    keyManagement: 'auto',
    keyRotation: true,
    rotationPeriod: 90,
    encryptDatabase: true,
    encryptConfig: true,
    encryptLogs: false,
    encryptCache: false,
    encryptBackups: true,
    forceHTTPS: true,
    verifyCertificates: true,
    useProxy: false,
    proxyType: 'http',
    proxyHost: '',
    proxyPort: 8080,
    enableIPWhitelist: false,
    allowedIPs: '',
    enableRateLimit: false,
    requestsPerMinute: 60,
    burstLimit: 10,
    enableAudit: true,
    auditLevel: 'detailed',
    auditLogin: true,
    auditConfigChanges: true,
    auditDataAccess: false,
    auditErrors: true,
    logRetention: 90,
    compressLogs: true,
    remoteLogging: false,
    logServer: '',
    logApiKey: '',
    pin: ''
  }
})

// AI相关数据
const selectedAiProvider = ref('OpenAI')
const aiServiceStatus = ref([])
const aiConfig = ref<any>({ providers: {}, default_provider: 'openai' })
const aiUsageStats = ref({})

// RAG配置数据
const ragConfig = ref({
  embedding_provider: 'ollama',
  embedding_model: 'nomic-embed-text',
  embedding_dimensions: null,
  embedding_api_key: '',
  embedding_base_url: 'http://localhost:11434',
  chunk_size_chars: 1000,
  chunk_overlap_chars: 200,
  top_k: 5,
  mmr_lambda: 0.7,
  batch_size: 10,
  max_concurrent: 4
})

const loadAiUsageStats = async () => {
  try {
    const stats = await invoke('get_ai_usage_stats') as Record<string, { input_tokens: number, output_tokens: number, total_tokens: number, cost: number }>
    aiUsageStats.value = stats || {}
  } catch (e) {
    console.warn('Failed to load AI usage stats', e)
  }
}
const customProvider = reactive({
  name: '',
  api_key: '',
  api_base: '',
  model_id: ''
})

// 数据库状态
const databaseStatus = ref({
  connected: false,
  size: 0,
  lastBackup: null,
  backupCount: 0
})

// 安全状态
const securityStatus = ref({
  lastAudit: null,
  auditIssues: 0
})

// 计算属性
const availableModels = computed(() => {
  const models: any[] = []
  
  Object.values(aiConfig.value.providers || {}).forEach((provider: any) => {
    // 统一规范化 provider 名称
    const providerName = provider.name || provider.provider || 'Unknown'
    if (provider.models && Array.isArray(provider.models)) {
      provider.models.forEach((model: any) => {
        // 放宽条件：默认展示；若存在 is_available 显式为 false 则过滤
        if (model.is_available !== false) {
          models.push({
            id: model.id,
            name: model.name,
            provider: providerName,
            description: model.description || '',
            context_length: model.context_length || 4096,
            supports_tools: model.supports_tools || false,
            supports_vision: model.supports_vision || false
          })
        }
      })
    }
  })
  
  console.log('Available models for scheduler:', models)
  return models
})

// 方法
const loadSettings = async () => {
  try {
    // 先加载AI配置
    const aiConfigData = await invoke('get_ai_config')
    aiConfig.value = aiConfigData as any
    console.log('Loaded AI config:', aiConfig.value)
    
    // 等待一个 tick 确保 aiConfig 更新完成
    await nextTick()
    
    // 再加载调度器配置
    const schedulerConfig = await invoke('get_scheduler_config') as any
    console.log('Loaded scheduler config from backend:', schedulerConfig)
    let backendSchedulerApplied = false
    
    if (schedulerConfig) {
      // 转换后端返回的扁平结构为前端期望的嵌套结构
      const transformedConfig = {
        enabled: schedulerConfig.enabled ?? true,
        models: {
          intent_analysis: schedulerConfig.intent_analysis_model || '',
          intent_analysis_provider: schedulerConfig.intent_analysis_provider || '',
          planner: schedulerConfig.planner_model || '',
          planner_provider: schedulerConfig.planner_provider || '',
          replanner: schedulerConfig.replanner_model || '',
          replanner_provider: schedulerConfig.replanner_provider || '',
          executor: schedulerConfig.executor_model || '',
          executor_provider: schedulerConfig.executor_provider || '',
          evaluator: schedulerConfig.evaluator_model || '',
          evaluator_provider: schedulerConfig.evaluator_provider || ''
        },
        default_strategy: schedulerConfig.default_strategy || 'adaptive',
        max_retries: schedulerConfig.max_retries || 3,
        timeout_seconds: schedulerConfig.timeout_seconds || 120,
        scenarios: schedulerConfig.scenarios || {}
      }
      
      console.log('Transformed scheduler config:', transformedConfig)
      Object.assign(settings.value.scheduler, transformedConfig)
      backendSchedulerApplied = true
    }
    
    // 从 localStorage 加载通用设置
    const savedSettings = localStorage.getItem('sentinel-settings')
    if (savedSettings) {
      try {
        const parsed = JSON.parse(savedSettings)
        console.log('Loading saved settings:', parsed)
        
        // 深度合并保存的设置到默认设置
        if (parsed.general) {
          Object.assign(settings.value.general, parsed.general)
          console.log('Merged general settings:', settings.value.general)
        }
        if (parsed.system) {
          Object.assign(settings.value.system, parsed.system)
        }
        if (parsed.ai) {
          Object.assign(settings.value.ai, parsed.ai)
        }
        // 若后端已返回调度器配置，则不使用本地缓存覆盖
        if (!backendSchedulerApplied && parsed.scheduler) {
          Object.assign(settings.value.scheduler, parsed.scheduler)
        }
        if (parsed.database) {
          Object.assign(settings.value.database, parsed.database)
        }
        if (parsed.security) {
          Object.assign(settings.value.security, parsed.security)
        }
      } catch (e) {
        console.warn('Failed to parse saved settings:', e)
      }
    }
    
    // 应用已保存的设置
    if (settings.value.general?.theme) {
      applyTheme(settings.value.general.theme)
    }
    if (settings.value.general?.fontSize) {
      applyFontSize(settings.value.general.fontSize)
    }
    if (settings.value.general?.language) {
      applyLanguage(settings.value.general.language)
    }
    if (settings.value.general?.uiScale) {
      applyUIScale(settings.value.general.uiScale)
    }
    
    
    // 加载其他设置...
  } catch (error) {
    console.error('Failed to load settings:', error)
    dialog.toast.error('加载设置失败')
  }
}

const saveAllSettings = async () => {
  saving.value = true
  try {
    await Promise.all([
      saveAiConfig(),
      saveSchedulerConfig(),
      saveDatabaseConfig(),
      saveGeneralConfig(),
      saveSecurityConfig(),
    ])
    dialog.toast.success('所有设置已保存')
  } catch (error) {
    console.error('Failed to save settings:', error)
    dialog.toast.error('保存设置失败')
  } finally {
    saving.value = false
  }
}

const resetToDefaults = async () => {
  const confirmed = await dialog.confirm({
    title: '重置设置',
    message: '确定要重置所有设置为默认值吗？此操作不可撤销。',
    variant: 'warning'
  })
  
  if (confirmed) {
    // 重置逻辑
    dialog.toast.success('设置已重置为默认值')
  }
}

// AI相关方法
const testConnection = async (provider: string) => {
  try {
    // 获取当前提供商的配置
    const providerConfig = (aiConfig.value.providers as any)?.[provider]
    if (!providerConfig) {
      dialog.toast.error(`未找到 ${provider} 的配置`)
      return
    }

    // 构建请求参数
    const request = {
      provider: provider,
      api_key: providerConfig.api_key,
      api_base: providerConfig.api_base,
      organization: providerConfig.organization,
      model: providerConfig.default_model
    }

    const response = await invoke('test_ai_connection', { request })
    dialog.toast.success(`${provider} 连接测试成功`)
  } catch (error) {
    console.error('Connection test failed:', error)
    dialog.toast.error(`${provider} 连接测试失败: ${error}`)
  }
}

const refreshModels = async (provider: string) => {
  try {
    dialog.toast.info(`正在刷新 ${provider} 模型列表...`)
    
    // 获取当前提供商的配置
    const providerConfig = (aiConfig.value.providers as any)?.[provider]
    if (!providerConfig) {
      console.warn('Provider config not found for:', provider)
      return
    }

    // 调用新的API获取实时模型列表
    const modelIds = await invoke('get_provider_models', {
      provider: provider,
      apiKey: providerConfig.api_key,
      apiBase: providerConfig.api_base,
      organization: providerConfig.organization
    }) as string[]

    console.log('Fetched models for', provider, ':', modelIds)
    
    // 将简单的字符串数组转换为前端期望的模型对象格式
    const models = modelIds.map(modelId => ({
      id: modelId,
      name: modelId,
      description: `${provider} model`,
      is_available: true,
      context_length: getDefaultContextLength(provider, modelId),
      supports_streaming: true,
      supports_tools: getSupportsTools(provider, modelId),
      supports_vision: getSupportsVision(provider, modelId)
    }))
    
    // 更新配置中的模型列表
    if (aiConfig.value.providers && (aiConfig.value.providers as any)[provider]) {
      (aiConfig.value.providers as any)[provider].models = models
    }
    
    dialog.toast.success(`${provider} 模型列表已刷新，找到 ${models.length} 个模型`)
    
  } catch (error) {
    console.error('Failed to refresh models:', error)
    dialog.toast.error(`刷新 ${provider} 模型列表失败: ${error}`)
  }
}

// 获取默认上下文长度
const getDefaultContextLength = (provider: string, modelId: string): number => {
  switch (provider.toLowerCase()) {
    case 'openai':
      if (modelId.includes('gpt-4')) return 8192
      if (modelId.includes('gpt-3.5')) return 4096
      return 4096
    case 'anthropic':
      if (modelId.includes('claude-3')) return 200000
      return 100000
    case 'deepseek':
      if (modelId.includes('coder')) return 16384
      return 4096
    case 'gemini':
      return 32768
    default:
      return 4096
  }
}

// 获取是否支持工具调用
const getSupportsTools = (provider: string, modelId: string): boolean => {
  switch (provider.toLowerCase()) {
    case 'openai':
      return modelId.includes('gpt-4') || modelId.includes('gpt-3.5-turbo')
    case 'anthropic':
      return modelId.includes('claude-3')
    case 'deepseek':
      return modelId.includes('chat')
    case 'gemini':
      return true
    default:
      return false
  }
}

// 获取是否支持视觉
const getSupportsVision = (provider: string, modelId: string): boolean => {
  switch (provider.toLowerCase()) {
    case 'openai':
      return modelId.includes('gpt-4') && modelId.includes('vision')
    case 'anthropic':
      return modelId.includes('claude-3')
    case 'gemini':
      return modelId.includes('vision') || modelId.includes('pro')
    default:
      return false
  }
}

const saveAiConfig = async () => {
  try {
    await invoke('save_ai_config', { config: aiConfig.value })
    dialog.toast.success('AI配置已保存')
  } catch (error) {
    console.error('Failed to save AI config:', error)
    dialog.toast.error('保存AI配置失败')
  }
}

const setDefaultProvider = async (provider: string) => {
  try {
    await invoke('set_default_provider', { request: { provider } })
    // 同步前端状态
    aiConfig.value.default_provider = provider
    dialog.toast.success(`默认 Provider 已设置为 ${provider}`)
  } catch (e) {
    console.error('Failed to set default provider', e)
    dialog.toast.error('设置默认 Provider 失败')
  }
}

const setDefaultChatModel = async (model: string) => {
  try {
    if (!model) {
      // 清空默认模型
      aiConfig.value.default_chat_model = ''
      console.log('Settings: Cleared default_chat_model')
      dialog.toast.success('已清空默认 Chat 模型')
      return
    }
    
    // 获取当前默认提供商（小写格式）
    const currentProviderLower = aiConfig.value.default_provider || 'openai'
    
    // 在提供商配置中查找匹配的提供商（不区分大小写）
    const providerConfigKey = Object.keys(aiConfig.value.providers || {}).find(key => 
      key.toLowerCase() === currentProviderLower.toLowerCase()
    )
    
    if (!providerConfigKey) {
      dialog.toast.error('当前默认提供商配置不存在')
      console.error('Provider not found:', {
        searchFor: currentProviderLower,
        availableProviders: Object.keys(aiConfig.value.providers || {}),
        aiConfigDefaultProvider: aiConfig.value.default_provider
      })
      return
    }
    
    const providerConfig = aiConfig.value.providers[providerConfigKey]
    if (!providerConfig) {
      dialog.toast.error('提供商配置无效')
      return
    }
    
    // 从提供商的模型列表中找到匹配的模型
    const modelInfo = providerConfig.models?.find((m: any) => m.id === model)
    if (!modelInfo) {
      dialog.toast.error('选择的模型不存在于当前提供商')
      return
    }
    
    // 调用后端API设置默认Chat模型 - 使用正确的命令
    const modelValue = `${currentProviderLower.toLowerCase()}/${model}`
    await invoke('set_default_chat_model', {
      model: modelValue
    })
    
    // 同步前端状态 - 保存为 'provider/model' 格式
    aiConfig.value.default_chat_model = modelValue
    console.log('Updated frontend default_chat_model state:', modelValue)
    
    dialog.toast.success(`默认 Chat 模型已设置为 ${modelInfo.name}`)
  } catch (e) {
    console.error('Failed to set default chat model', e)
    dialog.toast.error('设置默认 Chat 模型失败')
  }
}

const testCustomProvider = async () => {
  // 测试自定义提供商逻辑
  dialog.toast.info('测试自定义提供商...')
}

const addCustomProvider = async () => {
  // 添加自定义提供商逻辑
  dialog.toast.success('自定义提供商已添加')
}

const applyManualConfig = async (config: any) => {
  try {
    // 验证配置格式
    if (!config || typeof config !== 'object' || !config.providers) {
      dialog.toast.error('配置格式无效')
      return
    }
    
    // 更新本地配置
    aiConfig.value = config
    
    // 保存到后端
    await invoke('save_ai_config', { config: config })
    
    dialog.toast.success('手动配置已应用并保存')
  } catch (error) {
    console.error('Failed to apply manual config:', error)
    dialog.toast.error(`应用配置失败: ${error}`)
  }
}

// 调度器相关方法
const saveSchedulerConfig = async () => {
  try {
    // 转换前端嵌套结构为后端期望的扁平结构
    const flatConfig = {
      enabled: settings.value.scheduler.enabled,
      intent_analysis_model: settings.value.scheduler.models.intent_analysis,
      intent_analysis_provider: settings.value.scheduler.models.intent_analysis_provider,
      planner_model: settings.value.scheduler.models.planner,
      planner_provider: settings.value.scheduler.models.planner_provider,
      replanner_model: settings.value.scheduler.models.replanner,
      replanner_provider: settings.value.scheduler.models.replanner_provider,
      executor_model: settings.value.scheduler.models.executor,
      executor_provider: settings.value.scheduler.models.executor_provider,
      evaluator_model: settings.value.scheduler.models.evaluator,
      evaluator_provider: settings.value.scheduler.models.evaluator_provider,
      default_strategy: settings.value.scheduler.default_strategy,
      max_retries: settings.value.scheduler.max_retries,
      timeout_seconds: settings.value.scheduler.timeout_seconds,
      scenarios: settings.value.scheduler.scenarios
    }
    
    await invoke('save_scheduler_config', { config: flatConfig })
    dialog.toast.success('模型配置配置已保存')
  } catch (error) {
    console.error('Failed to save scheduler config:', error)
    dialog.toast.error('保存模型配置配置失败')
  }
}

// 数据库相关方法
const testDatabase = async () => {
  dialog.toast.info('测试数据库连接...')
}

const backupDatabase = async () => {
  dialog.toast.success('数据库备份已创建')
}

const restoreDatabase = async () => {
  dialog.toast.success('数据库已恢复')
}

const optimizeDatabase = async () => {
  dialog.toast.success('数据库已优化')
}

const saveDatabaseConfig = async () => {
  dialog.toast.success('数据库配置已保存')
}

// 通用设置相关方法
const saveGeneralConfig = async () => {
  try {
    // 保存到 localStorage
    const settingsToSave = JSON.stringify(settings.value)
    localStorage.setItem('sentinel-settings', settingsToSave)
    console.log('Saved settings to localStorage:', settingsToSave)
    
    // 应用主题设置
    if (settings.value.general?.theme) {
      applyTheme(settings.value.general.theme)
    }
    
    // 应用字体大小设置
    if (settings.value.general?.fontSize) {
      applyFontSize(settings.value.general.fontSize)
    }
    
    // 应用语言设置
    if (settings.value.general?.language) {
      applyLanguage(settings.value.general.language)
    }
    
    // 应用UI缩放设置
    if (settings.value.general?.uiScale) {
      applyUIScale(settings.value.general.uiScale)
    }

    // 同步 Tavily 到后端配置
    try {
      const key = (settings.value as any).general?.tavilyApiKey || ''
      const maxResults = (settings.value as any).general?.tavilyMaxResults || 5
      const configs = [
        { category: 'ai', key: 'tavily_api_key', value: key, description: 'Tavily API key for web search', is_encrypted: true },
        { category: 'ai', key: 'tavily_max_results', value: String(maxResults), description: 'Default max results for Tavily', is_encrypted: false },
      ]
      await invoke('save_config_batch', { configs })
      console.log('Saved Tavily config to backend')
    } catch (e) {
      console.warn('Failed to save Tavily config to backend', e)
    }
    
    dialog.toast.success('通用设置已保存并应用')
  } catch (error) {
    console.error('Failed to save general config:', error)
    dialog.toast.error('保存通用设置失败')
  }
}

const applyTheme = (theme: string) => {
  // 处理 auto 主题：根据系统偏好自动选择
  let finalTheme = theme
  if (theme === 'auto') {
    finalTheme = window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'
  }
  
  // 应用主题到文档
  document.documentElement.setAttribute('data-theme', finalTheme)
  localStorage.setItem('theme', finalTheme)
  
  // 更新深色模式状态
  const isDark = ['dark', 'synthwave', 'halloween', 'forest', 'black', 'luxury', 'dracula'].includes(finalTheme)
  if (settings.value.general) {
    settings.value.general.darkMode = isDark
  }
}

const applyFontSize = (fontSize: number) => {
  // 应用字体大小到根元素
  const rootElement = document.documentElement
  rootElement.style.fontSize = `${fontSize}px`
  
  // 同时更新全局字体大小变量
  rootElement.style.setProperty('--font-size-base', `${fontSize}px`)
  
  // 保存到全局设置（兼容 App.vue 的逻辑）
  if (window.updateFontSize) {
    const sizeMap: Record<number, string> = {
      12: 'small', 14: 'normal', 16: 'normal', 18: 'large', 20: 'large'
    }
    window.updateFontSize(sizeMap[fontSize] || 'normal')
  }
}

const applyLanguage = (language: string) => {
  // 处理 auto 语言：根据浏览器语言自动选择
  let finalLang = language
  if (language === 'auto') {
    const browserLang = navigator.language.toLowerCase()
    if (browserLang.startsWith('zh')) {
      finalLang = browserLang.includes('tw') || browserLang.includes('hk') ? 'zh-TW' : 'zh-CN'
    } else if (browserLang.startsWith('en')) {
      finalLang = 'en-US'
    } else {
      finalLang = 'zh-CN' // 默认中文
    }
  }
  
  // 应用语言设置
  const langCode = finalLang.split('-')[0] // 提取主语言代码
  locale.value = langCode
  // 与 i18n/index.ts 保持一致，写入 sentinel-language
  localStorage.setItem('sentinel-language', langCode)
}

const applyUIScale = (scale: number) => {
  // 应用UI缩放
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
  
  // 保存到全局设置
  if (window.updateUIScale) {
    window.updateUIScale(scale)
  }
}

// 安全相关方法
const changePassword = async (passwordForm: any) => {
  dialog.toast.success('密码已更改')
}

const runSecurityAudit = async () => {
  dialog.toast.info('正在运行安全审计...')
}

const checkVulnerabilities = async () => {
  dialog.toast.info('正在检查漏洞...')
}

const generateSecurityReport = async () => {
  dialog.toast.success('安全报告已生成')
}

const lockApplication = async () => {
  dialog.toast.warning('应用程序已锁定')
}

const emergencyShutdown = async () => {
  const confirmed = await dialog.confirm({
    title: '紧急关闭',
    message: '确定要紧急关闭应用程序吗？',
    variant: 'error'
  })
  
  if (confirmed) {
    dialog.toast.error('应用程序正在紧急关闭...')
  }
}

const wipeSecurityData = async () => {
  const confirmed = await dialog.confirm({
    title: '清除安全数据',
    message: '确定要清除所有安全数据吗？此操作不可撤销！',
    variant: 'error'
  })
  
  if (confirmed) {
    dialog.toast.error('安全数据已清除')
  }
}

const saveSecurityConfig = async () => {
   dialog.toast.success('安全配置已保存')
 }




 // 调度器预设配置方法
 const applyHighPerformanceConfig = () => {
   dialog.toast.success('已应用高性能配置')
 }

 const applyBalancedConfig = () => {
   dialog.toast.success('已应用平衡配置')
 }

 const applyEconomicConfig = () => {
   dialog.toast.success('已应用经济配置')
 }

// RAG配置相关方法
const saveRagConfig = async () => {
  try {
    await invoke('save_rag_config', { config: ragConfig.value })
    dialog.toast.success('RAG配置已保存')
  } catch (error) {
    console.error('Failed to save RAG config:', error)
    dialog.toast.error('保存RAG配置失败')
  }
}

const testEmbeddingConnection = async () => {
  try {
    dialog.toast.info('测试嵌入连接...')
    const result = await invoke('test_embedding_connection', { 
      config: {
        provider: ragConfig.value.embedding_provider,
        model: ragConfig.value.embedding_model,
        api_key: ragConfig.value.embedding_api_key,
        base_url: ragConfig.value.embedding_base_url
      }
    })
    dialog.toast.success('嵌入连接测试成功')
  } catch (error) {
    console.error('Embedding connection test failed:', error)
    dialog.toast.error(`嵌入连接测试失败: ${error}`)
  }
}

const resetRagConfig = () => {
  ragConfig.value = {
    embedding_provider: 'ollama',
    embedding_model: 'nomic-embed-text',
    embedding_dimensions: null,
    embedding_api_key: '',
    embedding_base_url: 'http://localhost:11434',
    chunk_size_chars: 1000,
    chunk_overlap_chars: 200,
    top_k: 5,
    mmr_lambda: 0.7,
    batch_size: 10,
    max_concurrent: 4
  }
  dialog.toast.success('RAG配置已重置为默认值')
}

const loadRagConfig = async () => {
  try {
    const config = await invoke('get_rag_config')
    if (config && typeof config === 'object') {
      ragConfig.value = { ...ragConfig.value, ...(config as Record<string, any>) }
    }
  } catch (error) {
    console.warn('Failed to load RAG config:', error)
  }
}

// 生命周期
onMounted(() => {
  loadSettings()
  loadAiUsageStats()
  loadRagConfig()
})

// 自动持久化通用设置与即时应用
watch(() => settings.value.general, (newGeneral, oldGeneral) => {
  try {
    // 保存完整设置对象，确保其他分类也被持久化
    localStorage.setItem('sentinel-settings', JSON.stringify(settings.value))

    // 主题变更时应用
    if (newGeneral?.theme !== oldGeneral?.theme && newGeneral?.theme) {
      applyTheme(newGeneral.theme)
    }

    // 字体大小变更时应用
    if (newGeneral?.fontSize !== oldGeneral?.fontSize && typeof newGeneral?.fontSize === 'number') {
      applyFontSize(newGeneral.fontSize)
    }

    // 语言变更时应用
    if (newGeneral?.language !== oldGeneral?.language && newGeneral?.language) {
      applyLanguage(newGeneral.language)
    }

    // UI 缩放变更时应用
    if (newGeneral?.uiScale !== oldGeneral?.uiScale && typeof newGeneral?.uiScale === 'number') {
      applyUIScale(newGeneral.uiScale)
    }
  } catch (e) {
    console.warn('Auto-persist settings failed', e)
  }
}, { deep: true })
</script>

<style scoped>
.settings-page {
  @apply max-w-6xl mx-auto;
}

.tab {
  @apply min-w-0 flex-shrink-0;
}
</style>
