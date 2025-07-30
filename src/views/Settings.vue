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
    <div v-show="activeCategory === 'ai'" class="card bg-base-200 shadow-xl">
      <div class="card-body">
        <h2 class="card-title">
          <i class="fas fa-robot"></i>
          {{ t('settings.aiServiceConfig') }}
        </h2>
        
        <!-- AI提供商状态总览 -->
        <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4 mb-6">
          <div v-for="status in aiServiceStatus" :key="status.provider" 
               class="stat bg-base-100 rounded-lg">
            <div class="stat-figure text-2xl">
              <i :class="getProviderIcon(status.provider)"></i>
            </div>
            <div class="stat-title">{{ getProviderName(status.provider) }}</div>
            <div class="stat-value text-sm" :class="status.is_available ? 'text-success' : 'text-error'">
              {{ status.is_available ? t('settings.ai.connected') : t('settings.ai.disconnected') }}
            </div>
            <div class="stat-desc">{{ status.models_loaded }} {{ t('settings.ai.modelsCount') }}</div>
          </div>
        </div>

        <!-- AI提供商配置选项卡 -->
        <div class="tabs tabs-bordered mb-4">
          <a v-for="provider in Object.keys(aiConfig.providers)" 
             :key="provider"
             class="tab"
             :class="{ 'tab-active': selectedAiProvider === provider }"
             @click="selectedAiProvider = provider">
            {{ getProviderName(provider) }}
          </a>
        </div>

        <!-- 当前选中的AI提供商配置 -->
        <div v-if="selectedProviderConfig" class="space-y-4">
          <!-- 启用/禁用 -->
          <div class="form-control">
            <label class="label cursor-pointer">
              <span class="label-text">{{ t('settings.ai.enable') }} {{ getProviderName(selectedAiProvider) }}</span>
              <input type="checkbox" class="toggle toggle-primary" 
                     v-model="selectedProviderConfig.enabled">
            </label>
          </div>

          <!-- API密钥配置 -->
          <div class="form-control" v-if="needsApiKey(selectedAiProvider)">
            <label class="label">
              <span class="label-text">{{ t('settings.ai.apiKey') }}</span>
            </label>
            <div class="input-group">
              <input type="password" :placeholder="t('settings.apiKeyPlaceholder')" 
                     class="input input-bordered flex-1"
                     v-model="selectedProviderConfig.api_key">
              <button class="btn btn-outline" @click="testConnection(selectedAiProvider)">
                <i class="fas fa-plug"></i>
                {{ t('settings.testConnection') }}
              </button>
            </div>
          </div>

          <!-- API Base URL -->
          <div class="form-control" v-if="selectedProviderConfig.api_base">
            <label class="label">
              <span class="label-text">{{ t('settings.ai.apiBaseUrl') }}</span>
            </label>
            <input type="url" :placeholder="t('settings.ai.apiBaseUrl')" 
                   class="input input-bordered"
                   v-model="selectedProviderConfig.api_base">
          </div>

                     <!-- 组织ID (OpenAI) -->
           <div class="form-control" v-if="selectedAiProvider === 'OpenAI' && selectedProviderConfig && 'organization' in selectedProviderConfig">
             <label class="label">
               <span class="label-text">{{ t('settings.ai.organizationId') }}</span>
             </label>
             <input type="text" :placeholder="t('settings.ai.organizationId')" 
                    class="input input-bordered"
                    v-model="(selectedProviderConfig as any).organization">
          </div>

          <!-- 默认模型选择 -->
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ t('settings.ai.defaultModel') }}</span>
            </label>
            <select class="select select-bordered" v-model="selectedProviderConfig.default_model">
              <option v-for="model in selectedProviderConfig.models" 
                      :key="model.id" :value="model.id">
                {{ model.name }} - {{ model.description }}
              </option>
            </select>
          </div>
          
          <!-- 温度设置 -->
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ t('settings.ai.temperature') }}</span>
            </label>
            <div class="flex items-center gap-4">
              <input 
                v-model.number="settings.ai.temperature"
                type="range"
                min="0"
                max="1"
                step="0.1"
                class="range range-primary flex-1"
              />
              <span class="text-sm min-w-[60px]">{{ settings.ai.temperature }}</span>
            </div>
            <label class="label">
              <span class="label-text-alt">{{ t('settings.ai.temperatureHint') }}</span>
            </label>
          </div>
          
          <!-- 最大Token设置 -->
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ t('settings.ai.maxTokens') }}</span>
            </label>
            <div class="flex items-center gap-4">
              <input 
                v-model.number="settings.ai.maxTokens"
                type="range"
                min="500"
                max="8000"
                step="500"
                class="range range-primary flex-1"
              />
              <span class="text-sm min-w-[60px]">{{ settings.ai.maxTokens }}</span>
            </div>
            <label class="label">
              <span class="label-text-alt">{{ t('settings.ai.maxTokensHint') }}</span>
            </label>
          </div>

          <!-- 可用模型列表 -->
          <div class="collapse collapse-arrow bg-base-100">
            <input type="checkbox">
            <div class="collapse-title text-xl font-medium">
              {{ t('settings.ai.availableModels') }} ({{ selectedProviderConfig.models?.length || 0 }})
            </div>
            <div class="collapse-content">
              <div class="overflow-x-auto">
                <table class="table table-compact w-full">
                  <thead>
                    <tr>
                      <th>{{ t('settings.ai.modelName') }}</th>
                      <th>{{ t('settings.ai.contextLength') }}</th>
                      <th>{{ t('settings.ai.features') }}</th>
                      <th>{{ t('settings.ai.status') }}</th>
                    </tr>
                  </thead>
                  <tbody>
                    <tr v-for="model in selectedProviderConfig.models" :key="model.id">
                      <td>
                        <div>
                          <div class="font-bold">{{ model.name }}</div>
                          <div class="text-sm opacity-50">{{ model.description }}</div>
                        </div>
                      </td>
                      <td>{{ model.context_length?.toLocaleString() }}</td>
                      <td>
                        <div class="flex gap-1">
                          <div v-if="model.supports_streaming" class="badge badge-primary badge-sm">{{ t('settings.ai.streaming') }}</div>
                          <div v-if="model.supports_tools" class="badge badge-secondary badge-sm">{{ t('settings.ai.tools') }}</div>
                          <div v-if="model.supports_vision" class="badge badge-accent badge-sm">{{ t('settings.ai.vision') }}</div>
                        </div>
                      </td>
                      <td>
                        <div :class="model.is_available ? 'badge badge-success' : 'badge badge-error'">
                          {{ model.is_available ? t('settings.ai.available') : t('settings.ai.unavailable') }}
                        </div>
                      </td>
                    </tr>
                  </tbody>
                </table>
              </div>
            </div>
          </div>
        </div>

        <!-- 保存按钮 -->
        <div class="card-actions justify-end">
          <button class="btn btn-primary" @click="saveAiConfig">
            <i class="fas fa-save"></i>
            {{ t('settings.ai.saveConfig') }}
          </button>
        </div>
      </div>
    </div>

    <!-- 自定义AI提供商 -->
    <div v-show="activeCategory === 'ai'" class="card bg-base-200 shadow-xl mt-6">
      <div class="card-body">
        <h2 class="card-title">
          <i class="fas fa-plus-circle"></i>
          {{ t('settings.ai.customProvider') }}
        </h2>
        
        <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ t('settings.ai.providerName') }}</span>
            </label>
            <input type="text" :placeholder="t('settings.ai.providerNamePlaceholder')" 
                   class="input input-bordered"
                   v-model="customProvider.name">
          </div>
          
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ t('settings.ai.apiKey') }}</span>
            </label>
            <input type="password" :placeholder="t('settings.apiKeyPlaceholder')" 
                   class="input input-bordered"
                   v-model="customProvider.api_key">
          </div>
          
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ t('settings.ai.apiBaseUrl') }}</span>
            </label>
            <input type="url" placeholder="https://api.example.com/v1" 
                   class="input input-bordered"
                   v-model="customProvider.api_base">
          </div>
          
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ t('settings.ai.modelId') }}</span>
            </label>
            <input type="text" :placeholder="t('settings.ai.modelIdPlaceholder')" 
                   class="input input-bordered"
                   v-model="customProvider.model_id">
          </div>
        </div>
        
        <div class="card-actions justify-end mt-4">
          <button class="btn btn-outline" @click="testCustomProvider">
            <i class="fas fa-vial"></i>
            {{ t('settings.ai.testCustomProvider') }}
          </button>
          <button class="btn btn-primary" @click="addCustomProvider">
            <i class="fas fa-plus"></i>
            {{ t('settings.ai.addCustomProvider') }}
          </button>
        </div>
      </div>
    </div>

    <!-- AI使用统计 -->
    <div v-show="activeCategory === 'ai'" class="card bg-base-200 shadow-xl mt-6">
      <div class="card-body">
        <h2 class="card-title">
          <i class="fas fa-chart-bar"></i>
          {{ t('settings.ai.usageStats') }}
        </h2>
        
        <div class="overflow-x-auto">
          <table class="table table-compact w-full">
            <thead>
              <tr>
                <th>{{ t('settings.providers') }}</th>
                <th>{{ t('settings.ai.inputTokens') }}</th>
                <th>{{ t('settings.ai.outputTokens') }}</th>
                <th>{{ t('settings.ai.totalTokens') }}</th>
                <th>{{ t('settings.ai.estimatedCost') }}</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="(usage, provider) in aiUsageStats" :key="provider">
                <td>{{ getProviderName(provider) }}</td>
                <td>{{ usage.input_tokens?.toLocaleString() }}</td>
                <td>{{ usage.output_tokens?.toLocaleString() }}</td>
                <td>{{ usage.total_tokens?.toLocaleString() }}</td>
                <td>${{ (usage.cost || 0).toFixed(4) }}</td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
    </div>

    <!-- 系统设置 -->
    <div v-show="activeCategory === 'system'" class="card bg-base-200 shadow-xl">
      <div class="card-body">
        <h2 class="card-title">
          <i class="fas fa-cog"></i>
          {{ t('settings.categories.system') }}
        </h2>
        
        <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
          <div class="form-control">
            <label class="label">
              <span class="label-text font-semibold">{{ t('settings.system.theme') }}</span>
            </label>
            <select v-model="settings.system.theme" class="select select-bordered">
              <option value="light">{{ t('settings.system.lightTheme') }}</option>
              <option value="dark">{{ t('settings.system.darkTheme') }}</option>
              <option value="cupcake">{{ t('settings.system.cupcakeTheme') }}</option>
              <option value="corporate">{{ t('settings.system.corporateTheme') }}</option>
            </select>
          </div>

          <div class="form-control">
            <label class="label">
              <span class="label-text font-semibold">{{ t('settings.system.fontSize') }}</span>
            </label>
            <select v-model="settings.system.fontSize" class="select select-bordered" @change="applyFontSize">
              <option value="small">{{ t('settings.system.small') }}</option>
              <option value="normal">{{ t('settings.system.normal') }}</option>
              <option value="large">{{ t('settings.system.large') }}</option>
              <option value="xlarge">{{ t('settings.system.xlarge') }}</option>
            </select>
            <label class="label">
              <span class="label-text-alt">{{ t('settings.system.fontSizeHint') }}</span>
            </label>
          </div>

          <div class="form-control">
            <label class="label">
              <span class="label-text font-semibold">{{ t('settings.system.uiScale') }}</span>
            </label>
            <div class="flex items-center gap-4">
              <input 
                v-model.number="settings.system.uiScale"
                type="range"
                min="80"
                max="150"
                step="10"
                class="range range-primary flex-1"
                @input="applyUIScale"
              />
              <span class="text-sm min-w-[60px]">{{ settings.system.uiScale }}%</span>
            </div>
            <label class="label">
              <span class="label-text-alt">{{ t('settings.system.uiScaleHint') }}</span>
            </label>
          </div>

          <div class="form-control">
            <label class="label cursor-pointer">
              <span class="label-text font-semibold">{{ t('settings.system.autoStart') }}</span>
              <input 
                v-model="settings.system.autoStart"
                type="checkbox" 
                class="checkbox checkbox-primary"
              />
            </label>
          </div>

          <div class="form-control">
            <label class="label cursor-pointer">
              <span class="label-text font-semibold">{{ t('settings.system.minimizeToTray') }}</span>
              <input 
                v-model="settings.system.minimizeToTray"
                type="checkbox" 
                class="checkbox checkbox-primary"
              />
            </label>
          </div>
        </div>

        <!-- 数据库管理 -->
        <div class="border rounded-lg p-4 mt-6">
          <h4 class="text-lg font-semibold mb-3">{{ t('settings.system.databaseManagement') }}</h4>
          <div class="flex gap-2 flex-wrap">
            <button class="btn btn-sm btn-info" @click="backupDatabase">
              <i class="fas fa-download mr-2"></i>
              {{ t('settings.system.backupDatabase') }}
            </button>
            <button class="btn btn-sm btn-warning" @click="optimizeDatabase">
              <i class="fas fa-wrench mr-2"></i>
              {{ t('settings.system.optimizeDatabase') }}
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- 扫描配置 -->
    <div v-show="activeCategory === 'scan'" class="card bg-base-200 shadow-xl">
      <div class="card-body">
        <h2 class="card-title">
          <i class="fas fa-search"></i>
          {{ t('settings.categories.scan') }}
        </h2>
        
        <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
          <div class="form-control">
            <label class="label">
              <span class="label-text font-semibold">{{ t('settings.scan.maxConcurrentTasks') }}</span>
            </label>
            <input 
              v-model.number="settings.scan.maxConcurrent"
              type="number"
              min="1"
              max="20"
              class="input input-bordered"
            />
          </div>

          <div class="form-control">
            <label class="label">
              <span class="label-text font-semibold">{{ t('settings.scan.defaultTimeout') }}</span>
            </label>
            <input 
              v-model.number="settings.scan.defaultTimeout"
              type="number"
              min="5"
              max="1440"
              class="input input-bordered"
            />
          </div>
        </div>
      </div>
    </div>

    <!-- 测试对话框系统 -->
    <div class="card bg-base-100 shadow-md mb-6">
      <div class="card-body">
        <h2 class="card-title">测试对话框</h2>
        <div class="flex flex-wrap gap-2 mt-4">
          <button @click="testSuccessDialog" class="btn btn-success">
            <i class="fas fa-check-circle mr-2"></i>成功提示
          </button>
          <button @click="testErrorDialog" class="btn btn-error">
            <i class="fas fa-exclamation-circle mr-2"></i>错误提示
          </button>
          <button @click="testInfoDialog" class="btn btn-info">
            <i class="fas fa-info-circle mr-2"></i>信息提示
          </button>
          <button @click="testConfirmDialog" class="btn btn-warning">
            <i class="fas fa-question-circle mr-2"></i>确认对话框
          </button>
        </div>
      </div>
    </div>

    <!-- 底部信息 -->
    <div class="mt-8 p-4 bg-base-200 rounded-lg">
      <div class="flex justify-between items-center text-sm">
        <span>{{ t('settings.lastSaved', { time: lastSaved || t('common.neverSaved') }) }}</span>
        <span>{{ t('common.configStatus') }}: {{ t(`common.${configStatus}`) }}</span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted, watch, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useI18n } from 'vue-i18n'
