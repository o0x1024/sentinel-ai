import type {
  TrafficPluginRuntimeQueueHistoryPoint,
  TrafficPluginRuntimeQueueHistorySnapshot,
  TrafficPluginRuntimeQueueStats,
  TrafficPluginRuntimeQueueStatsSnapshot,
} from './trafficPluginRuntimeQueueTypes'
import type { TrafficPluginRuntimeSettings } from './proxyConfigurationTypes'
import type { TrafficPluginRuntimePolicyId } from './pluginRuntimeSettingsSupport'
import {
  getPendingPressureLevel,
  getRejectedTrendLevel,
  type PluginRuntimeSummaryLevel,
} from '@/components/PluginManagement/pluginRuntimeSchedulerSummarySupport'
import type { TrafficPluginRuntimePreset } from './pluginRuntimeSettingsSupport'
import {
  createTrafficPluginRuntimePreset,
  normalizeTrafficPluginRuntimeSettings,
} from './pluginRuntimeSettingsSupport'

type PolicySnapshotKey = 'activeProbe'

const POLICY_KEY_BY_ID: Record<TrafficPluginRuntimePolicyId, PolicySnapshotKey> = {
  activeProbe: 'activeProbe',
}

export interface TrafficPluginRuntimeLauncherSummary {
  pendingCount: number
  pendingLimit: number
  runningCount: number
  runningLimit: number
  rejectedCount: number
  rejectedSeries: number[]
}

export interface TrafficPluginRuntimeLauncherPresetRecommendation {
  preset: TrafficPluginRuntimePreset
  severity: Exclude<PluginRuntimeSummaryLevel, 'neutral'>
  reason: string
  alternativeExplanation: string
}

function formatRatio(current: number, limit: number): string {
  const safeLimit = Math.max(1, limit)
  return `${Math.round((current / safeLimit) * 100)}%`
}

function getPendingAction(level: PluginRuntimeSummaryLevel): string {
  switch (level) {
    case 'error':
      return 'action: raise maxQueueDepth or reduce producer rate now'
    case 'warning':
      return 'action: watch queue growth; if it persists, raise maxQueueDepth or reduce producer rate'
    default:
      return 'action: no immediate adjustment needed'
  }
}

function getRunningAction(current: number, limit: number): string {
  const safeLimit = Math.max(1, limit)
  const ratio = current / safeLimit
  if (ratio >= 0.9) {
    return 'action: raise maxGlobalConcurrent if targets can tolerate it, or slow request producers'
  }
  if (ratio >= 0.75) {
    return 'action: monitor saturation; raise maxGlobalConcurrent only if wait time keeps climbing'
  }
  return 'action: no immediate adjustment needed'
}

function getRunningLevel(current: number, limit: number): PluginRuntimeSummaryLevel {
  const safeLimit = Math.max(1, limit)
  const ratio = current / safeLimit
  if (ratio >= 0.9) {
    return 'error'
  }
  if (ratio >= 0.75) {
    return 'warning'
  }
  return 'neutral'
}

function getRejectedAction(level: PluginRuntimeSummaryLevel): string {
  switch (level) {
    case 'error':
      return 'action: inspect queue depth and per-run or per-plugin pending limits; requests are being dropped'
    case 'warning':
      return 'action: review queue and pending limits before rejection growth becomes sustained'
    default:
      return 'action: no immediate adjustment needed'
  }
}

function getRejectedRecentGrowth(series: number[]): { growth: number; sampleCount: number } {
  if (series.length < 2) {
    return { growth: 0, sampleCount: series.length }
  }

  const recent = series.slice(-4)
  let growth = 0
  for (let index = 1; index < recent.length; index += 1) {
    growth += Math.max(0, recent[index] - recent[index - 1])
  }

  return {
    growth,
    sampleCount: recent.length,
  }
}

function sumHistoryMetric(
  history: TrafficPluginRuntimeQueueHistorySnapshot,
  policyIds: TrafficPluginRuntimePolicyId[],
  metric: keyof Omit<TrafficPluginRuntimeQueueHistoryPoint, 'recordedAt'>
): number[] {
  const series = policyIds.map(policyId =>
    history[POLICY_KEY_BY_ID[policyId]].map(point => point[metric])
  )
  const maxLength = series.reduce((max, current) => Math.max(max, current.length), 0)
  if (maxLength === 0) {
    return []
  }

  return Array.from({ length: maxLength }, (_, index) =>
    series.reduce((sum, current) => {
      const alignedIndex = current.length - maxLength + index
      return sum + (alignedIndex >= 0 ? (current[alignedIndex] ?? 0) : 0)
    }, 0)
  )
}

function pickPolicyStats(
  snapshot: TrafficPluginRuntimeQueueStatsSnapshot,
  policyIds: TrafficPluginRuntimePolicyId[]
): TrafficPluginRuntimeQueueStats[] {
  return policyIds.map(policyId => snapshot[POLICY_KEY_BY_ID[policyId]])
}

