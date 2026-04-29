import { ref } from 'vue'
import { mount } from '@vue/test-utils'
import { describe, it, vi } from 'vitest'
import ProxyHistoryDetailsPanel from './ProxyHistoryDetailsPanel.vue'
import { createHttpMessageSurfaceStub, TrafficMessageReaderStub } from './trafficMessageViewTestStubs'
import {
  expectHttpSurfaceModes,
  expectNoHttpSurfaces,
  expectNoTrafficReaders,
  expectTrafficReaderCount,
} from './trafficMessageViewTestAssertions'
import { createTrafficMessageViewTestGlobal } from './trafficMessageViewTestMount'
import { createSelectedProxyRequest } from './trafficMessageViewTestData'

vi.mock('vue-i18n', async () => {
  const { createVueI18nMock } = await import('./trafficMessageViewTestMocks')
  return createVueI18nMock('zh-CN')
})

vi.mock('./proxyHistoryHttpSupport', () => ({
  formatProxyHistorySchemeLabel: () => 'HTTPS',
  normalizeProxyHistoryHttpVersion: () => 'HTTP/2',
}))

vi.mock('./proxyHistoryFormattingSupport', () => ({
  formatRequest: () => 'formatted request',
  formatRequestRaw: () => 'raw request',
  formatResponse: () => 'formatted response',
  formatResponseRawFast: () => 'raw response',
  formatResponseRaw: () => 'raw response',
  getResponseContentType: () => 'application/json',
  hasEditedRequest: () => false,
  hasEditedResponse: () => false,
  isResponseCompressed: () => false,
}))

vi.mock('./trafficContextEvidenceHighlightSupport', () => ({
  resolveTrafficContextEvidenceHighlights: () => [],
  findTrafficContextEvidenceSelectionRange: () => null,
  findTrafficContextEvidenceSelectionRangeBySearchTerms: () => null,
}))

vi.mock('./trafficResponseDecodingSupport', () => ({
  resolveStoredTrafficResponseBodyText: (value: string) => value,
}))

vi.mock('./trafficDisplaySettings', () => ({
  useTrafficDisplaySettings: () => ({
    settings: ref({
      fontFamily: 'monospace',
      fontSize: 13,
      wrapLongLines: true,
      showLineEndings: false,
    }),
  }),
}))

vi.mock('./useTrafficPaneCompactMode', async () => {
  const { createTrafficPaneCompactModeMock } = await import('./trafficMessageViewTestMocks')
  return createTrafficPaneCompactModeMock()
})

vi.mock('@/services/immersiveDrillMode', async () => {
  const { createImmersiveDrillModeMock } = await import('./trafficMessageViewTestMocks')
  return createImmersiveDrillModeMock()
})

const HttpMessageSurfaceStub = createHttpMessageSurfaceStub()

function mountPanel(props?: Record<string, unknown>) {
  return mount(ProxyHistoryDetailsPanel, {
    props: {
      selectedRequest: createSelectedProxyRequest(),
      isLoadingSelectedRequest: false,
      leftPanelWidth: 480,
      requestTab: 'pretty',
      responseTab: 'raw',
      requestViewMode: 'original',
      responseViewMode: 'original',
      showDetailContextMenu: vi.fn(),
      startVerticalResize: vi.fn(),
      ...(props || {}),
    },
    global: createTrafficMessageViewTestGlobal({
      httpMessageSurface: HttpMessageSurfaceStub,
      trafficMessageReader: TrafficMessageReaderStub,
      stubs: {
        TrafficMessageDisplayControls: true,
        TrafficMessageViewTabs: true,
        TrafficResponseRenderPane: true,
      },
    }),
  })
}

describe('ProxyHistoryDetailsPanel', () => {
  it('uses HttpMessageSurface for non-hex request and response views', () => {
    const wrapper = mountPanel({
      requestTab: 'pretty',
      responseTab: 'raw',
    })

    expectHttpSurfaceModes(wrapper, ['pretty', 'raw'])
    expectNoTrafficReaders(wrapper)
  })

  it('uses TrafficMessageReader for hex request and response views', () => {
    const wrapper = mountPanel({
      requestTab: 'hex',
      responseTab: 'hex',
    })

    expectTrafficReaderCount(wrapper, 2)
    expectNoHttpSurfaces(wrapper)
  })
})
