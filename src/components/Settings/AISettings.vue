<template>
  <div class="ai-settings">
    <!-- AI提供商状态总览 -->
    <div class="space-y-4 mb-6">
      <div v-for="status in aiServiceStatus" :key="status.provider" 
           class="card bg-base-100 shadow-sm border">
        <div class="card-body p-4">
          <div class="flex items-center gap-4">
            <div class="text-2xl">
              <i :class="getProviderIcon(status.provider)"></i>
            </div>
            <div class="flex-1">
              <h3 class="font-semibold text-lg">{{ getProviderName(status.provider) }}</h3>
              <div class="flex items-center gap-2 mt-1">
                <div class="badge" :class="status.is_available ? 'badge-success' : 'badge-error'">
                  {{ status.is_available ? t('settings.ai.connected') : t('settings.ai.disconnected') }}
                </div>
                <span class="text-sm text-base-content/70">{{ status.models_loaded }} {{ t('settings.ai.modelsCount') }}</span>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- AI提供商配置选项卡 - 垂直布局 -->
    <div class="flex flex-col lg:flex-row gap-6 mb-6">
      <!-- 左侧：提供商选择 -->
      <div class="w-full lg:w-64 flex-shrink-0">
        <h3 class="text-lg font-semibold mb-3">AI 提供商</h3>
        <div class="menu bg-base-200 rounded-box p-2 space-y-1">
          <li v-for="provider in Object.keys(aiConfig.providers)" :key="provider">
            <a class="flex items-center gap-3 p-3 rounded-lg transition-all duration-200"
               :class="{ 'bg-primary text-primary-content': selectedAiProvider === provider }"
               @click="selectedAiProvider = provider">
              <div class="text-xl">
                <i :class="getProviderIcon(provider)"></i>
              </div>
              <span class="font-medium">{{ getProviderName(provider) }}</span>
            </a>
          </li>
        </div>
      </div>
      
      <!-- 右侧：配置内容 -->
      <div class="flex-1">

        <!-- 当前选中的AI提供商配置 -->
        <div v-if="selectedProviderConfig" class="grid grid-cols-1 xl:grid-cols-2 gap-6">
      <!-- 左侧：基本配置 -->
      <div class="space-y-4">
        <h3 class="text-lg font-semibold border-b pb-2">{{ t('settings.ai.basicConfig') }}</h3>
        
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
            <button class="btn btn-outline" @click="refreshModels(selectedAiProvider)">
              <i class="fas fa-sync-alt"></i>
              {{ t('settings.ai.refreshModels') }}
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
            <option value="">{{ t('settings.ai.selectModel') }}</option>
            <option v-for="model in selectedProviderConfig.models" 
                    :key="model.id" :value="model.id">
              {{ model.name }}{{ model.description ? ' - ' + model.description : '' }}
            </option>
          </select>
        </div>
      </div>

      <!-- 右侧：高级配置 -->
      <div class="space-y-4">
        <h3 class="text-lg font-semibold border-b pb-2">{{ t('settings.ai.advancedConfig') }}</h3>
        
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
      </div>
        </div>
      </div>
    </div>

    <!-- 可用模型列表 - 重构为卡片布局 -->
    <div class="mt-6">
      <h3 class="text-lg font-semibold border-b pb-2 mb-4">
        {{ t('settings.ai.availableModels') }} ({{ selectedProviderConfig?.models?.length || 0 }})
      </h3>
      
      <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
        <div v-for="model in selectedProviderConfig?.models" :key="model.id" 
             class="card bg-base-100 shadow-sm border">
          <div class="card-body p-4">
            <div class="flex justify-between items-start mb-2">
              <h4 class="card-title text-sm">{{ model.name }}</h4>
              <div :class="model.is_available ? 'badge badge-success badge-sm' : 'badge badge-error badge-sm'">
                {{ model.is_available ? t('settings.ai.available') : t('settings.ai.unavailable') }}
              </div>
            </div>
            
            <p class="text-xs text-base-content/70 mb-3">{{ model.description }}</p>
            
            <div class="space-y-2">
              <div class="flex justify-between text-xs">
                <span class="text-base-content/60">{{ t('settings.ai.contextLength') }}:</span>
                <span>{{ model.context_length?.toLocaleString() || 'N/A' }}</span>
              </div>
              
              <div class="flex flex-wrap gap-1">
                <div v-if="model.supports_streaming" class="badge badge-primary badge-xs">
                  {{ t('settings.ai.streaming') }}
                </div>
                <div v-if="model.supports_tools" class="badge badge-secondary badge-xs">
                  {{ t('settings.ai.tools') }}
                </div>
                <div v-if="model.supports_vision" class="badge badge-accent badge-xs">
                  {{ t('settings.ai.vision') }}
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- 自定义AI提供商 -->
    <div class="card bg-base-100 shadow-sm mt-6">
      <div class="card-body">
        <h3 class="card-title">
          <i class="fas fa-plus-circle"></i>
          {{ t('settings.ai.customProvider') }}
        </h3>
        
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
    <div class="card bg-base-100 shadow-sm mt-6">
      <div class="card-body">
        <h3 class="card-title">
          <i class="fas fa-chart-bar"></i>
          {{ t('settings.ai.usageStats') }}
        </h3>
        
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
                <td>{{ getProviderName(String(provider)) }}</td>
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

    <!-- 保存按钮 -->
    <div class="flex justify-end mt-6">
      <button class="btn btn-primary" @click="saveAiConfig">
        <i class="fas fa-save"></i>
        {{ t('settings.ai.saveConfig') }}
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'

