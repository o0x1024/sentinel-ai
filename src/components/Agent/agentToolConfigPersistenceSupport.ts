import {
  normalizeUiToolConfigPayload,
  type UiToolConfigPayload,
} from './toolConfigRuntime'

export const buildPersistableToolConfig = (config: UiToolConfigPayload) => {
  const normalized = normalizeUiToolConfigPayload(config)
  return {
    disabled_tools: normalized.disabled_tools,
    enabled: normalized.enabled,
    preselected_tools: normalized.preselected_tools,
    max_tools: normalized.max_tools,
    selection_strategy: normalized.selection_strategy,
    allowed_tools: normalized.allowed_tools || [],
  }
}

export const buildPersistableToolConfigSignature = (config: UiToolConfigPayload): string => (
  JSON.stringify(buildPersistableToolConfig(config))
)

export const persistToolConfig = async (params: {
  config: UiToolConfigPayload
  saveToolConfig: (toolConfig: ReturnType<typeof buildPersistableToolConfig>) => Promise<void>
}): Promise<string> => {
  const persistableToolConfig = buildPersistableToolConfig(params.config)
  await params.saveToolConfig(persistableToolConfig)
  return JSON.stringify(persistableToolConfig)
}