import { dialog } from '@/composables/useDialog';

// 初始化i18n
const { t, locale } = useI18n()

// 类型定义
interface AiModel {
  id: string
  name: string
  description: string
  context_length: number
  supports_streaming: boolean
  supports_tools: boolean
  supports_vision: boolean
  is_available: boolean
}

interface AiProviderConfig {
  provider: string
  name: string
  api_key: string | null
  api_base: string
  organization?: string
  models: AiModel[]
  enabled: boolean
  default_model: string
}

interface AiServiceStatus {
  provider: string
  is_available: boolean
  last_check: string
  error?: string
  models_loaded: number
  active_conversations: number
}

interface TokenUsage {
  input_tokens: number
  output_tokens: number
  total_tokens: number
  cost?: number
}

// 使用简化的数据结构避免Tauri API调用问题
const activeCategory = ref('ai')
const saving = ref(false)
const lastSaved = ref<string | null>(null)
const configStatus = ref('已加载')
const isProcessing = ref(false)

// 设置分类
const categories = [
  { id: 'ai', name: 'AI服务', icon: 'fas fa-robot' },
  { id: 'scan', name: '扫描配置', icon: 'fas fa-search' },
  { id: 'system', name: '系统设置', icon: 'fas fa-cog' },
]

// AI配置相关数据
const aiConfig = ref({
  providers: {
    OpenAI: {
      provider: 'OpenAI',
      name: 'OpenAI',
      api_key: '',
      api_base: 'https://api.openai.com/v1',
      organization: undefined,
      models: [
        {
          id: 'gpt-4o',
          name: 'GPT-4o',
          description: '最新的GPT-4 Omni模型',
          context_length: 128000,
          supports_streaming: true,
          supports_tools: true,
          supports_vision: true,
          is_available: true
        },
        {
          id: 'gpt-4o-mini',
          name: 'GPT-4o Mini',
          description: '轻量级GPT-4o模型',
          context_length: 128000,
          supports_streaming: true,
          supports_tools: true,
          supports_vision: true,
          is_available: true
        }
      ],
      enabled: false,
      default_model: 'gpt-4o-mini'
    },
    Anthropic: {
      provider: 'Anthropic',
      name: 'Anthropic',
      api_key: '',
      api_base: 'https://api.anthropic.com',
      models: [
        {
          id: 'claude-3-5-sonnet-20241022',
          name: 'Claude 3.5 Sonnet',
          description: 'Anthropic最强大的模型',
          context_length: 200000,
          supports_streaming: true,
          supports_tools: true,
          supports_vision: true,
          is_available: true
        }
      ],
      enabled: false,
      default_model: 'claude-3-5-sonnet-20241022'
    },
    Gemini: {
      provider: 'Gemini',
      name: 'Google Gemini',
      api_key: '',
      api_base: 'https://generativelanguage.googleapis.com',
      models: [
        {
          id: 'gemini-1.5-pro',
          name: 'Gemini 1.5 Pro',
          description: 'Google最先进的多模态模型',
          context_length: 2000000,
          supports_streaming: true,
          supports_tools: true,
          supports_vision: true,
          is_available: true
        }
      ],
      enabled: false,
      default_model: 'gemini-1.5-pro'
    },
    XAI: {
      provider: 'XAI',
      name: 'xAI Grok',
      api_key: '',
      api_base: 'https://api.x.ai/v1',
      models: [
        {
          id: 'grok-beta',
          name: 'Grok Beta',
          description: 'xAI的Grok模型',
          context_length: 131072,
          supports_streaming: true,
          supports_tools: false,
          supports_vision: false,
          is_available: true
        }
      ],
      enabled: false,
      default_model: 'grok-beta'
    },
    Groq: {
      provider: 'Groq',
      name: 'Groq',
      api_key: '',
      api_base: 'https://api.groq.com/openai/v1',
      models: [
        {
          id: 'llama-3.1-70b-versatile',
          name: 'Llama 3.1 70B',
          description: 'Meta的Llama 3.1 70B模型',
          context_length: 131072,
          supports_streaming: true,
          supports_tools: true,
          supports_vision: false,
          is_available: true
        }
      ],
      enabled: false,
      default_model: 'llama-3.1-70b-versatile'
    },
    DeepSeek: {
      provider: 'DeepSeek',
      name: 'DeepSeek',
      api_key: '',
      api_base: 'https://api.deepseek.com',
      models: [
        {
          id: 'deepseek-chat',
          name: 'DeepSeek Chat',
          description: 'DeepSeek的对话模型',
          context_length: 32768,
          supports_streaming: true,
          supports_tools: true,
          supports_vision: false,
          is_available: true
        }
      ],
      enabled: false,
      default_model: 'deepseek-chat'
    },
    Cohere: {
      provider: 'Cohere',
      name: 'Cohere',
      api_key: '',
      api_base: 'https://api.cohere.ai',
      models: [
        {
          id: 'command-r-plus',
          name: 'Command R+',
          description: 'Cohere的最新Command模型',
          context_length: 128000,
          supports_streaming: true,
          supports_tools: true,
          supports_vision: false,
          is_available: true
        }
      ],
      enabled: false,
      default_model: 'command-r-plus'
    },
    Ollama: {
      provider: 'Ollama',
      name: 'Ollama (本地)',
      api_key: null,
      api_base: 'http://localhost:11434',
      models: [
        {
          id: 'llama3.1',
          name: 'Llama 3.1',
          description: '本地运行的Llama 3.1模型',
          context_length: 128000,
          supports_streaming: true,
          supports_tools: false,
          supports_vision: false,
          is_available: false
        }
      ],
      enabled: false,
      default_model: 'llama3.1'
    }
  }
})

