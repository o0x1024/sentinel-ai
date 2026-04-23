import { ref } from 'vue'
import { flushPromises, mount } from '@vue/test-utils'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import ProxyRepeater from './ProxyRepeater.vue'
import type { RawReplayCommandResult } from './http/response'
import {
  AppDialogStub,
  createHttpMessageSurfaceStub,
  createTrafficMessageViewTabsStub,
  TrafficMessageReaderStub,
} from './trafficMessageViewTestStubs'
import {
  expectNoTrafficReaders,
  expectReadonlyHttpSurface,
  expectTrafficReaderCount,
} from './trafficMessageViewTestAssertions'
import { createTrafficMessageViewTestGlobal } from './trafficMessageViewTestMount'
import { createInitialHttpExchangeRequest, createReplayResult } from './trafficMessageViewTestData'

vi.mock('vue-i18n', async () => {
  const { createVueI18nMock } = await import('./trafficMessageViewTestMocks')
  return createVueI18nMock('zh-CN')
})

vi.mock('@/composables/useDialog', async () => {
  const { createDialogConfirmAndToastMock } = await import('./trafficMessageViewTestMocks')
  return createDialogConfirmAndToastMock()
})

vi.mock('./useTrafficPaneCompactMode', async () => {
  const { createTrafficPaneCompactModeMock } = await import('./trafficMessageViewTestMocks')
  return createTrafficPaneCompactModeMock()
})

const HttpMessageSurfaceStub = createHttpMessageSurfaceStub({ responseSearchTestId: 'response-search' })
const TrafficMessageViewTabsStub = createTrafficMessageViewTabsStub('traffic-tab')

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
        initialRequest: createInitialHttpExchangeRequest(),
      },
      global: createTrafficMessageViewTestGlobal({
        appDialog: AppDialogStub,
        httpMessageSurface: HttpMessageSurfaceStub,
        trafficMessageReader: TrafficMessageReaderStub,
        trafficMessageViewTabs: TrafficMessageViewTabsStub,
        stubs: {
          TrafficResponseRenderPane: true,
          TrafficContextMenuSections: true,
        },
      }),
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

  it('renders repeater responses with HttpMessageSurface for pretty/raw and TrafficMessageReader for hex', async () => {
    global.testUtils.mockInvoke.mockResolvedValueOnce({
      success: true,
      data: createReplayResult('response body', 'HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n\r\nresponse body'),
    })

    const wrapper = mount(ProxyRepeater, {
      props: {
        initialRequest: createInitialHttpExchangeRequest(),
      },
      global: createTrafficMessageViewTestGlobal({
        appDialog: AppDialogStub,
        httpMessageSurface: HttpMessageSurfaceStub,
        trafficMessageReader: TrafficMessageReaderStub,
        trafficMessageViewTabs: TrafficMessageViewTabsStub,
        stubs: {
          TrafficResponseRenderPane: true,
          TrafficContextMenuSections: true,
        },
      }),
    })

    await wrapper.get('button.btn-primary.btn-sm').trigger('click')
    await flushPromises()

    const responseSurface = wrapper.findAll('.http-message-surface-stub').find(node =>
      node.attributes('data-state-key')?.includes(':response:'),
    )
    expect(responseSurface).toBeTruthy()
    expect(wrapper.text()).toContain('120 ms | 58 B')
    expectReadonlyHttpSurface(wrapper, { mode: 'pretty', stateKeyIncludes: ':response:' })
    expectNoTrafficReaders(wrapper)

    const responseTabs = wrapper.findAll('.traffic-message-view-tabs-stub')[1]
    expect(responseTabs).toBeTruthy()

    const rawButton = responseTabs!.find('[data-testid="traffic-tab-raw"]')
    await rawButton.trigger('click')
    await flushPromises()

    const rawSurface = wrapper.findAll('.http-message-surface-stub').find(node =>
      node.attributes('data-state-key')?.includes(':response:raw'),
    )
    expect(rawSurface).toBeTruthy()
    expectReadonlyHttpSurface(wrapper, { mode: 'raw', stateKeyIncludes: ':response:raw' })
    expectNoTrafficReaders(wrapper)

    const hexButton = responseTabs!.find('[data-testid="traffic-tab-hex"]')
    await hexButton.trigger('click')
    await flushPromises()

    const readers = wrapper.findAll('[data-testid="traffic-reader"]')
    expectTrafficReaderCount(wrapper, 1)
    expect(readers[0]?.attributes('data-state-key')).toContain(':response:hex')
    expect(wrapper.findAll('.http-message-surface-stub').find(node =>
      node.attributes('data-state-key')?.includes(':response:'),
    )).toBeFalsy()
  })
})
