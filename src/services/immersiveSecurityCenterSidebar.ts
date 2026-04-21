import { readonly, ref } from 'vue'
import { immersiveDrillModeEnabled } from './immersiveDrillMode'
import {
  clearImmersiveToolMinimized,
  markImmersiveToolMinimized,
} from './immersiveMinimizedToolTray'
import {
  closeOtherImmersiveTools,
  registerImmersiveToolCloser,
} from './immersiveToolCoordinator'

const immersiveSecurityCenterSidebarOpenState = ref(false)
const immersiveSecurityCenterSidebarMinimizedState = ref(false)
const immersiveSecurityCenterReturnPathState = ref<string | null>(null)

export const immersiveSecurityCenterSidebarOpen = readonly(immersiveSecurityCenterSidebarOpenState)
export const immersiveSecurityCenterSidebarMinimized = readonly(immersiveSecurityCenterSidebarMinimizedState)
export const immersiveSecurityCenterReturnPath = readonly(immersiveSecurityCenterReturnPathState)

export function openImmersiveSecurityCenterSidebar(returnPath?: string | null) {
  if (immersiveDrillModeEnabled.value) {
    closeOtherImmersiveTools('security-center')
  }

  immersiveSecurityCenterSidebarOpenState.value = true
  immersiveSecurityCenterSidebarMinimizedState.value = false
  clearImmersiveToolMinimized('security-center')
  if (typeof returnPath === 'string' && returnPath.trim()) {
    immersiveSecurityCenterReturnPathState.value = returnPath
  }
}

export function closeImmersiveSecurityCenterSidebar() {
  immersiveSecurityCenterSidebarOpenState.value = false
  immersiveSecurityCenterSidebarMinimizedState.value = false
  clearImmersiveToolMinimized('security-center')
}

export function clearImmersiveSecurityCenterReturnPath() {
  immersiveSecurityCenterReturnPathState.value = null
}

export function minimizeImmersiveSecurityCenterSidebar() {
  if (!immersiveSecurityCenterSidebarOpenState.value) {
    return
  }

  immersiveSecurityCenterSidebarOpenState.value = false
  immersiveSecurityCenterSidebarMinimizedState.value = true
  markImmersiveToolMinimized('security-center')
}

export function restoreImmersiveSecurityCenterSidebar() {
  if (
    !immersiveSecurityCenterSidebarMinimizedState.value
    && !immersiveSecurityCenterSidebarOpenState.value
  ) {
    return
  }

  if (immersiveDrillModeEnabled.value) {
    closeOtherImmersiveTools('security-center')
  }

  immersiveSecurityCenterSidebarOpenState.value = true
  immersiveSecurityCenterSidebarMinimizedState.value = false
  clearImmersiveToolMinimized('security-center')
}

export function toggleImmersiveSecurityCenterSidebar() {
  if (!immersiveSecurityCenterSidebarOpenState.value && immersiveDrillModeEnabled.value) {
    closeOtherImmersiveTools('security-center')
  }

  immersiveSecurityCenterSidebarOpenState.value = !immersiveSecurityCenterSidebarOpenState.value
  immersiveSecurityCenterSidebarMinimizedState.value = false
  if (immersiveSecurityCenterSidebarOpenState.value) {
    clearImmersiveToolMinimized('security-center')
  }
}

registerImmersiveToolCloser('security-center', () => {
  closeImmersiveSecurityCenterSidebar()
  clearImmersiveSecurityCenterReturnPath()
}, () => immersiveSecurityCenterSidebarOpenState.value)