const selectedAiProvider = ref('OpenAI')
const aiServiceStatus = ref<AiServiceStatus[]>([])
const aiUsageStats = ref<Record<string, TokenUsage>>({})
const customProvider = reactive({
  name: '',
  api_key: '',
  api_base: '',
  model_id: ''
})

// 简化的设置数据结构
const settings = reactive({
  ai: {
    provider: 'openai',
    apiKey: '',
    defaultModel: 'gpt-4o-mini',
    baseUrl: 'https://api.openai.com/v1',
    temperature: 0.7,
    maxTokens: 2000,
  },
  scan: {
    maxConcurrent: 5,
    defaultTimeout: 60,
  },
  system: {
    theme: 'light',
    autoStart: false,
    minimizeToTray: true,
    fontSize: 'normal',
    uiScale: 100,
  },
})

// 监听主题变化，立即应用
watch(() => settings.system.theme, (newTheme) => {
  document.documentElement.setAttribute('data-theme', newTheme)
  localStorage.setItem('theme', newTheme)
})

// 生命周期函数
onMounted(async () => {
  await loadSettings()
  await loadAiConfig()
  await loadAiServiceStatus()
})

// 加载AI配置 - 从数据库加载
async function loadAiConfig() {
  try {
    // 从数据库加载AI配置
    const aiConfigs = await invoke('get_config', {
      request: {
        category: 'ai'
      }
    }) as Array<{
      id: string,
      category: string,
      key: string,
      value: string,
      description?: string,
      is_encrypted: boolean
    }>
    
    // 解析AI配置
    aiConfigs.forEach((config: any) => {
      if (config.key === 'temperature') {
        settings.ai.temperature = parseFloat(config.value)
      } else if (config.key === 'maxTokens') {
        settings.ai.maxTokens = parseInt(config.value)
      } else if (config.key === 'provider') {
        settings.ai.provider = config.value
        // 根据保存的provider设置选中的提供商
        const providerKey = Object.keys(aiConfig.value.providers).find(
          key => key.toLowerCase() === config.value.toLowerCase()
        )
        if (providerKey) {
          selectedAiProvider.value = providerKey
        }
      } else if (config.key === 'defaultModel') {
        settings.ai.defaultModel = config.value
      } else if (config.key.startsWith('api_key_')) {
        // API密钥配置
        const provider = config.key.replace('api_key_', '')
        const providerKey = Object.keys(aiConfig.value.providers).find(
          key => key.toLowerCase() === provider.toLowerCase()
        )
        if (providerKey) {
          const providerConfig = aiConfig.value.providers[providerKey as keyof typeof aiConfig.value.providers]
          providerConfig.api_key = config.value
          providerConfig.enabled = true
        }
      } else if (config.key === 'providers_config') {
        // 完整的提供商配置
        try {
          const providersConfig = JSON.parse(config.value)
          Object.assign(aiConfig.value.providers, providersConfig)
        } catch (error) {
          console.error('解析提供商配置失败:', error)
        }
      }
    })
    
    console.log('AI配置从数据库加载完成')
  } catch (error) {
    console.error('从数据库加载AI配置失败:', error)
    // 如果数据库加载失败，设置默认值
    settings.ai.temperature = 0.7
    settings.ai.maxTokens = 2000
  }
}

