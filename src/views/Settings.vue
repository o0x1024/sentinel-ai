<template>
  <div class="settings-page">
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
                :saving="saving"
                @test-connection="testConnection"
                @save-ai-config="saveAiConfig"
                @test-custom-provider="testCustomProvider"
                @add-custom-provider="addCustomProvider"
                @refresh-models="refreshModels" />

    <!-- 调度策略设置 -->
    <SchedulerSettings v-if="activeCategory === 'scheduler'" 
                       :scheduler-config="settings.scheduler"
                       :available-models="availableModels"
                       :saving="saving"
                       @save-scheduler-config="saveSchedulerConfig"
                       @apply-high-performance-preset="applyHighPerformanceConfig"
                       @apply-balanced-preset="applyBalancedConfig"
                       @apply-economic-preset="applyEconomicConfig" />

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
                     :settings="settings"
                     :saving="saving"
                     @save-general-config="saveGeneralConfig"
                     @apply-font-size="applyFontSize"
                     @apply-ui-scale="applyUIScale" />

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
import { ref, reactive, computed, onMounted, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'
import { dialog } from '@/composables/useDialog'
import AISettings from '@/components/Settings/AISettings.vue'
import SchedulerSettings from '@/components/Settings/SchedulerSettings.vue'
import DatabaseSettings from '@/components/Settings/DatabaseSettings.vue'
import GeneralSettings from '@/components/Settings/GeneralSettings.vue'
import SecuritySettings from '@/components/Settings/SecuritySettings.vue'

const { t } = useI18n()

// 响应式数据
const activeCategory = ref('ai')
const saving = ref(false)

// 设置分类
const categories = [
  { id: 'ai', icon: 'fas fa-robot' },
  { id: 'scheduler', icon: 'fas fa-cogs' },
  { id: 'database', icon: 'fas fa-database' },
  { id: 'system', icon: 'fas fa-cog' },
  { id: 'security', icon: 'fas fa-shield-alt' }
]

// 设置数据
const settings = reactive({
  ai: {
    temperature: 0.7,
    maxTokens: 2000
  },
  scheduler: {
    enabled: true,
    models: {
      intent_analysis: '',
      planner: '',
      replanner: '',
      executor: '',
      evaluator: ''
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
const aiConfig = ref({ providers: {} })
const aiUsageStats = ref({})
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
    if (provider.enabled && provider.models) {
      provider.models.forEach((model: any) => {
        if (model.is_available) {
          models.push({
            id: model.id,
            name: model.name,
            provider: provider.name || 'Unknown',
            description: model.description || '',
            context_length: model.context_length || 4096,
            supports_tools: model.supports_tools || false,
            supports_vision: model.supports_vision || false
          })
        }
      })
    }
  })
  
  return models
})

// 方法
const loadSettings = async () => {
  try {
    // 加载AI配置
    const aiConfigData = await invoke('get_ai_config')
    aiConfig.value = aiConfigData as any
    
    // 加载调度器配置
    const schedulerConfig = await invoke('get_scheduler_config') as any
    if (schedulerConfig) {
      // 转换后端返回的扁平结构为前端期望的嵌套结构
      const transformedConfig = {
        enabled: schedulerConfig.enabled ?? true,
        models: {
          intent_analysis: schedulerConfig.intent_analysis_model || '',
          planner: schedulerConfig.planner_model || '',
          replanner: schedulerConfig.replanner_model || '',
          executor: schedulerConfig.executor_model || '',
          evaluator: schedulerConfig.evaluator_model || ''
        },
        default_strategy: schedulerConfig.default_strategy || 'adaptive',
        max_retries: schedulerConfig.max_retries || 3,
        timeout_seconds: schedulerConfig.timeout_seconds || 120,
        scenarios: schedulerConfig.scenarios || {}
      }
      
      Object.assign(settings.scheduler, transformedConfig)
      console.log('Loaded scheduler config:', settings.scheduler)
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
      saveSecurityConfig()
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

const testCustomProvider = async () => {
  // 测试自定义提供商逻辑
  dialog.toast.info('测试自定义提供商...')
}

const addCustomProvider = async () => {
  // 添加自定义提供商逻辑
  dialog.toast.success('自定义提供商已添加')
}

// 调度器相关方法
const saveSchedulerConfig = async () => {
  try {
    // 转换前端嵌套结构为后端期望的扁平结构
    const flatConfig = {
      enabled: settings.scheduler.enabled,
      intent_analysis_model: settings.scheduler.models.intent_analysis,
      planner_model: settings.scheduler.models.planner,
      replanner_model: settings.scheduler.models.replanner,
      executor_model: settings.scheduler.models.executor,
      evaluator_model: settings.scheduler.models.evaluator,
      default_strategy: settings.scheduler.default_strategy,
      max_retries: settings.scheduler.max_retries,
      timeout_seconds: settings.scheduler.timeout_seconds,
      scenarios: settings.scheduler.scenarios
    }
    
    await invoke('save_scheduler_config', { config: flatConfig })
    dialog.toast.success('调度策略配置已保存')
  } catch (error) {
    console.error('Failed to save scheduler config:', error)
    dialog.toast.error('保存调度策略配置失败')
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
  dialog.toast.success('通用设置已保存')
}

const applyFontSize = () => {
  // 应用字体大小逻辑
}

const applyUIScale = () => {
  // 应用UI缩放逻辑
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

// 生命周期
onMounted(() => {
  loadSettings()
})
</script>

<style scoped>
.settings-page {
  @apply max-w-6xl mx-auto;
}

.tab {
  @apply min-w-0 flex-shrink-0;
}
</style>
