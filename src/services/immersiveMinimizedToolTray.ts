import { computed, readonly, ref } from 'vue'

export type ImmersiveMinimizedToolId = 'security-center' | 'traffic-assistant'

const minimizedToolOrderState = ref<ImmersiveMinimizedToolId[]>([])

export const immersiveMinimizedToolOrder = readonly(minimizedToolOrderState)
export const immersiveMinimizedToolCount = computed(() => minimizedToolOrderState.value.length)

export function markImmersiveToolMinimized(id: ImmersiveMinimizedToolId) {
  minimizedToolOrderState.value = [
    id,
    ...minimizedToolOrderState.value.filter(item => item !== id),
  ]
}

export function clearImmersiveToolMinimized(id: ImmersiveMinimizedToolId) {
  minimizedToolOrderState.value = minimizedToolOrderState.value.filter(item => item !== id)
}

export function resetImmersiveMinimizedToolTray() {
  minimizedToolOrderState.value = []
}
