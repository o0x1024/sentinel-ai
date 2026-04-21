import { computed, ref, type Ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { getModelSupportsVision, getModelVisionCapability } from '@/services/aiModelCapabilities'
import {
  normalizeToolIdList,
  normalizeUiToolConfigPayload,
  parseToolSelectionStrategy,
  type UiToolConfigPayload,
} from './toolConfigRuntime'
import type { AssistantModelOption } from './agentDraftTypes'

const ASSISTANT_MODEL_STORAGE_KEY = 'sentinel:agent:assistant-model'
const DEFAULT_MAX_CONTEXT_TOKENS = 128000

const createBaseToolConfig = (): UiToolConfigPayload => ({
  enabled: true,
  selection_strategy: 'Keyword',
  max_tools: 5,
  fixed_tools: ['interactive_shell', 'ask_user_question'],
  disabled_tools: [],
  allowed_tools: [],
})

const toPositiveTokenLimit = (value: unknown): number | null => {
  const parsed = Number(value)
  if (!Number.isFinite(parsed) || parsed <= 0) return null
  return Math.floor(parsed)
}

const normalizeProviderName = (provider: string) => {
  const lower = provider.toLowerCase()
  const names: Record<string, string> = {
    openai: 'OpenAI',
    anthropic: 'Anthropic',
    gemini: 'Gemini',
    deepseek: 'DeepSeek',
    moonshot: 'Moonshot',
    ollama: 'Ollama',
    openrouter: 'OpenRouter',
    modelscope: 'ModelScope',
    groq: 'Groq',
    perplexity: 'Perplexity',
    togetherai: 'TogetherAI',
    xai: 'xAI',
    cohere: 'Cohere',
    'lm studio': 'LM Studio',
    lmstudio: 'LM Studio',
    lm_studio: 'LM Studio',
  }
  return names[lower] || provider
}

const extractModelId = (item: any): string => {
  if (!item) return ''
  if (typeof item === 'string') return item
  if (typeof item.id === 'string') return item.id
  if (typeof item.name === 'string') return item.name
  return ''
}

export const useAgentModelAndToolConfig = (params: {
  localError: Ref<string | null>
  getFailedToSaveToolConfigLabel: () => string
}) => {
  const defaultToolConfig = ref<UiToolConfigPayload>(createBaseToolConfig())
  const toolConfig = ref<UiToolConfigPayload>(createBaseToolConfig())
  const toolsEnabled = ref(true)
  const assistantModelOptions = ref<AssistantModelOption[]>([])
  const assistantSelectedModel = ref('')
  const isLoadingAssistantModels = ref(false)
  const assistantProviderMaxContextMap = ref<Record<string, number>>({})

  const assistantDefaultMaxContextTokens = computed(() => {
    const selected = (assistantSelectedModel.value || '').trim()
    const providerKey = selected.includes('/') ? selected.split('/')[0].toLowerCase() : ''
    if (providerKey) {
      const configured = assistantProviderMaxContextMap.value[providerKey]
      if (typeof configured === 'number' && configured > 0) {
        return configured
      }
    }
    return DEFAULT_MAX_CONTEXT_TOKENS
  })

  const loadAssistantModelOptions = async () => {
    isLoadingAssistantModels.value = true
    try {
      const aiConfig = await invoke<any>('get_ai_config')
      const providers = aiConfig?.providers && typeof aiConfig.providers === 'object'
        ? aiConfig.providers
        : {}
      const defaultModel = typeof aiConfig?.default_llm_model === 'string' ? aiConfig.default_llm_model : ''
      const options: AssistantModelOption[] = []
      const providerMaxContextMap: Record<string, number> = {}

      Object.entries(providers).forEach(([providerKey, providerValue]) => {
        const cfg = providerValue as any
        if (cfg?.enabled === false) return

        const providerRaw = String(cfg?.provider || providerKey).trim()
        const provider = providerRaw.toLowerCase()
        if (!provider) return
        const maxContextLength = toPositiveTokenLimit(cfg?.max_context_length) ?? DEFAULT_MAX_CONTEXT_TOKENS
        providerMaxContextMap[provider] = maxContextLength
        providerMaxContextMap[String(providerKey).toLowerCase()] = maxContextLength

        const modelsRaw = Array.isArray(cfg?.models) ? cfg.models : []
        const modelRecords = new Map<string, any>()
        modelsRaw.forEach((model: any) => {
          const modelId = extractModelId(model).trim()
          if (!modelId) return
          modelRecords.set(modelId, model)
        })
        const modelIds = Array.from(modelRecords.keys())
        if (typeof cfg?.default_model === 'string' && cfg.default_model.trim()) {
          const providerDefaultModel = cfg.default_model.trim()
          if (!modelIds.some((id) => id === providerDefaultModel)) {
            modelIds.push(providerDefaultModel)
          }
        }

        Array.from(new Set<string>(modelIds)).forEach((modelId: string) => {
          const modelRecord = modelRecords.get(modelId) || { id: modelId, name: modelId }
          const visionCapability = getModelVisionCapability(providerRaw, modelRecord)
          const supportsVision = getModelSupportsVision(providerRaw, modelRecord)
          options.push({
            value: `${provider}/${modelId}`,
            label: modelId,
            description: normalizeProviderName(providerRaw),
            supportsVision,
            visionCapability,
          })
        })
      })

      options.sort((a, b) => a.label.localeCompare(b.label))

      if (defaultModel && defaultModel.includes('/')) {
        const [defaultProvider, ...defaultModelParts] = defaultModel.split('/')
        const defaultModelName = defaultModelParts.join('/')
        const providerLower = defaultProvider.toLowerCase()
        const key = `${providerLower}/${defaultModelName}`
        if (
          !options.some((item) => item.value.toLowerCase() === key.toLowerCase()) &&
          providerLower &&
          defaultModelName
        ) {
          const fallbackModel = { id: defaultModelName, name: defaultModelName }
          const visionCapability = getModelVisionCapability(providerLower, fallbackModel)
          const supportsVision = getModelSupportsVision(providerLower, fallbackModel)
          options.unshift({
            value: key,
            label: defaultModelName,
            description: normalizeProviderName(providerLower),
            supportsVision,
            visionCapability,
          })
        }
      }

      assistantModelOptions.value = options
      assistantProviderMaxContextMap.value = providerMaxContextMap

      let stored = ''
      try {
        stored = localStorage.getItem(ASSISTANT_MODEL_STORAGE_KEY) || ''
      } catch {
        stored = ''
      }

      const preferred = stored || assistantSelectedModel.value || defaultModel
      if (preferred && options.some((item) => item.value === preferred)) {
        assistantSelectedModel.value = preferred
      } else if (defaultModel && options.some((item) => item.value === defaultModel)) {
        assistantSelectedModel.value = defaultModel
      } else if (options.length > 0) {
        assistantSelectedModel.value = options[0].value
      } else {
        assistantSelectedModel.value = ''
      }
    } catch (error) {
      console.warn('[useAgentModelAndToolConfig] Failed to load assistant model options:', error)
      assistantModelOptions.value = []
      assistantProviderMaxContextMap.value = {}
    } finally {
      isLoadingAssistantModels.value = false
    }
  }

  const setAssistantSelectedModel = (value: string, options?: { persist?: boolean }) => {
    assistantSelectedModel.value = value
    if (options?.persist === false) {
      return
    }
    try {
      if (value) {
        localStorage.setItem(ASSISTANT_MODEL_STORAGE_KEY, value)
      } else {
        localStorage.removeItem(ASSISTANT_MODEL_STORAGE_KEY)
      }
    } catch {
      // ignore storage errors
    }
  }

  const handleAssistantModelChange = (value: string) => {
    setAssistantSelectedModel(value)
  }

  const buildTeamToolPolicyFromUiConfig = (config: UiToolConfigPayload) => {
    const disabledSet = new Set(normalizeToolIdList(config.disabled_tools))
    const fixedSet = new Set(normalizeToolIdList(config.fixed_tools))
    const manualFallback = normalizeToolIdList((config as any).manual_tools)
    const strategy = parseToolSelectionStrategy(config.selection_strategy, manualFallback)

    const denylist = [...disabledSet]
    let allowlist: string[] | undefined

    if (!config.enabled) {
      allowlist = []
    } else if (strategy.mode === 'Manual') {
      const manualSet = new Set([...strategy.manualTools, ...fixedSet])
      allowlist = [...manualSet].filter((tool) => !disabledSet.has(tool))
    }

    return {
      enabled: config.enabled,
      allowlist: allowlist ?? null,
      denylist,
      selection_strategy: config.selection_strategy,
    } as Record<string, unknown>
  }

  const flushPendingToolConfigSave = async () => {}

  const handleToolConfigUpdate = (config: UiToolConfigPayload) => {
    const normalized = normalizeUiToolConfigPayload(config, defaultToolConfig.value)
    toolConfig.value = normalized
    toolsEnabled.value = normalized.enabled
  }

  const loadToolConfig = async () => {
    try {
      const savedConfig = await invoke<any>('get_tool_config')
      if (!savedConfig) {
        defaultToolConfig.value = createBaseToolConfig()
        toolConfig.value = createBaseToolConfig()
        toolsEnabled.value = toolConfig.value.enabled
        return
      }
      const normalized = normalizeUiToolConfigPayload(savedConfig, createBaseToolConfig())
      defaultToolConfig.value = normalized
      toolConfig.value = normalized
      toolsEnabled.value = normalized.enabled
    } catch (error) {
      console.error('[useAgentModelAndToolConfig] Failed to load tool config:', error)
    }
  }

  return {
    assistantDefaultMaxContextTokens,
    assistantModelOptions,
    assistantSelectedModel,
    buildTeamToolPolicyFromUiConfig,
    defaultToolConfig,
    flushPendingToolConfigSave,
    handleAssistantModelChange,
    handleToolConfigUpdate,
    isLoadingAssistantModels,
    loadAssistantModelOptions,
    loadToolConfig,
    setAssistantSelectedModel,
    toolConfig,
    toolsEnabled,
  }
}
