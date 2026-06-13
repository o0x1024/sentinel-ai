import type {
  TrafficPluginRuntimeQueueHistoryPoint,
  TrafficPluginRuntimeQueueHistorySnapshot,
  TrafficPluginRuntimeQueueStatsSnapshot,
  TrafficPluginRuntimePolicyKind,
} from './trafficPluginRuntimeQueueTypes'

export const TRAFFIC_PLUGIN_RUNTIME_HISTORY_LIMIT = 120

const POLICY_KEY_BY_KIND: Record<TrafficPluginRuntimePolicyKind, 'activeProbe'> = {
  traffic_active_probe: 'activeProbe',
}

function trimHistory<T>(points: T[]): T[] {
  if (points.length <= TRAFFIC_PLUGIN_RUNTIME_HISTORY_LIMIT) {
    return points
  }
  return points.slice(points.length - TRAFFIC_PLUGIN_RUNTIME_HISTORY_LIMIT)
}

function createHistoryPoint(
  stats: TrafficPluginRuntimeQueueStatsSnapshot['activeProbe'],
  recordedAt: string
): TrafficPluginRuntimeQueueHistoryPoint {
  return {
    recordedAt,
    pendingCount: stats.pendingCount,
    runningCount: stats.runningCount,
    rejectedTotalCount: stats.rejectedTotalCount,
    recentWindowTotalCount: stats.recentWindowTotalCount,
  }
}

export function createEmptyTrafficPluginRuntimeQueueHistory(): TrafficPluginRuntimeQueueHistorySnapshot {
  return {
    activeProbe: [],
  }
}

export function appendTrafficPluginRuntimeQueueHistory(
  current: TrafficPluginRuntimeQueueHistorySnapshot,
  snapshot: TrafficPluginRuntimeQueueStatsSnapshot,
  recordedAt: string
): TrafficPluginRuntimeQueueHistorySnapshot {
  return {
    activeProbe: trimHistory([
      ...current.activeProbe,
      createHistoryPoint(snapshot.activeProbe, recordedAt),
    ]),
  }
}

export function getTrafficPluginRuntimeHistorySeries(
  history: TrafficPluginRuntimeQueueHistorySnapshot,
  kind: TrafficPluginRuntimePolicyKind,
  metric: keyof Omit<TrafficPluginRuntimeQueueHistoryPoint, 'recordedAt'>
): number[] {
  const key = POLICY_KEY_BY_KIND[kind]
  return history[key].map(point => point[metric])
}

export function buildTrafficPluginRuntimeSparklinePath(
  values: number[],
  width: number,
  height: number,
  padding = 3
): string {
  if (values.length === 0) {
    return ''
  }

  const safeWidth = Math.max(width, padding * 2 + 1)
  const safeHeight = Math.max(height, padding * 2 + 1)
  const minValue = Math.min(...values)
  const maxValue = Math.max(...values)
  const range = Math.max(1, maxValue - minValue)
  const step = values.length <= 1 ? 0 : (safeWidth - padding * 2) / (values.length - 1)

  return values
    .map((value, index) => {
      const x = padding + index * step
      const y = safeHeight - padding - ((value - minValue) / range) * (safeHeight - padding * 2)
      return `${index === 0 ? 'M' : 'L'} ${x.toFixed(2)} ${y.toFixed(2)}`
    })
    .join(' ')
}
