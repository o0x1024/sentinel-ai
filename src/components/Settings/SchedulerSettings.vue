<template>
  <div class="scheduler-settings">
    <!-- 调度策略概览 -->
    <div class="grid grid-cols-1 md:grid-cols-3 gap-4 mb-6">
      <div class="stat bg-base-100 rounded-lg">
        <div class="stat-figure text-2xl">
          <i class="fas fa-cogs"></i>
        </div>
        <div class="stat-title">{{ t('settings.scheduler.currentStrategy') }}</div>
        <div class="stat-value text-sm">{{ getCurrentStrategyName() }}</div>
        <div class="stat-desc">{{ t('settings.scheduler.status') }}: {{ schedulerConfig.enabled ? t('settings.scheduler.enabled') : t('settings.scheduler.disabled') }}</div>
      </div>
      
      <div class="stat bg-base-100 rounded-lg">
        <div class="stat-figure text-2xl">
          <i class="fas fa-brain"></i>
        </div>
        <div class="stat-title">{{ t('settings.scheduler.stageModels') }}</div>
        <div class="stat-value text-sm">{{ getConfiguredModelsCount() }}</div>
        <div class="stat-desc">{{ t('settings.scheduler.modelsConfigured') }}</div>
      </div>
      
      <div class="stat bg-base-100 rounded-lg">
        <div class="stat-figure text-2xl">
          <i class="fas fa-chart-line"></i>
        </div>
        <div class="stat-title">{{ t('settings.scheduler.performance') }}</div>
        <div class="stat-value text-sm">{{ getPerformanceLevel() }}</div>
        <div class="stat-desc">{{ t('settings.scheduler.estimatedCost') }}: ${{ getEstimatedCost() }}</div>
      </div>
    </div>

    <!-- 启用/禁用调度器 -->
    <div class="card bg-base-100 shadow-sm mb-6">
      <div class="card-body">
        <div class="form-control">
          <label class="label cursor-pointer">
            <span class="label-text text-lg font-semibold">
              <i class="fas fa-power-off mr-2"></i>
              {{ t('settings.scheduler.enableScheduler') }}
            </span>
            <input type="checkbox" class="toggle toggle-primary toggle-lg" 
                   v-model="schedulerConfig.enabled"
                   @change="saveSchedulerConfig">
          </label>
          <label class="label">
            <span class="label-text-alt">{{ t('settings.scheduler.enableSchedulerDesc') }}</span>
          </label>
        </div>
      </div>
    </div>

    <!-- 快速预设配置 -->
    <div class="card bg-base-100 shadow-sm mb-6">
      <div class="card-body">
        <h3 class="card-title mb-4">
          <i class="fas fa-magic"></i>
          {{ t('settings.scheduler.quickPresets') }}
        </h3>
        
        <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
          <div class="card bg-gradient-to-br from-red-50 to-red-100 border border-red-200 cursor-pointer hover:shadow-md transition-all"
               @click="applyHighPerformancePreset">
            <div class="card-body p-4 text-center">
              <i class="fas fa-rocket text-2xl text-red-600 mb-2"></i>
              <h4 class="font-semibold text-red-800">{{ t('settings.scheduler.presets.highPerformance.title') }}</h4>
              <p class="text-sm text-red-600 mt-2">{{ t('settings.scheduler.presets.highPerformance.description') }}</p>
              <div class="badge badge-error badge-sm mt-2">{{ t('settings.scheduler.highCost') }}</div>
            </div>
          </div>
          
          <div class="card bg-gradient-to-br from-blue-50 to-blue-100 border border-blue-200 cursor-pointer hover:shadow-md transition-all"
               @click="applyBalancedPreset">
            <div class="card-body p-4 text-center">
              <i class="fas fa-balance-scale text-2xl text-blue-600 mb-2"></i>
              <h4 class="font-semibold text-blue-800">{{ t('settings.scheduler.presets.balanced.title') }}</h4>
              <p class="text-sm text-blue-600 mt-2">{{ t('settings.scheduler.presets.balanced.description') }}</p>
              <div class="badge badge-info badge-sm mt-2">{{ t('settings.scheduler.mediumCost') }}</div>
            </div>
          </div>
          
          <div class="card bg-gradient-to-br from-green-50 to-green-100 border border-green-200 cursor-pointer hover:shadow-md transition-all"
               @click="applyEconomicPreset">
            <div class="card-body p-4 text-center">
              <i class="fas fa-piggy-bank text-2xl text-green-600 mb-2"></i>
              <h4 class="font-semibold text-green-800">{{ t('settings.scheduler.presets.economic.title') }}</h4>
              <p class="text-sm text-green-600 mt-2">{{ t('settings.scheduler.presets.economic.description') }}</p>
              <div class="badge badge-success badge-sm mt-2">{{ t('settings.scheduler.lowCost') }}</div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- 阶段模型配置 -->
    <div class="card bg-base-100 shadow-sm mb-6">
      <div class="card-body">
        <h3 class="card-title mb-4">
          <i class="fas fa-layer-group"></i>
          {{ t('settings.scheduler.stageModels') }}
        </h3>
        
        <div class="space-y-6">
          <!-- 意图分析模型 -->
          <div class="border rounded-lg p-4">
            <div class="flex items-center justify-between mb-3">
              <div class="flex items-center gap-3">
                <i class="fas fa-search text-primary"></i>
                <div>
                  <h4 class="font-semibold">{{ t('settings.scheduler.intentAnalysisModel') }}</h4>
                  <p class="text-sm text-base-content/70">{{ t('settings.scheduler.intentAnalysisModelDesc') }}</p>
                </div>
              </div>
              <div class="badge badge-primary">{{ t('settings.scheduler.required') }}</div>
            </div>
            
            <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
              <div class="form-control">
                <label class="label">
                  <span class="label-text-alt">{{ t('settings.scheduler.provider') }}</span>
                </label>
                <select class="select select-bordered" 
                        v-model="schedulerConfig.models.intent_analysis_provider" 
                        @change="onProviderChange('intent_analysis', $event.target.value)">
                  <option value="">{{ t('settings.scheduler.selectProvider') }}</option>
                  <option v-for="provider in availableProviders" :key="provider" :value="provider">
                    {{ provider }}
                  </option>
                </select>
              </div>
              <div class="form-control">
                <label class="label">
                  <span class="label-text-alt">{{ t('settings.scheduler.model') }}</span>
                </label>
                <select class="select select-bordered" 
                        v-model="schedulerConfig.models.intent_analysis" 
                        @change="saveSchedulerConfig"
                        :disabled="!schedulerConfig.models.intent_analysis_provider">
                  <option value="">{{ t('settings.scheduler.selectModel') }}</option>
                  <option v-for="model in getProviderModels(schedulerConfig.models.intent_analysis_provider)" 
                          :key="model.id" :value="model.id">
                    {{ model.name }}
                  </option>
                </select>
              </div>
            </div>
          </div>
          
          <!-- 规划器模型 -->
          <div class="border rounded-lg p-4">
            <div class="flex items-center justify-between mb-3">
              <div class="flex items-center gap-3">
                <i class="fas fa-route text-secondary"></i>
                <div>
                  <h4 class="font-semibold">{{ t('settings.scheduler.plannerModel') }}</h4>
                  <p class="text-sm text-base-content/70">{{ t('settings.scheduler.plannerModelDesc') }}</p>
                </div>
              </div>
              <div class="badge badge-secondary">{{ t('settings.scheduler.required') }}</div>
            </div>
            
            <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
              <div class="form-control">
                <label class="label">
                  <span class="label-text-alt">{{ t('settings.scheduler.provider') }}</span>
                </label>
                <select class="select select-bordered" 
                        v-model="schedulerConfig.models.planner_provider" 
                        @change="onProviderChange('planner', $event.target.value)">
                  <option value="">{{ t('settings.scheduler.selectProvider') }}</option>
                  <option v-for="provider in availableProviders" :key="provider" :value="provider">
                    {{ provider }}
                  </option>
                </select>
              </div>
              <div class="form-control">
                <label class="label">
                  <span class="label-text-alt">{{ t('settings.scheduler.model') }}</span>
                </label>
                <select class="select select-bordered" 
                        v-model="schedulerConfig.models.planner" 
                        @change="saveSchedulerConfig"
                        :disabled="!schedulerConfig.models.planner_provider">
                  <option value="">{{ t('settings.scheduler.selectModel') }}</option>
                  <option v-for="model in getProviderModels(schedulerConfig.models.planner_provider)" 
                          :key="model.id" :value="model.id">
                    {{ model.name }}
                  </option>
                </select>
              </div>
            </div>
          </div>
          
          <!-- 执行器模型 -->
          <div class="border rounded-lg p-4">
            <div class="flex items-center justify-between mb-3">
              <div class="flex items-center gap-3">
                <i class="fas fa-play text-accent"></i>
                <div>
                  <h4 class="font-semibold">{{ t('settings.scheduler.executorModel') }}</h4>
                  <p class="text-sm text-base-content/70">{{ t('settings.scheduler.executorModelDesc') }}</p>
                </div>
              </div>
              <div class="badge badge-accent">{{ t('settings.scheduler.required') }}</div>
            </div>
            
            <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
              <div class="form-control">
                <label class="label">
                  <span class="label-text-alt">{{ t('settings.scheduler.provider') }}</span>
                </label>
                <select class="select select-bordered" 
                        v-model="schedulerConfig.models.executor_provider" 
                        @change="onProviderChange('executor', $event.target.value)">
                  <option value="">{{ t('settings.scheduler.selectProvider') }}</option>
                  <option v-for="provider in availableProviders" :key="provider" :value="provider">
                    {{ provider }}
                  </option>
                </select>
              </div>
              <div class="form-control">
                <label class="label">
                  <span class="label-text-alt">{{ t('settings.scheduler.model') }}</span>
                </label>
                <select class="select select-bordered" 
                        v-model="schedulerConfig.models.executor" 
                        @change="saveSchedulerConfig"
                        :disabled="!schedulerConfig.models.executor_provider">
                  <option value="">{{ t('settings.scheduler.selectModel') }}</option>
                  <option v-for="model in getProviderModels(schedulerConfig.models.executor_provider)" 
                          :key="model.id" :value="model.id">
                    {{ model.name }}
                  </option>
                </select>
              </div>
            </div>
          </div>
          
          <!-- 重规划器模型 -->
          <div class="border rounded-lg p-4">
            <div class="flex items-center justify-between mb-3">
              <div class="flex items-center gap-3">
                <i class="fas fa-redo text-warning"></i>
                <div>
                  <h4 class="font-semibold">{{ t('settings.scheduler.replannerModel') }}</h4>
                  <p class="text-sm text-base-content/70">{{ t('settings.scheduler.replannerModelDesc') }}</p>
                </div>
              </div>
              <div class="badge badge-warning">{{ t('settings.scheduler.optional') }}</div>
            </div>
            
            <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
              <div class="form-control">
                <label class="label">
                  <span class="label-text-alt">{{ t('settings.scheduler.provider') }}</span>
                </label>
                <select class="select select-bordered" 
                        v-model="schedulerConfig.models.replanner_provider" 
                        @change="onProviderChange('replanner', $event.target.value)">
                  <option value="">{{ t('settings.scheduler.selectProvider') }}</option>
                  <option v-for="provider in availableProviders" :key="provider" :value="provider">
                    {{ provider }}
                  </option>
                </select>
              </div>
              <div class="form-control">
                <label class="label">
                  <span class="label-text-alt">{{ t('settings.scheduler.model') }}</span>
                </label>
                <select class="select select-bordered" 
                        v-model="schedulerConfig.models.replanner" 
                        @change="saveSchedulerConfig"
                        :disabled="!schedulerConfig.models.replanner_provider">
                  <option value="">{{ t('settings.scheduler.selectModel') }}</option>
                  <option v-for="model in getProviderModels(schedulerConfig.models.replanner_provider)" 
                          :key="model.id" :value="model.id">
                    {{ model.name }}
                  </option>
                </select>
              </div>
            </div>
          </div>
          
          <!-- 评估器模型 -->
          <div class="border rounded-lg p-4">
            <div class="flex items-center justify-between mb-3">
              <div class="flex items-center gap-3">
                <i class="fas fa-check-circle text-success"></i>
                <div>
                  <h4 class="font-semibold">{{ t('settings.scheduler.evaluatorModel') }}</h4>
                  <p class="text-sm text-base-content/70">{{ t('settings.scheduler.evaluatorModelDesc') }}</p>
                </div>
              </div>
              <div class="badge badge-success">{{ t('settings.scheduler.optional') }}</div>
            </div>
            
            <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
              <div class="form-control">
                <label class="label">
                  <span class="label-text-alt">{{ t('settings.scheduler.provider') }}</span>
                </label>
                <select class="select select-bordered" 
                        v-model="schedulerConfig.models.evaluator_provider" 
                        @change="onProviderChange('evaluator', $event.target.value)">
                  <option value="">{{ t('settings.scheduler.selectProvider') }}</option>
                  <option v-for="provider in availableProviders" :key="provider" :value="provider">
                    {{ provider }}
                  </option>
                </select>
              </div>
              <div class="form-control">
                <label class="label">
                  <span class="label-text-alt">{{ t('settings.scheduler.model') }}</span>
                </label>
                <select class="select select-bordered" 
                        v-model="schedulerConfig.models.evaluator" 
                        @change="saveSchedulerConfig"
                        :disabled="!schedulerConfig.models.evaluator_provider">
                  <option value="">{{ t('settings.scheduler.selectModel') }}</option>
                  <option v-for="model in getProviderModels(schedulerConfig.models.evaluator_provider)" 
                          :key="model.id" :value="model.id">
                    {{ model.name }}
                  </option>
                </select>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- 重新规划策略 -->
    <div class="card bg-base-100 shadow-sm mb-6">
      <div class="card-body">
        <h3 class="card-title mb-4">
          <i class="fas fa-redo"></i>
          {{ t('settings.scheduler.replanningStrategy') }}
        </h3>
        
        <div class="form-control mb-4">
          <label class="label">
            <span class="label-text">{{ t('settings.scheduler.defaultStrategy') }}</span>
          </label>
          <select class="select select-bordered" v-model="schedulerConfig.default_strategy" @change="saveSchedulerConfig">
            <option value="adaptive">{{ t('settings.scheduler.strategies.adaptive') }}</option>
            <option value="conservative">{{ t('settings.scheduler.strategies.conservative') }}</option>
            <option value="aggressive">{{ t('settings.scheduler.strategies.aggressive') }}</option>
            <option value="cost_optimized">{{ t('settings.scheduler.strategies.costOptimized') }}</option>
          </select>
          <label class="label">
            <span class="label-text-alt">{{ getStrategyDescription(schedulerConfig.default_strategy) }}</span>
          </label>
        </div>
        
        <!-- 策略参数配置 -->
        <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ t('settings.scheduler.maxRetries') }}</span>
            </label>
            <input type="number" class="input input-bordered" 
                   v-model.number="schedulerConfig.max_retries" 
                   min="1" max="10"
                   @blur="saveSchedulerConfig">
          </div>
          
          <div class="form-control">
            <label class="label">
              <span class="label-text">{{ t('settings.scheduler.timeoutSeconds') }}</span>
            </label>
            <input type="number" class="input input-bordered" 
                   v-model.number="schedulerConfig.timeout_seconds" 
                   min="30" max="300"
                   @blur="saveSchedulerConfig">
          </div>
        </div>
      </div>
    </div>

    <!-- 场景配置 -->
    <div class="card bg-base-100 shadow-sm mb-6">
      <div class="card-body">
        <h3 class="card-title mb-4">
          <i class="fas fa-sitemap"></i>
          {{ t('settings.scheduler.scenarios') }}
        </h3>
        
        <div class="space-y-4">
          <div v-for="(scenario, key) in schedulerConfig.scenarios" :key="key" 
               class="border rounded-lg p-4">
            <div class="flex items-center justify-between mb-3">
              <h4 class="font-semibold">{{ t(`settings.scheduler.scenarios.${key}.title`) }}</h4>
              <input type="checkbox" class="toggle toggle-primary" 
                     v-model="scenario.enabled"
                     @change="saveSchedulerConfig">
            </div>
            
            <p class="text-sm text-base-content/70 mb-3">
              {{ t(`settings.scheduler.scenarios.${key}.description`) }}
            </p>
            
            <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-5 gap-3">
              <div class="form-control">
                <label class="label">
                  <span class="label-text-alt">{{ t('settings.scheduler.intentAnalysisModel') }}</span>
                </label>
                <select class="select select-bordered select-sm" 
                        v-model="scenario.models.intent_analysis"
                        @change="saveSchedulerConfig">
                  <option value="">{{ t('settings.scheduler.useDefault') }}</option>
                  <option v-for="model in availableModels" :key="model.id" :value="model.id">
                    {{ model.name }}
                  </option>
                </select>
              </div>
              
              <div class="form-control">
                <label class="label">
                  <span class="label-text-alt">{{ t('settings.scheduler.plannerModel') }}</span>
                </label>
                <select class="select select-bordered select-sm" 
                        v-model="scenario.models.planner"
                        @change="saveSchedulerConfig">
                  <option value="">{{ t('settings.scheduler.useDefault') }}</option>
                  <option v-for="model in availableModels" :key="model.id" :value="model.id">
                    {{ model.name }}
                  </option>
                </select>
              </div>
              
              <div class="form-control">
                <label class="label">
                  <span class="label-text-alt">{{ t('settings.scheduler.replannerModel') }}</span>
                </label>
                <select class="select select-bordered select-sm" 
                        v-model="scenario.models.replanner"
                        @change="saveSchedulerConfig">
                  <option value="">{{ t('settings.scheduler.useDefault') }}</option>
                  <option v-for="model in availableModels" :key="model.id" :value="model.id">
                    {{ model.name }}
                  </option>
                </select>
              </div>
              
              <div class="form-control">
                <label class="label">
                  <span class="label-text-alt">{{ t('settings.scheduler.executorModel') }}</span>
                </label>
                <select class="select select-bordered select-sm" 
                        v-model="scenario.models.executor"
                        @change="saveSchedulerConfig">
                  <option value="">{{ t('settings.scheduler.useDefault') }}</option>
                  <option v-for="model in availableModels" :key="model.id" :value="model.id">
                    {{ model.name }}
                  </option>
                </select>
              </div>
              
              <div class="form-control">
                <label class="label">
                  <span class="label-text-alt">{{ t('settings.scheduler.evaluatorModel') }}</span>
                </label>
                <select class="select select-bordered select-sm" 
                        v-model="scenario.models.evaluator"
                        @change="saveSchedulerConfig">
                  <option value="">{{ t('settings.scheduler.useDefault') }}</option>
                  <option v-for="model in availableModels" :key="model.id" :value="model.id">
                    {{ model.name }}
                  </option>
                </select>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>


  </div>
