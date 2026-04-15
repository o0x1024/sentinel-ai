import type { AssistantProfileOption } from '@/components/Agent/assistantProfiles'
import type { AgentListFilterOption } from '@/components/Settings/agentListItemSupport'

export type AssistantAgentListBadge = {
  label: string
  className: string
}

export function getAssistantAgentRunModeBadge(profile: AssistantProfileOption): AssistantAgentListBadge {
  if (profile.runMode === 'team') {
    return {
      label: 'Team',
      className: 'badge-secondary',
    }
  }

  return {
    label: 'Assistant',
    className: 'badge-ghost',
  }
}

export function getAssistantAgentModelBadge(profile: AssistantProfileOption): AssistantAgentListBadge {
  if (profile.defaultModel?.trim()) {
    return {
      label: '模型已覆盖',
      className: 'badge-info',
    }
  }

  return {
    label: '跟随全局模型',
    className: 'badge-outline',
  }
}

export function getAssistantAgentToolsBadge(profile: AssistantProfileOption): AssistantAgentListBadge {
  if (profile.defaultToolsEnabled) {
    return {
      label: 'Tools On',
      className: 'badge-primary',
    }
  }

  return {
    label: 'Tools Off',
    className: 'badge-outline',
  }
}

export function getAssistantAgentSecondarySummary(profile: AssistantProfileOption): string {
  const parts: string[] = []

  parts.push(
    profile.contextMode === 'codex-like'
      ? 'Codex-like 上下文'
      : profile.contextMode === 'sentinel-like'
        ? 'Sentinel-like 上下文'
        : 'Claude-like 上下文',
  )
  parts.push(profile.defaultModel?.trim() || '模型跟随全局')

  if (profile.runMode === 'team' && profile.defaultTeamOrchestrationPresetId) {
    parts.push(`Preset ${profile.defaultTeamOrchestrationPresetId}`)
  }

  return parts.join(' · ')
}

export const ASSISTANT_AGENT_LIST_FILTER_OPTIONS: AgentListFilterOption[] = [
  { key: 'default', label: '默认入口' },
  { key: 'team', label: 'Team' },
  { key: 'assistant', label: 'Assistant' },
  { key: 'model-override', label: '模型已覆盖' },
  { key: 'tools-on', label: 'Tools On' },
  { key: 'tools-off', label: 'Tools Off' },
]