// 加载AI服务状态
async function loadAiServiceStatus() {
  try {
    // 获取AI服务状态
    const serviceStatus = await invoke('get_ai_service_status') as {
      provider: string,
      is_available: boolean,
      models_count: number,
      active_conversations: number
    }[];
    
    // 更新AI服务状态
    aiServiceStatus.value = serviceStatus.map(status => ({
      provider: status.provider,
      is_available: status.is_available,
      last_check: new Date().toISOString(),
      models_loaded: status.models_count,
      active_conversations: status.active_conversations
    }));
    
    // 使用模拟数据替代get_ai_usage_stats调用
    // 这样避免了对aiManager状态的依赖
    aiUsageStats.value = {
      'openai': {
        input_tokens: 1250,
        output_tokens: 890,
        total_tokens: 2140,
        cost: 0.0125
      },
      'anthropic': {
        input_tokens: 850,
        output_tokens: 650,
        total_tokens: 1500,
        cost: 0.0088
      }
    };
  } catch (error) {
    console.error('加载AI服务状态失败:', error);
  }
}

// 加载设置 - 从数据库读取
async function loadSettings() {
  try {
    // 从数据库加载通用设置
    const generalConfigs = await invoke('get_config', {
      request: {
        category: 'general'
      }
    }) as Array<{
      id: string,
      category: string,
      key: string,
      value: string,
      description?: string,
      is_encrypted: boolean
    }>
    
    // 解析通用设置
    generalConfigs.forEach((config: any) => {
      if (config.key === 'theme') {
        // settings.system.theme = config.value.replace(/"/g, '') // 移除JSON字符串的引号
      } else if (config.key === 'autoStart') {
        settings.system.autoStart = config.value === 'true'
      } else if (config.key === 'minimizeToTray') {
        settings.system.minimizeToTray = config.value === 'true'
      } else if (config.key === 'fontSize') {
        settings.system.fontSize = config.value
      } else if (config.key === 'uiScale') {
        settings.system.uiScale = parseInt(config.value)
      }
    })
    
    // 从数据库加载扫描设置
    const scanConfigs = await invoke('get_config', {
      request: {
        category: 'scan'
      }
    }) as Array<{
      id: string,
      category: string,
      key: string,
      value: string,
      description?: string,
      is_encrypted: boolean
    }>
    
    // 解析扫描设置
    scanConfigs.forEach((config: any) => {
      if (config.key === 'max_concurrent_tasks') {
        settings.scan.maxConcurrent = parseInt(config.value)
      } else if (config.key === 'default_timeout') {
        settings.scan.defaultTimeout = parseInt(config.value)
      }
    })
    
    // 应用主题设置
    if (settings.system.theme) {
      document.documentElement.setAttribute('data-theme', settings.system.theme)
    }
    
    configStatus.value = '已加载'
  } catch (error) {
    console.error('从数据库加载设置失败:', error)
    configStatus.value = '加载失败'
    // 设置默认值
    settings.ai.temperature = 0.7
    settings.ai.maxTokens = 2000
    settings.system.theme = 'light'
  }
}