</template>

<script setup lang="ts">
import { computed, watch, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'

const { t } = useI18n()

// Props
interface Props {
  schedulerConfig: any
  availableModels: any[]
  saving: boolean
}

const props = defineProps<Props>()

// Emits
interface Emits {
  'update:schedulerConfig': [value: any]
  'applyHighPerformancePreset': []
  'applyBalancedPreset': []
  'applyEconomicPreset': []
  'saveSchedulerConfig': []
}

const emit = defineEmits<Emits>()

// Computed
const schedulerConfig = computed({
  get: () => {
    const defaultConfig = {
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
    }
    
    if (!props.schedulerConfig) {
      return defaultConfig
    }
    
    // 确保所有必需字段都存在
    return {
      enabled: props.schedulerConfig.enabled ?? defaultConfig.enabled,
      models: {
        intent_analysis: props.schedulerConfig.models?.intent_analysis ?? defaultConfig.models.intent_analysis,
        intent_analysis_provider: props.schedulerConfig.models?.intent_analysis_provider ?? defaultConfig.models.intent_analysis_provider,
        planner: props.schedulerConfig.models?.planner ?? defaultConfig.models.planner,
        planner_provider: props.schedulerConfig.models?.planner_provider ?? defaultConfig.models.planner_provider,
        replanner: props.schedulerConfig.models?.replanner ?? defaultConfig.models.replanner,
        replanner_provider: props.schedulerConfig.models?.replanner_provider ?? defaultConfig.models.replanner_provider,
        executor: props.schedulerConfig.models?.executor ?? defaultConfig.models.executor,
        executor_provider: props.schedulerConfig.models?.executor_provider ?? defaultConfig.models.executor_provider,
        evaluator: props.schedulerConfig.models?.evaluator ?? defaultConfig.models.evaluator,
        evaluator_provider: props.schedulerConfig.models?.evaluator_provider ?? defaultConfig.models.evaluator_provider
      },
      default_strategy: props.schedulerConfig.default_strategy ?? defaultConfig.default_strategy,
      max_retries: props.schedulerConfig.max_retries ?? defaultConfig.max_retries,
      timeout_seconds: props.schedulerConfig.timeout_seconds ?? defaultConfig.timeout_seconds,
      scenarios: props.schedulerConfig.scenarios ?? defaultConfig.scenarios
    }
  },
  set: (value: any) => emit('update:schedulerConfig', value)
})

// Computed properties for providers
const availableProviders = computed(() => {
  const providers = new Set<string>()
  props.availableModels.forEach(model => {
    providers.add(model.provider)
  })
  return Array.from(providers).sort()
})

// Methods
const getProviderModels = (provider: string) => {
  if (!provider) return []
  return props.availableModels.filter(model => model.provider === provider)
}

const onProviderChange = (stage: string, provider: string) => {
  // 当提供商改变时，重置该阶段的模型选择
  const modelKey = stage as keyof typeof schedulerConfig.value.models
  if (schedulerConfig.value.models[modelKey] !== undefined) {
    schedulerConfig.value.models[modelKey] = ''
  }
  saveSchedulerConfig()
}

// 根据模型ID推断提供商
const inferProviderFromModel = (modelId: string) => {
  if (!modelId) return ''
  const model = props.availableModels.find(m => m.id === modelId)
  return model ? model.provider : ''
}

// 初始化提供商字段（基于现有的模型ID）
const initializeProviders = () => {
  const config = schedulerConfig.value
  if (config.models.intent_analysis && !config.models.intent_analysis_provider) {
    config.models.intent_analysis_provider = inferProviderFromModel(config.models.intent_analysis)
  }
  if (config.models.planner && !config.models.planner_provider) {
    config.models.planner_provider = inferProviderFromModel(config.models.planner)
  }
  if (config.models.replanner && !config.models.replanner_provider) {
    config.models.replanner_provider = inferProviderFromModel(config.models.replanner)
  }
  if (config.models.executor && !config.models.executor_provider) {
    config.models.executor_provider = inferProviderFromModel(config.models.executor)
  }
  if (config.models.evaluator && !config.models.evaluator_provider) {
    config.models.evaluator_provider = inferProviderFromModel(config.models.evaluator)
  }
}

const getCurrentStrategyName = () => {
  const strategies: Record<string, string> = {
    'adaptive': t('settings.scheduler.strategies.adaptive'),
    'conservative': t('settings.scheduler.strategies.conservative'),
    'aggressive': t('settings.scheduler.strategies.aggressive'),
    'cost_optimized': t('settings.scheduler.strategies.costOptimized')
  }
  return strategies[props.schedulerConfig.default_strategy] || t('settings.scheduler.strategies.adaptive')
}

const getConfiguredModelsCount = () => {
  const models = props.schedulerConfig.models
  let count = 0
  if (models.intent_analysis) count++
  if (models.planner) count++
  if (models.replanner) count++
  if (models.executor) count++
  if (models.evaluator) count++
  return `${count}/5`
}

const getPerformanceLevel = () => {
  const count = getConfiguredModelsCount()
  if (count === '5/5') return t('settings.scheduler.high')
  if (count === '4/5' || count === '3/5') return t('settings.scheduler.medium')
  return t('settings.scheduler.low')
}

const getEstimatedCost = () => {
  // 简单的成本估算逻辑
  const models = props.schedulerConfig.models
  let cost = 0
  
  // 根据配置的模型数量和类型估算成本
  if (models.intent_analysis) cost += 0.01
  if (models.planner) cost += 0.05
  if (models.replanner) cost += 0.03
  if (models.executor) cost += 0.10
  if (models.evaluator) cost += 0.02
  
  return cost.toFixed(3)
}

const getStrategyDescription = (strategy: string) => {
  const descriptions: Record<string, string> = {
    'adaptive': t('settings.scheduler.strategies.adaptiveDesc'),
    'conservative': t('settings.scheduler.strategies.conservativeDesc'),
    'aggressive': t('settings.scheduler.strategies.aggressiveDesc'),
    'cost_optimized': t('settings.scheduler.strategies.costOptimizedDesc')
  }
  return descriptions[strategy] || ''
}

const applyHighPerformancePreset = () => {
  emit('applyHighPerformancePreset')
}

const applyBalancedPreset = () => {
  emit('applyBalancedPreset')
}

const applyEconomicPreset = () => {
  emit('applyEconomicPreset')
}

const saveSchedulerConfig = () => {
  emit('saveSchedulerConfig')
}

// 监听props变化，当模型列表可用时初始化提供商
watch(() => [props.availableModels, props.schedulerConfig], () => {
  if (props.availableModels.length > 0 && props.schedulerConfig) {
    initializeProviders()
  }
}, { deep: true, immediate: true })

// 组件挂载时初始化提供商
onMounted(() => {
  if (props.availableModels.length > 0 && props.schedulerConfig) {
    initializeProviders()
  }
})
</script>

<style scoped>
.scheduler-settings {
  @apply space-y-6;
}

.card {
  @apply transition-all duration-200 hover:shadow-md;
}

.stat {
  @apply transition-all duration-200 hover:scale-105;
}

.border {
  @apply transition-all duration-200;
}

.border:hover {
  @apply border-primary/30;
}
</style>