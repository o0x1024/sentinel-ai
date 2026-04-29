import type { ActiveProbeQueueEntry } from './trafficActiveProbeTypes'

export const ACTIVE_PROBE_INITIAL_QUEUE_LIMIT = 24
export const ACTIVE_PROBE_INITIAL_RECENT_LIMIT = 16
export const ACTIVE_PROBE_QUEUE_PAGE_SIZE = 24
export const ACTIVE_PROBE_RECENT_PAGE_SIZE = 16

export interface ActiveProbePanelDisplayEntry {
  entry: ActiveProbeQueueEntry
  phaseLabel: string
  pathLabel: string
  queueMeta: string
  recentMeta: string
  urlSummary: string
  finishedTimeLabel: string
}

export interface ActiveProbePanelHotPath {
  key: string
  label: string
  pending: number
  running: number
  recent: number
}

export interface ActiveProbePanelDerivedState {
  pluginOptions: string[]
  hotPaths: ActiveProbePanelHotPath[]
  filteredPendingCount: number
  filteredRunningCount: number
  filteredRecentCount: number
  queueHiddenCount: number
  recentHiddenCount: number
  completedCount: number
  failedCount: number
  latestFilteredTrafficRequestId: string
  visibleQueueEntries: ActiveProbePanelDisplayEntry[]
  visibleRecentEntries: ActiveProbePanelDisplayEntry[]
}

const EMPTY_DERIVED_STATE: ActiveProbePanelDerivedState = {
  pluginOptions: [],
  hotPaths: [],
  filteredPendingCount: 0,
  filteredRunningCount: 0,
  filteredRecentCount: 0,
  queueHiddenCount: 0,
  recentHiddenCount: 0,
  completedCount: 0,
  failedCount: 0,
  latestFilteredTrafficRequestId: '',
  visibleQueueEntries: [],
  visibleRecentEntries: [],
}

export function buildActiveProbePanelDerivedState(params: {
  collapsed: boolean
  pendingEntries: ActiveProbeQueueEntry[]
  runningEntries: ActiveProbeQueueEntry[]
  recentEntries: ActiveProbeQueueEntry[]
  selectedPluginId: string | null
  selectedCooldownKey: string | null
  visibleQueueLimit: number
  visibleRecentLimit: number
}): ActiveProbePanelDerivedState {
  if (params.collapsed) {
    return EMPTY_DERIVED_STATE
  }

  const allEntries = [
    ...params.runningEntries,
    ...params.pendingEntries,
    ...params.recentEntries,
  ]
  const urlSummaryCache = new Map<string, { pathLabel: string; urlSummary: string }>()
  const pluginOptions = [...new Set(allEntries.map(entry => entry.plugin_id).filter(Boolean))].sort()
  const completedCount = params.recentEntries.filter(entry => entry.phase === 'completed').length
  const failedCount = params.recentEntries.filter(entry => entry.phase === 'failed').length

  const filteredPendingEntries = filterEntries(
    params.pendingEntries,
    params.selectedPluginId,
    params.selectedCooldownKey,
  )
  const filteredRunningEntries = filterEntries(
    params.runningEntries,
    params.selectedPluginId,
    params.selectedCooldownKey,
  )
  const filteredRecentEntries = filterEntries(
    params.recentEntries,
    params.selectedPluginId,
    params.selectedCooldownKey,
  )
  const queueEntries = [...filteredRunningEntries, ...filteredPendingEntries]
  const visibleQueueEntries = queueEntries
    .slice(0, params.visibleQueueLimit)
    .map(entry => buildDisplayEntry(entry, urlSummaryCache))
  const visibleRecentEntries = filteredRecentEntries
    .slice(0, params.visibleRecentLimit)
    .map(entry => buildDisplayEntry(entry, urlSummaryCache))
  const latestFilteredTrafficRequestId = [...queueEntries, ...filteredRecentEntries]
    .find(entry => entry.traffic_request_id)?.traffic_request_id || ''

  return {
    pluginOptions,
    hotPaths: buildHotPaths(allEntries),
    filteredPendingCount: filteredPendingEntries.length,
    filteredRunningCount: filteredRunningEntries.length,
    filteredRecentCount: filteredRecentEntries.length,
    queueHiddenCount: Math.max(0, queueEntries.length - visibleQueueEntries.length),
    recentHiddenCount: Math.max(0, filteredRecentEntries.length - visibleRecentEntries.length),
    completedCount,
    failedCount,
    latestFilteredTrafficRequestId,
    visibleQueueEntries,
    visibleRecentEntries,
  }
}

