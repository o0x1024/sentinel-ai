import { ref } from 'vue'
import { flushPromises, mount } from '@vue/test-utils'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import ProxyRepeater from './ProxyRepeater.vue'
import { buildHttpReplayResponseFromCommandResult, type RawReplayCommandResult } from './http/response'
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
import { useTrafficWorkbenchStore } from './workbench/stores/useTrafficWorkbenchStore'

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
    useTrafficWorkbenchStore().drafts.resetDraftStore()
    useTrafficWorkbenchStore().replay.resetReplayStore()
  })

  afterEach(() => {
    vi.restoreAllMocks()
    vi.unstubAllGlobals()
  })

  it('clears all repeater tabs and drafts from the new toolbar action', async () => {
    const workbenchState = useTrafficWorkbenchStore()
    const wrapper = mount(ProxyRepeater, {
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

    await flushPromises()

    expect(workbenchState.drafts.drafts.value).toHaveLength(1)

    await wrapper.get('[data-testid="repeater-clear-all-tabs"]').trigger('click')
    await flushPromises()

    expect(workbenchState.drafts.drafts.value).toHaveLength(0)
    expect(wrapper.find('button.btn-primary.btn-sm').exists()).toBe(false)
  })

  it('deletes other tabs from the tab context menu while keeping the targeted tab', async () => {
    const workbenchState = useTrafficWorkbenchStore()
    const wrapper = mount(ProxyRepeater, {
      global: createTrafficMessageViewTestGlobal({
        appDialog: AppDialogStub,
        httpMessageSurface: HttpMessageSurfaceStub,
        trafficMessageReader: TrafficMessageReaderStub,
        trafficMessageViewTabs: TrafficMessageViewTabsStub,
        stubs: {
          TrafficResponseRenderPane: true,
          TrafficContextMenuSections: false,
        },
      }),
      attachTo: document.body,
    })

    await flushPromises()
    await wrapper.get('[title="trafficAnalysis.repeater.contextMenu.newTab"]').trigger('click')
    await flushPromises()

    expect(workbenchState.drafts.drafts.value).toHaveLength(2)

    await wrapper.get('[data-testid="repeater-tab-0"]').trigger('contextmenu', { clientX: 32, clientY: 40 })
    await wrapper.get('[data-testid="traffic-context-deleteOthers"]').trigger('click')
    await flushPromises()

    expect(workbenchState.drafts.drafts.value).toHaveLength(1)
    expect(wrapper.findAll('[data-testid^="repeater-tab-"]')).toHaveLength(1)
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

  it('sends the current request to intruder with Cmd/Ctrl+I inside repeater scope', async () => {
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

    Object.defineProperty(wrapper.element, 'offsetParent', {
      configurable: true,
      get: () => document.body,
    })

    wrapper.element.dispatchEvent(new KeyboardEvent('keydown', {
      bubbles: true,
      cancelable: true,
      key: 'i',
      metaKey: true,
    }))
    await flushPromises()

    const emitted = wrapper.emitted('createAttackWorkspace')
    expect(emitted).toHaveLength(1)
    expect(emitted?.[0]?.[0]).toMatchObject({
      absoluteUrl: 'https://example.com/api/test',
      request: {
        method: 'GET',
        target: '/api/test',
      },
    })

    wrapper.unmount()
  })

  it('does not create a draft when opening a preview request', async () => {
    const workbenchState = useTrafficWorkbenchStore()

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

    await flushPromises()

    expect(workbenchState.drafts.drafts.value).toHaveLength(0)

    wrapper.unmount()
  })

  it('shows preview variant switches beside request and response titles and emits variant changes', async () => {
    const wrapper = mount(ProxyRepeater, {
      props: {
        initialRequest: createInitialHttpExchangeRequest(),
        activeRequestContext: {
          sourceKind: 'history',
          sourceLabel: '历史记录',
          requestId: 1,
          sourceRequestId: 1,
          method: 'GET',
          host: 'example.com',
          path: '/api/test',
          statusCode: 200,
          variant: 'edited',
          hasEditedVariant: true,
          hasEditedResponseVariant: true,
          mode: 'preview',
          modeLabel: '预览',
        },
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

    await flushPromises()

    expect(wrapper.get('[data-testid="preview-request-variant-switch"]').text()).toContain('修改后')
    expect(wrapper.get('[data-testid="preview-response-variant-switch"]').text()).toContain('修改后')

    await wrapper.get('[data-testid="preview-request-variant-switch"]').trigger('click')
    await flushPromises()

    const requestOriginalLink = Array.from(document.body.querySelectorAll('a'))
      .find(node => node.textContent?.includes('trafficAnalysis.history.detailsPanel.originalRequest'))
    expect(requestOriginalLink).toBeTruthy()

    requestOriginalLink!.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }))
    await flushPromises()

    expect(wrapper.emitted('switchPreviewVariant')?.[0]).toEqual(['original'])

    wrapper.unmount()
  })

  it('opens all persisted drafts and focuses the active draft when mounted after workbench hydration', async () => {
    const workbenchState = useTrafficWorkbenchStore()
    const draft = workbenchState.drafts.createDraftFromExchangeRequest({
      request: createInitialHttpExchangeRequest(),
      source: { kind: 'repeater', label: '重放器' },
      title: 'persisted draft',
    })
    workbenchState.drafts.createDraftFromExchangeRequest({
      request: {
        ...createInitialHttpExchangeRequest(),
        absoluteUrl: 'https://example.com/api/second',
        request: {
          ...createInitialHttpExchangeRequest().request,
          target: '/api/second',
        },
      },
      source: { kind: 'repeater', label: '重放器' },
      title: 'second persisted draft',
    })
    workbenchState.drafts.selectDraft(draft.id)

    const wrapper = mount(ProxyRepeater, {
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

    await flushPromises()

    expect(wrapper.get('[data-testid="request-content"]').text()).toContain('GET /api/test HTTP/1.1')
    expect(wrapper.get('[data-testid="request-content"]').text()).toContain('Host: example.com')
    expect(wrapper.emitted('tabStatsChanged')?.at(-1)?.[0]).toEqual({ editedTabCount: 2 })

    wrapper.unmount()
  })

  it('restores persisted response bodies when replay runs hydrate after draft tabs', async () => {
    const workbenchState = useTrafficWorkbenchStore()
    const draft = workbenchState.drafts.createDraftFromExchangeRequest({
      request: createInitialHttpExchangeRequest(),
      source: { kind: 'repeater', label: '重放器' },
      title: 'persisted draft',
    })

    const wrapper = mount(ProxyRepeater, {
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

    await flushPromises()
    expect(wrapper.find('[data-testid="response-content"]').exists()).toBe(false)

    workbenchState.replay.replaceState([{
      id: 'run-1',
      draftId: draft.id,
      draftRevisionId: draft.activeRevisionId,
      state: 'done',
      response: buildHttpReplayResponseFromCommandResult(
        createReplayResult('restored body', 'HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n\r\nrestored body'),
      ),
      error: null,
      createdAt: 1,
      updatedAt: 2,
    }])
    await flushPromises()

    expect(wrapper.get('[data-testid="response-content"]').text()).toContain('restored body')

    wrapper.unmount()
  })

  it('emits edited tab count after the user edits a preview request', async () => {
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

    await flushPromises()

    const requestSurface = wrapper.findAllComponents(HttpMessageSurfaceStub).find(component =>
      component.attributes('data-state-key')?.includes(':request:pretty'),
    )
    expect(requestSurface).toBeTruthy()

    requestSurface!.vm.$emit(
      'update:modelValue',
      'GET /api/test?edited=1 HTTP/1.1\r\nHost: example.com\r\n\r\n',
    )
    await flushPromises()

    const statsEvents = wrapper.emitted('tabStatsChanged')
    expect(statsEvents?.at(-1)?.[0]).toEqual({ editedTabCount: 1 })

    wrapper.unmount()
  })

  it('emits edited tab count after sending a preview request', async () => {
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

    await flushPromises()
    await wrapper.get('button.btn-primary.btn-sm').trigger('click')
    await flushPromises()

    const statsEvents = wrapper.emitted('tabStatsChanged')
    expect(statsEvents?.at(-1)?.[0]).toEqual({ editedTabCount: 1 })

    wrapper.unmount()
  })

  it('keeps local request deletions when the active draft is reselected after sending', async () => {
    const workbenchState = useTrafficWorkbenchStore()

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

    await flushPromises()
    await wrapper.get('button.btn-primary.btn-sm').trigger('click')
    await flushPromises()

    const draftId = workbenchState.drafts.activeDraftId.value
    expect(draftId).toBeTruthy()

    const requestSurface = wrapper.findAllComponents(HttpMessageSurfaceStub).find(component =>
      component.attributes('data-state-key')?.includes(':request:pretty'),
    )
    expect(requestSurface).toBeTruthy()

    requestSurface!.vm.$emit(
      'update:modelValue',
      'GET /api/tes HTTP/1.1\r\nHost: example.com\r\n\r\n',
    )
    workbenchState.drafts.selectDraft(null)
    workbenchState.drafts.selectDraft(draftId)
    await flushPromises()

    expect(wrapper.get('[data-testid="request-content"]').text()).toContain('GET /api/tes HTTP/1.1')
    expect(wrapper.get('[data-testid="request-content"]').text()).not.toContain('GET /api/test HTTP/1.1')

    wrapper.unmount()
  })

  it('renders the first replay response with the real HTTP surface', async () => {
    const ResizeObserverMock = vi.fn(() => ({
      observe: vi.fn(),
      disconnect: vi.fn(),
      unobserve: vi.fn(),
    }))
    vi.stubGlobal('ResizeObserver', ResizeObserverMock)

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
        trafficMessageReader: TrafficMessageReaderStub,
        trafficMessageViewTabs: TrafficMessageViewTabsStub,
        stubs: {
          TrafficResponseRenderPane: true,
          TrafficContextMenuSections: true,
        },
      }),
      attachTo: document.body,
    })

    await flushPromises()
    await wrapper.get('button.btn-primary.btn-sm').trigger('click')
    await flushPromises()

    expect(wrapper.text()).toContain('200')

    wrapper.unmount()
  })
})
