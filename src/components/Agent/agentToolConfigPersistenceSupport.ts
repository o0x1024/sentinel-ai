import type { UiToolConfigPayload } from './toolConfigRuntime'

export const buildPersistableToolConfig = (config: UiToolConfigPayload) => ({
  disabled_tools: config.disabled_tools,
  enabled: config.enabled,
  fixed_tools: config.fixed_tools,
  max_tools: config.max_tools,
  selection_strategy: config.selection_strategy,
})

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
