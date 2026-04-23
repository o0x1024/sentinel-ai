import { ref } from 'vue'
import { flushPromises, mount } from '@vue/test-utils'
import { describe, it, vi, beforeEach } from 'vitest'
import ProxyIntercept from './ProxyIntercept.vue'
import { createHttpMessageSurfaceStub, createTrafficMessageViewTabsStub } from './trafficMessageViewTestStubs'
import { expectReadonlyHttpSurface } from './trafficMessageViewTestAssertions'
import { createTrafficMessageViewTestGlobal } from './trafficMessageViewTestMount'
import { createInterceptedRequest, createInterceptedResponse } from './trafficMessageViewTestData'

const listenHandlers = new Map<string, (event: { payload: unknown }) => void>()

vi.mock('vue-i18n', async () => {
  const { createVueI18nMock } = await import('./trafficMessageViewTestMocks')
  return createVueI18nMock('zh-CN')
})

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(async (command: string) => {
    switch (command) {
      case 'get_proxy_status':
        return {
          success: true,
          data: {
            running: true,
            port: 8080,
            mitm: false,
            stats: {
              http_requests: 0,
              https_requests: 0,
              errors: 0,
              qps: 0,
            },
          },
        }
      case 'get_intercept_enabled':
      case 'get_response_intercept_enabled':
      case 'get_websocket_intercept_enabled':
        return { success: true, data: false }
      default:
        return { success: true }
    }
  }),
}))

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(async (eventName: string, handler: (event: { payload: unknown }) => void) => {
    listenHandlers.set(eventName, handler)
    return () => {
      listenHandlers.delete(eventName)
    }
  }),
  emit: vi.fn(),
}))

vi.mock('@/composables/useDialog', async () => {
  const { createDialogConfirmAndToastMock } = await import('./trafficMessageViewTestMocks')
  return createDialogConfirmAndToastMock()
})

vi.mock('@/services/trafficAssistantWorkspace', () => ({
  openTrafficAssistantPanel: vi.fn(),
}))

vi.mock('./useTrafficPaneCompactMode', async () => {
  const { createTrafficPaneCompactModeMock } = await import('./trafficMessageViewTestMocks')
  return createTrafficPaneCompactModeMock()
})

vi.mock('./trafficSendTargets', () => ({
  useTrafficSendTargets: () => ({
    enabledTargets: ref({
      repeater: false,
      comparer: false,
      intruder: false,
    }),
  }),
}))

vi.mock('./trafficRequestActionMenuSupport', () => ({
  buildTrafficRequestActionMenuItems: () => [],
}))

vi.mock('./trafficContextMenuSectionSupport', () => ({
  buildTrafficContextMenuSections: () => [],
}))

vi.mock('./trafficContextSubmenuSupport', () => ({
  buildTrafficContextSubmenu: () => null,
}))

vi.mock('./trafficSendMenuSupport', () => ({
  buildTrafficRequestSendMenuItems: () => [],
}))

const HttpMessageSurfaceStub = createHttpMessageSurfaceStub()
const TrafficMessageViewTabsStub = createTrafficMessageViewTabsStub('intercept-tab')

function mountIntercept() {
  return mount(ProxyIntercept, {
    global: createTrafficMessageViewTestGlobal({
      httpMessageSurface: HttpMessageSurfaceStub,
      trafficMessageViewTabs: TrafficMessageViewTabsStub,
      stubs: {
        AppDialog: true,
        TrafficMessageDisplayControls: true,
        TrafficContextMenuSections: true,
        TrafficContextSubmenu: true,
      },
    }),
  })
}

describe('ProxyIntercept', () => {
  beforeEach(() => {
    listenHandlers.clear()
  })

  it('uses readonly HttpMessageSurface for intercepted request raw and pretty views when editing is disabled', async () => {
    const wrapper = mountIntercept()
    await flushPromises()

    listenHandlers.get('intercept:request')?.({ payload: createInterceptedRequest() })
    await flushPromises()

    ;(wrapper.vm as any).isEditable = false
    await flushPromises()

    expectReadonlyHttpSurface(wrapper, {
      mode: 'pretty',
      stateKey: 'intercept:request:0:pretty',
    })

    await wrapper.get('[data-testid="intercept-tab-raw"]').trigger('click')
    await flushPromises()

    expectReadonlyHttpSurface(wrapper, {
      mode: 'raw',
      stateKey: 'intercept:request:0:raw',
    })
  })

  it('uses readonly HttpMessageSurface for intercepted response raw and pretty views when editing is disabled', async () => {
    const wrapper = mountIntercept()
    await flushPromises()

    listenHandlers.get('intercept:request')?.({ payload: createInterceptedRequest() })
    listenHandlers.get('intercept:response')?.({ payload: createInterceptedResponse() })
    await flushPromises()

    ;(wrapper.vm as any).selectItem(1)
    ;(wrapper.vm as any).isEditable = false
    await flushPromises()

    expectReadonlyHttpSurface(wrapper, {
      mode: 'pretty',
      stateKey: 'intercept:response:1:pretty',
    })

    await wrapper.get('[data-testid="intercept-tab-raw"]').trigger('click')
    await flushPromises()

    expectReadonlyHttpSurface(wrapper, {
      mode: 'raw',
      stateKey: 'intercept:response:1:raw',
    })
  })
})
