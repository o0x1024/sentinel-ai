<template>
  <div class="settings-page page-content-padded safe-top">
    <!-- 顶部标题栏 -->
    <div class="flex justify-between items-center mb-6">
      <h2 class="text-2xl font-bold">{{ t('settings.title', '系统设置') }}</h2>
      <!-- <div class="flex gap-2">
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
      </div> -->
    </div>

    <!-- 左右布局容器 -->
    <div class="settings-layout flex ">
      <!-- 左侧导航栏 -->
      <div class="settings-sidebar flex-shrink-0">
        <div class="menu bg-base-200 rounded-box p-2">
          <li 
            v-for="category in categories" 
            :key="category.id"
            class="mb-1"
          >
            <a 
              class="menu-item"
              :class="{ 'active': activeCategory === category.id }"
              @click="activeCategory = category.id"
            >
              <i :class="category.icon"></i>
              <span>{{ t(`settings.categories.${category.id}`) }}</span>
            </a>
          </li>
        </div>
      </div>

      <!-- 右侧内容区域 -->
      <div class="settings-content flex-1 min-w-0">
        <!-- AI服务配置 -->
        <AISettings v-if="activeCategory === 'ai'" 
                    :ai-service-status="aiServiceStatus"
                    :ai-config="aiConfig"
                    v-model:selected-ai-provider="selectedAiProvider"
                    :settings="settings"
                    :custom-provider="customProvider"
                    :ai-usage-stats="aiUsageStats"
                    :saving="saving"
                    :testing-custom-provider="testingCustomProvider"
                    :adding-custom-provider="addingCustomProvider"
                    @test-connection="testConnection"
                    @save-ai-config="saveAiConfig"
                    @test-custom-provider="testCustomProvider"
                    @add-custom-provider="addCustomProvider"
                    @refresh-models="refreshModels"
                    @apply-manual-config="applyManualConfig"
                    @set-default-provider="setDefaultProvider"
                    @set-default-chat-model="setDefaultChatModel"
                    @set-default-vlm-provider="setDefaultVlmProvider"
                    @set-default-vision-model="setDefaultVisionModel"
                    @set-enable-multimodal="setEnableMultimodal"
                    @clear-usage-stats="clearAiUsageStats" />

        <!-- 知识库配置 -->
        <RAGSettings v-if="activeCategory === 'rag'"
                     v-model:rag-config="ragConfig"
                     :available-providers="availableProviders"
                     :available-models="availableModels"
                     :saving="saving"
                     @save-rag-config="saveRagConfig"
                     @test-embedding-connection="testEmbeddingConnection"
                     @reset-rag-config="resetRagConfig" />


        <!-- Agent设置 -->
        <AgentSettings v-if="activeCategory === 'agent'" />

        <!-- 数据库设置 -->
        <DatabaseSettings v-if="activeCategory === 'database'" 
                          v-model:settings="settings"
                          :database-status="databaseStatus"
                          :saving="saving"
                          @selectDatabasePath="selectDatabasePath"
                          @selectBackupPath="selectBackupPath"
                          @testDatabaseConnection="testDatabaseConnection"
                          @createBackup="createBackup"
                          @selectBackupFile="selectBackupFile"
                          @exportData="exportData"
                          @importData="importData"
                          @migrateDatabase="migrateDatabase"
                          @cleanupNow="cleanupNow"
                          @optimizeDatabase="optimizeDatabase"
                          @rebuildIndexes="rebuildIndexes"
                          @resetDatabase="resetDatabase"
                          @saveDatabaseConfig="saveDatabaseConfig" />

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
    </div>

  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted, watch, nextTick } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { dialog } from '@/composables/useDialog'
import AISettings from '@/components/Settings/AISettings.vue'
import RAGSettings from '@/components/Settings/RAGSettings.vue'
import DatabaseSettings from '@/components/Settings/DatabaseSettings.vue'
import GeneralSettings from '@/components/Settings/GeneralSettings.vue'
import SecuritySettings from '@/components/Settings/SecuritySettings.vue'
import NetworkSettings from '@/components/Settings/NetworkSettings.vue'
import AgentSettings from '@/components/Settings/AgentSettings.vue'

const { t, locale } = useI18n()

// 响应式数据
const activeCategory = ref('ai')
const saving = ref(false)

defineOptions({
  name: 'Settings'
});

// Database configuration interface
interface DatabaseConfig {
  db_type: string
  path?: string
  host?: string
  port?: number
  database?: string
  username?: string
  password?: string
  enable_wal?: boolean
  enable_ssl?: boolean
  max_connections?: number
  query_timeout?: number
}

