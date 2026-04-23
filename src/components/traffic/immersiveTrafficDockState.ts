import { ref } from 'vue'
import { registerImmersiveToolCloser } from '@/services/immersiveToolCoordinator'

export type ImmersiveTrafficWorkbenchTool = 'repeater' | 'intruder' | 'comparer' | 'oast'

const workbenchOpen = ref(false)
const activeWorkbenchTool = ref<ImmersiveTrafficWorkbenchTool>('repeater')
const interceptDrawerOpen = ref(false)
const proxySettingsOpen = ref(false)
const basketOpen = ref(false)

const repeaterCount = ref(0)
const intruderCount = ref(0)
const comparerCount = ref(0)
const oastCount = ref(0)
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
    oastCount,
    controlInterceptCount,
    basketCount,
  }
}

export function openImmersiveTrafficWorkbenchTool(tool: ImmersiveTrafficWorkbenchTool) {
  activeWorkbenchTool.value = tool
  workbenchOpen.value = true
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
  interceptDrawerOpen.value = true
}

export function toggleImmersiveTrafficBasket() {
  if (basketOpen.value) {
    basketOpen.value = false
    return
  }

  openImmersiveTrafficBasket()
}

export function openImmersiveTrafficBasket() {
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
  proxySettingsOpen.value = true
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
  oastCount: number
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
  oastCount.value = payload.oastCount
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
  oastCount.value = 0
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
