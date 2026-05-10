import type { AssistantConversationBinding } from './agentDraftTypes'
import type { AssistantProfileOption, TeamProfileOption } from './assistantProfiles'
import { buildBaseAssistantConversationBinding } from './useAssistantSessionSettings'

export const buildNewAssistantConversationBinding = (params: {
  profileId: string
  runMode: 'assistant' | 'team'
  teamProfileId: string
  defaultTeamProfileId: string
  teamProfileOptions: TeamProfileOption[]
  workingDirectoryOverride: string
  getAssistantProfileOption: (profileId: string) => AssistantProfileOption | null
}): AssistantConversationBinding => {
  const profileId = params.profileId.trim()
  const profile = params.getAssistantProfileOption(profileId)
  if (!profile) {
    throw new Error(`Assistant profile is unavailable: ${profileId}`)
  }

  const binding = buildBaseAssistantConversationBinding({
    profile,
    workingDirectoryOverride: params.workingDirectoryOverride,
  })
  if (params.runMode !== 'team') {
    return binding
  }

  const teamProfileId =
    params.teamProfileId.trim() ||
    params.defaultTeamProfileId.trim() ||
    params.teamProfileOptions[0]?.id ||
    ''
  if (!teamProfileId) {
    throw new Error('Team mode requires a Team Profile.')
  }

  return {
    ...binding,
    runMode: 'team',
    teamProfileId,
  }
}