// 设置分类
const categories = [
  { id: 'ai', icon: 'fas fa-robot' },
  { id: 'rag', icon: 'fas fa-database' },
  { id: 'agent', icon: 'fas fa-user-cog' },
  { id: 'database', icon: 'fas fa-server' },
  { id: 'system', icon: 'fas fa-cog' },
  { id: 'security', icon: 'fas fa-shield-alt' },
  { id: 'network', icon: 'fas fa-network-wired' },
]

// 设置数据
const settings = ref({
  ai: {
    temperature: 0.7,
    maxTokens: 2000,
    toolOutputLimit: 50000,
    maxTurns: 100
  },
  database: {
    type: 'sqlite',
    path: '',
    host: 'localhost',
    port: 5432,
    name: 'sentinel_ai',
    username: '',
    password: '',
    maxConnections: 10,
    queryTimeout: 30,
    enableWAL: true,
    enableSSL: false,
    autoBackup: true,
    backupFrequency: 'daily',
    backupRetention: 7,
    backupPath: '',
    autoCleanup: false,
    retentionDays: 30,
    cleanupLogs: true,
    cleanupTempFiles: true,
    cleanupOldSessions: true
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
const aiConfig = ref<any>({ providers: {}, default_llm_provider: 'openai' })
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
  chunking_strategy: 'RecursiveCharacter',
  min_chunk_size_chars: 100,
  max_chunk_size_chars: 3000,
  top_k: 5,
  mmr_lambda: 0.7,
  similarity_threshold: 0.7,
  batch_size: 10,
  max_concurrent: 4,
  reranking_enabled: false,
  reranking_provider: '',
  reranking_model: '',
  augmentation_enabled: false
})

const loadAiUsageStats = async () => {
  try {
    const stats = await invoke('get_ai_usage_stats') as Record<string, { input_tokens: number, output_tokens: number, total_tokens: number, cost: number }>
    aiUsageStats.value = stats || {}
  } catch (e) {
    console.warn('Failed to load AI usage stats', e)
  }
}

const clearAiUsageStats = async () => {
  const confirmed = await dialog.confirm({
    title: t('common.confirm'),
    message: t('settings.ai.confirmClearStats', 'Are you sure you want to clear AI usage statistics?'),
    variant: 'warning'
  })
  
  if (confirmed) {
    try {
      await invoke('clear_ai_usage_stats')
      await loadAiUsageStats()
      dialog.toast.success(t('settings.ai.statsCleared', 'Usage statistics cleared'))
    } catch (e) {
      console.error('Failed to clear AI usage stats', e)
      dialog.toast.error('Failed to clear statistics')
    }
  }
}

// 单独加载 AI 配置
const loadAiConfig = async () => {
  try {
    const aiConfigData = await invoke('get_ai_config')
    aiConfig.value = aiConfigData as any
    
    // 加载 enable_multimodal 配置
    try {
      const configs = await invoke('get_config', { request: { category: 'ai', key: 'enable_multimodal' } }) as Array<{ key: string, value: string }>
      if (configs && configs.length > 0) {
        aiConfig.value.enable_multimodal = configs[0].value === 'true'
      } else {
        // 默认启用多模态
        aiConfig.value.enable_multimodal = true
      }
    } catch {
      // 默认启用多模态
      aiConfig.value.enable_multimodal = true
    }
    
    console.log('Reloaded AI config:', aiConfig.value)
  } catch (e) {
    console.error('Failed to load AI config', e)
  }
}
const customProvider = reactive({
  name: '',
  api_key: '',
  api_base: '',
  model_id: '',
  display_name: '',
  rig_provider: '',
  compat_mode: 'openai', // openai, anthropic, rig_openai, rig_anthropic
  extra_headers_json: '',
  timeout: 120,
  max_retries: 3,
})

// 测试/添加自定义提供商的状态
const testingCustomProvider = ref(false)
const addingCustomProvider = ref(false)

// 数据库状态
const databaseStatus = ref({
  connected: false,
  type: 'SQLite',
  size: 0,
  tables: 0,
  lastBackup: null as string | null,
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
  
  Object.entries(aiConfig.value.providers || {}).forEach(([providerKey, provider]: [string, any]) => {
    if (provider.models && Array.isArray(provider.models)) {
      provider.models.forEach((model: any) => {
        // 放宽条件：默认展示；若存在 is_available 显式为 false 则过滤
        if (model.is_available !== false) {
          models.push({
            ...model,
            provider: providerKey, // 使用提供商的 KEY 作为名称，确保一致性
          })
        }
      })
    }
  })
  
  console.log('Available models for scheduler:', models)
  return models
})

const availableProviders = computed(() => {
  const providers: string[] = []
  
  Object.entries(aiConfig.value.providers || {}).forEach(([providerKey, provider]: [string, any]) => {
    if (provider.enabled) {
      if (!providers.includes(providerKey)) {
        providers.push(providerKey)
      }
    }
  })
  
  console.log('Available providers:', providers)
  return providers
})

// 方法
const loadSettings = async () => {
  try {
    // Load database configuration from persistent file first
    try {
      const dbConfig = await invoke('load_db_config') as DatabaseConfig | null
      if (dbConfig) {
        console.log('Loaded saved database config:', dbConfig)
        settings.value.database.type = dbConfig.db_type
        settings.value.database.path = dbConfig.path || ''
        settings.value.database.host = dbConfig.host || 'localhost'
        settings.value.database.port = dbConfig.port || 5432
        settings.value.database.name = dbConfig.database || 'sentinel_ai'
        settings.value.database.username = dbConfig.username || ''
        settings.value.database.password = dbConfig.password || ''
        settings.value.database.enableWAL = dbConfig.enable_wal
        settings.value.database.enableSSL = dbConfig.enable_ssl
        settings.value.database.maxConnections = dbConfig.max_connections
        settings.value.database.queryTimeout = dbConfig.query_timeout
      }
    } catch (dbConfigError) {
      console.error('Failed to load database config:', dbConfigError)
      // Continue with default SQLite config
    }

    // 先加载AI配置
    const aiConfigData = await invoke('get_ai_config')
    aiConfig.value = aiConfigData as any
    
    // 加载高级AI配置参数
    try {
      const configs = await invoke('get_config', { request: { category: 'ai', key: null } }) as Array<{ key: string, value: string }>
      
      const configMap = new Map(configs.map(c => [c.key, c.value]))
      
      // enable_multimodal
      if (configMap.has('enable_multimodal')) {
        aiConfig.value.enable_multimodal = configMap.get('enable_multimodal') === 'true'
      } else {
        aiConfig.value.enable_multimodal = true
      }
      
      // temperature
      if (configMap.has('temperature')) {
         const temp = parseFloat(configMap.get('temperature') || '0.7')
         settings.value.ai.temperature = temp
      }
      
      // max_tokens
      if (configMap.has('max_tokens')) {
         const tokens = parseInt(configMap.get('max_tokens') || '2000')
         settings.value.ai.maxTokens = tokens
      }
      
      // tool_output_limit
      if (configMap.has('tool_output_limit')) {
         const limit = parseInt(configMap.get('tool_output_limit') || '50000')
         settings.value.ai.toolOutputLimit = limit
      }
      
      // max_turns
      if (configMap.has('max_turns')) {
         const maxTurns = parseInt(configMap.get('max_turns') || '100')
         settings.value.ai.maxTurns = maxTurns
      }
    } catch (e) {
      console.warn('Failed to load extra AI configs', e)
      // 默认启用多模态
      aiConfig.value.enable_multimodal = true
    }
    
    console.log('Loaded AI config:', aiConfig.value)
    
    // 等待一个 tick 确保 aiConfig 更新完成
    await nextTick()
    
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
          // Skip loading AI settings from localStorage as they are managed by the backend
          // and loaded via get_config above. This prevents stale localStorage data
          // from overwriting fresh backend configuration.
          // Object.assign(settings.value.ai, parsed.ai)
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

    // 使用 rig_provider 来调用后端 API，如果没有配置则使用 provider 名称的小写形式
    const rigProvider = providerConfig.rig_provider || provider.toLowerCase()
    console.log(`Testing connection for ${provider} using rig_provider: ${rigProvider}`)

    // 构建请求参数
    const request = {
      provider: rigProvider,
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

    // 使用 rig_provider 来调用后端 API，如果没有配置则使用 provider 名称的小写形式
    const rigProvider = providerConfig.rig_provider || provider.toLowerCase()
    console.log(`Refreshing models for ${provider} using rig_provider: ${rigProvider}`)

    // 调用新的API获取实时模型列表
    const modelIds = await invoke('get_provider_models', {
      provider: rigProvider,
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
    
    // 立即保存到数据库以持久化模型列表
    await saveAiConfig()
    
    dialog.toast.success(`${provider} 模型列表已刷新并保存，找到 ${models.length} 个模型`)
    
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
    // 先保存高级配置参数到数据库（在 reload_services 之前）
    try {
       const temperature = settings.value.ai?.temperature ?? 0.7
       const maxTokens = settings.value.ai?.maxTokens ?? 2000
       const toolLimit = settings.value.ai?.toolOutputLimit || 50000
       const maxTurns = settings.value.ai?.maxTurns || 100
       const configs = [
          { category: 'ai', key: 'temperature', value: String(temperature), description: 'Temperature for AI responses', is_encrypted: false },
          { category: 'ai', key: 'max_tokens', value: String(maxTokens), description: 'Max tokens for AI generation', is_encrypted: false },
          { category: 'ai', key: 'tool_output_limit', value: String(toolLimit), description: 'Max chars for tool output', is_encrypted: false },
          { category: 'ai', key: 'max_turns', value: String(maxTurns), description: 'Max conversation turns for tool calls', is_encrypted: false }
       ]
       await invoke('save_config_batch', { configs })
    } catch(e) {
       console.error('Failed to save AI advanced configs', e)
    }
    
    // 然后保存 AI 配置（这会触发 reload_services，加载上面保存的配置）
    await invoke('save_ai_config', { config: aiConfig.value })

    dialog.toast.success('AI配置已保存')
  } catch (error) {
    console.error('Failed to save AI config:', error)
    dialog.toast.error('保存AI配置失败')
  }
}

const setDefaultProvider = async (provider: string) => {
  try {
    await invoke('set_default_llm_provider', { request: { provider } })
    // 同步前端状态
    aiConfig.value.default_llm_provider = provider
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
      aiConfig.value.default_llm_model = ''
      console.log('Settings: Cleared default_llm_model')
      dialog.toast.success('已清空默认 Chat 模型')
      return
    }
    
    // 获取当前默认提供商（小写格式）
    const currentProviderLower = aiConfig.value.default_llm_provider || 'openai'
    
    // 在提供商配置中查找匹配的提供商（不区分大小写）
    const providerConfigKey = Object.keys(aiConfig.value.providers || {}).find(key => 
      key.toLowerCase() === currentProviderLower.toLowerCase()
    )
    
    if (!providerConfigKey) {
      dialog.toast.error('当前默认提供商配置不存在')
      console.error('Provider not found:', {
        searchFor: currentProviderLower,
        availableProviders: Object.keys(aiConfig.value.providers || {}),
        aiConfigDefaultLlmProvider: aiConfig.value.default_llm_provider
      })
      return
    }
    
    const providerConfig = aiConfig.value.providers[providerConfigKey]
    if (!providerConfig) {
      dialog.toast.error('提供商配置无效')
      return
    }
    
    // 从提供商的模型列表中找到匹配的模型（可选，用于显示友好名称）
    const modelInfo = providerConfig.models?.find((m: any) => m.id === model)
    
    // 调用后端API设置默认LLM模型 - 使用正确的命令
    const modelValue = `${currentProviderLower.toLowerCase()}/${model}`
    await invoke('set_default_llm_model', {
      model: modelValue
    })
    
    // 同步前端状态 - 保存为 'provider/model' 格式
    aiConfig.value.default_llm_model = modelValue
    console.log('Updated frontend default_llm_model state:', modelValue)
    
    // 如果找到模型信息则显示友好名称，否则显示模型ID
    const displayName = modelInfo ? modelInfo.name : model
    dialog.toast.success(`默认 Chat 模型已设置为 ${displayName}`)
  } catch (e) {
    dialog.toast.error('设置默认 Chat 模型失败')
  }
}

const setDefaultVlmProvider = async (provider: string) => {
  try {
    await invoke('save_config_batch', {
      configs: [{
        category: 'ai',
        key: 'default_vlm_provider',
        value: String(provider),
        description: 'Default VLM provider',
        is_encrypted: false
      }, {
        category: 'ai',
        key: 'default_vlm_provider',
        value: String(provider),
        description: 'Deprecated: use default_vlm_provider',
        is_encrypted: false
      }]
    })
    aiConfig.value.default_vlm_provider = provider
    dialog.toast.success(`默认 VLM Provider 已设置为 ${provider}`)
  } catch (e) {
    console.error('Failed to set default VLM provider', e)
    dialog.toast.error('设置默认 VLM Provider 失败')
  }
}

const setDefaultVisionModel = async (model: string) => {
  try {
    if (!model) {
      // 清空默认模型
      aiConfig.value.default_vlm_model = ''
      console.log('Settings: Cleared default_vlm_model')
      dialog.toast.success('已清空默认 VLM 模型')
      return
    }
    
    // 保存默认 VLM 模型配置
    await invoke('save_config_batch', {
      configs: [{
        category: 'ai',
        key: 'default_vlm_model',
        value: String(model),
        description: 'Default VLM model',
        is_encrypted: false
      }]
    })
    
    // 同步前端状态 - 保存为 'provider/model' 格式
    aiConfig.value.default_vlm_model = model
    console.log('Updated frontend default_vlm_model state:', model)
    
    let modelName = model
    if (model.includes('/')) {
      const [providerLower, modelId] = model.split('/', 2)
      const providerConfigKey = Object.keys(aiConfig.value.providers || {}).find(key => 
        key.toLowerCase() === providerLower.toLowerCase()
      )
      const providerConfig = providerConfigKey ? aiConfig.value.providers[providerConfigKey] : null
      const modelInfo = providerConfig?.models?.find((m: any) => m.id === modelId)
      modelName = modelInfo?.name || modelId
    }
    
    dialog.toast.success(`默认 VLM 模型已设置为 ${modelName}`)
  } catch (e) {
    console.error('Failed to set default vision model', e)
    dialog.toast.error('设置默认 VLM 模型失败')
  }
}

const setEnableMultimodal = async (enabled: boolean) => {
  try {
    // 保存到数据库配置
    await invoke('save_config_batch', {
      configs: [{
        category: 'ai',
        key: 'enable_multimodal',
        value: String(enabled),
        description: '是否启用多模态模式（截图）',
        is_encrypted: false
      }]
    })
    // 同步前端状态
    aiConfig.value.enable_multimodal = enabled
    dialog.toast.success(enabled ? '已启用多模态模式' : '已切换到文本模式')
  } catch (e) {
    console.error('Failed to set enable multimodal', e)
    dialog.toast.error('设置多模态模式失败')
  }
}

const testCustomProvider = async () => {
  testingCustomProvider.value = true
  try {
    // 解析额外请求头
    let extraHeaders: Record<string, string> = {}
    if (customProvider.extra_headers_json && customProvider.extra_headers_json.trim()) {
      try {
        extraHeaders = JSON.parse(customProvider.extra_headers_json)
      } catch {
        dialog.toast.error('额外请求头 JSON 格式无效')
        return
      }
    }
    
    const request = {
      name: customProvider.name.trim(),
      api_key: customProvider.api_key || null,
      api_base: customProvider.api_base.trim(),
      model_id: customProvider.model_id.trim(),
      compat_mode: customProvider.compat_mode,
      extra_headers: Object.keys(extraHeaders).length > 0 ? extraHeaders : null,
      timeout: customProvider.timeout || 120,
    }
    
    dialog.toast.info('正在测试自定义提供商连接...')
    const result = await invoke('test_custom_provider', { request }) as { success: boolean, message: string }
    
    if (result.success) {
      dialog.toast.success(result.message || '连接测试成功')
    } else {
      dialog.toast.error(result.message || '连接测试失败')
    }
  } catch (e) {
    console.error('Test custom provider failed:', e)
    dialog.toast.error(`测试失败: ${e}`)
  } finally {
    testingCustomProvider.value = false
  }
}

const addCustomProvider = async () => {
  addingCustomProvider.value = true
  try {
    // 解析额外请求头
    let extraHeaders: Record<string, string> = {}
    if (customProvider.extra_headers_json && customProvider.extra_headers_json.trim()) {
      try {
        extraHeaders = JSON.parse(customProvider.extra_headers_json)
      } catch {
        dialog.toast.error('额外请求头 JSON 格式无效')
        return
      }
    }
    
    const providerName = customProvider.name.trim()
    const displayName = customProvider.display_name.trim() || providerName
    
    const request = {
      name: providerName,
      display_name: displayName,
      api_key: customProvider.api_key || null,
      api_base: customProvider.api_base.trim(),
      model_id: customProvider.model_id.trim(),
      compat_mode: customProvider.compat_mode,
      extra_headers: Object.keys(extraHeaders).length > 0 ? extraHeaders : null,
      timeout: customProvider.timeout || 120,
      max_retries: customProvider.max_retries || 3,
    }
    
    await invoke('add_custom_provider', { request })
    dialog.toast.success(`自定义提供商 "${displayName}" 已添加`)
    
    // 重置表单
    customProvider.name = ''
    customProvider.display_name = ''
    customProvider.api_key = ''
    customProvider.api_base = ''
    customProvider.model_id = ''
    customProvider.compat_mode = 'openai'
    customProvider.extra_headers_json = ''
    customProvider.timeout = 120
    customProvider.max_retries = 3
    
    // 重新加载 AI 配置
    await loadAiConfig()
  } catch (e) {
    console.error('Add custom provider failed:', e)
    dialog.toast.error(`添加失败: ${e}`)
  } finally {
    addingCustomProvider.value = false
  }
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

// 调度器相关方法已移除 - 模型配置现在使用 AISettings 中的默认 LLM 模型

// 数据库相关方法
const loadDatabaseStatus = async () => {
  try {
    const status = await invoke('get_database_status') as any
    console.log('Database status from backend:', status)
    
    databaseStatus.value = {
      connected: status.connected ?? false,
      type: status.db_type ?? 'SQLite',
      size: status.size ?? 0,
      tables: status.tables ?? 0,
      lastBackup: status.last_backup ?? null,
      backupCount: 0
    }
    
    // 同步数据库路径到设置
    if (status.path) {
      settings.value.database.path = status.path
    }
  } catch (error) {
    console.error('Failed to load database status:', error)
  }
}


const selectDatabasePath = async () => {
  try {
    const { open } = await import('@tauri-apps/plugin-dialog')
    const selected = await open({
      directory: false,
      multiple: false,
      filters: [{ name: 'Database', extensions: ['db', 'sqlite', 'sqlite3'] }]
    })
    if (selected) {
      settings.value.database.path = selected as string
      dialog.toast.success('数据库路径已选择')
    }
  } catch (error) {
    console.error('Failed to select database path:', error)
    dialog.toast.error('选择数据库路径失败')
  }
}

const selectBackupPath = async () => {
  try {
    const { open } = await import('@tauri-apps/plugin-dialog')
    const selected = await open({
      directory: true,
      multiple: false,
      title: '选择备份保存目录'
    })
    if (selected) {
      settings.value.database.backupPath = selected as string
      dialog.toast.success('备份路径已选择')
    }
  } catch (error) {
    console.error('Failed to select backup path:', error)
    dialog.toast.error('选择备份路径失败')
  }
}

const testDatabaseConnection = async () => {
  dialog.toast.info('正在测试数据库连接...')
  try {
    // Build database config from settings
    const dbConfig = {
      db_type: settings.value.database.type,
      path: settings.value.database.type === 'sqlite' ? settings.value.database.path : null,
      enable_wal: settings.value.database.enableWAL,
      host: settings.value.database.type !== 'sqlite' ? settings.value.database.host : null,
      port: settings.value.database.type !== 'sqlite' ? settings.value.database.port : null,
      database: settings.value.database.type !== 'sqlite' ? settings.value.database.name : null,
      username: settings.value.database.type !== 'sqlite' ? settings.value.database.username : null,
      password: settings.value.database.type !== 'sqlite' ? settings.value.database.password : null,
      enable_ssl: settings.value.database.enableSSL,
      max_connections: settings.value.database.maxConnections,
      query_timeout: settings.value.database.queryTimeout,
    }
    
    const result = await invoke('test_db_connection', { config: dbConfig })
    if (result) {
      dialog.toast.success('数据库连接正常')
    } else {
      dialog.toast.error('数据库连接失败')
    }
  } catch (error) {
    console.error('Database connection test failed:', error)
    dialog.toast.error(`数据库连接测试失败: ${error}`)
  }
}

const createBackup = async () => {
  dialog.toast.info('正在创建数据库备份...')
  try {
    const backupPath = await invoke('create_database_backup', { 
      backupPath: settings.value.database.backupPath || null 
    }) as string
    await loadDatabaseStatus()
    dialog.toast.success(`数据库备份已创建: ${backupPath}`)
  } catch (error) {
    console.error('Failed to create backup:', error)
    dialog.toast.error(`创建备份失败: ${error}`)
  }
}

const selectBackupFile = async () => {
  try {
    const { open } = await import('@tauri-apps/plugin-dialog')
    const selected = await open({
      directory: false,
      multiple: false,
      filters: [{ name: 'Database Backup', extensions: ['db'] }],
      title: '选择要恢复的备份文件'
    })
    if (selected) {
      const confirmed = await dialog.confirm({
        title: '确认恢复',
        message: '恢复备份将覆盖当前数据库，是否继续？此操作不可撤销。',
        variant: 'warning'
      })
      if (confirmed) {
        dialog.toast.info('正在恢复数据库...')
        await invoke('restore_database_backup', { backupPath: selected as string })
        await loadDatabaseStatus()
        dialog.toast.success('数据库已从备份恢复，请重启应用以使更改生效')
      }
    }
  } catch (error) {
    console.error('Failed to restore backup:', error)
    dialog.toast.error(`恢复备份失败: ${error}`)
  }
}

const exportData = async () => {
  try {
    const { save } = await import('@tauri-apps/plugin-dialog')
    
    // Ask user to choose export format
    const format = await dialog.confirm({
      title: '选择导出格式',
      message: '请选择导出格式:\n\nJSON - 完整的数据结构\nSQL - SQL脚本文件',
      confirmText: 'JSON',
      cancelText: 'SQL'
    })
    
    const extension = format ? 'json' : 'sql'
    const filterName = format ? 'JSON' : 'SQL'
    
    const selected = await save({
      defaultPath: `sentinel_export_${new Date().toISOString().split('T')[0]}.${extension}`,
      filters: [{ name: filterName, extensions: [extension] }],
      title: '导出数据'
    })
    
    if (selected) {
      dialog.toast.info('正在导出数据...')
      
      if (format) {
        // Export as JSON
        await invoke('export_db_to_json', { outputPath: selected })
      } else {
        // Export as SQL
        await invoke('export_db_to_sql', { outputPath: selected })
      }
      
      dialog.toast.success(`数据已导出到: ${selected}`)
    }
  } catch (error) {
    console.error('Failed to export data:', error)
    dialog.toast.error(`导出数据失败: ${error}`)
  }
}

const importData = async () => {
  try {
    const { open } = await import('@tauri-apps/plugin-dialog')
    const selected = await open({
      directory: false,
      multiple: false,
      filters: [
        { name: 'JSON', extensions: ['json'] },
        { name: 'All Files', extensions: ['*'] }
      ],
      title: '选择要导入的数据文件'
    })
    if (selected) {
      const confirmed = await dialog.confirm({
        title: '确认导入',
        message: '导入数据可能会影响现有数据，是否继续？',
        variant: 'warning'
      })
      if (confirmed) {
        dialog.toast.info('正在导入数据...')
        const result = await invoke('import_db_from_json', { inputPath: selected as string }) as string
        dialog.toast.success(result)
      }
    }
  } catch (error) {
    console.error('Failed to import data:', error)
    dialog.toast.error(`导出数据失败: ${error}`)
  }
}

const migrateDatabase = async () => {
  try {
    const confirmed = await dialog.confirm({
      title: '数据库迁移',
      message: '此操作将把当前数据库的所有数据迁移到目标数据库。请确保:\n\n1. 目标数据库已创建并可访问\n2. 目标数据库的表结构已初始化\n3. 已备份源数据库\n\n是否继续?',
      variant: 'warning'
    })
    
    if (!confirmed) return
    
    // Build target database config
    const targetConfig = {
      db_type: settings.value.database.type,
      path: settings.value.database.type === 'sqlite' ? settings.value.database.path : null,
      enable_wal: settings.value.database.enableWAL,
      host: settings.value.database.type !== 'sqlite' ? settings.value.database.host : null,
      port: settings.value.database.type !== 'sqlite' ? settings.value.database.port : null,
      database: settings.value.database.type !== 'sqlite' ? settings.value.database.name : null,
      username: settings.value.database.type !== 'sqlite' ? settings.value.database.username : null,
      password: settings.value.database.type !== 'sqlite' ? settings.value.database.password : null,
      enable_ssl: settings.value.database.enableSSL,
      max_connections: settings.value.database.maxConnections,
      query_timeout: settings.value.database.queryTimeout,
    }
    
    dialog.toast.info('正在迁移数据库...')
    const result = await invoke('migrate_database', { targetConfig }) as string
    dialog.toast.success(result)

    // Save database config to a persistent configuration file
    dialog.toast.info('正在保存数据库配置...')
    try {
      await invoke('save_db_config', { config: targetConfig })
      dialog.toast.success('数据库配置已保存')
    } catch (saveError) {
      console.error('Failed to save database config:', saveError)
      dialog.toast.error(`保存数据库配置失败: ${saveError}`)
    }

    // Show restart prompt
    const restart = await dialog.confirm({
      title: '迁移完成',
      message: '数据库迁移已完成。需要重启应用以使用新数据库。\n\n是否立即重启?',
      variant: 'info'
    })

    if (restart) {
      // Restart the application
      window.location.reload()
    }
  } catch (error) {
    console.error('Failed to migrate database:', error)
    dialog.toast.error(`数据库迁移失败: ${error}`)
  }
}

const cleanupNow = async () => {
  const confirmed = await dialog.confirm({
    title: '确认清理',
    message: '将根据设置清理旧数据，是否继续？',
    variant: 'warning'
  })
  if (confirmed) {
    dialog.toast.info('正在清理数据库...')
    try {
      const result = await invoke('cleanup_database', {
        retentionDays: settings.value.database.retentionDays || 30,
        cleanupLogs: settings.value.database.cleanupLogs ?? true,
        cleanupTempFiles: settings.value.database.cleanupTempFiles ?? true,
        cleanupOldSessions: settings.value.database.cleanupOldSessions ?? true
      }) as string
      await loadDatabaseStatus()
      dialog.toast.success(result)
    } catch (error) {
      console.error('Failed to cleanup database:', error)
      dialog.toast.error(`清理数据库失败: ${error}`)
    }
  }
}

const optimizeDatabase = async () => {
  dialog.toast.info('正在优化数据库...')
  try {
    const result = await invoke('optimize_database') as string
    await loadDatabaseStatus()
    dialog.toast.success(result)
  } catch (error) {
    console.error('Failed to optimize database:', error)
    dialog.toast.error(`优化数据库失败: ${error}`)
  }
}

const rebuildIndexes = async () => {
  dialog.toast.info('正在重建索引...')
  try {
    const result = await invoke('rebuild_database_indexes') as string
    dialog.toast.success(result)
  } catch (error) {
    console.error('Failed to rebuild indexes:', error)
    dialog.toast.error(`重建索引失败: ${error}`)
  }
}

const resetDatabase = async () => {
  const confirmed = await dialog.confirm({
    title: '危险操作',
    message: '此操作将删除所有数据（会自动创建备份）。请输入 "CONFIRM_RESET" 确认。',
    variant: 'error'
  })
  if (confirmed) {
    try {
      dialog.toast.info('正在重置数据库...')
      const result = await invoke('reset_database', { confirmText: 'CONFIRM_RESET' }) as string
      await loadDatabaseStatus()
      dialog.toast.warning(result)
    } catch (error) {
      console.error('Failed to reset database:', error)
      dialog.toast.error(`重置数据库失败: ${error}`)
    }
  }
}

const saveDatabaseConfig = async () => {
  try {
    // 构建数据库配置对象
    const dbConfig = {
      db_type: settings.value.database.type,
      path: settings.value.database.type === 'sqlite' ? settings.value.database.path : null,
      enable_wal: settings.value.database.enableWAL,
      host: settings.value.database.type !== 'sqlite' ? settings.value.database.host : null,
      port: settings.value.database.type !== 'sqlite' ? settings.value.database.port : null,
      database: settings.value.database.type !== 'sqlite' ? settings.value.database.name : null,
      username: settings.value.database.type !== 'sqlite' ? settings.value.database.username : null,
      password: settings.value.database.type !== 'sqlite' ? settings.value.database.password : null,
      enable_ssl: settings.value.database.enableSSL,
      max_connections: settings.value.database.maxConnections,
      query_timeout: settings.value.database.queryTimeout,
    }

    // 保存到后端
    await invoke('save_db_config', { config: dbConfig })

    // 保存数据库相关配置到 localStorage
    const settingsToSave = JSON.stringify(settings.value)
    localStorage.setItem('sentinel-settings', settingsToSave)
    
    dialog.toast.success('数据库配置已保存，请重启应用以生效')
  } catch (error) {
    console.error('Failed to save database config:', error)
    dialog.toast.error(`保存数据库配置失败: ${error}`)
  }
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
  let langCode = finalLang.split('-')[0] // 提取主语言代码
  
  // 仅支持 zh 和 en，其他语言映射到 en
  if (langCode !== 'zh') {
    langCode = 'en'
  }
  
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

// 数据库相关方法
const saveRagConfig = async () => {
  try {
    saving.value = true
    await invoke('save_rag_config', { config: ragConfig.value })
    
    // 重载RAG服务以应用新配置
    try {
      await invoke('reload_rag_service')
      dialog.toast.success('RAG配置已保存并应用')
    } catch (reloadError) {
      console.error('Failed to reload RAG service:', reloadError)
      dialog.toast.warning('RAG配置已保存，但服务重载失败。请重启应用以应用新配置。')
    }
  } catch (error) {
    console.error('Failed to save RAG config:', error)
    dialog.toast.error('保存RAG配置失败')
  } finally {
    saving.value = false
  }
}

const testEmbeddingConnection = async () => {
  try {
    dialog.toast.info('测试嵌入连接...')
    // 只传递 provider 和 model，后端会从 AI 配置中获取 api_key 和 base_url
    const result = await invoke('test_embedding_connection', { 
      config: {
        provider: ragConfig.value.embedding_provider,
        model: ragConfig.value.embedding_model
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
    chunking_strategy: 'RecursiveCharacter',
    min_chunk_size_chars: 100,
    max_chunk_size_chars: 3000,
    top_k: 5,
    mmr_lambda: 0.7,
    similarity_threshold: 0.7,
    batch_size: 10,
    max_concurrent: 4,
    reranking_enabled: false,
    reranking_provider: '',
    reranking_model: '',
    augmentation_enabled: false
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
onMounted(async () => {
  // 首先加载保存的设置
  await loadSettings()
  
  // 然后加载动态数据
  await Promise.all([
    loadAiUsageStats(),
    loadRagConfig(),
  ])
  
  // 最后加载数据库状态，这样可以覆盖 localStorage 中可能过时的数据库路径
  await loadDatabaseStatus()
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
  @apply max-w-full mx-auto;
}

/* 左右布局容器 */
.settings-layout {
  @apply flex gap-6;
  min-height: calc(100vh - 200px);
}

/* 左侧导航栏 */
.settings-sidebar {
  @apply flex-shrink-0;
  width: 200px;
}

.settings-sidebar .menu {
  @apply sticky top-6;
}

.settings-sidebar .menu-item {
  @apply flex items-center gap-3 px-4 py-3 rounded-lg cursor-pointer transition-all;
  @apply hover:bg-base-300;
}

.settings-sidebar .menu-item i {
  @apply w-5 text-center;
}

.settings-sidebar .menu-item.active {
  @apply bg-primary text-primary-content font-medium;
}

.settings-sidebar .menu-item.active:hover {
  @apply bg-primary;
}

/* 右侧内容区域 */
.settings-content {
  @apply flex-1 min-w-0;
}

/* 响应式设计 */
@media (max-width: 1024px) {
  .settings-layout {
    @apply flex-col;
  }
  
  .settings-sidebar {
    @apply w-full;
  }
  
  .settings-sidebar .menu {
    @apply static;
    @apply flex flex-row overflow-x-auto;
  }
  
  .settings-sidebar .menu li {
    @apply mb-0 flex-shrink-0;
  }
  
  .settings-sidebar .menu-item {
    @apply whitespace-nowrap;
  }
}
</style>
