import { describe, expect, it } from 'vitest'

import {
  shouldShowDefaultToolCallPanel,
  shouldShowRegularMessageBlock,
} from './messageVisibilitySupport'

describe('messageVisibilitySupport', () => {
  it('shows the default tool call panel for tasks tool messages', () => {
    expect(
      shouldShowDefaultToolCallPanel({
        messageType: 'tool_call',
        hasContent: true,
        hasToolArgs: true,
        hasToolResult: false,
        hasToolCallId: true,
        isSkillsTool: false,
        isAskUserQuestionTool: false,
        isWebSearchTool: false,
      }),
    ).toBe(true)
  })

  it('keeps specialized tool renderers out of the default tool call panel', () => {
    expect(
      shouldShowDefaultToolCallPanel({
        messageType: 'tool_call',
        hasContent: true,
        hasToolArgs: true,
        hasToolResult: true,
        hasToolCallId: true,
        isSkillsTool: false,
        isAskUserQuestionTool: false,
        isWebSearchTool: true,
      }),
    ).toBe(false)
  })

  it('allows fallback tool_result messages to render as regular blocks', () => {
    expect(
      shouldShowRegularMessageBlock({
        messageType: 'tool_result',
        hasContent: true,
        hasToolArgs: false,
        isAgentTaskUpdate: false,
      }),
    ).toBe(true)
  })

  it('hides agent task update system messages from the regular block renderer', () => {
    expect(
      shouldShowRegularMessageBlock({
        messageType: 'system',
        hasContent: true,
        hasToolArgs: false,
        isAgentTaskUpdate: true,
      }),
    ).toBe(false)
  })
})