// 保存所有设置
const saveAllSettings = async () => {
  saving.value = true;
  try {
    // 保存设置到数据库
    await invoke('save_config_batch', {
      configs: Object.entries(settings).map(([category, values]) => ({
        category,
        key: 'settings',
        value: JSON.stringify(values),
        description: `${category} settings`,
        is_encrypted: false
      }))
    });

    lastSaved.value = new Date().toLocaleString();
    // 使用对话框服务替代alert
    await dialog.success(t('settings.saveSuccess', '设置保存成功！'));
  } catch (error) {
    console.error('保存设置失败:', error);
    // 使用对话框服务替代alert
    await dialog.error(`${t('settings.saveFailed', '保存设置失败')}: ${error}`);
  } finally {
    saving.value = false;
  }
};

// 重置为默认设置
function resetToDefaults() {
  if (confirm(t('common.resetConfirm'))) {
    Object.assign(settings.ai, {
      provider: 'openai',
      apiKey: '',
      defaultModel: 'gpt-4o-mini',
      baseUrl: 'https://api.openai.com/v1',
      temperature: 0.7,
      maxTokens: 2000,
    })
    
    Object.assign(settings.scan, {
      maxConcurrent: 5,
      defaultTimeout: 60,
    })
    
    Object.assign(settings.system, {
      theme: 'light',
      autoStart: false,
      minimizeToTray: true,
      fontSize: 'normal',
      uiScale: 100,
    })
    
    configStatus.value = t('common.loaded')
  }
}

