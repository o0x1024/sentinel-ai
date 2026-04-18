import { describe, expect, it } from 'vitest'
import { buildFocusedMemoryToolsRoute, deriveFocusBannerState } from './focusBannerSupport'

describe('focusBannerSupport', () => {
  it('keeps the last memory id around after the route is promoted to a focused message id', () => {
    expect(deriveFocusBannerState({
      focusedMemoryId: null,
      focusedMessageId: 'msg-7',
      resolvedMessageId: 'msg-7',
      lastFocusedMemoryId: 'memory-3',
    })).toEqual({
      visible: true,
      memoryId: 'memory-3',
      messageId: 'msg-7',
      nextLastFocusedMemoryId: 'memory-3',
    })
  })

  it('prefers the active focused memory id over the historical one', () => {
    expect(deriveFocusBannerState({
      focusedMemoryId: 'memory-9',
      focusedMessageId: null,
      resolvedMessageId: 'msg-10',
      lastFocusedMemoryId: 'memory-3',
    })).toEqual({
      visible: true,
      memoryId: 'memory-9',
      messageId: 'msg-10',
      nextLastFocusedMemoryId: 'memory-9',
    })
  })

  it('hides the banner when no focus state exists', () => {
    expect(deriveFocusBannerState({
      focusedMemoryId: null,
      focusedMessageId: null,
      resolvedMessageId: null,
      lastFocusedMemoryId: null,
    })).toEqual({
      visible: false,
      memoryId: '',
      messageId: '',
      nextLastFocusedMemoryId: null,
    })
  })

  it('builds a deep link back to Tools only when a memory id exists', () => {
    expect(buildFocusedMemoryToolsRoute('memory-12')).toEqual({
      name: 'McpTools',
      query: {
        tab: 'builtin_tools',
        memoryId: 'memory-12',
      },
    })
    expect(buildFocusedMemoryToolsRoute('   ')).toBeNull()
  })
})
