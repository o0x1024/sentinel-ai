import { describe, expect, it } from 'vitest'
import type { AgentMessage } from '@/types/agent'
import {
  extractMemoryIdsFromMessage,
  parseMemoryToolPayload,
  resolveFocusedMemoryMessageId,
} from './memoryFocusSupport'

const buildMemoryMessage = (toolResult: unknown, id = 'tool-msg'): AgentMessage => ({
  id,
  type: 'tool_call',
  content: 'Memory tool',
  timestamp: Date.now(),
  metadata: {
    tool_name: 'memory',
    tool_result: toolResult as string,
  },
})

describe('memoryFocusSupport', () => {
  it('parses nested text payloads returned by tool messages', () => {
    expect(parseMemoryToolPayload([
      {
        type: 'text',
        text: JSON.stringify({
          items: [{ id: 'memory-1' }],
        }),
      },
    ])).toEqual({
      items: [{ id: 'memory-1' }],
    })
  })

  it('extracts ids from both store and retrieve memory results', () => {
    const message = buildMemoryMessage(JSON.stringify({
      store: { memory_id: 'memory-store-1' },
      items: [
        { id: 'memory-hit-1' },
        { id: 'memory-hit-2' },
      ],
    }))

    expect(extractMemoryIdsFromMessage(message)).toEqual([
      'memory-store-1',
      'memory-hit-1',
      'memory-hit-2',
    ])
  })

  it('returns the most recent message that references the target memory id', () => {
    const messages: AgentMessage[] = [
      buildMemoryMessage(JSON.stringify({
        items: [{ id: 'memory-2' }],
      }), 'older'),
      buildMemoryMessage(JSON.stringify({
        store: { memory_id: 'memory-1' },
      }), 'middle'),
      buildMemoryMessage(JSON.stringify({
        items: [{ id: 'memory-1' }],
      }), 'newer'),
    ]

    expect(resolveFocusedMemoryMessageId(messages, 'memory-1')).toBe('newer')
    expect(resolveFocusedMemoryMessageId(messages, 'memory-2')).toBe('older')
    expect(resolveFocusedMemoryMessageId(messages, 'missing-memory')).toBeNull()
  })

  it('ignores non-memory tool messages', () => {
    const message: AgentMessage = {
      id: 'web-search-msg',
      type: 'tool_call',
      content: 'Web Search',
      timestamp: Date.now(),
      metadata: {
        tool_name: 'web_search',
        tool_result: JSON.stringify({ items: [{ id: 'memory-1' }] }),
      },
    }

    expect(extractMemoryIdsFromMessage(message)).toEqual([])
  })
})
