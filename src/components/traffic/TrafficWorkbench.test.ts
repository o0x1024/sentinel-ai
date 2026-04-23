import { shallowMount } from '@vue/test-utils'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { nextTick } from 'vue'
import TrafficWorkbench from './TrafficWorkbench.vue'
import {
  openImmersiveTrafficWorkbenchTool,
  resetImmersiveTrafficDockState,
  toggleImmersiveTrafficProxySettings,
  toggleImmersiveTrafficBasket,
  toggleImmersiveTrafficInterceptDrawer,
  useImmersiveTrafficDockState,
} from './immersiveTrafficDockState'
import { setImmersiveDrillModeEnabled } from '@/services/immersiveDrillMode'

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}))

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}))

vi.mock('vue-i18n', async importOriginal => {
  const actual = await importOriginal<typeof import('vue-i18n')>()

  return {
    ...actual,
    useI18n: () => ({
      t: (key: string, fallback?: string) => fallback ?? key,
    }),
  }
})

describe('TrafficWorkbench', () => {
  beforeEach(() => {
    window.localStorage.clear()
    resetImmersiveTrafficDockState()
    setImmersiveDrillModeEnabled(false)
  })

  afterEach(() => {
    resetImmersiveTrafficDockState()
    setImmersiveDrillModeEnabled(false)
  })

  it('defaults back to history when mounted in immersive drill mode', async () => {
    setImmersiveDrillModeEnabled(true)
    openImmersiveTrafficWorkbenchTool('intruder')
    toggleImmersiveTrafficProxySettings()

    const dockState = useImmersiveTrafficDockState()
    expect(dockState.proxySettingsOpen.value).toBe(true)

    const wrapper = shallowMount(TrafficWorkbench, {
      global: {
        mocks: {
          $t: (key: string, fallback?: string) => fallback ?? key,
        },
        stubs: {
          AppModal: {
            template: '<div><slot /></div>',
          },
        },
      },
    })

    await nextTick()

    expect(dockState.workbenchOpen.value).toBe(false)
    expect(dockState.interceptDrawerOpen.value).toBe(false)
    expect(dockState.proxySettingsOpen.value).toBe(false)
    expect(dockState.basketOpen.value).toBe(false)

    wrapper.unmount()
  })

  it('hides immersive toolbar windows in layer order with Escape', async () => {
    setImmersiveDrillModeEnabled(true)
    const dockState = useImmersiveTrafficDockState()
    const wrapper = shallowMount(TrafficWorkbench, {
      global: {
        mocks: {
          $t: (key: string, fallback?: string) => fallback ?? key,
        },
        stubs: {
          AppModal: {
            template: '<div><slot /></div>',
          },
        },
      },
    })

    await nextTick()

    openImmersiveTrafficWorkbenchTool('repeater')
    toggleImmersiveTrafficInterceptDrawer()
    toggleImmersiveTrafficBasket()
    toggleImmersiveTrafficProxySettings()

    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }))
    expect(dockState.proxySettingsOpen.value).toBe(false)
    expect(dockState.basketOpen.value).toBe(true)
    expect(dockState.interceptDrawerOpen.value).toBe(true)
    expect(dockState.workbenchOpen.value).toBe(true)

    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }))
    expect(dockState.basketOpen.value).toBe(false)
    expect(dockState.interceptDrawerOpen.value).toBe(true)
    expect(dockState.workbenchOpen.value).toBe(true)

    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }))
    expect(dockState.interceptDrawerOpen.value).toBe(false)
    expect(dockState.workbenchOpen.value).toBe(true)

    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }))
    expect(dockState.workbenchOpen.value).toBe(false)

    wrapper.unmount()
  })
})
