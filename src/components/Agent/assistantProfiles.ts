import { invoke } from '@tauri-apps/api/core'
import { computed, ref } from 'vue'
import type { AssistantContextMode, AssistantRunMode } from './agentDraftTypes'

export interface AssistantProfileOption {
  id: string
  label: string
  description: string
  teamRole?: 'assistant' | 'commander' | 'solver' | 'observer'
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
  defaultTeamProfileId?: string | null
  contextMode: AssistantContextMode
  runMode: AssistantRunMode
}

export interface TeamProfileOption {
  id: string
  name: string
  description: string
  commanderProfileId: string
  solverProfileIds: string[]
  observerProfileId: string
  defaultModel?: string | null
  contextMode: AssistantContextMode
  memoryPolicy: Record<string, any>
  toolPolicyMatrix: Record<string, any>
  harnessPolicy: Record<string, any>
  concurrencyPolicy: Record<string, any>
  safetyPolicy: Record<string, any>
}

const profileOptions = ref<AssistantProfileOption[]>([])
const isLoadingAssistantProfiles = ref(false)
const isSavingAssistantProfiles = ref(false)
const defaultAssistantProfileId = ref('')
const teamProfileOptions = ref<TeamProfileOption[]>([])
const defaultTeamProfileId = ref('')
const isLoadingDefaultAssistantProfile = ref(false)
const isSavingDefaultAssistantProfile = ref(false)
const isLoadingTeamProfiles = ref(false)
const isSavingTeamProfiles = ref(false)
const isLoadingDefaultTeamProfile = ref(false)
const isSavingDefaultTeamProfile = ref(false)
let loadAssistantProfilesPromise: Promise<void> | null = null
let loadDefaultAssistantProfilePromise: Promise<void> | null = null
let loadTeamProfilesPromise: Promise<void> | null = null
let loadDefaultTeamProfilePromise: Promise<void> | null = null

const TOOL_SELECTION_STRATEGIES = new Set(['Keyword', 'LLM', 'Hybrid', 'Manual', 'All'])
const DEFAULT_TEAM_ROLE_TOOLS: Record<string, string[]> = {
  commander: ['ask_user_question'],
  solver: ['interactive_shell', 'shell', 'file_read', 'grep', 'http_request', 'web_search'],
  observer: ['tenth_man_review'],
}

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
  teamRole: ['assistant', 'commander', 'solver', 'observer'].includes(profile.teamRole || '')
    ? profile.teamRole
    : 'assistant',
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
  defaultTeamProfileId: profile.defaultTeamProfileId?.trim() || null,
})

const normalizeJsonObject = (value: unknown): Record<string, any> =>
  value && typeof value === 'object' && !Array.isArray(value)
    ? value as Record<string, any>
    : {}

const normalizeTeamToolPolicyMatrix = (value: unknown): Record<string, any> => {
  const matrix = normalizeJsonObject(value)
  return Object.fromEntries(
    Object.entries(DEFAULT_TEAM_ROLE_TOOLS).map(([role, defaultTools]) => {
      const rolePolicy = normalizeJsonObject(matrix[role])
      const tools = normalizeToolIds(
        rolePolicy.tools || rolePolicy.allowed || rolePolicy.manualTools || rolePolicy.fixedTools,
      )
      return [role, { tools: tools.length > 0 ? tools : defaultTools }]
    }),
  )
}

