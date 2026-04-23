import { nextTick } from 'vue'
import { mount } from '@vue/test-utils'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import ImmersiveMinimizedToolTray from './ImmersiveMinimizedToolTray.vue'
import { resetImmersiveMinimizedToolTray } from '@/services/immersiveMinimizedToolTray'
import {
  clearImmersiveSecurityCenterReturnPath,
  closeImmersiveSecurityCenterSidebar,
} from '@/services/immersiveSecurityCenterSidebar'
import {
  closeTrafficAssistant,
  minimizeTrafficAssistant,
  openTrafficAssistantPanel,
} from '@/services/trafficAssistantWorkspace'
import { setImmersiveDrillModeEnabled } from '@/services/immersiveDrillMode'

vi.mock('vue-i18n', () => ({
  useI18n: () => ({
    t: (key: string, fallback?: string) => fallback ?? key,
  }),
}))

describe('ImmersiveMinimizedToolTray', () => {
  beforeEach(() => {
    window.localStorage.clear()
    setImmersiveDrillModeEnabled(false)
    closeTrafficAssistant()
    closeImmersiveSecurityCenterSidebar()
    clearImmersiveSecurityCenterReturnPath()
    resetImmersiveMinimizedToolTray()

    vi.stubGlobal('requestAnimationFrame', ((callback: FrameRequestCallback) => {
      callback(16)
      return 1
    }) as typeof window.requestAnimationFrame)
    vi.stubGlobal('cancelAnimationFrame', vi.fn())
  })

  afterEach(() => {
    closeTrafficAssistant()
    closeImmersiveSecurityCenterSidebar()
    clearImmersiveSecurityCenterReturnPath()
    resetImmersiveMinimizedToolTray()
    setImmersiveDrillModeEnabled(false)
    vi.unstubAllGlobals()
  })

  it('announces keyboard docking and exposes tray panel semantics', async () => {
    openTrafficAssistantPanel()
    minimizeTrafficAssistant()

    const wrapper = mount(ImmersiveMinimizedToolTray, {
      attachTo: document.body,
    })

    await nextTick()

    const launcher = wrapper.get('button[aria-controls="immersive-minimized-tool-tray-panel"]')
    expect(launcher.attributes('aria-expanded')).toBe('false')

    await launcher.trigger('click')
    await nextTick()

    expect(wrapper.get('#immersive-minimized-tool-tray-panel').attributes('aria-label')).toBe('最小化工具托盘')

    const dragHandle = wrapper.get('button[aria-label^="工具托盘。"]')
    await dragHandle.trigger('keydown', { key: 'Home' })
    await nextTick()

    expect(wrapper.get('[aria-live="polite"]').text()).toBe('工具托盘已停靠到左侧边缘。')

    wrapper.unmount()
  })
})
