import { invoke } from '@tauri-apps/api/core'

export const loadAiUsageStats = async () => {
  const stats = (await invoke('get_ai_usage_stats')) as Record<
    string,
    { input_tokens: number; output_tokens: number; total_tokens: number; cost: number }
  >
  return stats || {}
}

export const loadAiConfig = async () => {
  const aiConfig = (await invoke('get_ai_config')) as any
  try {
    const configs = (await invoke('get_config', {
      request: { category: 'ai', key: 'enable_multimodal' },
    })) as Array<{ key: string; value: string }>
    aiConfig.enable_multimodal = configs && configs.length > 0 ? configs[0].value === 'true' : true
  } catch {
    aiConfig.enable_multimodal = true
  }
  return aiConfig
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
