import { flushPromises, mount } from '@vue/test-utils'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { defineComponent, h, ref, type Ref } from 'vue'
import { useAgentEvents } from './useAgentEvents'
import type { UseAgentEventsReturn } from './useAgentEventTypes'

const eventListeners: Record<string, Array<(event: { payload: any }) => void>> = {}

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}))

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(async (eventName: string, handler: (event: { payload: any }) => void) => {
    eventListeners[eventName] ||= []
    eventListeners[eventName].push(handler)
    return () => {
      eventListeners[eventName] = (eventListeners[eventName] || []).filter(item => item !== handler)
    }
  }),
}))

const emitTauriEvent = async (eventName: string, payload: any) => {
  for (const handler of eventListeners[eventName] || []) {
    handler({ payload })
  }
  await flushPromises()
}

describe('useAgentEvents', () => {
  let api: UseAgentEventsReturn | null = null
  let executionId: Ref<string>

  beforeEach(() => {
    Object.keys(eventListeners).forEach((key) => {
      delete eventListeners[key]
    })
    executionId = ref('conversation-1')
    api = null
  })

  afterEach(() => {
    api?.stopListening()
    api = null
  })

  const mountHarness = async () => {
    const Harness = defineComponent({
      setup() {
        api = useAgentEvents(executionId)
        return () => h('div')
      },
    })
    const wrapper = mount(Harness)
    await flushPromises()
    return wrapper
  }

  it('accepts a newer generation after the previous execution has finished', async () => {
    const wrapper = await mountHarness()

    await emitTauriEvent('agent:user_message', {
      execution_id: 'conversation-1',
      generation: 1,
      message_id: 'user-1',
      content: 'first message',
      timestamp: 1000,
    })
    expect(api?.messages.value.map(message => message.content)).toEqual(['first message'])

    await emitTauriEvent('agent:execution_finished', {
      execution_id: 'conversation-1',
      generation: 1,
      outcome: 'succeeded',
      success: true,
    })
    expect(api?.isExecuting.value).toBe(false)

    await emitTauriEvent('agent:user_message', {
      execution_id: 'conversation-1',
      generation: 2,
      message_id: 'user-2',
      content: 'resent message',
      timestamp: 2000,
    })

    expect(api?.messages.value.map(message => message.content)).toEqual([
      'first message',
      'resent message',
    ])

    wrapper.unmount()
  })

  it('rejects older generation events after a newer generation is active', async () => {
    const wrapper = await mountHarness()

    await emitTauriEvent('agent:user_message', {
      execution_id: 'conversation-1',
      generation: 2,
      message_id: 'user-2',
      content: 'current message',
      timestamp: 2000,
    })

    await emitTauriEvent('agent:user_message', {
      execution_id: 'conversation-1',
      generation: 1,
      message_id: 'user-1',
      content: 'stale message',
      timestamp: 1000,
    })

    expect(api?.messages.value.map(message => message.content)).toEqual(['current message'])

    wrapper.unmount()
  })
})