export function formatCooldownKeyLabel(key: string) {
  if (!key) {
    return 'global'
  }
  return key.length > 44 ? `${key.slice(0, 41)}...` : key
}

function filterEntries(
  entries: ActiveProbeQueueEntry[],
  selectedPluginId: string | null,
  selectedCooldownKey: string | null,
) {
  return entries.filter(entry => {
    if (selectedPluginId && entry.plugin_id !== selectedPluginId) {
      return false
    }
    if (selectedCooldownKey && entry.cooldown_key !== selectedCooldownKey) {
      return false
    }
    return true
  })
}

function buildHotPaths(entries: ActiveProbeQueueEntry[]): ActiveProbePanelHotPath[] {
  const map = new Map<string, ActiveProbePanelHotPath>()
  for (const entry of entries) {
    const current = map.get(entry.cooldown_key) || {
      key: entry.cooldown_key,
      label: formatCooldownKeyLabel(entry.cooldown_key),
      pending: 0,
      running: 0,
      recent: 0,
    }
    if (entry.phase === 'queued' || entry.phase === 'scheduled') {
      current.pending += 1
    } else if (entry.phase === 'running') {
      current.running += 1
    } else {
      current.recent += 1
    }
    map.set(entry.cooldown_key, current)
  }

  return [...map.values()]
    .sort((left, right) => (
      (right.pending + right.running + right.recent) - (left.pending + left.running + left.recent)
    ))
    .slice(0, 4)
}

function buildDisplayEntry(
  entry: ActiveProbeQueueEntry,
  urlSummaryCache: Map<string, { pathLabel: string; urlSummary: string }>,
): ActiveProbePanelDisplayEntry {
  const cachedSummary = getUrlSummary(entry.url, urlSummaryCache)
  return {
    entry,
    phaseLabel: formatPhase(entry.phase),
    pathLabel: cachedSummary.pathLabel,
    queueMeta: formatQueueMeta(entry),
    recentMeta: formatRecentMeta(entry),
    urlSummary: cachedSummary.urlSummary,
    finishedTimeLabel: formatTime(entry.finished_at || entry.updated_at),
  }
}

function getUrlSummary(
  rawUrl: string,
  cache: Map<string, { pathLabel: string; urlSummary: string }>,
) {
  const cached = cache.get(rawUrl)
  if (cached) {
    return cached
  }

  let summary = { pathLabel: rawUrl, urlSummary: rawUrl }
  try {
    const parsed = new URL(rawUrl)
    summary = {
      pathLabel: `${parsed.pathname || '/'}${parsed.search || ''}`,
      urlSummary: `${parsed.host}${parsed.pathname}`,
    }
  } catch {
    summary = { pathLabel: rawUrl, urlSummary: rawUrl }
  }

  cache.set(rawUrl, summary)
  return summary
}

function parseTime(value?: string | null): number {
  if (!value) {
    return 0
  }
  const parsed = Date.parse(value)
  return Number.isFinite(parsed) ? parsed : 0
}

function formatPhase(phase: ActiveProbeQueueEntry['phase']) {
  switch (phase) {
    case 'queued':
      return 'Queued'
    case 'scheduled':
      return 'Scheduled'
    case 'running':
      return 'Running'
    case 'completed':
      return 'Completed'
    case 'failed':
      return 'Failed'
    case 'cancelled':
      return 'Cancelled'
  }
}

function formatQueueMeta(entry: ActiveProbeQueueEntry) {
  const waits = []
  if (typeof entry.total_wait_ms === 'number') {
    waits.push(`wait ${entry.total_wait_ms} ms`)
  }
  waits.push(`depth ${entry.queue_depth}`)
  waits.push(`slots ${entry.active_slots}/${entry.max_concurrent_per_host}`)
  if (entry.target_name) {
    waits.push(`target ${entry.target_name}`)
  }
  return waits.join(' · ')
}

function formatRecentMeta(entry: ActiveProbeQueueEntry) {
  const parts = []
  if (typeof entry.status === 'number' && entry.status > 0) {
    parts.push(`status ${entry.status}`)
  }
  if (typeof entry.response_elapsed_ms === 'number') {
    parts.push(`elapsed ${entry.response_elapsed_ms} ms`)
  }
  if (entry.reason) {
    parts.push(entry.reason)
  }
  if (entry.error) {
    parts.push(entry.error)
  }
  if (parts.length === 0) {
    parts.push(`updated ${formatTime(entry.updated_at)}`)
  }
  return parts.join(' · ')
}

function formatTime(value?: string | null) {
  if (!value) {
    return '--'
  }
  const parsed = parseTime(value)
  if (!parsed) {
    return value
  }
  return new Date(parsed).toLocaleTimeString()
}
