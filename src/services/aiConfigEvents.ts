export const AI_CONFIG_UPDATED_EVENT = 'sentinel:ai-config-updated'

const cloneAiConfig = <T>(config: T): T => {
  if (!config) return config
  try {
    return JSON.parse(JSON.stringify(config)) as T
  } catch {
    return config
  }
}

export const emitAiConfigUpdated = (config: unknown) => {
  if (typeof window === 'undefined') return
  window.dispatchEvent(new CustomEvent(AI_CONFIG_UPDATED_EVENT, {
    detail: {
      config: cloneAiConfig(config),
    },
  }))
}

export const getAiConfigFromUpdateEvent = (event: Event) =>
  (event as CustomEvent<{ config?: any }>).detail?.config ?? null
