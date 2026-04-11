import { invoke } from '@tauri-apps/api/core'
import { computed, ref } from 'vue'
import type { AssistantContextMode, AssistantRunMode } from './agentDraftTypes'

export interface AssistantProfileOption {
  id: string
  label: string
  description: string
  defaultModel?: string | null
  defaultRagEnabled?: boolean | null
  defaultWebSearchEnabled?: boolean | null
  defaultToolsEnabled?: boolean | null
  defaultTenthManEnabled?: boolean | null
  defaultToolSelectionStrategy?: string | null
  defaultMaxTools?: number | null
  defaultFixedTools?: string[] | null
  defaultDisabledTools?: string[] | null
  defaultManualTools?: string[] | null
  defaultTeamOrchestrationPresetId?: string | null
  defaultTeamRecoveryPresetId?: string | null
  contextMode: AssistantContextMode
  runMode: AssistantRunMode
}

const profileOptions = ref<AssistantProfileOption[]>([])
const isLoadingAssistantProfiles = ref(false)
const isSavingAssistantProfiles = ref(false)
const defaultAssistantProfileId = ref('')
const isLoadingDefaultAssistantProfile = ref(false)
const isSavingDefaultAssistantProfile = ref(false)
let loadAssistantProfilesPromise: Promise<void> | null = null
let loadDefaultAssistantProfilePromise: Promise<void> | null = null

const TOOL_SELECTION_STRATEGIES = new Set(['Keyword', 'LLM', 'Hybrid', 'Manual', 'All'])

const normalizeToolIds = (items: string[] | null | undefined) => {
  const seen = new Set<string>()
  const out: string[] = []
  for (const item of Array.isArray(items) ? items : []) {
    const normalized = item.trim().replace(/::/g, '__')
    if (!normalized || seen.has(normalized)) continue
    seen.add(normalized)
    out.push(normalized)
  }
  return out
}

const normalizeAssistantProfile = (profile: AssistantProfileOption): AssistantProfileOption => ({
  ...profile,
  id: profile.id.trim(),
  label: profile.label.trim(),
  description: profile.description.trim(),
  defaultModel: profile.defaultModel?.trim() || null,
  defaultRagEnabled: profile.defaultRagEnabled === true,
  defaultWebSearchEnabled: profile.defaultWebSearchEnabled === true,
  defaultToolsEnabled: profile.defaultToolsEnabled === true,
  defaultTenthManEnabled: profile.defaultTenthManEnabled === true,
  defaultToolSelectionStrategy: TOOL_SELECTION_STRATEGIES.has(profile.defaultToolSelectionStrategy || '')
    ? profile.defaultToolSelectionStrategy
    : 'Keyword',
  defaultMaxTools: Math.max(1, Math.floor(Number(profile.defaultMaxTools) || 1)),
  defaultFixedTools: normalizeToolIds(profile.defaultFixedTools),
  defaultDisabledTools: normalizeToolIds(profile.defaultDisabledTools),
  defaultManualTools: normalizeToolIds(profile.defaultManualTools),
  defaultTeamOrchestrationPresetId: profile.defaultTeamOrchestrationPresetId?.trim() || null,
  defaultTeamRecoveryPresetId: profile.defaultTeamRecoveryPresetId?.trim() || null,
})

export const useAssistantProfiles = () => {
  const loadAssistantProfiles = async (force = false) => {
    if (!force && profileOptions.value.length > 0) {
      return
    }
    if (loadAssistantProfilesPromise) {
      await loadAssistantProfilesPromise
      return
    }

    isLoadingAssistantProfiles.value = true
    loadAssistantProfilesPromise = invoke<AssistantProfileOption[]>('list_assistant_profiles')
      .then(profiles => {
        profileOptions.value = profiles.map(normalizeAssistantProfile)
      })
      .finally(() => {
        isLoadingAssistantProfiles.value = false
        loadAssistantProfilesPromise = null
      })
    await loadAssistantProfilesPromise
  }

  const profileOptionsById = computed(() => {
    const out = new Map<string, AssistantProfileOption>()
    for (const option of profileOptions.value) {
      out.set(option.id, option)
    }
    return out
  })

  const getAssistantProfileOption = (profileId: string) =>
    profileOptionsById.value.get(profileId) || null

  const loadDefaultAssistantProfile = async (force = false) => {
    if (!force && defaultAssistantProfileId.value.trim()) {
      return
    }
    if (loadDefaultAssistantProfilePromise) {
      await loadDefaultAssistantProfilePromise
      return
    }

    isLoadingDefaultAssistantProfile.value = true
    loadDefaultAssistantProfilePromise = invoke<string>('get_default_assistant_profile_id')
      .then(profileId => {
        defaultAssistantProfileId.value = profileId
      })
      .finally(() => {
        isLoadingDefaultAssistantProfile.value = false
        loadDefaultAssistantProfilePromise = null
      })
    await loadDefaultAssistantProfilePromise
  }

  const saveAssistantProfiles = async (profiles: AssistantProfileOption[]) => {
    isSavingAssistantProfiles.value = true
    try {
      const normalizedProfiles = profiles.map(normalizeAssistantProfile)
      await invoke('save_assistant_profiles', { profiles: normalizedProfiles })
      profileOptions.value = normalizedProfiles.map(profile => ({ ...profile }))
      if (!profileOptions.value.some(profile => profile.id === defaultAssistantProfileId.value)) {
        defaultAssistantProfileId.value = profileOptions.value[0]?.id || 'assistant.default'
      }
    } finally {
      isSavingAssistantProfiles.value = false
    }
  }

  const saveDefaultAssistantProfile = async (profileId: string) => {
    isSavingDefaultAssistantProfile.value = true
    try {
      await invoke('save_default_assistant_profile_id', { profileId })
      defaultAssistantProfileId.value = profileId
    } finally {
      isSavingDefaultAssistantProfile.value = false
    }
  }

  return {
    defaultAssistantProfileId,
    getAssistantProfileOption,
    isLoadingDefaultAssistantProfile,
    isLoadingAssistantProfiles,
    isSavingDefaultAssistantProfile,
    isSavingAssistantProfiles,
    loadDefaultAssistantProfile,
    loadAssistantProfiles,
    profileOptions,
    saveAssistantProfiles,
    saveDefaultAssistantProfile,
  }
}
