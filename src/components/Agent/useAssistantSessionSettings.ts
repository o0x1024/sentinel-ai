import { computed, ref } from 'vue'
import type {
  AssistantConversationBinding,
  AssistantContextMode,
  AssistantRunMode,
  AssistantSessionSettings,
} from './agentDraftTypes'
import { buildPersistableToolConfig } from './agentToolConfigPersistenceSupport'
import {
  DEFAULT_AGENT_HARNESS_MAX_CONTINUATIONS,
  normalizeHarnessMaxContinuations,
  type AssistantProfileOption,
} from './assistantProfiles'
import type { UiToolConfigPayload } from './toolConfigRuntime'

export const DEFAULT_ASSISTANT_PROFILE_ID = 'assistant.default'
export const DEFAULT_ASSISTANT_CONTEXT_MODE: AssistantContextMode = 'claude-like'
export const DEFAULT_ASSISTANT_RUN_MODE: AssistantRunMode = 'assistant'
export const ASSISTANT_CONVERSATION_BINDING_VERSION = 4

export const createDefaultAssistantSessionSettings = (): AssistantSessionSettings => ({
  profileId: DEFAULT_ASSISTANT_PROFILE_ID,
  teamProfileId: '',
  contextMode: DEFAULT_ASSISTANT_CONTEXT_MODE,
  runMode: DEFAULT_ASSISTANT_RUN_MODE,
  workingDirectoryOverride: '',
  ragEnabled: false,
  webSearchEnabled: false,
  tenthManEnabled: false,
  harnessMaxContinuations: DEFAULT_AGENT_HARNESS_MAX_CONTINUATIONS,
})

export const buildBaseAssistantConversationBinding = (params: {
  profile: AssistantProfileOption
  workingDirectoryOverride: string
}): AssistantConversationBinding => {
  const profileId = params.profile.id.trim()
  if (!profileId) {
    throw new Error('Assistant profile id is required for a new conversation binding.')
  }

  return {
    schemaVersion: ASSISTANT_CONVERSATION_BINDING_VERSION,
    profileId,
    teamProfileId: '',
    contextMode: params.profile.contextMode,
    runMode: params.profile.runMode,
    workingDirectoryOverride: params.workingDirectoryOverride.trim(),
    ragEnabled: params.profile.defaultRagEnabled === true,
    webSearchEnabled: params.profile.defaultWebSearchEnabled === true,
    tenthManEnabled: params.profile.defaultTenthManEnabled === true,
    harnessMaxContinuations: normalizeHarnessMaxContinuations(
      params.profile.defaultHarnessMaxContinuations
    ),
    browserShellDirectWriteEnabled: false,
    browserShellSessionId: null,
    selectedModel: null,
    toolConfig: null,
  }
}

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

  const setTeamProfileId = (teamProfileId: string) => {
    applySessionSettings({
      teamProfileId: teamProfileId.trim(),
    })
  }

  const applyProfilePreset = (profile: {
    id: string
    contextMode: AssistantContextMode
    runMode: AssistantRunMode
    defaultRagEnabled?: boolean | null
    defaultWebSearchEnabled?: boolean | null
    defaultTenthManEnabled?: boolean | null
    defaultHarnessMaxContinuations?: number | null
  }) => {
    const normalized = profile.id.trim()
    if (!normalized) return
    applySessionSettings({
      profileId: normalized,
      teamProfileId: '',
      contextMode: profile.contextMode,
      runMode: profile.runMode,
      ragEnabled: profile.defaultRagEnabled === true,
      webSearchEnabled: profile.defaultWebSearchEnabled === true,
      tenthManEnabled: profile.defaultTenthManEnabled === true,
      harnessMaxContinuations: normalizeHarnessMaxContinuations(
        profile.defaultHarnessMaxContinuations
      ),
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

  const setWorkingDirectoryOverride = (workingDirectoryOverride: string) => {
    applySessionSettings({
      workingDirectoryOverride: workingDirectoryOverride.trim(),
    })
  }

  const toConversationBinding = (extras?: {
    browserShellDirectWriteEnabled?: boolean
    browserShellSessionId?: string | null
    selectedModel?: string | null
    toolsEnabled?: boolean
    toolConfig?: UiToolConfigPayload | null
  }): AssistantConversationBinding => {
    const selectedModel = extras?.selectedModel?.trim()
    const browserShellSessionId = extras?.browserShellSessionId?.trim()
    return {
      schemaVersion: ASSISTANT_CONVERSATION_BINDING_VERSION,
      ...sessionSettings.value,
      browserShellDirectWriteEnabled: extras?.browserShellDirectWriteEnabled === true,
      browserShellSessionId: browserShellSessionId || null,
      selectedModel: selectedModel || null,
      toolsEnabled: extras?.toolsEnabled,
      toolConfig: extras?.toolConfig ? buildPersistableToolConfig(extras.toolConfig) : null,
    }
  }

  const applyConversationBinding = (
    binding: Partial<AssistantConversationBinding> | null | undefined
  ) => {
    if (!binding) {
      resetSessionSettings()
      return
    }

    applySessionSettings({
      profileId: binding.profileId || DEFAULT_ASSISTANT_PROFILE_ID,
      teamProfileId: String(binding.teamProfileId || '').trim(),
      contextMode: binding.contextMode || DEFAULT_ASSISTANT_CONTEXT_MODE,
      runMode: binding.runMode || DEFAULT_ASSISTANT_RUN_MODE,
      workingDirectoryOverride: String(binding.workingDirectoryOverride || '').trim(),
      ragEnabled: binding.ragEnabled === true,
      webSearchEnabled: binding.webSearchEnabled === true,
      tenthManEnabled: binding.tenthManEnabled === true,
      harnessMaxContinuations: normalizeHarnessMaxContinuations(binding.harnessMaxContinuations),
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
    setTeamProfileId,
    setWorkingDirectoryOverride,
    teamModeEnabled,
    tenthManEnabled,
    toConversationBinding,
    webSearchEnabled,
  }
}