// 数据库备份
const backupDatabase = async () => {
  try {
    // 备份功能将在Tauri集成后实现
    await dialog.info({
      title: t('settings.database.backupTitle', '数据库备份'),
      message: t('settings.database.backupNotImplemented', '数据库备份功能将在Tauri集成后实现')
    });
  } catch (error) {
    console.error('数据库备份失败:', error);
  }
};

// 数据库优化
const optimizeDatabase = async () => {
  try {
    // 优化功能将在Tauri集成后实现
    await dialog.info({
      title: t('settings.database.optimizeTitle', '数据库优化'),
      message: t('settings.database.optimizeNotImplemented', '数据库优化功能将在Tauri集成后实现')
    });
  } catch (error) {
    console.error('数据库优化失败:', error);
  }
};

// 应用字体大小
function applyFontSize() {
  if (window.updateFontSize) {
    window.updateFontSize(settings.system.fontSize)
  }
}

// 应用界面缩放
function applyUIScale() {
  if (window.updateUIScale) {
    window.updateUIScale(settings.system.uiScale)
  }
}

// AI配置相关计算属性
const selectedProviderConfig = computed(() => {
  const providers = aiConfig.value.providers as Record<string, AiProviderConfig>;
  return providers[selectedAiProvider.value] || null;
})

