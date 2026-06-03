import { invoke } from '@tauri-apps/api/core'

const MODEL_DERIVED_KEYS = [
  'vision_capability_status',
  'vision_capability_source',
  'vision_capability_evidence',
]

export const loadAiUsageStats = async () => {
  const stats = (await invoke('get_ai_usage_stats')) as Record<
    string,
    { input_tokens: number; output_tokens: number; total_tokens: number; cost: number }
  >
  return stats || {}
}

export const loadAiConfig = async () => {
  const aiConfig = (await invoke('get_ai_config')) as any
  return aiConfig
}

export const stripDerivedAiConfigFields = (aiConfig: any) => {
  if (!aiConfig || typeof aiConfig !== 'object') {
    return aiConfig
  }

  const cloned = JSON.parse(JSON.stringify(aiConfig))
  const providers = cloned?.providers
  if (!providers || typeof providers !== 'object') {
    return cloned
  }

  Object.values(providers).forEach((provider: any) => {
    if (!Array.isArray(provider?.models)) return
    provider.models = provider.models.map((model: any) => {
      if (!model || typeof model !== 'object') return model
      const sanitized = { ...model }
      MODEL_DERIVED_KEYS.forEach((key) => {
        delete sanitized[key]
      })
      return sanitized
    })
  })

  return cloned
}

export const buildAvailableModels = (aiConfig: any) => {
  const models: any[] = []
  Object.entries(aiConfig.providers || {}).forEach(([providerKey, provider]: [string, any]) => {
    if (!Array.isArray(provider.models)) return
    provider.models.forEach((model: any) => {
      if (model.is_available !== false) {
        models.push({ ...model, provider: providerKey })
      }
    })
  })
  return models
}

export const buildAvailableProviders = (aiConfig: any) => {
  const providers: string[] = []
  Object.entries(aiConfig.providers || {}).forEach(([providerKey, provider]: [string, any]) => {
    if (provider.enabled && !providers.includes(providerKey)) {
      providers.push(providerKey)
    }
  })
  return providers
}
