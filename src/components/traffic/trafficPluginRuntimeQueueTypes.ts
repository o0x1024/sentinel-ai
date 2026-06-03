export type TrafficPluginRuntimePolicyKind =
  | 'traffic_active_probe'
  | 'bounty_fetch'
  | 'monitor_fetch'
  | 'agent_fetch'
  | 'plugin_test_fetch'

export interface TrafficPluginRuntimeQueueStats {
  kind: TrafficPluginRuntimePolicyKind
  pendingCount: number
  queuedCount: number
  scheduledCount: number
  runningCount: number
  recentCount: number
  maxQueueDepth: number
  activeGlobal: number
  activeRuns: number
  activePlugins: number
  activeHosts: number
  hottestHost: string | null
  hottestHostActive: number
  cancelledRunCount: number
  configuredMaxQueueDepth: number
  configuredMaxGlobalConcurrent: number
  configuredMaxConcurrentPerHost: number
  configuredMaxConcurrentPerRun: number
  configuredMaxConcurrentPerPlugin: number
  rejectedTotalCount: number
  rejectedCancelledRunCount: number
  rejectedQueueLimitCount: number
  rejectedRunPendingLimitCount: number
  rejectedPluginPendingLimitCount: number
  recentWindowTotalCount: number
  recentWindowCompletedCount: number
  recentWindowFailedCount: number
  recentWindowCancelledCount: number
  recentWindowAvgQueueWaitMs: number
  recentWindowAvgResponseElapsedMs: number
}

export interface TrafficPluginRuntimeQueueStatsSnapshot {
  activeProbe: TrafficPluginRuntimeQueueStats
  bountyFetch: TrafficPluginRuntimeQueueStats
  monitorFetch: TrafficPluginRuntimeQueueStats
  agentFetch: TrafficPluginRuntimeQueueStats
  pluginTestFetch: TrafficPluginRuntimeQueueStats
}

export interface TrafficPluginRuntimeQueueHistoryPoint {
  recordedAt: string
  pendingCount: number
  runningCount: number
  rejectedTotalCount: number
  recentWindowTotalCount: number
}

export interface TrafficPluginRuntimeQueueHistorySnapshot {
  activeProbe: TrafficPluginRuntimeQueueHistoryPoint[]
  bountyFetch: TrafficPluginRuntimeQueueHistoryPoint[]
  monitorFetch: TrafficPluginRuntimeQueueHistoryPoint[]
  agentFetch: TrafficPluginRuntimeQueueHistoryPoint[]
  pluginTestFetch: TrafficPluginRuntimeQueueHistoryPoint[]
}

function normalizeNullableString(value: unknown): string | null {
  return typeof value === 'string' && value.trim() ? value : null
}

function normalizeNumber(value: unknown, fallback = 0): number {
  return typeof value === 'number' && Number.isFinite(value) ? value : fallback
}

function normalizeKind(value: unknown): TrafficPluginRuntimePolicyKind | null {
  switch (value) {
    case 'traffic_active_probe':
    case 'bounty_fetch':
    case 'monitor_fetch':
    case 'agent_fetch':
    case 'plugin_test_fetch':
      return value
    default:
      return null
  }
}

function normalizeStats(payload: unknown): TrafficPluginRuntimeQueueStats | null {
  if (!payload || typeof payload !== 'object') {
    return null
  }

  const candidate = payload as Record<string, unknown>
  const kind = normalizeKind(candidate.kind)
  if (!kind) {
    return null
  }

  return {
    kind,
    pendingCount: normalizeNumber(candidate.pendingCount),
    queuedCount: normalizeNumber(candidate.queuedCount),
    scheduledCount: normalizeNumber(candidate.scheduledCount),
    runningCount: normalizeNumber(candidate.runningCount),
    recentCount: normalizeNumber(candidate.recentCount),
    maxQueueDepth: normalizeNumber(candidate.maxQueueDepth),
    activeGlobal: normalizeNumber(candidate.activeGlobal),
    activeRuns: normalizeNumber(candidate.activeRuns),
    activePlugins: normalizeNumber(candidate.activePlugins),
    activeHosts: normalizeNumber(candidate.activeHosts),
    hottestHost: normalizeNullableString(candidate.hottestHost),
    hottestHostActive: normalizeNumber(candidate.hottestHostActive),
    cancelledRunCount: normalizeNumber(candidate.cancelledRunCount),
    configuredMaxQueueDepth: normalizeNumber(candidate.configuredMaxQueueDepth, 1),
    configuredMaxGlobalConcurrent: normalizeNumber(candidate.configuredMaxGlobalConcurrent, 1),
    configuredMaxConcurrentPerHost: normalizeNumber(candidate.configuredMaxConcurrentPerHost, 1),
    configuredMaxConcurrentPerRun: normalizeNumber(candidate.configuredMaxConcurrentPerRun, 1),
    configuredMaxConcurrentPerPlugin: normalizeNumber(
      candidate.configuredMaxConcurrentPerPlugin,
      1
    ),
    rejectedTotalCount: normalizeNumber(candidate.rejectedTotalCount),
    rejectedCancelledRunCount: normalizeNumber(candidate.rejectedCancelledRunCount),
    rejectedQueueLimitCount: normalizeNumber(candidate.rejectedQueueLimitCount),
    rejectedRunPendingLimitCount: normalizeNumber(candidate.rejectedRunPendingLimitCount),
    rejectedPluginPendingLimitCount: normalizeNumber(candidate.rejectedPluginPendingLimitCount),
    recentWindowTotalCount: normalizeNumber(candidate.recentWindowTotalCount),
    recentWindowCompletedCount: normalizeNumber(candidate.recentWindowCompletedCount),
    recentWindowFailedCount: normalizeNumber(candidate.recentWindowFailedCount),
    recentWindowCancelledCount: normalizeNumber(candidate.recentWindowCancelledCount),
    recentWindowAvgQueueWaitMs: normalizeNumber(candidate.recentWindowAvgQueueWaitMs),
    recentWindowAvgResponseElapsedMs: normalizeNumber(candidate.recentWindowAvgResponseElapsedMs),
  }
}

export function normalizeTrafficPluginRuntimeQueueStatsSnapshot(
  payload: unknown
): TrafficPluginRuntimeQueueStatsSnapshot | null {
  if (!payload || typeof payload !== 'object') {
    return null
  }

  const candidate = payload as Record<string, unknown>
  const activeProbe = normalizeStats(candidate.activeProbe)
  const bountyFetch = normalizeStats(candidate.bountyFetch)
  const monitorFetch = normalizeStats(candidate.monitorFetch)
  const agentFetch = normalizeStats(candidate.agentFetch)
  const pluginTestFetch = normalizeStats(candidate.pluginTestFetch)

  if (!activeProbe || !bountyFetch || !monitorFetch || !agentFetch || !pluginTestFetch) {
    return null
  }

  return {
    activeProbe,
    bountyFetch,
    monitorFetch,
    agentFetch,
    pluginTestFetch,
  }
}
