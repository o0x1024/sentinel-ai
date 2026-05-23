import { flushPromises, mount } from '@vue/test-utils'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { defineComponent } from 'vue'
import AskUserQuestionModal from './AskUserQuestionModal.vue'

const markdownRendererStub = defineComponent({
  props: {
    content: {
      type: String,
      default: '',
    },
  },
  template: '<div class="markdown-renderer-stub">{{ content }}</div>',
})

const pendingRequest = {
  id: 'question-1',
  execution_id: 'session-1',
  questions: [
    {
      header: 'Mode',
      question: 'How should the assistant continue?',
      options: [
        {
          label: 'Continue',
          description: 'Continue with the current plan.',
        },
        {
          label: 'Stop',
          description: 'Stop and wait for more detail.',
        },
      ],
    },
  ],
  timestamp: 1,
  timeout_secs: 120,
  timeout_policy: 'use_default',
  default_answers: {},
  expires_at: 121,
}

const mountModal = () => mount(AskUserQuestionModal, {
  attachTo: document.body,
  global: {
    stubs: {
      MarkdownRenderer: markdownRendererStub,
    },
  },
})

describe('AskUserQuestionModal', () => {
  const mockInvoke = globalThis.testUtils.mockInvoke
  const mockListen = globalThis.testUtils.mockListen
  let consoleErrorSpy: ReturnType<typeof vi.spyOn>

  beforeEach(() => {
    mockInvoke.mockReset()
    mockListen.mockReset()
    consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {})
  })

  afterEach(() => {
    document.body.innerHTML = ''
    consoleErrorSpy.mockRestore()
  })

  it('still renders pending questions when event listener registration fails', async () => {
    mockInvoke.mockImplementation(command => {
      if (command === 'get_pending_ask_user_questions') {
        return Promise.resolve([pendingRequest])
      }
      return Promise.resolve(null)
    })
    mockListen.mockRejectedValue(new Error('event bridge unavailable'))

    const wrapper = mountModal()
    await flushPromises()

    expect(mockInvoke).toHaveBeenCalledWith('get_pending_ask_user_questions')
    expect(document.body.textContent).toContain('需要你的选择')
    expect(document.body.textContent).toContain('How should the assistant continue?')

    wrapper.unmount()
  })

  it('renders a question pushed through the event channel', async () => {
    let listener: ((event: { payload: unknown }) => void) | null = null
    const unlisten = vi.fn()
    mockInvoke.mockResolvedValue([])
    mockListen.mockImplementation(async (_eventName, callback) => {
      listener = callback
      return unlisten
    })

    const wrapper = mountModal()
    await flushPromises()

    listener?.({ payload: pendingRequest })
    await flushPromises()

    expect(document.body.textContent).toContain('需要你的选择')
    expect(document.body.textContent).toContain('Continue')

    wrapper.unmount()
    expect(unlisten).toHaveBeenCalled()
  })
})