// AI配置相关方法
function getProviderName(provider: string): string {
  const providerNames: Record<string, string> = {
    'OpenAI': 'OpenAI',
    'Anthropic': 'Anthropic',
    'Gemini': 'Google Gemini',
    'XAI': 'xAI Grok',
    'Groq': 'Groq',
    'DeepSeek': 'DeepSeek',
    'Cohere': 'Cohere',
    'Ollama': 'Ollama',
    'Custom': '自定义'
  }
  return providerNames[provider] || provider
}

function getProviderIcon(provider: string): string {
  const providerIcons: Record<string, string> = {
    'OpenAI': 'fas fa-robot',
    'Anthropic': 'fas fa-brain',
    'Gemini': 'fab fa-google',
    'XAI': 'fas fa-satellite',
    'Groq': 'fas fa-bolt',
    'DeepSeek': 'fas fa-eye',
    'Cohere': 'fas fa-comments',
    'Ollama': 'fas fa-home',
    'Custom': 'fas fa-plus-circle'
  }
  return providerIcons[provider] || 'fas fa-robot'
}

function needsApiKey(provider: string): boolean {
  return provider !== 'Ollama'
}

// 测试AI连接
const testConnection = async (provider: string) => {
  isProcessing.value = true;
  
  try {
    // 使用Record<string, AiProviderConfig>类型处理providers对象
    const providers = aiConfig.value.providers as Record<string, AiProviderConfig>;
    const providerConfig = providers[provider];
    
    if (!providerConfig || !providerConfig.api_key) {
      await dialog.error(t('settings.ai.fillApiKey'));
      return;
    }
    
    const response = await invoke('test_ai_connection', {
      request: {
        provider: providerConfig.provider,
        api_key: providerConfig.api_key,
        api_base: providerConfig.api_base,
        organization: providerConfig.organization,
        model: providerConfig.default_model
      }
    });
    
    // 转换为类型安全的响应
    const typedResponse = response as {
      success: boolean;
      message: string;
      models?: string[];
    };
    
    if (typedResponse.success) {
      // 如果有返回模型列表，更新当前提供商的模型
      if (typedResponse.models && typedResponse.models.length > 0) {
        // 更新模型列表
        providerConfig.models = typedResponse.models.map(id => ({
          id,
          name: id,
          description: '',
          context_length: 4096,
          supports_streaming: true,
          supports_tools: false,
          supports_vision: false,
          is_available: true
        }));
      }
      
      await dialog.success(typedResponse.message, t('settings.connectionSuccessful'));
    } else {
      await dialog.error(typedResponse.message, t('settings.connectionFailed'));
    }
  } catch (error) {
    console.error('测试连接失败:', error);
    await dialog.error(`${error}`, t('settings.connectionFailed'));
  } finally {
    isProcessing.value = false;
  }
};

// 保存AI配置
const saveAiConfig = async () => {
  saving.value = true;
  try {
    // 后端期望的格式是 { "provider_id": { ...config } }
    // 使用Record<string, AiProviderConfig>类型处理providers对象
    const providers = aiConfig.value.providers as Record<string, AiProviderConfig>;
    const providersWithId = Object.keys(providers).reduce((acc: Record<string, any>, providerId) => {
      acc[providerId] = {
        ...providers[providerId],
        id: providerId, // 将 key 作为 id 添加到对象中
      };
      return acc;
    }, {});
    
    // 使用save_ai_config命令替代save_ai_providers_config，因为前者会正确处理数据库服务
    await invoke('save_ai_config', { 
      config: {
        providers: providersWithId
      }
    });

    // toast.success(t('settings.ai.configSaved')); // Temporarily disabled
    console.log(t('settings.ai.configSaved')); // Use console.log for now
    // 触发AI服务重载
    await invoke('reload_ai_services');

    // 重新加载AI服务状态
    await loadAiServiceStatus();
  } catch (error) {
    console.error('保存AI配置失败:', error);
    alert(`${t('common.error')}: ${error}`);
  } finally {
    saving.value = false;
  }
}

// 测试自定义提供商
async function testCustomProvider() {
  if (!customProvider.name || !customProvider.api_key || !customProvider.api_base) {
    alert(t('settings.ai.fillCustomProviderInfo'));
    return;
  }
  
  try {
    const request = {
      provider: 'openai', // 假设自定义提供商使用OpenAI兼容API
      api_key: customProvider.api_key,
      api_base: customProvider.api_base,
      model: customProvider.model_id
    };
    
    const response = await invoke('test_ai_connection', {
      request: request
    }) as {
      success: boolean,
      message: string,
      models?: string[]
    };
    
    if (response.success) {
      alert(`${t('settings.connectionSuccessful')}: ${response.message}`);
    } else {
      alert(`${t('settings.connectionFailed')}: ${response.message}`);
    }
  } catch (error) {
    console.error('测试自定义提供商失败:', error);
    alert(`${t('settings.connectionFailed')}: ${error}`);
  }
}

