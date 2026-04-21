import { beforeEach, describe, expect, it } from 'vitest'
import {
  claimAssistantPresentationTarget,
  releaseAssistantPresentationTarget,
  activeAssistantPresentationTarget,
} from './assistantPresentation'
import { setImmersiveDrillModeEnabled } from './immersiveDrillMode'
import { immersiveMinimizedToolOrder, resetImmersiveMinimizedToolTray } from './immersiveMinimizedToolTray'
import {
  closeImmersiveSecurityCenterSidebar,
  immersiveSecurityCenterSidebarOpen,
  openImmersiveSecurityCenterSidebar,
} from './immersiveSecurityCenterSidebar'
import {
  openImmersiveTrafficWorkbenchTool,
  resetImmersiveTrafficDockState,
  useImmersiveTrafficDockState,
} from '@/components/traffic/immersiveTrafficDockState'
import {
  closeTrafficAssistant,
  minimizeTrafficAssistant,
  openTrafficAssistantImmersive,
  openTrafficAssistantPanel,
  restoreTrafficAssistant,
  setTrafficAssistantDisplayMode,
  trafficAssistantDisplayMode,
  trafficAssistantMinimized,
  trafficAssistantOpen,
} from './trafficAssistantWorkspace'

describe('trafficAssistantWorkspace', () => {
  beforeEach(() => {
    closeTrafficAssistant()
    closeImmersiveSecurityCenterSidebar()
    resetImmersiveTrafficDockState()
    resetImmersiveMinimizedToolTray()
    setImmersiveDrillModeEnabled(false)
    releaseAssistantPresentationTarget('page')
    releaseAssistantPresentationTarget('traffic')
  })

  it('opens side panel and claims traffic presentation target', () => {
    openTrafficAssistantPanel()

    expect(trafficAssistantOpen.value).toBe(true)
    expect(trafficAssistantDisplayMode.value).toBe('panel')
    expect(activeAssistantPresentationTarget.value).toBe('traffic')
  })

  it('switches to immersive mode without releasing ownership', () => {
    openTrafficAssistantPanel()
    setTrafficAssistantDisplayMode('immersive')

    expect(trafficAssistantOpen.value).toBe(true)
    expect(trafficAssistantDisplayMode.value).toBe('immersive')
    expect(activeAssistantPresentationTarget.value).toBe('traffic')
  })

  it('supports minimizing and restoring without losing the selected mode', () => {
    openTrafficAssistantImmersive()

    minimizeTrafficAssistant()
    expect(trafficAssistantOpen.value).toBe(false)
    expect(trafficAssistantMinimized.value).toBe(true)
    expect(trafficAssistantDisplayMode.value).toBe('immersive')
    expect(immersiveMinimizedToolOrder.value).toEqual(['traffic-assistant'])

    restoreTrafficAssistant()
    expect(trafficAssistantOpen.value).toBe(true)
    expect(trafficAssistantMinimized.value).toBe(false)
    expect(trafficAssistantDisplayMode.value).toBe('immersive')
    expect(immersiveMinimizedToolOrder.value).toEqual([])
  })

  it('releases traffic ownership on close without disturbing page ownership', () => {
    openTrafficAssistantImmersive()
    closeTrafficAssistant()

    expect(trafficAssistantOpen.value).toBe(false)
    expect(trafficAssistantDisplayMode.value).toBe('panel')
    expect(activeAssistantPresentationTarget.value).toBe(null)

    claimAssistantPresentationTarget('page')
    closeTrafficAssistant()
    expect(activeAssistantPresentationTarget.value).toBe('page')
  })

  it('closes other immersive tools before opening in drill mode', () => {
    const { workbenchOpen } = useImmersiveTrafficDockState()

    setImmersiveDrillModeEnabled(true)
    openImmersiveSecurityCenterSidebar('/traffic')
    openImmersiveTrafficWorkbenchTool('repeater')

    openTrafficAssistantPanel()

    expect(trafficAssistantOpen.value).toBe(true)
    expect(workbenchOpen.value).toBe(false)
    expect(immersiveSecurityCenterSidebarOpen.value).toBe(false)
  })

  it('does not clear minimized tools when another immersive tool opens', () => {
    setImmersiveDrillModeEnabled(true)

    openTrafficAssistantPanel()
    minimizeTrafficAssistant()
    openImmersiveSecurityCenterSidebar('/traffic')

    expect(trafficAssistantMinimized.value).toBe(true)
    expect(immersiveMinimizedToolOrder.value).toEqual(['traffic-assistant'])
    expect(immersiveSecurityCenterSidebarOpen.value).toBe(true)
  })
})