const normalizeTeamProfile = (profile: TeamProfileOption): TeamProfileOption => ({
  ...profile,
  id: profile.id.trim(),
  name: profile.name.trim(),
  description: profile.description.trim(),
  commanderProfileId: profile.commanderProfileId.trim(),
  solverProfileIds: Array.from(new Set((profile.solverProfileIds || []).map(id => id.trim()).filter(Boolean))),
  observerProfileId: profile.observerProfileId.trim(),
  defaultModel: profile.defaultModel?.trim() || null,
  contextMode: profile.contextMode || 'claude-like',
  memoryPolicy: normalizeJsonObject(profile.memoryPolicy),
  toolPolicyMatrix: normalizeTeamToolPolicyMatrix(profile.toolPolicyMatrix),
  harnessPolicy: normalizeJsonObject(profile.harnessPolicy),
  concurrencyPolicy: normalizeJsonObject(profile.concurrencyPolicy),
  safetyPolicy: normalizeJsonObject(profile.safetyPolicy),
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

  const teamProfileOptionsById = computed(() => {
    const out = new Map<string, TeamProfileOption>()
    for (const option of teamProfileOptions.value) {
      out.set(option.id, option)
    }
    return out
  })

  const getTeamProfileOption = (profileId: string) =>
    teamProfileOptionsById.value.get(profileId) || null

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

  const loadTeamProfiles = async (force = false) => {
    if (!force && teamProfileOptions.value.length > 0) {
      return
    }
    if (loadTeamProfilesPromise) {
      await loadTeamProfilesPromise
      return
    }

    isLoadingTeamProfiles.value = true
    loadTeamProfilesPromise = invoke<TeamProfileOption[]>('list_team_profiles')
      .then(profiles => {
        teamProfileOptions.value = profiles.map(normalizeTeamProfile)
      })
      .finally(() => {
        isLoadingTeamProfiles.value = false
        loadTeamProfilesPromise = null
      })
    await loadTeamProfilesPromise
  }

  const saveTeamProfiles = async (profiles: TeamProfileOption[]) => {
    isSavingTeamProfiles.value = true
    try {
      const normalizedProfiles = profiles.map(normalizeTeamProfile)
      await invoke('save_team_profiles', { profiles: normalizedProfiles })
      teamProfileOptions.value = normalizedProfiles.map(profile => ({ ...profile }))
      if (!teamProfileOptions.value.some(profile => profile.id === defaultTeamProfileId.value)) {
        defaultTeamProfileId.value = teamProfileOptions.value[0]?.id || 'team.profile.default'
      }
    } finally {
      isSavingTeamProfiles.value = false
    }
  }

  const loadDefaultTeamProfile = async (force = false) => {
    if (!force && defaultTeamProfileId.value.trim()) {
      return
    }
    if (loadDefaultTeamProfilePromise) {
      await loadDefaultTeamProfilePromise
      return
    }

    isLoadingDefaultTeamProfile.value = true
    loadDefaultTeamProfilePromise = invoke<string>('get_default_team_profile_id')
      .then(profileId => {
        defaultTeamProfileId.value = profileId
      })
      .finally(() => {
        isLoadingDefaultTeamProfile.value = false
        loadDefaultTeamProfilePromise = null
      })
    await loadDefaultTeamProfilePromise
  }

  const saveDefaultTeamProfile = async (profileId: string) => {
    isSavingDefaultTeamProfile.value = true
    try {
      await invoke('save_default_team_profile_id', { profileId })
      defaultTeamProfileId.value = profileId
    } finally {
      isSavingDefaultTeamProfile.value = false
    }
  }

  return {
    defaultAssistantProfileId,
    defaultTeamProfileId,
    getAssistantProfileOption,
    getTeamProfileOption,
    isLoadingDefaultAssistantProfile,
    isLoadingAssistantProfiles,
    isLoadingDefaultTeamProfile,
    isLoadingTeamProfiles,
    isSavingDefaultAssistantProfile,
    isSavingAssistantProfiles,
    isSavingDefaultTeamProfile,
    isSavingTeamProfiles,
    loadDefaultAssistantProfile,
    loadAssistantProfiles,
    loadDefaultTeamProfile,
    loadTeamProfiles,
    profileOptions,
    teamProfileOptions,
    saveAssistantProfiles,
    saveDefaultAssistantProfile,
    saveDefaultTeamProfile,
    saveTeamProfiles,
  }
}