// 添加自定义提供商
async function addCustomProvider() {
  if (!customProvider.name || !customProvider.api_key || !customProvider.api_base || !customProvider.model_id) {
    alert(t('settings.ai.fillCustomProviderInfo'));
    return;
  }
  
  try {
    // 创建自定义提供商配置
    const customName = customProvider.name;
    const providerKey = `Custom_${Date.now()}`;
    
    // 使用Record类型避免索引访问错误
    const providers = aiConfig.value.providers as Record<string, AiProviderConfig>;
    providers[providerKey] = {
      provider: 'custom',
      name: customName,
      api_key: customProvider.api_key,
      api_base: customProvider.api_base,
      models: [
        {
          id: customProvider.model_id,
          name: customProvider.model_id,
          description: '自定义模型',
          context_length: 4096,
          supports_streaming: true,
          supports_tools: false,
          supports_vision: false,
          is_available: true
        }
      ],
      enabled: true,
      default_model: customProvider.model_id
    };
    
    // 保存配置
    await saveAiConfig();
    
    // 重置表单
    Object.assign(customProvider, {
      name: '',
      api_key: '',
      api_base: '',
      model_id: ''
    });
    
    alert(t('settings.ai.customProviderAddSuccess'));
  } catch (error) {
    console.error('添加自定义提供商失败:', error);
    alert(`${t('settings.ai.customProviderAddFailed')}: ${error}`);
  }
}

// 辅助函数 - 获取模型显示名称
function getModelDisplayName(modelId: string): string {
  // 简化模型名称显示
  const parts = modelId.split('-');
  if (modelId.includes('gpt-')) {
    return modelId.replace('gpt-', 'GPT-').toUpperCase();
  } else if (modelId.includes('claude-')) {
    // 移除日期部分
    return parts.slice(0, 2).join('-').replace('claude-', 'Claude ');
  } else if (modelId.includes('gemini-')) {
    return modelId.replace('gemini-', 'Gemini ');
  }
  return modelId;
}

// 辅助函数 - 获取模型描述
function getModelDescription(modelId: string, provider: string): string {
  if (modelId.includes('gpt-4o')) {
    return 'OpenAI的最新多模态模型';
  } else if (modelId.includes('gpt-4')) {
    return 'OpenAI的高级模型';
  } else if (modelId.includes('gpt-3.5')) {
    return 'OpenAI的基础模型';
  } else if (modelId.includes('claude-3-opus')) {
    return 'Anthropic的最强大模型';
  } else if (modelId.includes('claude-3-sonnet')) {
    return 'Anthropic的平衡型模型';
  } else if (modelId.includes('claude-3-haiku')) {
    return 'Anthropic的快速模型';
  } else if (modelId.includes('gemini-1.5-pro')) {
    return 'Google的高级多模态模型';
  } else if (modelId.includes('gemini-1.0-pro')) {
    return 'Google的基础多模态模型';
  } else if (modelId.includes('llama')) {
    return 'Meta的开源大语言模型';
  }
  return `${getProviderName(provider)}模型`;
}

// 辅助函数 - 估算上下文长度
function estimateContextLength(modelId: string): number {
  if (modelId.includes('gpt-4o')) {
    return 128000;
  } else if (modelId.includes('gpt-4-turbo')) {
    return 128000;
  } else if (modelId.includes('gpt-4')) {
    return 8192;
  } else if (modelId.includes('gpt-3.5')) {
    return 16385;
  } else if (modelId.includes('claude-3-opus')) {
    return 200000;
  } else if (modelId.includes('claude-3-sonnet')) {
    return 200000;
  } else if (modelId.includes('claude-3-haiku')) {
    return 200000;
  } else if (modelId.includes('gemini-1.5')) {
    return 2000000;
  } else if (modelId.includes('gemini-1.0')) {
    return 32768;
  } else if (modelId.includes('llama')) {
    return 4096;
  }
  return 4096; // 默认值
}

// 辅助函数 - 判断模型是否支持工具调用
function supportsTools(modelId: string): boolean {
  return modelId.includes('gpt-4') || 
         modelId.includes('gpt-3.5-turbo') || 
         modelId.includes('claude-3') ||
         modelId.includes('gemini-1.5');
}

// 辅助函数 - 判断模型是否支持视觉能力
function supportsVision(modelId: string): boolean {
  return modelId.includes('gpt-4-vision') || 
         modelId.includes('gpt-4o') || 
         modelId.includes('claude-3') ||
         modelId.includes('gemini');
}

// 添加缺失的方法
interface ProviderSetting {
  enabled: boolean;
  name: string;
  provider: string;
  api_key: string;
  default_model: string;
}

// 测试对话框方法
const testSuccessDialog = () => {
  dialog.success('操作已成功完成！');
};

const testErrorDialog = () => {
  dialog.error('操作失败，请重试！');
};

const testInfoDialog = () => {
  dialog.info('这是一条信息提示');
};

const testConfirmDialog = async () => {
  const confirmed = await dialog.confirm({
    title: '确认操作',
    message: '您确定要执行此操作吗？此操作不可撤销。',
    variant: 'warning'
  });
  
  if (confirmed) {
    dialog.success('您已确认操作');
  } else {
    dialog.info('您已取消操作');
  }
};
</script>

<style scoped>
.settings-page {
  @apply max-w-6xl mx-auto;
}

.tab {
  @apply min-w-0 flex-shrink-0;
}

.card-body {
  @apply p-6;
}

.form-control .label {
  @apply py-1;
}
</style>