const { t } = useI18n()

// Props
interface Props {
  aiServiceStatus: any[]
  aiConfig: any
  selectedAiProvider: string
  settings: any
  customProvider: any
  aiUsageStats: any
  saving: boolean
}

const props = defineProps<Props>()

// Emits
interface Emits {
  'update:selectedAiProvider': [value: string]
  'update:settings': [value: any]
  'update:customProvider': [value: any]
  'testConnection': [provider: string]
  'testCustomProvider': []
  'addCustomProvider': []
  'saveAiConfig': []
  'refreshModels': [provider: string]
}

const emit = defineEmits<Emits>()

// Computed
const selectedAiProvider = computed({
  get: () => props.selectedAiProvider,
  set: (value: string) => {
    emit('update:selectedAiProvider', value)
  }
})

const settings = computed({
  get: () => props.settings,
  set: (value) => emit('update:settings', value)
})

const customProvider = computed({
  get: () => props.customProvider,
  set: (value) => emit('update:customProvider', value)
})

const selectedProviderConfig = computed(() => {
  return props.aiConfig.providers[props.selectedAiProvider]
})

// Methods
const getProviderIcon = (provider: string) => {
  const icons: Record<string, string> = {
    'OpenAI': 'fas fa-brain',
    'Anthropic': 'fas fa-robot',
      'Google': 'fab fa-google',
      'Gemini': 'fab fa-google',
    'Ollama': 'fas fa-server',
    'DeepSeek': 'fas fa-eye'
  }
  return icons[provider] || 'fas fa-cog'
}

const getProviderName = (provider: string) => {
  const names: Record<string, string> = {
    'OpenAI': 'OpenAI',
    'Anthropic': 'Anthropic',
      'Google': 'Google',
      'Gemini': 'Gemini',
    'Ollama': 'Ollama',
    'DeepSeek': 'DeepSeek'
  }
  return names[provider] || provider
}

const needsApiKey = (provider: string) => {
  return !['Ollama'].includes(provider)
}

const testConnection = (provider: string) => {
  emit('testConnection', provider)
}

const refreshModels = (provider: string) => {
  emit('refreshModels', provider)
}

const testCustomProvider = () => {
  emit('testCustomProvider')
}

const addCustomProvider = () => {
  emit('addCustomProvider')
}

const saveAiConfig = () => {
  emit('saveAiConfig')
}
</script>

<style scoped>
.ai-settings {
  @apply space-y-6;
}

.card {
  @apply transition-all duration-200 hover:shadow-md;
}

.stat {
  @apply transition-all duration-200 hover:scale-105;
}

.tab {
  @apply transition-all duration-200;
}

.tab:hover {
  @apply bg-base-300;
}

.tab-active {
  @apply bg-primary text-primary-content;
}
</style>