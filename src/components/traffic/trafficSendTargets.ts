import { computed } from 'vue'
import { useTrafficDisplaySettings } from './trafficDisplaySettings'

export type TrafficSendTarget = 'draft' | 'compare' | 'attackWorkspace'

export function useTrafficSendTargets() {
  const { settings } = useTrafficDisplaySettings()

  const enabledTargets = computed(() => ({
    draft: settings.value.showSendToRepeater,
    compare: settings.value.showSendToComparer,
    attackWorkspace: settings.value.showSendToIntruder,
  }))

  const isTargetEnabled = (target: TrafficSendTarget): boolean => enabledTargets.value[target]

  return {
    enabledTargets,
    isTargetEnabled,
  }
}
