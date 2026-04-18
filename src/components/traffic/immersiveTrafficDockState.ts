import { ref } from 'vue'

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
  activeWorkbenchTool.value = tool
  workbenchOpen.value = true
}

export function closeImmersiveTrafficWorkbench() {
  workbenchOpen.value = false
}

export function showImmersiveTrafficHistory() {
  workbenchOpen.value = false
}

export function toggleImmersiveTrafficInterceptDrawer() {
  interceptDrawerOpen.value = !interceptDrawerOpen.value
}

export function toggleImmersiveTrafficBasket() {
  basketOpen.value = !basketOpen.value
}

export function toggleImmersiveTrafficProxySettings() {
  proxySettingsOpen.value = !proxySettingsOpen.value
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
