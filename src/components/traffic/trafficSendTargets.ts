import { computed } from 'vue'
import { useTrafficDisplaySettings } from './trafficDisplaySettings'

export type TrafficSendTarget = 'repeater' | 'comparer' | 'intruder'

export function useTrafficSendTargets() {
  const { settings } = useTrafficDisplaySettings()

  const enabledTargets = computed(() => ({
    repeater: settings.value.showSendToRepeater,
    comparer: settings.value.showSendToComparer,
    intruder: settings.value.showSendToIntruder,
  }))

  const isTargetEnabled = (target: TrafficSendTarget): boolean => enabledTargets.value[target]

  return {
    enabledTargets,
    isTargetEnabled,
  }
}
