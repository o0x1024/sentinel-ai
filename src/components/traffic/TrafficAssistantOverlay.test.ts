import { defineComponent, h, nextTick } from 'vue'
import { mount } from '@vue/test-utils'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import TrafficAssistantOverlay from './TrafficAssistantOverlay.vue'
import {
  closeTrafficAssistant,
  openTrafficAssistantPanel,
  trafficAssistantOpen,
} from '@/services/trafficAssistantWorkspace'

vi.mock('vue-i18n', () => ({
  useI18n: () => ({
    t: (key: string, fallback?: string) => fallback ?? key,
  }),
}))

vi.mock('@/components/assistant/AIAssistantWorkspace.vue', () => ({
  default: defineComponent({
    name: 'AIAssistantWorkspace',
    setup() {
      return () => h('div', 'assistant-workspace')
    },
  }),
}))

describe('TrafficAssistantOverlay', () => {
  beforeEach(() => {
    closeTrafficAssistant()
    vi.stubGlobal('ResizeObserver', class {
      observe() {}
      disconnect() {}
    })
  })

  afterEach(() => {
    closeTrafficAssistant()
    vi.unstubAllGlobals()
  })

  it('hides the assistant overlay on Escape', async () => {
    openTrafficAssistantPanel()

    const wrapper = mount(TrafficAssistantOverlay, {
      attachTo: document.body,
    })

    await nextTick()
    expect(trafficAssistantOpen.value).toBe(true)

    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }))
    await nextTick()

    expect(trafficAssistantOpen.value).toBe(false)

    wrapper.unmount()
  })
})
