import { defineComponent, h, ref } from 'vue'
import { flushPromises, mount } from '@vue/test-utils'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import ProxyRepeater from './ProxyRepeater.vue'
import type { HttpExchangeRequest } from './http/model'
import type { RawReplayCommandResult } from './http/response'

vi.mock('vue-i18n', () => ({
  useI18n: () => ({
    t: (key: string, fallback?: string) => fallback ?? key,
  }),
}))

vi.mock('@/composables/useDialog', () => ({
  dialog: {
    confirm: vi.fn().mockResolvedValue(true),
    toast: {
      success: vi.fn(),
      error: vi.fn(),
      warning: vi.fn(),
      info: vi.fn(),
    },
  },
}))

const HttpMessageSurfaceStub = defineComponent({
  name: 'HttpMessageSurface',
  props: {
    modelValue: {
      type: String,
      default: '',
    },
    showSearchBar: {
      type: Boolean,
      default: false,
    },
    stateKey: {
      type: String,
      default: '',
    },
  },
  setup(props) {
    const searchValue = ref('')

    return () => h('div', { class: 'http-message-surface-stub', 'data-state-key': props.stateKey }, [
      props.showSearchBar && props.stateKey.includes(':response:')
        ? h('input', {
            'data-testid': 'response-search',
            value: searchValue.value,
            onInput: (event: Event) => {
              searchValue.value = (event.target as HTMLInputElement).value
            },
          })
        : null,
      h(
        'div',
        {
          'data-testid': props.stateKey.includes(':response:') ? 'response-content' : 'request-content',
        },
        props.modelValue,
      ),
    ])
  },
})

function createReplayResult(bodyText: string, rawResponse: string): RawReplayCommandResult {
  return {
    raw_response: rawResponse,
    response_time_ms: 120,
    final_url: 'https://example.com/api/test',
    redirect_chain: [],
    status_code: 200,
    version_observed: 'HTTP/1.1',
    status_text: 'OK',
    headers: [
      { name: 'Content-Type', value: 'text/plain' },
    ],
    body_text: bodyText,
  }
}

function createInitialRequest(): HttpExchangeRequest {
  return {
    endpoint: {
      scheme: 'https',
      host: 'example.com',
      port: 443,
    },
    absoluteUrl: 'https://example.com/api/test',
    request: {
      method: 'GET',
      target: '/api/test',
      versionPreference: 'HTTP/1.1',
      headers: [
        { name: 'Host', value: 'example.com' },
      ],
      bodyText: '',
    },
  }
}

describe('ProxyRepeater', () => {
  beforeEach(() => {
    global.testUtils.mockInvoke.mockReset()
    window.localStorage.clear()
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  it('clears the response content while keeping the response search input mounted during a new request', async () => {
    let resolveSecondResponse: ((value: { success: boolean; data: RawReplayCommandResult }) => void) | null = null

    global.testUtils.mockInvoke
      .mockResolvedValueOnce({
        success: true,
        data: createReplayResult('first body', 'HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n\r\nfirst body'),
      })
      .mockImplementationOnce(() => new Promise((resolve) => {
        resolveSecondResponse = resolve
      }))

    const wrapper = mount(ProxyRepeater, {
      props: {
        initialRequest: createInitialRequest(),
      },
      global: {
        components: {
          AppDialog: defineComponent({
            name: 'AppDialog',
            setup(_, { slots }) {
              return () => h('div', slots.default?.())
            },
          }),
        },
        stubs: {
          HttpMessageSurface: HttpMessageSurfaceStub,
          TrafficResponseRenderPane: true,
          TrafficContextMenuSections: true,
        },
        mocks: {
          $t: (key: string, fallback?: string) => fallback ?? key,
        },
      },
      attachTo: document.body,
    })

    const sendButton = wrapper.get('button.btn-primary.btn-sm')

    await sendButton.trigger('click')
    await flushPromises()

    const responseSearch = wrapper.get('[data-testid="response-search"]')
    await responseSearch.setValue('trace_id')
    expect((responseSearch.element as HTMLInputElement).value).toBe('trace_id')
    expect(wrapper.get('[data-testid="response-content"]').text()).toContain('first body')

    await sendButton.trigger('click')
    await flushPromises()

    const pendingResponseSearch = wrapper.get('[data-testid="response-search"]')
    expect((pendingResponseSearch.element as HTMLInputElement).value).toBe('trace_id')
    expect(wrapper.get('[data-testid="response-content"]').text()).toBe('')

    resolveSecondResponse?.({
      success: true,
      data: createReplayResult('second body', 'HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n\r\nsecond body'),
    })

    await flushPromises()

    const refreshedResponseSearch = wrapper.get('[data-testid="response-search"]')
    expect((refreshedResponseSearch.element as HTMLInputElement).value).toBe('trace_id')
    expect(wrapper.get('[data-testid="response-content"]').text()).toContain('second body')

    wrapper.unmount()
  })
})
