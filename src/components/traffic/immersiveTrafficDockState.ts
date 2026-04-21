import { ref } from 'vue'
import { immersiveDrillModeEnabled } from '@/services/immersiveDrillMode'
import {
  closeOtherImmersiveTools,
  registerImmersiveToolCloser,
} from '@/services/immersiveToolCoordinator'

export type ImmersiveTrafficWorkbenchTool = 'repeater' | 'intruder' | 'comparer'

const workbenchOpen = ref(false)
const activeWorkbenchTool = ref<ImmersiveTrafficWorkbenchTool>('repeater')
const interceptDrawerOpen = ref(false)
const proxySettingsOpen = ref(false)
const basketOpen = ref(false)

const repeaterCount = ref(0)
const intruderCount = ref(0)
const comparerCount = ref(0)
const controlInterceptCount = ref(0)
const basketCount = ref(0)

export function useImmersiveTrafficDockState() {
  return {
    workbenchOpen,
    activeWorkbenchTool,
    interceptDrawerOpen,
    proxySettingsOpen,
    basketOpen,
    repeaterCount,
    intruderCount,
    comparerCount,
    controlInterceptCount,
    basketCount,
  }
}

export function openImmersiveTrafficWorkbenchTool(tool: ImmersiveTrafficWorkbenchTool) {
  if (immersiveDrillModeEnabled.value) {
    closeOtherImmersiveTools('traffic-workbench')
  }

  activeWorkbenchTool.value = tool
  workbenchOpen.value = true
  interceptDrawerOpen.value = false
  proxySettingsOpen.value = false
  basketOpen.value = false
}

export function toggleImmersiveTrafficWorkbenchTool(tool: ImmersiveTrafficWorkbenchTool) {
  if (workbenchOpen.value && activeWorkbenchTool.value === tool) {
    workbenchOpen.value = false
    return
  }

  activeWorkbenchTool.value = tool
  workbenchOpen.value = true
}

export function closeImmersiveTrafficWorkbench() {
  workbenchOpen.value = false
}

export function showImmersiveTrafficHistory() {
  workbenchOpen.value = false
  interceptDrawerOpen.value = false
  proxySettingsOpen.value = false
  basketOpen.value = false
}

export function toggleImmersiveTrafficInterceptDrawer() {
  if (interceptDrawerOpen.value) {
    interceptDrawerOpen.value = false
    return
  }

  openImmersiveTrafficInterceptDrawer()
}

export function openImmersiveTrafficInterceptDrawer() {
  if (immersiveDrillModeEnabled.value) {
    closeOtherImmersiveTools('traffic-control')
  }

  workbenchOpen.value = false
  interceptDrawerOpen.value = true
  proxySettingsOpen.value = false
  basketOpen.value = false
}

export function toggleImmersiveTrafficBasket() {
  if (basketOpen.value) {
    basketOpen.value = false
    return
  }

  openImmersiveTrafficBasket()
}

export function openImmersiveTrafficBasket() {
  if (immersiveDrillModeEnabled.value) {
    closeOtherImmersiveTools('traffic-basket')
  }

  workbenchOpen.value = false
  interceptDrawerOpen.value = false
  proxySettingsOpen.value = false
  basketOpen.value = true
}

export function toggleImmersiveTrafficProxySettings() {
  if (proxySettingsOpen.value) {
    proxySettingsOpen.value = false
    return
  }

  openImmersiveTrafficProxySettings()
}

export function openImmersiveTrafficProxySettings() {
  if (immersiveDrillModeEnabled.value) {
    closeOtherImmersiveTools('traffic-settings')
  }

  workbenchOpen.value = false
  interceptDrawerOpen.value = false
  proxySettingsOpen.value = true
  basketOpen.value = false
}

export function syncImmersiveTrafficDockState(payload: {
  workbenchOpen: boolean
  activeWorkbenchTool: ImmersiveTrafficWorkbenchTool
  interceptDrawerOpen: boolean
  proxySettingsOpen: boolean
  basketOpen: boolean
  repeaterCount: number
  intruderCount: number
  comparerCount: number
  controlInterceptCount: number
  basketCount: number
}) {
  workbenchOpen.value = payload.workbenchOpen
  activeWorkbenchTool.value = payload.activeWorkbenchTool
  interceptDrawerOpen.value = payload.interceptDrawerOpen
  proxySettingsOpen.value = payload.proxySettingsOpen
  basketOpen.value = payload.basketOpen
  repeaterCount.value = payload.repeaterCount
  intruderCount.value = payload.intruderCount
  comparerCount.value = payload.comparerCount
  controlInterceptCount.value = payload.controlInterceptCount
  basketCount.value = payload.basketCount
}

export function resetImmersiveTrafficDockState() {
  workbenchOpen.value = false
  activeWorkbenchTool.value = 'repeater'
  interceptDrawerOpen.value = false
  proxySettingsOpen.value = false
  basketOpen.value = false
  repeaterCount.value = 0
  intruderCount.value = 0
  comparerCount.value = 0
  controlInterceptCount.value = 0
  basketCount.value = 0
}

registerImmersiveToolCloser('traffic-workbench', closeImmersiveTrafficWorkbench, () => workbenchOpen.value)
registerImmersiveToolCloser('traffic-control', () => {
  interceptDrawerOpen.value = false
}, () => interceptDrawerOpen.value)
registerImmersiveToolCloser('traffic-basket', () => {
  basketOpen.value = false
}, () => basketOpen.value)
registerImmersiveToolCloser('traffic-settings', () => {
  proxySettingsOpen.value = false
}, () => proxySettingsOpen.value)
