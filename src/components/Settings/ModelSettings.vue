<template>
  <div class="scheduler-settings">
    <!-- 模型配置概览 -->
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
                        @change="onProviderChange('intent_analysis', $event)">
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
                        @change="onProviderChange('planner', $event)">
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
                        @change="onProviderChange('executor', $event)">
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
                        @change="onProviderChange('replanner', $event)">
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
                        @change="onProviderChange('evaluator', $event)">
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
import { ref, computed, watch, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { invoke } from '@tauri-apps/api/core'

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

// 本地可变配置 + 深度同步父组件
const buildDefaultConfig = () => ({
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
    evaluator_provider: '',
    rag_embedding: '',
    rag_embedding_provider: '',
    rag_reranking: '',
    rag_reranking_provider: ''
  },
  default_strategy: 'adaptive',
  max_retries: 3,
  timeout_seconds: 120,
  scenarios: {},
  rag_config: {
    embedding: {
      batch_size: 32,
      max_concurrent: 5
    },
    reranking: {
      enabled: false
    },
    retrieval: {
      top_k: 10,
      mmr_lambda: 0.7,
      similarity_threshold: 0.3
    }
  }
})

const normalizeConfig = (cfg: any) => {
  const d = buildDefaultConfig()
  if (!cfg) return d
  return {
    enabled: cfg.enabled ?? d.enabled,
    models: {
      intent_analysis: cfg.models?.intent_analysis ?? d.models.intent_analysis,
      intent_analysis_provider: cfg.models?.intent_analysis_provider ?? d.models.intent_analysis_provider,
      planner: cfg.models?.planner ?? d.models.planner,
      planner_provider: cfg.models?.planner_provider ?? d.models.planner_provider,
      replanner: cfg.models?.replanner ?? d.models.replanner,
      replanner_provider: cfg.models?.replanner_provider ?? d.models.replanner_provider,
      executor: cfg.models?.executor ?? d.models.executor,
      executor_provider: cfg.models?.executor_provider ?? d.models.executor_provider,
      evaluator: cfg.models?.evaluator ?? d.models.evaluator,
      evaluator_provider: cfg.models?.evaluator_provider ?? d.models.evaluator_provider,
      rag_embedding: cfg.models?.rag_embedding ?? d.models.rag_embedding,
      rag_embedding_provider: cfg.models?.rag_embedding_provider ?? d.models.rag_embedding_provider,
      rag_reranking: cfg.models?.rag_reranking ?? d.models.rag_reranking,
      rag_reranking_provider: cfg.models?.rag_reranking_provider ?? d.models.rag_reranking_provider,
    },
    default_strategy: cfg.default_strategy ?? d.default_strategy,
    max_retries: cfg.max_retries ?? d.max_retries,
    timeout_seconds: cfg.timeout_seconds ?? d.timeout_seconds,
    scenarios: cfg.scenarios ?? d.scenarios,
    rag_config: {
      embedding: {
        batch_size: cfg.rag_config?.embedding?.batch_size ?? d.rag_config.embedding.batch_size,
        max_concurrent: cfg.rag_config?.embedding?.max_concurrent ?? d.rag_config.embedding.max_concurrent
      },
      reranking: {
        enabled: cfg.rag_config?.reranking?.enabled ?? d.rag_config.reranking.enabled
      },
      retrieval: {
        top_k: cfg.rag_config?.retrieval?.top_k ?? d.rag_config.retrieval.top_k,
        mmr_lambda: cfg.rag_config?.retrieval?.mmr_lambda ?? d.rag_config.retrieval.mmr_lambda,
        similarity_threshold: cfg.rag_config?.retrieval?.similarity_threshold ?? d.rag_config.retrieval.similarity_threshold
      }
    }
  }
}

const schedulerConfig = ref<any>(normalizeConfig(props.schedulerConfig))


watch(() => props.schedulerConfig, (v) => {
  schedulerConfig.value = normalizeConfig(v)
}, { deep: true })

watch(schedulerConfig, (v) => {
  emit('update:schedulerConfig', v)
}, { deep: true })


