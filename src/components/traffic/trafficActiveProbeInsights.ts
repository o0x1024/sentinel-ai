import type { ActiveProbeEntry } from './trafficActiveProbeTypes'

export interface ActiveProbeCooldownGroup {
  key: string
  label: string
  maxQueueDepth: number
  maxActiveSlots: number
  maxConcurrentPerHost: number
  latestUpdatedAt: number
  sampleCount: number
  averageWaitMs: number
  averageResponseMs: number
  failureRate: number
  latestTrafficRequestId: string | null
}

export interface ActiveProbeFunnelSummary {
  scanCount: number
  targetCount: number
  replayCount: number
  findingCount: number
}

export function formatCooldownKeyLabel(key: string, maxLength = 48) {
  if (!key) {
    return 'global'
  }

  return key.length > maxLength ? `${key.slice(0, maxLength - 3)}...` : key
}

export function filterActiveProbeEntries(
  entries: ActiveProbeEntry[],
  options: {
    pluginId?: string | null
    cooldownKey?: string | null
  } = {},
) {
  return entries.filter((entry) => {
    if (options.pluginId && entry.plugin_id !== options.pluginId) {
      return false
    }
    if (options.cooldownKey && entry.cooldown_key !== options.cooldownKey) {
      return false
    }
    return true
  })
}

export function buildActiveProbeCooldownGroups(entries: ActiveProbeEntry[]): ActiveProbeCooldownGroup[] {
  const groups = new Map<string, {
    key: string
    label: string
    maxQueueDepth: number
    maxActiveSlots: number
    maxConcurrentPerHost: number
    latestUpdatedAt: number
    sampleCount: number
    waitSamples: number[]
    responseSamples: number[]
    failureCount: number
    latestTrafficRequestId: string | null
  }>()

  for (const entry of entries) {
    if (!entry.cooldown_key) {
      continue
    }

    const current = groups.get(entry.cooldown_key) ?? {
      key: entry.cooldown_key,
      label: formatCooldownKeyLabel(entry.cooldown_key),
      maxQueueDepth: 0,
      maxActiveSlots: 0,
      maxConcurrentPerHost: 0,
      latestUpdatedAt: 0,
      sampleCount: 0,
      waitSamples: [],
      responseSamples: [],
      failureCount: 0,
      latestTrafficRequestId: null,
    }

    current.maxQueueDepth = Math.max(current.maxQueueDepth, entry.queue_depth ?? 0)
    current.maxActiveSlots = Math.max(current.maxActiveSlots, entry.active_slots ?? 0)
    current.maxConcurrentPerHost = Math.max(
      current.maxConcurrentPerHost,
      entry.max_concurrent_per_host ?? 0,
    )
    current.latestUpdatedAt = Math.max(current.latestUpdatedAt, entry.lastUpdatedAt)
    current.sampleCount += 1
    if (typeof entry.total_wait_ms === 'number') {
      current.waitSamples.push(entry.total_wait_ms)
    }
    if (typeof entry.response_elapsed_ms === 'number') {
      current.responseSamples.push(entry.response_elapsed_ms)
    }
    if (entry.phase === 'failed') {
      current.failureCount += 1
    }
    if (!current.latestTrafficRequestId && entry.traffic_request_id) {
      current.latestTrafficRequestId = entry.traffic_request_id
    }
    groups.set(entry.cooldown_key, current)
  }

  return [...groups.values()]
    .map((group) => ({
      key: group.key,
      label: group.label,
      maxQueueDepth: group.maxQueueDepth,
      maxActiveSlots: group.maxActiveSlots,
      maxConcurrentPerHost: group.maxConcurrentPerHost,
      latestUpdatedAt: group.latestUpdatedAt,
      sampleCount: group.sampleCount,
      averageWaitMs: average(group.waitSamples),
      averageResponseMs: average(group.responseSamples),
      failureRate: group.sampleCount > 0 ? group.failureCount / group.sampleCount : 0,
      latestTrafficRequestId: group.latestTrafficRequestId,
    }))
    .filter((group) => group.maxQueueDepth > 0 || group.maxActiveSlots > 0 || group.sampleCount > 0)
    .sort((left, right) => {
      if (right.maxQueueDepth !== left.maxQueueDepth) {
        return right.maxQueueDepth - left.maxQueueDepth
      }
      if (right.maxActiveSlots !== left.maxActiveSlots) {
        return right.maxActiveSlots - left.maxActiveSlots
      }
      return right.latestUpdatedAt - left.latestUpdatedAt
    })
}

export function buildActiveProbeFunnel(entries: ActiveProbeEntry[]): ActiveProbeFunnelSummary {
  const replayRequests = new Set<string>()
  let scanCount = 0
  let targetCount = 0
  let findingCount = 0

  for (const entry of entries) {
    if (entry.phase === 'plugin_invoked') {
      scanCount += 1
    }
    if (entry.phase === 'scan_started' && typeof entry.target_count === 'number') {
      targetCount += entry.target_count
    }
    if (
      entry.target_path
      && ['dispatching', 'completed', 'failed'].includes(entry.phase)
    ) {
      replayRequests.add(entry.request_id)
    }
    if (entry.phase === 'plugin_completed' && typeof entry.target_count === 'number') {
      findingCount += entry.target_count
    }
  }

  return {
    scanCount,
    targetCount,
    replayCount: replayRequests.size,
    findingCount,
  }
}

export function buildActiveProbeTimeline(entries: ActiveProbeEntry[], limit = 8) {
  return [...entries]
    .sort((left, right) => right.lastUpdatedAt - left.lastUpdatedAt)
    .slice(0, limit)
}

export function buildActiveProbePluginOptions(entries: ActiveProbeEntry[]) {
  return [...new Set(entries.map(entry => entry.plugin_id).filter((value): value is string => Boolean(value)))]
    .sort((left, right) => left.localeCompare(right))
}

function average(values: number[]) {
  if (values.length === 0) {
    return 0
  }
  return Math.round(values.reduce((sum, value) => sum + value, 0) / values.length)
}
