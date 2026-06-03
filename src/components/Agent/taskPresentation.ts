import type { AgentTaskStatus } from '@/types/agentTask'
import {
  getAgentTaskStatusTone,
  getTaskToneContentClass,
  getTaskToneIndicatorClass,
  getTaskToneRowClass,
} from './taskStatusPresentation'

export const getAgentTaskIndicator = (status: AgentTaskStatus): string => {
  switch (status) {
    case 'in_progress':
      return '→'
    case 'completed':
      return '✓'
    case 'failed':
      return '!'
    case 'blocked':
      return '…'
    case 'cancelled':
      return '×'
    default:
      return '○'
  }
}

export const getAgentTaskIndicatorClass = (status: AgentTaskStatus): string => {
  return getTaskToneIndicatorClass(getAgentTaskStatusTone(status))
}

export const getAgentTaskContentClass = (status: AgentTaskStatus): string => {
  switch (status) {
    case 'completed':
      return getTaskToneContentClass(getAgentTaskStatusTone(status), { struckThrough: true })
    case 'failed':
      return getTaskToneContentClass(getAgentTaskStatusTone(status))
    case 'blocked':
      return getTaskToneContentClass(getAgentTaskStatusTone(status))
    case 'cancelled':
      return getTaskToneContentClass(getAgentTaskStatusTone(status), { struckThrough: true })
    default:
      return getTaskToneContentClass(getAgentTaskStatusTone(status))
  }
}

export const getAgentTaskRowClass = (status: AgentTaskStatus): string => {
  return getTaskToneRowClass(getAgentTaskStatusTone(status))
}
