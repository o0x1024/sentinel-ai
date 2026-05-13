import { mount } from '@vue/test-utils'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { nextTick } from 'vue'
import TrafficWorkbenchSidebar from './TrafficWorkbenchSidebar.vue'

vi.mock('vue-i18n', () => ({
  useI18n: () => ({
    t: (_key: string, fallback?: string | Record<string, unknown>) => (
      typeof fallback === 'string' ? fallback : ''
    ),
  }),
}))

class ResizeObserverMock {
  private callback: ResizeObserverCallback

  constructor(callback: ResizeObserverCallback) {
    this.callback = callback
  }

  observe = vi.fn(() => {
    this.callback(
      [{ contentRect: { width: 320 } } as ResizeObserverEntry],
      this as unknown as ResizeObserver,
    )
  })

  disconnect = vi.fn()
}

describe('TrafficWorkbenchSidebar', () => {
  beforeEach(() => {
    vi.stubGlobal('ResizeObserver', ResizeObserverMock)
    vi.spyOn(window, 'requestAnimationFrame').mockImplementation(callback => {
      callback(0)
      return 1
    })
    vi.spyOn(window, 'cancelAnimationFrame').mockImplementation(() => {})
  })

  afterEach(() => {
    document.body.innerHTML = ''
    vi.restoreAllMocks()
    vi.unstubAllGlobals()
  })

  it('renders the compact action menu outside the clipped sidebar card', async () => {
    const wrapper = mount(TrafficWorkbenchSidebar, {
      props: {
        basketCount: 1,
        controlInterceptCount: 1,
        repeaterHistoryCount: 0,
        intruderHistoryCount: 0,
        runningAttackCount: 0,
        layoutToggleLabel: '左右布局',
        layoutToggleIcon: 'fas fa-columns',
        effectiveLayoutLabel: '左右布局',
      },
    })

    await nextTick()
    await wrapper.get('button[aria-haspopup="menu"]').trigger('click')
    await nextTick()

    const menu = document.body.querySelector('.workbench-action-floating-menu')
    expect(menu).not.toBeNull()
    expect(wrapper.element.contains(menu)).toBe(false)

    wrapper.unmount()
  })
})
