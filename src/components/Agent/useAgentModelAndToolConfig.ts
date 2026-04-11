import { computed, ref, type Ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { buildPersistableToolConfigSignature, persistToolConfig } from './agentToolConfigPersistenceSupport'
import { normalizeToolIdList, parseToolSelectionStrategy, type UiToolConfigPayload } from './toolConfigRuntime'
import type { AssistantModelOption } from './agentDraftTypes'

const ASSISTANT_MODEL_STORAGE_KEY = 'sentinel:agent:assistant-model'
const DEFAULT_MAX_CONTEXT_TOKENS = 128000
const TOOL_CONFIG_SAVE_DEBOUNCE_MS = 300

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
  const toolConfig = ref<UiToolConfigPayload>({
    enabled: true,
    selection_strategy: 'Keyword',
    max_tools: 5,
    fixed_tools: ['interactive_shell'],
    disabled_tools: [],
  } as any)
  const toolsEnabled = ref(true)
  const assistantModelOptions = ref<AssistantModelOption[]>([])
  const assistantSelectedModel = ref('')
  const isLoadingAssistantModels = ref(false)
  const assistantProviderMaxContextMap = ref<Record<string, number>>({})
  const lastPersistedToolConfigSignature = ref('')
  let pendingToolConfigSignature = ''
  let toolConfigSaveTimer: ReturnType<typeof setTimeout> | null = null

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
        const modelIds = modelsRaw.map(extractModelId).filter((value: string) => !!value)
        if (typeof cfg?.default_model === 'string' && cfg.default_model.trim()) {
          const providerDefaultModel = cfg.default_model.trim()
          if (!modelIds.some((id) => id === providerDefaultModel)) {
            modelIds.push(providerDefaultModel)
          }
        }

        Array.from(new Set<string>(modelIds)).forEach((modelId: string) => {
          options.push({
            value: `${provider}/${modelId}`,
            label: modelId,
            description: normalizeProviderName(providerRaw),
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
          options.unshift({
            value: key,
            label: defaultModelName,
            description: normalizeProviderName(providerLower),
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

  const schedulePersistToolConfig = (config: UiToolConfigPayload) => {
    const signature = buildPersistableToolConfigSignature(config)
    if (signature === lastPersistedToolConfigSignature.value || signature === pendingToolConfigSignature) {
      return
    }

    pendingToolConfigSignature = signature
    if (toolConfigSaveTimer) {
      clearTimeout(toolConfigSaveTimer)
    }

    toolConfigSaveTimer = setTimeout(async () => {
      try {
        lastPersistedToolConfigSignature.value = await persistToolConfig({
          config,
          saveToolConfig: async (toolConfig) => {
            await invoke('save_tool_config', { toolConfig })
          },
        })
      } catch (error) {
        console.error('[useAgentModelAndToolConfig] Failed to save tool config:', error)
        params.localError.value = `${params.getFailedToSaveToolConfigLabel()}: ${error}`
      } finally {
        if (pendingToolConfigSignature === signature) {
          pendingToolConfigSignature = ''
        }
        toolConfigSaveTimer = null
      }
    }, TOOL_CONFIG_SAVE_DEBOUNCE_MS)
  }

  const flushPendingToolConfigSave = async () => {
    const config = toolConfig.value as unknown as UiToolConfigPayload
    const signature = buildPersistableToolConfigSignature(config)

    if (toolConfigSaveTimer) {
      clearTimeout(toolConfigSaveTimer)
      toolConfigSaveTimer = null
    }

    if (signature === lastPersistedToolConfigSignature.value) {
      pendingToolConfigSignature = ''
      return
    }

    try {
      lastPersistedToolConfigSignature.value = await persistToolConfig({
        config,
        saveToolConfig: async (toolConfig) => {
          await invoke('save_tool_config', { toolConfig })
        },
      })
      pendingToolConfigSignature = ''
    } catch (error) {
      console.error('[useAgentModelAndToolConfig] Failed to flush tool config:', error)
      params.localError.value = `${params.getFailedToSaveToolConfigLabel()}: ${error}`
      throw error
    }
  }

  const handleToolConfigUpdate = (config: UiToolConfigPayload) => {
    toolConfig.value = { ...config } as any
    toolsEnabled.value = config.enabled
    schedulePersistToolConfig(config)
  }

  const loadToolConfig = async () => {
    try {
      const savedConfig = await invoke<any>('get_tool_config')
      if (!savedConfig) return
      toolConfig.value = { ...savedConfig }
      toolsEnabled.value = savedConfig.enabled
      lastPersistedToolConfigSignature.value = buildPersistableToolConfigSignature({
        ...(savedConfig as any),
      })
    } catch (error) {
      console.error('[useAgentModelAndToolConfig] Failed to load tool config:', error)
    }
  }

  return {
    assistantDefaultMaxContextTokens,
    assistantModelOptions,
    assistantSelectedModel,
    buildTeamToolPolicyFromUiConfig,
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
