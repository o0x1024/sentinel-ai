import { beforeEach, describe, expect, it } from 'vitest'
import {
  closeImmersiveTrafficWorkbench,
  openImmersiveTrafficWorkbenchTool,
  toggleImmersiveTrafficBasket,
  toggleImmersiveTrafficInterceptDrawer,
  toggleImmersiveTrafficPluginsPanel,
  toggleImmersiveTrafficProxySettings,
  resetImmersiveTrafficDockState,
  showImmersiveTrafficHistory,
  toggleImmersiveTrafficWorkbenchTool,
  useImmersiveTrafficDockState,
} from './immersiveTrafficDockState'

describe('immersiveTrafficDockState', () => {
  beforeEach(() => {
    resetImmersiveTrafficDockState()
  })

  it('opens the requested workbench tool', () => {
    const { workbenchOpen, activeWorkbenchTool } = useImmersiveTrafficDockState()

    openImmersiveTrafficWorkbenchTool('intruder')

    expect(workbenchOpen.value).toBe(true)
    expect(activeWorkbenchTool.value).toBe('intruder')
  })

  it('toggles the active tool closed on repeated clicks', () => {
    const { workbenchOpen, activeWorkbenchTool } = useImmersiveTrafficDockState()

    toggleImmersiveTrafficWorkbenchTool('repeater')
    expect(workbenchOpen.value).toBe(true)
    expect(activeWorkbenchTool.value).toBe('repeater')

    toggleImmersiveTrafficWorkbenchTool('repeater')
    expect(workbenchOpen.value).toBe(false)
    expect(activeWorkbenchTool.value).toBe('repeater')
  })

  it('switches to another tool while staying open', () => {
    const { workbenchOpen, activeWorkbenchTool } = useImmersiveTrafficDockState()

    openImmersiveTrafficWorkbenchTool('repeater')
    toggleImmersiveTrafficWorkbenchTool('comparer')

    expect(workbenchOpen.value).toBe(true)
    expect(activeWorkbenchTool.value).toBe('comparer')

    closeImmersiveTrafficWorkbench()
    expect(workbenchOpen.value).toBe(false)
  })

  it('returns to history by closing all immersive traffic overlays', () => {
    const {
      workbenchOpen,
      interceptDrawerOpen,
      proxySettingsOpen,
      trafficPluginsOpen,
      basketOpen,
    } = useImmersiveTrafficDockState()

    openImmersiveTrafficWorkbenchTool('repeater')
    toggleImmersiveTrafficInterceptDrawer()
    toggleImmersiveTrafficProxySettings()
    toggleImmersiveTrafficPluginsPanel()
    toggleImmersiveTrafficBasket()

    showImmersiveTrafficHistory()

    expect(workbenchOpen.value).toBe(false)
    expect(interceptDrawerOpen.value).toBe(false)
    expect(proxySettingsOpen.value).toBe(false)
    expect(trafficPluginsOpen.value).toBe(false)
    expect(basketOpen.value).toBe(false)
  })

  it('keeps traffic workbench and drawers open at the same time', () => {
    const {
      workbenchOpen,
      interceptDrawerOpen,
      proxySettingsOpen,
      trafficPluginsOpen,
      basketOpen,
    } = useImmersiveTrafficDockState()

    openImmersiveTrafficWorkbenchTool('repeater')
    toggleImmersiveTrafficInterceptDrawer()
    expect(workbenchOpen.value).toBe(true)
    expect(interceptDrawerOpen.value).toBe(true)

    toggleImmersiveTrafficProxySettings()
    expect(interceptDrawerOpen.value).toBe(true)
    expect(proxySettingsOpen.value).toBe(true)

    toggleImmersiveTrafficPluginsPanel()
    expect(proxySettingsOpen.value).toBe(true)
    expect(trafficPluginsOpen.value).toBe(true)

    toggleImmersiveTrafficBasket()
    expect(trafficPluginsOpen.value).toBe(true)
    expect(basketOpen.value).toBe(true)
  })

  it('toggles the traffic plugin panel independently', () => {
    const { trafficPluginsOpen } = useImmersiveTrafficDockState()

    toggleImmersiveTrafficPluginsPanel()
    expect(trafficPluginsOpen.value).toBe(true)

    toggleImmersiveTrafficPluginsPanel()
    expect(trafficPluginsOpen.value).toBe(false)
  })
})