export function buildTrafficPluginRuntimeLauncherSummary(
  snapshot: TrafficPluginRuntimeQueueStatsSnapshot | null,
  history: TrafficPluginRuntimeQueueHistorySnapshot,
  policyIds: TrafficPluginRuntimePolicyId[]
): TrafficPluginRuntimeLauncherSummary | null {
  if (!snapshot || policyIds.length === 0) {
    return null
  }

  const stats = pickPolicyStats(snapshot, policyIds)

  return {
    pendingCount: stats.reduce((sum, item) => sum + item.pendingCount, 0),
    pendingLimit: stats.reduce((sum, item) => sum + Math.max(1, item.configuredMaxQueueDepth), 0),
    runningCount: stats.reduce((sum, item) => sum + item.runningCount, 0),
    runningLimit: stats.reduce(
      (sum, item) => sum + Math.max(1, item.configuredMaxGlobalConcurrent),
      0
    ),
    rejectedCount: stats.reduce((sum, item) => sum + item.rejectedTotalCount, 0),
    rejectedSeries: sumHistoryMetric(history, policyIds, 'rejectedTotalCount'),
  }
}

function maxSeverity(...levels: PluginRuntimeSummaryLevel[]): PluginRuntimeSummaryLevel {
  if (levels.includes('error')) {
    return 'error'
  }
  if (levels.includes('warning')) {
    return 'warning'
  }
  return 'neutral'
}

export function getTrafficPluginRuntimeLauncherPresetRecommendation(
  summary: TrafficPluginRuntimeLauncherSummary | null
): TrafficPluginRuntimeLauncherPresetRecommendation | null {
  if (!summary) {
    return null
  }

  const pendingLevel = getPendingPressureLevel({
    pendingCount: summary.pendingCount,
    configuredMaxQueueDepth: summary.pendingLimit,
  })
  const runningLevel = getRunningLevel(summary.runningCount, summary.runningLimit)
  const rejectedLevel = getRejectedTrendLevel(summary.rejectedSeries)
  const severity = maxSeverity(pendingLevel, runningLevel, rejectedLevel)

  if (severity === 'neutral') {
    return null
  }

  return {
    preset: 'local_fast',
    severity,
    reason:
      severity === 'error'
        ? '当前队列已经接近或达到上限，建议切到本地快速预设，立即提高吞吐并减少排队或拒绝。'
        : '当前队列压力在上升，建议切到本地快速预设，提前增加吞吐余量。',
    alternativeExplanation:
      '这次不推荐 balanced，因为当前问题已经不是常规负载；也不推荐 conservative，因为它会进一步降低吞吐，更容易放大排队和拒绝。',
  }
}

function pickPolicySettings(
  settings: TrafficPluginRuntimeSettings,
  policyIds: TrafficPluginRuntimePolicyId[]
): Record<string, unknown> {
  const normalized = normalizeTrafficPluginRuntimeSettings(settings)
  const picked: Record<string, unknown> = {}
  for (const policyId of policyIds) {
    picked[policyId] = normalized[policyId]
  }
  return picked
}

export function isTrafficPluginRuntimePresetApplied(
  settings: TrafficPluginRuntimeSettings,
  preset: TrafficPluginRuntimePreset,
  policyIds: TrafficPluginRuntimePolicyId[]
): boolean {
  const presetSettings = createTrafficPluginRuntimePreset(preset)
  return (
    JSON.stringify(pickPolicySettings(settings, policyIds)) ===
    JSON.stringify(pickPolicySettings(presetSettings, policyIds))
  )
}

export function getTrafficPluginRuntimePendingTooltip(
  summary: TrafficPluginRuntimeLauncherSummary
): string {
  const level = getPendingPressureLevel({
    pendingCount: summary.pendingCount,
    configuredMaxQueueDepth: summary.pendingLimit,
  })
  return `pending ${summary.pendingCount}/${summary.pendingLimit}, ratio ${formatRatio(summary.pendingCount, summary.pendingLimit)}; ${getPendingAction(level)}`
}

export function getTrafficPluginRuntimeRunningTooltip(
  summary: TrafficPluginRuntimeLauncherSummary
): string {
  return `running ${summary.runningCount}/${summary.runningLimit}, ratio ${formatRatio(summary.runningCount, summary.runningLimit)}; ${getRunningAction(summary.runningCount, summary.runningLimit)}`
}

export function getTrafficPluginRuntimeRejectedTooltip(
  summary: TrafficPluginRuntimeLauncherSummary
): string {
  const recent = getRejectedRecentGrowth(summary.rejectedSeries)
  const level = getRejectedTrendLevel(summary.rejectedSeries)
  if (recent.sampleCount < 2) {
    return `rejected ${summary.rejectedCount}, no recent trend yet; ${getRejectedAction(level)}`
  }
  return `rejected ${summary.rejectedCount}, +${recent.growth} in last ${recent.sampleCount} samples; ${getRejectedAction(level)}`
}
