import type { TrafficPluginRuntimeQueueStats } from '@/components/traffic/trafficPluginRuntimeQueueTypes'

export type PluginRuntimeSummaryLevel = 'neutral' | 'warning' | 'error'

export function getPendingPressureLevel(
  stats: Pick<TrafficPluginRuntimeQueueStats, 'pendingCount' | 'configuredMaxQueueDepth'>
): PluginRuntimeSummaryLevel {
  const limit = Math.max(1, stats.configuredMaxQueueDepth)
  const ratio = stats.pendingCount / limit

  if (ratio >= 0.9) {
    return 'error'
  }
  if (ratio >= 0.75) {
    return 'warning'
  }
  return 'neutral'
}

export function getRejectedTrendLevel(rejectedSeries: number[]): PluginRuntimeSummaryLevel {
  if (rejectedSeries.length < 2) {
    return 'neutral'
  }

  const recent = rejectedSeries.slice(-4)
  const deltas: number[] = []
  for (let index = 1; index < recent.length; index += 1) {
    deltas.push(Math.max(0, recent[index] - recent[index - 1]))
  }

  const totalGrowth = deltas.reduce((sum, value) => sum + value, 0)
  const positiveSteps = deltas.filter(value => value > 0).length

  if (totalGrowth >= 3 || positiveSteps >= 3) {
    return 'error'
  }
  if (totalGrowth > 0) {
    return 'warning'
  }
  return 'neutral'
}

export function getRuntimeSummaryChipClass(level: PluginRuntimeSummaryLevel): string {
  switch (level) {
    case 'error':
      return 'border border-error/30 bg-error/12 text-error'
    case 'warning':
      return 'border border-warning/30 bg-warning/12 text-warning'
    default:
      return 'bg-base-200/70 text-base-content/70'
  }
}
