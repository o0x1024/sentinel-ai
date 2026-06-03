import { ref } from 'vue'
import { flushPromises, mount } from '@vue/test-utils'
import { describe, expect, it, vi } from 'vitest'
import ProxyComparer from './ProxyComparer.vue'
import type { TrafficComparePayload } from './transfers'
import { CodeDiffViewerStub, createHttpMessageSurfaceStub } from './trafficMessageViewTestStubs'
import {
  expectCodeDiffVisible,
  expectHttpSurfaceModes,
  expectNoHttpSurfaces,
  expectReadonlyHttpSurface,
} from './trafficMessageViewTestAssertions'
import { createTrafficMessageViewTestGlobal } from './trafficMessageViewTestMount'
import { createTrafficComparePayload } from './trafficMessageViewTestData'

vi.mock('vue-i18n', async () => {
  const { createVueI18nMock } = await import('./trafficMessageViewTestMocks')
  return createVueI18nMock('en')
})

vi.mock('@/services/immersiveDrillMode', async () => {
  const { createImmersiveDrillModeMock } = await import('./trafficMessageViewTestMocks')
  return createImmersiveDrillModeMock()
})

vi.mock('./useTrafficPaneCompactMode', async () => {
  const { createTrafficPaneCompactModeMock } = await import('./trafficMessageViewTestMocks')
  return createTrafficPaneCompactModeMock()
})

vi.mock('./trafficSendTargets', () => ({
  useTrafficSendTargets: () => ({
    enabledTargets: ref({
      repeater: false,
    }),
  }),
}))

vi.mock('@/composables/useDialog', async () => {
  const { createDialogToastMock } = await import('./trafficMessageViewTestMocks')
  return createDialogToastMock()
})

vi.mock('./trafficComparerActionMenuSupport', () => ({
  buildComparerActionMenuItems: () => [],
}))

vi.mock('./trafficContextMenuSectionSupport', () => ({
  buildTrafficContextMenuSections: () => [],
}))

vi.mock('./immersiveTrafficUi', () => ({
  IMMERSIVE_TRAFFIC_COMPACT_BADGE_CLASS: 'badge',
  IMMERSIVE_TRAFFIC_TOP_BAR_CLASS: 'topbar',
}))

const HttpMessageSurfaceStub = createHttpMessageSurfaceStub()

describe('ProxyComparer', () => {
  it('switches between diff and plain compare render modes', async () => {
    const wrapper = mount(ProxyComparer, {
      global: createTrafficMessageViewTestGlobal({
        httpMessageSurface: HttpMessageSurfaceStub,
        codeDiffViewer: CodeDiffViewerStub,
        stubs: {
          TrafficMessageDisplayControls: true,
          TrafficContextMenuSections: true,
        },
      }),
    })

    ;(wrapper.vm as unknown as { addComparison: (payload: TrafficComparePayload) => void }).addComparison(createTrafficComparePayload())
    await flushPromises()

    expectCodeDiffVisible(wrapper, true)
    expectNoHttpSurfaces(wrapper)

    const plainButton = wrapper.findAll('button').find(
      node => node.text() === 'trafficAnalysis.comparer.viewModes.plain',
    )
    expect(plainButton).toBeTruthy()
    await plainButton!.trigger('click')
    await flushPromises()

    expectCodeDiffVisible(wrapper, false)
    expectHttpSurfaceModes(wrapper, ['pretty', 'pretty'])
    expectReadonlyHttpSurface(wrapper, { mode: 'pretty', index: 0 })
  })
})
