import { computed, type Ref } from 'vue'
import type { AssistantSessionSettings } from './agentDraftTypes'
import type { AssistantProfileOption, TeamProfileOption } from './assistantProfiles'

export interface AssistantAgentSwitchOption {
  value: string
  label: string
  description: 'Assistant' | 'Team'
}

export const useAssistantAgentSwitchOptions = (params: {
  assistantProfileOptions: Ref<AssistantProfileOption[]>
  teamProfileOptions: Ref<TeamProfileOption[]>
  assistantSessionSettings: Ref<AssistantSessionSettings>
  defaultTeamProfileId: Ref<string>
  isLoadingAssistantProfiles: Ref<boolean>
  isLoadingTeamProfiles: Ref<boolean>
}) => {
  const assistantAgentOptions = computed<AssistantAgentSwitchOption[]>(() => [
    ...params.assistantProfileOptions.value
      .filter(profile => profile.runMode !== 'team')
      .map(profile => ({
        value: profile.id,
        label: profile.label,
        description: 'Assistant' as const,
      })),
    ...params.teamProfileOptions.value.map(profile => ({
      value: profile.id,
      label: profile.name,
      description: 'Team' as const,
    })),
  ])

  const selectedAssistantAgentOptionId = computed(() => {
    if (params.assistantSessionSettings.value.runMode === 'team') {
      return (
        params.assistantSessionSettings.value.teamProfileId.trim() ||
        params.defaultTeamProfileId.value.trim() ||
        params.teamProfileOptions.value[0]?.id ||
        ''
      )
    }
    return params.assistantSessionSettings.value.profileId
  })

  const isLoadingAssistantAgentOptions = computed(
    () => params.isLoadingAssistantProfiles.value || params.isLoadingTeamProfiles.value
  )

  return {
    assistantAgentOptions,
    isLoadingAssistantAgentOptions,
    selectedAssistantAgentOptionId,
  }
}
