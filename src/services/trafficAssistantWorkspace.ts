import { computed, readonly, ref } from 'vue'
import {
  claimAssistantPresentationTarget,
  releaseAssistantPresentationTarget,
} from './assistantPresentation'
import { immersiveDrillModeEnabled } from './immersiveDrillMode'
import {
  clearImmersiveToolMinimized,
  markImmersiveToolMinimized,
} from './immersiveMinimizedToolTray'
import {
  closeOtherImmersiveTools,
  registerImmersiveToolCloser,
} from './immersiveToolCoordinator'

export type TrafficAssistantDisplayMode = 'panel' | 'immersive'

const trafficAssistantOpenState = ref(false)
const trafficAssistantMinimizedState = ref(false)
const trafficAssistantDisplayModeState = ref<TrafficAssistantDisplayMode>('panel')

export const trafficAssistantOpen = readonly(trafficAssistantOpenState)
export const trafficAssistantMinimized = readonly(trafficAssistantMinimizedState)
export const trafficAssistantDisplayMode = readonly(trafficAssistantDisplayModeState)
export const trafficAssistantVisible = computed(() => trafficAssistantOpenState.value)

export function openTrafficAssistantPanel() {
  if (immersiveDrillModeEnabled.value) {
    closeOtherImmersiveTools('traffic-assistant')
  }

  trafficAssistantOpenState.value = true
  trafficAssistantMinimizedState.value = false
  clearImmersiveToolMinimized('traffic-assistant')
  trafficAssistantDisplayModeState.value = 'panel'
  claimAssistantPresentationTarget('traffic')
}

export function openTrafficAssistantImmersive() {
  if (immersiveDrillModeEnabled.value) {
    closeOtherImmersiveTools('traffic-assistant')
  }

  trafficAssistantOpenState.value = true
  trafficAssistantMinimizedState.value = false
  clearImmersiveToolMinimized('traffic-assistant')
  trafficAssistantDisplayModeState.value = 'immersive'
  claimAssistantPresentationTarget('traffic')
}

export function setTrafficAssistantDisplayMode(mode: TrafficAssistantDisplayMode) {
  if (immersiveDrillModeEnabled.value) {
    closeOtherImmersiveTools('traffic-assistant')
  }

  trafficAssistantOpenState.value = true
  trafficAssistantMinimizedState.value = false
  clearImmersiveToolMinimized('traffic-assistant')
  trafficAssistantDisplayModeState.value = mode
  claimAssistantPresentationTarget('traffic')
}

export function minimizeTrafficAssistant() {
  if (!trafficAssistantOpenState.value) {
    return
  }

  trafficAssistantOpenState.value = false
  trafficAssistantMinimizedState.value = true
  markImmersiveToolMinimized('traffic-assistant')
  releaseAssistantPresentationTarget('traffic')
}

export function restoreTrafficAssistant() {
  if (!trafficAssistantMinimizedState.value && !trafficAssistantOpenState.value) {
    return
  }

  if (immersiveDrillModeEnabled.value) {
    closeOtherImmersiveTools('traffic-assistant')
  }

  trafficAssistantOpenState.value = true
  trafficAssistantMinimizedState.value = false
  clearImmersiveToolMinimized('traffic-assistant')
  claimAssistantPresentationTarget('traffic')
}

export function closeTrafficAssistant() {
  trafficAssistantOpenState.value = false
  trafficAssistantMinimizedState.value = false
  clearImmersiveToolMinimized('traffic-assistant')
  trafficAssistantDisplayModeState.value = 'panel'
  releaseAssistantPresentationTarget('traffic')
}

registerImmersiveToolCloser('traffic-assistant', closeTrafficAssistant, () => trafficAssistantOpenState.value)
