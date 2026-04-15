import { computed, ref } from 'vue'
import type {
  AssistantConversationBinding,
  AssistantContextMode,
  AssistantRunMode,
  AssistantSessionSettings,
} from './agentDraftTypes'
import { buildPersistableToolConfig } from './agentToolConfigPersistenceSupport'
import type { UiToolConfigPayload } from './toolConfigRuntime'

export const DEFAULT_ASSISTANT_PROFILE_ID = 'assistant.default'
export const DEFAULT_ASSISTANT_CONTEXT_MODE: AssistantContextMode = 'claude-like'
export const DEFAULT_ASSISTANT_RUN_MODE: AssistantRunMode = 'assistant'
export const ASSISTANT_CONVERSATION_BINDING_VERSION = 2

export const createDefaultAssistantSessionSettings = (): AssistantSessionSettings => ({
  profileId: DEFAULT_ASSISTANT_PROFILE_ID,
  contextMode: DEFAULT_ASSISTANT_CONTEXT_MODE,
  runMode: DEFAULT_ASSISTANT_RUN_MODE,
  ragEnabled: false,
  webSearchEnabled: false,
  tenthManEnabled: false,
})

export const useAssistantSessionSettings = () => {
  const sessionSettings = ref<AssistantSessionSettings>(createDefaultAssistantSessionSettings())

  const applySessionSettings = (nextSettings: Partial<AssistantSessionSettings>) => {
    sessionSettings.value = {
      ...sessionSettings.value,
      ...nextSettings,
    }
  }

  const resetSessionSettings = () => {
    sessionSettings.value = createDefaultAssistantSessionSettings()
  }

  const ragEnabled = computed({
    get: () => sessionSettings.value.ragEnabled,
    set: (value: boolean) => {
      applySessionSettings({
        ragEnabled: value,
      })
    },
  })

  const webSearchEnabled = computed({
    get: () => sessionSettings.value.webSearchEnabled,
    set: (value: boolean) => {
      applySessionSettings({
        webSearchEnabled: value,
      })
    },
  })

  const tenthManEnabled = computed({
    get: () => sessionSettings.value.tenthManEnabled,
    set: (value: boolean) => {
      applySessionSettings({
        tenthManEnabled: value,
      })
    },
  })

  const teamModeEnabled = computed({
    get: () => sessionSettings.value.runMode === 'team',
    set: (enabled: boolean) => {
      applySessionSettings({
        runMode: enabled ? 'team' : DEFAULT_ASSISTANT_RUN_MODE,
      })
    },
  })

  const setProfileId = (profileId: string) => {
    const normalized = profileId.trim()
    if (!normalized) return
    applySessionSettings({
      profileId: normalized,
    })
  }

  const applyProfilePreset = (profile: {
    id: string
    contextMode: AssistantContextMode
    runMode: AssistantRunMode
    defaultRagEnabled?: boolean | null
    defaultWebSearchEnabled?: boolean | null
    defaultTenthManEnabled?: boolean | null
  }) => {
    const normalized = profile.id.trim()
    if (!normalized) return
    applySessionSettings({
      profileId: normalized,
      contextMode: profile.contextMode,
      runMode: profile.runMode,
      ragEnabled: profile.defaultRagEnabled === true,
      webSearchEnabled: profile.defaultWebSearchEnabled === true,
      tenthManEnabled: profile.defaultTenthManEnabled === true,
    })
  }

  const setContextMode = (contextMode: AssistantContextMode) => {
    applySessionSettings({
      contextMode,
    })
  }

  const setRunMode = (runMode: AssistantRunMode) => {
    applySessionSettings({
      runMode,
    })
  }

  const toConversationBinding = (extras?: {
    selectedModel?: string | null
    toolsEnabled?: boolean
    toolConfig?: UiToolConfigPayload | null
  }): AssistantConversationBinding => {
    const selectedModel = extras?.selectedModel?.trim()
    return {
      schemaVersion: ASSISTANT_CONVERSATION_BINDING_VERSION,
      ...sessionSettings.value,
      selectedModel: selectedModel || null,
      toolsEnabled: extras?.toolsEnabled,
      toolConfig: extras?.toolConfig ? buildPersistableToolConfig(extras.toolConfig) : null,
    }
  }

  const applyConversationBinding = (binding: Partial<AssistantConversationBinding> | null | undefined) => {
    if (!binding) {
      resetSessionSettings()
      return
    }

    applySessionSettings({
      profileId: binding.profileId || DEFAULT_ASSISTANT_PROFILE_ID,
      contextMode: binding.contextMode || DEFAULT_ASSISTANT_CONTEXT_MODE,
      runMode: binding.runMode || DEFAULT_ASSISTANT_RUN_MODE,
      ragEnabled: binding.ragEnabled === true,
      webSearchEnabled: binding.webSearchEnabled === true,
      tenthManEnabled: binding.tenthManEnabled === true,
    })
  }

  return {
    applyConversationBinding,
    applySessionSettings,
    applyProfilePreset,
    createDefaultAssistantSessionSettings,
    resetSessionSettings,
    sessionSettings,
    ragEnabled,
    setContextMode,
    setProfileId,
    setRunMode,
    teamModeEnabled,
    tenthManEnabled,
    toConversationBinding,
    webSearchEnabled,
  }
}
