import { defineComponent, h, nextTick } from 'vue'
import { mount } from '@vue/test-utils'
import {
  createMemoryHistory,
  createRouter,
  type RouteRecordRaw,
} from 'vue-router'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import ImmersiveDrillDock from './ImmersiveDrillDock.vue'
import {
  immersiveDrillModeEnabled,
  setImmersiveDrillModeEnabled,
} from '@/services/immersiveDrillMode'
import {
  clearImmersiveSecurityCenterReturnPath,
  closeImmersiveSecurityCenterSidebar,
  immersiveSecurityCenterSidebarOpen,
  openImmersiveSecurityCenterSidebar,
} from '@/services/immersiveSecurityCenterSidebar'
import {
  closeTrafficAssistant,
  openTrafficAssistantPanel,
  trafficAssistantOpen,
} from '@/services/trafficAssistantWorkspace'
import {
  openImmersiveTrafficWorkbenchTool,
  resetImmersiveTrafficDockState,
  useImmersiveTrafficDockState,
} from '@/components/traffic/immersiveTrafficDockState'

vi.mock('vue-i18n', () => ({
  useI18n: () => ({
    t: (key: string, fallback?: string) => fallback ?? key,
  }),
}))

function createTestRouter() {
  const TrafficPage = defineComponent({
    name: 'TrafficPage',
    setup() {
      return () => h('div', 'traffic')
    },
  })

  const routes: RouteRecordRaw[] = [
    {
      path: '/traffic',
      component: TrafficPage,
    },
  ]

  return createRouter({
    history: createMemoryHistory(),
    routes,
  })
}

describe('ImmersiveDrillDock', () => {
  beforeEach(() => {
    window.localStorage.clear()
    setImmersiveDrillModeEnabled(false)
    closeTrafficAssistant()
    closeImmersiveSecurityCenterSidebar()
    clearImmersiveSecurityCenterReturnPath()
    resetImmersiveTrafficDockState()

    vi.stubGlobal('requestAnimationFrame', ((callback: FrameRequestCallback) => {
      callback(16)
      return 1
    }) as typeof window.requestAnimationFrame)
    vi.stubGlobal('cancelAnimationFrame', vi.fn())
  })

  afterEach(() => {
    setImmersiveDrillModeEnabled(false)
    closeTrafficAssistant()
    closeImmersiveSecurityCenterSidebar()
    clearImmersiveSecurityCenterReturnPath()
    resetImmersiveTrafficDockState()
    vi.unstubAllGlobals()
  })

  it('exposes toolbar semantics and announces keyboard docking', async () => {
    const router = createTestRouter()
    await router.push('/traffic')
    await router.isReady()

    const wrapper = mount(ImmersiveDrillDock, {
      global: {
        plugins: [router],
      },
      attachTo: document.body,
    })

    await nextTick()

    const toolbar = wrapper.get('[role="toolbar"]')
    expect(toolbar.attributes('aria-orientation')).toBe('vertical')
    expect(toolbar.attributes('aria-label')).toBe('沉浸式挖洞模式')

    const dragHandle = wrapper.get('button[aria-label^="沉浸式挖洞模式。"]')
    await dragHandle.trigger('keydown', { key: 'Home' })
    await nextTick()

    expect(wrapper.get('[aria-live="polite"]').text()).toBe('沉浸式挖洞模式已停靠到左侧边缘。')

    wrapper.unmount()
  })

  it('opens the OAST workbench tool from the immersive toolbar', async () => {
    const router = createTestRouter()
    await router.push('/traffic')
    await router.isReady()

    const wrapper = mount(ImmersiveDrillDock, {
      global: {
        plugins: [router],
      },
      attachTo: document.body,
    })

    await nextTick()

    await wrapper.get('button[aria-label="OAST"]').trigger('click')
    await nextTick()

    const { workbenchOpen, activeWorkbenchTool } = useImmersiveTrafficDockState()
    expect(workbenchOpen.value).toBe(true)
    expect(activeWorkbenchTool.value).toBe('oast')

    wrapper.unmount()
  })

  it('exits immersive drill mode when Escape is pressed with no immersive windows open', async () => {
    const router = createTestRouter()
    await router.push('/traffic')
    await router.isReady()
    setImmersiveDrillModeEnabled(true)

    const wrapper = mount(ImmersiveDrillDock, {
      global: {
        plugins: [router],
      },
      attachTo: document.body,
    })

    await nextTick()

    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }))
    await nextTick()

    expect(immersiveDrillModeEnabled.value).toBe(false)

    wrapper.unmount()
  })

  it('hides immersive tools from top layer to bottom layer before exiting drill mode', async () => {
    const router = createTestRouter()
    await router.push('/traffic')
    await router.isReady()
    setImmersiveDrillModeEnabled(true)

    const wrapper = mount(ImmersiveDrillDock, {
      global: {
        plugins: [router],
      },
      attachTo: document.body,
    })

    await nextTick()

    openImmersiveTrafficWorkbenchTool('repeater')
    openTrafficAssistantPanel()
    openImmersiveSecurityCenterSidebar('/traffic')

    const { workbenchOpen } = useImmersiveTrafficDockState()

    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }))
    await nextTick()
    expect(immersiveSecurityCenterSidebarOpen.value).toBe(false)
    expect(trafficAssistantOpen.value).toBe(true)
    expect(workbenchOpen.value).toBe(true)
    expect(immersiveDrillModeEnabled.value).toBe(true)

    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }))
    await nextTick()
    expect(trafficAssistantOpen.value).toBe(false)
    expect(workbenchOpen.value).toBe(true)
    expect(immersiveDrillModeEnabled.value).toBe(true)

    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }))
    await nextTick()
    expect(workbenchOpen.value).toBe(false)
    expect(immersiveDrillModeEnabled.value).toBe(true)

    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }))
    await nextTick()
    expect(immersiveDrillModeEnabled.value).toBe(false)

    wrapper.unmount()
  })
})