// Computed properties for providers
const availableProviders = computed(() => {
  const providers = new Set<string>()
  console.log('SchedulerSettings: computing availableProviders from models:', props.availableModels)
  props.availableModels.forEach(model => {
    if (model.provider) {
      providers.add(model.provider)
    }
  })
  
  // 为RAG特殊用途，确保包含常用的嵌入模型提供商
  const ragProviders = ['LM Studio', 'OpenAI', 'Ollama', 'Cohere', 'HuggingFace', 'Azure']
  ragProviders.forEach(provider => providers.add(provider))
  
  const result = Array.from(providers).sort()
  console.log('SchedulerSettings: computed availableProviders:', result)
  return result
})

// Methods
const normalize = (s: string) => (s || '').trim().toLowerCase()

const getProviderModels = (provider: string) => {
  if (!provider) {
    console.log('SchedulerSettings: getProviderModels called with empty provider')
    return []
  }
  
  const pv = normalize(provider)
  
  // 从props.availableModels中筛选
  const result = props.availableModels.filter(model => normalize(model.provider) === pv)
  console.log(`SchedulerSettings: getProviderModels for '${provider}' (normalized: '${pv}'):`, result)
  return result
}


const onProviderChange = (stage: string, evt: Event) => {
  // 兼容从模板传入的原生事件，安全读取值
  const target = evt?.target as HTMLSelectElement | null
  const provider = target?.value || ''
  // 当提供商改变时，重置该阶段的模型选择
  const modelKey = stage as keyof typeof schedulerConfig.value.models
  if (schedulerConfig.value.models[modelKey] !== undefined) {
    schedulerConfig.value.models[modelKey] = ''
  }
  // 若该提供商确有可用模型，则无需额外处理；若没有，保持空列表
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
  console.log('SchedulerSettings: initializeProviders called')
  const config = schedulerConfig.value
  console.log('SchedulerSettings: current config models:', config.models)
  console.log('SchedulerSettings: available models for inference:', props.availableModels)
  
  const ensureProvider = (modelKey: keyof typeof config.models, providerKey: keyof typeof config.models) => {
    const modelId = config.models[modelKey] as string
    let provider = (config.models[providerKey] as string) || ''
    // 如果 provider 在 availableProviders 中不存在，则尝试通过 model 反推
    if (!provider || !availableProviders.value.includes(provider)) {
      const inferred = inferProviderFromModel(modelId)
      if (inferred) {
        console.log(`SchedulerSettings: correcting provider for ${String(modelKey)} from '${provider}' to '${inferred}'`)
        provider = inferred
        ;(config.models as any)[providerKey] = inferred
      }
    }
  }

  ensureProvider('intent_analysis', 'intent_analysis_provider')
  ensureProvider('planner', 'planner_provider')
  ensureProvider('replanner', 'replanner_provider')
  ensureProvider('executor', 'executor_provider')
  ensureProvider('evaluator', 'evaluator_provider')
  
  console.log('SchedulerSettings: final config after initialization:', config.models)
}


const getCurrentStrategyName = () => {
  const strategies: Record<string, string> = {
    'adaptive': t('settings.scheduler.strategies.adaptive'),
    'conservative': t('settings.scheduler.strategies.conservative'),
    'aggressive': t('settings.scheduler.strategies.aggressive'),
    'cost_optimized': t('settings.scheduler.strategies.costOptimized')
  }
  return strategies[schedulerConfig.value.default_strategy] || t('settings.scheduler.strategies.adaptive')
}

const getConfiguredModelsCount = () => {
  const models = schedulerConfig.value.models
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
  const models = schedulerConfig.value.models
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


// 监听变化，当模型列表可用时初始化提供商
watch(() => [props.availableModels, schedulerConfig.value], () => {
  console.log('SchedulerSettings: availableModels changed:', props.availableModels)
  console.log('SchedulerSettings: schedulerConfig changed:', schedulerConfig.value)
  if (props.availableModels.length > 0 && schedulerConfig.value) {
    initializeProviders()
  }
}, { deep: true, immediate: true })


// 组件挂载时初始化提供商
onMounted(() => {
  console.log('SchedulerSettings: mounted with availableModels:', props.availableModels)
  console.log('SchedulerSettings: mounted with schedulerConfig:', schedulerConfig.value)
  if (props.availableModels.length > 0 && schedulerConfig.value) {
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