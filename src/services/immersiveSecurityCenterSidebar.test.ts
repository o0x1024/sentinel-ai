import { beforeEach, describe, expect, it } from 'vitest'
import { setImmersiveDrillModeEnabled } from './immersiveDrillMode'
import { immersiveMinimizedToolOrder, resetImmersiveMinimizedToolTray } from './immersiveMinimizedToolTray'
import {
  clearImmersiveSecurityCenterReturnPath,
  closeImmersiveSecurityCenterSidebar,
  immersiveSecurityCenterSidebarMinimized,
  immersiveSecurityCenterSidebarOpen,
  immersiveSecurityCenterReturnPath,
  minimizeImmersiveSecurityCenterSidebar,
  openImmersiveSecurityCenterSidebar,
  restoreImmersiveSecurityCenterSidebar,
  toggleImmersiveSecurityCenterSidebar,
} from './immersiveSecurityCenterSidebar'
import {
  openImmersiveTrafficWorkbenchTool,
  resetImmersiveTrafficDockState,
  useImmersiveTrafficDockState,
} from '@/components/traffic/immersiveTrafficDockState'
import {
  closeTrafficAssistant,
  openTrafficAssistantPanel,
  trafficAssistantOpen,
} from './trafficAssistantWorkspace'

describe('immersiveSecurityCenterSidebar', () => {
  beforeEach(() => {
    closeImmersiveSecurityCenterSidebar()
    clearImmersiveSecurityCenterReturnPath()
    closeTrafficAssistant()
    resetImmersiveTrafficDockState()
    resetImmersiveMinimizedToolTray()
    setImmersiveDrillModeEnabled(false)
  })

  it('opens the immersive security center sidebar', () => {
    openImmersiveSecurityCenterSidebar()
    expect(immersiveSecurityCenterSidebarOpen.value).toBe(true)
  })

  it('toggles the immersive security center sidebar', () => {
    toggleImmersiveSecurityCenterSidebar()
    expect(immersiveSecurityCenterSidebarOpen.value).toBe(true)

    toggleImmersiveSecurityCenterSidebar()
    expect(immersiveSecurityCenterSidebarOpen.value).toBe(false)
  })

  it('keeps track of the return path for floating mode', () => {
    openImmersiveSecurityCenterSidebar('/traffic')

    expect(immersiveSecurityCenterSidebarOpen.value).toBe(true)
    expect(immersiveSecurityCenterReturnPath.value).toBe('/traffic')
  })

  it('supports minimizing and restoring the floating workspace', () => {
    openImmersiveSecurityCenterSidebar('/traffic')
    minimizeImmersiveSecurityCenterSidebar()

    expect(immersiveSecurityCenterSidebarOpen.value).toBe(false)
    expect(immersiveSecurityCenterSidebarMinimized.value).toBe(true)
    expect(immersiveMinimizedToolOrder.value).toEqual(['security-center'])

    restoreImmersiveSecurityCenterSidebar()
    expect(immersiveSecurityCenterSidebarOpen.value).toBe(true)
    expect(immersiveSecurityCenterSidebarMinimized.value).toBe(false)
    expect(immersiveMinimizedToolOrder.value).toEqual([])
  })

  it('closes other immersive tools when opened in drill mode', () => {
    const { workbenchOpen } = useImmersiveTrafficDockState()

    setImmersiveDrillModeEnabled(true)
    openTrafficAssistantPanel()
    openImmersiveTrafficWorkbenchTool('intruder')

    openImmersiveSecurityCenterSidebar('/traffic')

    expect(immersiveSecurityCenterSidebarOpen.value).toBe(true)
    expect(workbenchOpen.value).toBe(false)
    expect(trafficAssistantOpen.value).toBe(false)
  })

  it('keeps a minimized security center in the tray when another tool opens', () => {
    setImmersiveDrillModeEnabled(true)

    openImmersiveSecurityCenterSidebar('/traffic')
    minimizeImmersiveSecurityCenterSidebar()
    openTrafficAssistantPanel()

    expect(immersiveSecurityCenterSidebarMinimized.value).toBe(true)
    expect(immersiveMinimizedToolOrder.value).toEqual(['security-center'])
    expect(trafficAssistantOpen.value).toBe(true)
  })
})
