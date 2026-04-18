import { describe, expect, it } from 'vitest'
import {
  buildFocusedMessageQuery,
  clearFocusLocationQuery,
  readFocusLocationState,
} from './focusLocationSupport'

describe('focusLocationSupport', () => {
  it('reads memory and conversation focus state from route query', () => {
    expect(readFocusLocationState({
      conversationId: 'conv-1',
      memoryId: 'memory-1',
    })).toEqual({
      conversationId: 'conv-1',
      memoryId: 'memory-1',
      focusedMessageId: null,
    })

    expect(readFocusLocationState({
      conversation_id: 'conv-2',
      focusedMessageId: 'msg-9',
    })).toEqual({
      conversationId: 'conv-2',
      memoryId: null,
      focusedMessageId: 'msg-9',
    })
  })

  it('promotes a matched memory focus into a direct message focus', () => {
    const next = buildFocusedMessageQuery(
      {
        conversationId: 'conv-1',
        memoryId: 'memory-7',
        tab: 'assistant',
      },
      {
        memoryId: 'memory-7',
        messageId: 'msg-42',
      },
    )

    expect(next).toEqual({
      conversationId: 'conv-1',
      focusedMessageId: 'msg-42',
      tab: 'assistant',
    })
  })

  it('keeps the original query untouched when memory focus does not match', () => {
    const original = {
      conversationId: 'conv-1',
      memoryId: 'memory-7',
      tab: 'assistant',
    }

    expect(buildFocusedMessageQuery(original, {
      memoryId: 'memory-8',
      messageId: 'msg-42',
    })).toEqual(original)
  })

  it('clears both memory and message focus without touching other query params', () => {
    expect(clearFocusLocationQuery({
      conversationId: 'conv-1',
      memoryId: 'memory-7',
      focusedMessageId: 'msg-42',
      tab: 'assistant',
    })).toEqual({
      conversationId: 'conv-1',
      tab: 'assistant',
    })
  })
})
