import { invoke } from '@tauri-apps/api/core'
import { onUnmounted, ref, type Ref } from 'vue'
import type { AssistantProfileOption, TeamProfileOption } from './assistantProfiles'
import type { AssistantConversationBinding, AssistantContextMode } from './agentDraftTypes'
import {
  normalizeToolIdList,
  normalizeUiToolConfigPayload,
  type UiToolConfigPayload,
} from './toolConfigRuntime'

const CONVERSATION_BINDING_SAVE_DEBOUNCE_MS = 300

export const useAgentConversationBinding = (params: {
  bindBrowserShellSession: (sessionId: string | null) => void
  setBrowserShellDirectWriteEnabled: (enabled: boolean) => void
  conversationId: Ref<string | null>
  currentBrowserShellDirectWriteEnabled: Ref<boolean>
  currentBrowserShellSessionId: Ref<string | null>
  assistantGlobalDefaultModel: Ref<string>
  assistantSelectedModel: Ref<string>
  defaultAssistantProfileId: Ref<string>
  defaultTeamProfileId: Ref<string>
  defaultToolConfig: Ref<UiToolConfigPayload>
  toolConfig: Ref<UiToolConfigPayload>
  toolsEnabled: Ref<boolean>
  applyConversationBinding: (
    binding: Partial<AssistantConversationBinding> | null | undefined
  ) => void
  applyProfilePreset: (profile: AssistantProfileOption) => void
  getAssistantProfileOption: (profileId: string) => AssistantProfileOption | null
  getTeamProfileOption: (profileId: string) => TeamProfileOption | null
  handleToggleTeamMode: (enabled: boolean) => Promise<void>
  resetSessionSettings: () => void
  setAssistantSelectedModel: (value: string, options?: { persist?: boolean }) => void
  setContextMode: (contextMode: AssistantContextMode) => void
  setProfileId: (profileId: string) => void
  setRunMode: (runMode: 'assistant' | 'team') => void
  setTeamProfileId: (teamProfileId: string) => void
  toConversationBinding: (extras?: {
    browserShellDirectWriteEnabled?: boolean
    browserShellSessionId?: string | null
    selectedModel?: string | null
    toolsEnabled?: boolean
    toolConfig?: UiToolConfigPayload | null
  }) => AssistantConversationBinding
}) => {
  const assistantProfileRegistryReady = ref(false)
  const isHydratingConversationBinding = ref(false)
  const conversationBindingReadyId = ref<string | null>(null)
  let conversationBindingSaveTimer: ReturnType<typeof setTimeout> | null = null

  const buildProfileToolConfigDefault = (
    profile: AssistantProfileOption,
    enabledOverride?: boolean
  ) => {
    const enabled =
      typeof enabledOverride === 'boolean' ? enabledOverride : profile.defaultToolsEnabled === true
    return {
      ...params.defaultToolConfig.value,
      enabled,
      selection_strategy:
        profile.defaultToolSelectionStrategy || params.defaultToolConfig.value.selection_strategy,
      max_tools: Math.max(1, Math.floor(Number(profile.defaultMaxTools) || 1)),
      preselected_tools: normalizeToolIdList(profile.defaultPreselectedTools),
      disabled_tools: normalizeToolIdList(profile.defaultDisabledTools),
      manual_tools: normalizeToolIdList(profile.defaultManualTools),
    } as UiToolConfigPayload
  }

  const applyProfileModelDefault = (profile: AssistantProfileOption) => {
    const defaultModel =
      profile.defaultModel?.trim() || params.assistantGlobalDefaultModel.value.trim()
    params.setAssistantSelectedModel(defaultModel, { persist: false })
  }

  const applyProfileToolsDefault = (profile: AssistantProfileOption, enabledOverride?: boolean) => {
    const nextToolConfig = buildProfileToolConfigDefault(profile, enabledOverride)
    params.toolsEnabled.value = nextToolConfig.enabled
    params.toolConfig.value = nextToolConfig
  }

  const handleAssistantProfileChange = (profileId: string) => {
    const normalizedProfileId = profileId.trim()
    if (!normalizedProfileId) return

    const profile = params.getAssistantProfileOption(normalizedProfileId)
    if (profile) {
      params.applyProfilePreset(profile)
      applyProfileModelDefault(profile)
      applyProfileToolsDefault(profile)
      params.setTeamProfileId('')
      void params.handleToggleTeamMode(false)
      return
    }

    const teamProfile = params.getTeamProfileOption(normalizedProfileId)
    if (teamProfile) {
      params.setTeamProfileId(teamProfile.id)
      params.setRunMode('team')
      void params.handleToggleTeamMode(true)
      return
    }

    params.setProfileId(normalizedProfileId)
  }

  const handleAssistantContextModeChange = (
    mode: 'claude-like' | 'codex-like' | 'sentinel-like'
  ) => {
    params.setContextMode(mode)
  }

  const applyConversationBindingState = (binding: AssistantConversationBinding | null) => {
    params.applyConversationBinding(binding)
    params.bindBrowserShellSession(binding?.browserShellSessionId?.trim() || null)
    params.setBrowserShellDirectWriteEnabled(binding?.browserShellDirectWriteEnabled === true)
    const boundProfile = binding?.profileId
      ? params.getAssistantProfileOption(binding.profileId)
      : null
    const profileToolConfig = boundProfile
      ? buildProfileToolConfigDefault(
          boundProfile,
          typeof binding?.toolsEnabled === 'boolean' ? binding.toolsEnabled : undefined
        )
      : null

    if (binding?.selectedModel) {
      params.setAssistantSelectedModel(binding.selectedModel, { persist: false })
    } else if (boundProfile) {
      applyProfileModelDefault(boundProfile)
    }

    if (binding?.toolConfig) {
      const nextToolConfig = normalizeUiToolConfigPayload(
        binding.toolConfig,
        profileToolConfig || params.toolConfig.value
      )
      params.toolsEnabled.value = nextToolConfig.enabled
      params.toolConfig.value = nextToolConfig
    } else if (profileToolConfig) {
      params.toolsEnabled.value = profileToolConfig.enabled
      params.toolConfig.value = profileToolConfig
    } else if (typeof binding?.toolsEnabled === 'boolean') {
      params.toolsEnabled.value = binding.toolsEnabled
      params.toolConfig.value = {
        ...params.toolConfig.value,
        enabled: binding.toolsEnabled,
      } as UiToolConfigPayload
    }
  }

  const applyDefaultAssistantProfile = () => {
    params.resetSessionSettings()
    params.bindBrowserShellSession(null)
    params.setBrowserShellDirectWriteEnabled(false)
    const defaultProfileId = params.defaultAssistantProfileId.value.trim()
    if (!defaultProfileId) return
    const defaultProfile = params.getAssistantProfileOption(defaultProfileId)
    if (defaultProfile) {
      params.applyProfilePreset(defaultProfile)
      params.setTeamProfileId(params.defaultTeamProfileId.value.trim())
      applyProfileModelDefault(defaultProfile)
      applyProfileToolsDefault(defaultProfile)
      return
    }
    params.setProfileId(defaultProfileId)
  }

  const loadConversationBinding = async (targetConversationId: string | null) => {
    if (!targetConversationId) {
      conversationBindingReadyId.value = null
      applyDefaultAssistantProfile()
      return
    }

    isHydratingConversationBinding.value = true
    conversationBindingReadyId.value = null
    try {
      const binding = await invoke<AssistantConversationBinding | null>(
        'get_ai_conversation_binding',
        {
          conversationId: targetConversationId,
        }
      )
      if (params.conversationId.value !== targetConversationId) return
      if (binding) {
        applyConversationBindingState(binding)
      } else {
        applyDefaultAssistantProfile()
      }
      conversationBindingReadyId.value = targetConversationId
    } catch (error) {
      console.warn('[useAgentConversationBinding] Failed to load conversation binding:', error)
      if (params.conversationId.value === targetConversationId) {
        applyDefaultAssistantProfile()
        conversationBindingReadyId.value = targetConversationId
      }
    } finally {
      if (params.conversationId.value === targetConversationId) {
        isHydratingConversationBinding.value = false
      }
    }
  }

  const buildCurrentConversationBinding = () =>
    params.toConversationBinding({
      browserShellDirectWriteEnabled: params.currentBrowserShellDirectWriteEnabled.value,
      browserShellSessionId: params.currentBrowserShellSessionId.value,
      selectedModel: params.assistantSelectedModel.value,
      toolsEnabled: params.toolsEnabled.value,
      toolConfig: params.toolConfig.value,
    })

  const persistConversationBinding = async (targetConversationId: string) => {
    const binding = buildCurrentConversationBinding()
    await invoke('save_ai_conversation_binding', {
      conversationId: targetConversationId,
      binding,
    })
  }

  const schedulePersistConversationBinding = () => {
    const targetConversationId = params.conversationId.value
    if (!targetConversationId) return
    if (isHydratingConversationBinding.value) return
    if (conversationBindingReadyId.value !== targetConversationId) return

    if (conversationBindingSaveTimer) {
      clearTimeout(conversationBindingSaveTimer)
    }

    conversationBindingSaveTimer = setTimeout(async () => {
      try {
        if (!params.conversationId.value || params.conversationId.value !== targetConversationId)
          return
        await persistConversationBinding(targetConversationId)
      } catch (error) {
        console.warn('[useAgentConversationBinding] Failed to persist conversation binding:', error)
      } finally {
        conversationBindingSaveTimer = null
      }
    }, CONVERSATION_BINDING_SAVE_DEBOUNCE_MS)
  }

  onUnmounted(() => {
    if (!conversationBindingSaveTimer) return
    clearTimeout(conversationBindingSaveTimer)
    conversationBindingSaveTimer = null
  })

  return {
    assistantProfileRegistryReady,
    buildCurrentConversationBinding,
    handleAssistantContextModeChange,
    handleAssistantProfileChange,
    loadConversationBinding,
    schedulePersistConversationBinding,
  }
}
