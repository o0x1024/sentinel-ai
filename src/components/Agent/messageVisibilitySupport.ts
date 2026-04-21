import type { AgentMessage } from '@/types/agent'

type ToolCallVisibilityInput = {
  messageType: AgentMessage['type']
  hasContent: boolean
  hasToolArgs: boolean
  hasToolResult: boolean
  hasToolCallId: boolean
  isSkillsTool: boolean
  isAskUserQuestionTool: boolean
  isWebSearchTool: boolean
}

type RegularMessageVisibilityInput = {
  messageType: AgentMessage['type']
  hasContent: boolean
  hasToolArgs: boolean
  isAgentTaskUpdate: boolean
}

export const shouldShowDefaultToolCallPanel = (
  input: ToolCallVisibilityInput,
): boolean => {
  if (input.messageType !== 'tool_call') return false
  if (input.isSkillsTool) return false
  if (input.isAskUserQuestionTool) return false
  if (input.isWebSearchTool) return false

  return input.hasContent || input.hasToolArgs || input.hasToolResult || input.hasToolCallId
}

export const shouldShowRegularMessageBlock = (
  input: RegularMessageVisibilityInput,
): boolean => {
  if (input.isAgentTaskUpdate) return false
  if (input.messageType === 'tool_call') return false

  if (input.messageType === 'tool_result') {
    return input.hasContent || input.hasToolArgs
  }

  return input.hasContent
}
