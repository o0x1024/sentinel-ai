import { defineComponent, h, nextTick } from 'vue'
import { mount } from '@vue/test-utils'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import MessageFlow from './MessageFlow.vue'
import type { AgentMessage } from '@/types/agent'

vi.mock('vue-i18n', () => ({
  useI18n: () => ({
    t: (key: string, params?: Record<string, unknown>) => {
      if (key === 'agent.loadMoreMessages') {
        return `load more ${String(params?.count ?? '')}`.trim()
      }
      return key
    },
  }),
}))

const messageStub = defineComponent({
  props: {
    message: {
      type: Object,
      required: true,
    },
  },
  setup(props) {
    return () => h('div', { class: 'message-block-stub' }, String((props.message as AgentMessage).content || ''))
  },
})

const sampleMessages: AgentMessage[] = [
  {
    id: 'msg-1',
    type: 'final',
    content: 'first message',
    timestamp: 1,
  },
  {
    id: 'msg-2',
    type: 'final',
    content: 'second message',
    timestamp: 2,
  },
]

describe('MessageFlow', () => {
  let scrollIntoViewSpy: ReturnType<typeof vi.fn>
  let resizeObserverDisconnectSpy: ReturnType<typeof vi.fn>

  beforeEach(() => {
    vi.useFakeTimers()
    scrollIntoViewSpy = vi.fn()
    resizeObserverDisconnectSpy = vi.fn()
    Object.defineProperty(HTMLElement.prototype, 'scrollIntoView', {
      configurable: true,
      value: scrollIntoViewSpy,
    })
    vi.stubGlobal('ResizeObserver', class {
      observe = vi.fn()
      disconnect = resizeObserverDisconnectSpy
    })
  })

  afterEach(() => {
    vi.runOnlyPendingTimers()
    vi.useRealTimers()
    vi.unstubAllGlobals()
    vi.restoreAllMocks()
  })

  it('focuses the target message, emits an event, and clears the pulse animation after the timeout', async () => {
    const wrapper = mount(MessageFlow, {
      props: {
        messages: sampleMessages,
        focusedMessageId: null,
      },
      global: {
        stubs: {
          MessageBlock: messageStub,
        },
      },
      attachTo: document.body,
    })

    await nextTick()
    await wrapper.setProps({ focusedMessageId: 'msg-2' })
    await nextTick()
    await nextTick()

    const focused = wrapper.get('#agent-message-msg-2')
    const focusedClass = focused.attributes('class')
    expect(scrollIntoViewSpy).toHaveBeenCalledWith({ behavior: 'smooth', block: 'center' })
    expect(wrapper.emitted('message-focused')?.[0]).toEqual(['msg-2'])
    expect(focusedClass).toContain('ring-2')
    expect(focusedClass).toContain('animate-memory-focus-pulse')

    vi.advanceTimersByTime(1600)
    await nextTick()

    expect(wrapper.get('#agent-message-msg-2').attributes('class')).not.toContain('animate-memory-focus-pulse')

    wrapper.unmount()
  })

  it('does not emit focus when the target message cannot be found', async () => {
    const wrapper = mount(MessageFlow, {
      props: {
        messages: sampleMessages,
        focusedMessageId: null,
      },
      global: {
        stubs: {
          MessageBlock: messageStub,
        },
      },
      attachTo: document.body,
    })

    await nextTick()
    await wrapper.setProps({ focusedMessageId: 'missing-message' })
    await nextTick()

    expect(scrollIntoViewSpy).not.toHaveBeenCalled()
    expect(wrapper.emitted('message-focused')).toBeUndefined()

    wrapper.unmount()
  })
})
