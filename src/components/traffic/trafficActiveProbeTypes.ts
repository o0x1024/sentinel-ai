export type ActiveProbeQueuePhase =
  | 'queued'
  | 'scheduled'
  | 'running'
  | 'completed'
  | 'failed'
  | 'cancelled'

export interface ActiveProbeQueueEntry {
  plugin_id: string
  traffic_request_id: string
  request_id: string
  phase: ActiveProbeQueuePhase
  method: string
  url: string
  probe_label?: string | null
  target_name?: string | null
  target_path?: string | null
  target_location?: string | null
  probe_value?: string | null
  technique?: string | null
  probe_class: string
  probe_priority: number
  cooldown_key: string
  cooldown_wait_ms?: number | null
  jitter_wait_ms?: number | null
  total_wait_ms?: number | null
  adaptive_penalty_ms: number
  status?: number | null
  error?: string | null
  reason?: string | null
  active_slots: number
  max_concurrent_per_host: number
  queue_depth: number
  response_elapsed_ms?: number | null
  queued_at: string
  scheduled_at?: string | null
  dispatch_started_at?: string | null
  finished_at?: string | null
  updated_at: string
}

export interface ActiveProbeQueueSnapshot {
  pending: ActiveProbeQueueEntry[]
  running: ActiveProbeQueueEntry[]
  recent: ActiveProbeQueueEntry[]
}

export type ActiveProbePhase = ActiveProbeQueuePhase | string

export interface ActiveProbeEntry
  extends Omit<ActiveProbeQueueEntry, 'phase'> {
  phase: ActiveProbePhase
  lastUpdatedAt: number
  startedAt: number
  target_count?: number | null
  timestamp?: string | null
}

export type ActiveProbeEventPayload = ActiveProbeEntry

const ACTIVE_PROBE_PHASES: ActiveProbeQueuePhase[] = [
  'queued',
  'scheduled',
  'running',
  'completed',
  'failed',
  'cancelled',
]

function normalizeNullableString(value: unknown): string | null {
  return typeof value === 'string' && value.trim() ? value : null
}

function normalizeNumber(value: unknown, fallback = 0): number {
  return typeof value === 'number' && Number.isFinite(value) ? value : fallback
}

export function normalizeActiveProbeQueueEntry(payload: unknown): ActiveProbeQueueEntry | null {
  if (!payload || typeof payload !== 'object') {
    return null
  }

  const candidate = payload as Record<string, unknown>
  const requestId = normalizeNullableString(candidate.request_id)
  const phase = normalizeNullableString(candidate.phase)
  const method = normalizeNullableString(candidate.method)
  const url = normalizeNullableString(candidate.url)
  const pluginId = normalizeNullableString(candidate.plugin_id)
  const trafficRequestId = normalizeNullableString(candidate.traffic_request_id)
  const cooldownKey = normalizeNullableString(candidate.cooldown_key)
  const queuedAt = normalizeNullableString(candidate.queued_at)
  const updatedAt = normalizeNullableString(candidate.updated_at)

  if (
    !requestId
    || !phase
    || !ACTIVE_PROBE_PHASES.includes(phase as ActiveProbeQueuePhase)
    || !method
    || !url
    || !pluginId
    || !trafficRequestId
    || !cooldownKey
    || !queuedAt
    || !updatedAt
  ) {
    return null
  }

  return {
    plugin_id: pluginId,
    traffic_request_id: trafficRequestId,
    request_id: requestId,
    phase: phase as ActiveProbeQueuePhase,
    method,
    url,
    probe_label: normalizeNullableString(candidate.probe_label),
    target_name: normalizeNullableString(candidate.target_name),
    target_path: normalizeNullableString(candidate.target_path),
    target_location: normalizeNullableString(candidate.target_location),
    probe_value: normalizeNullableString(candidate.probe_value),
    technique: normalizeNullableString(candidate.technique),
    probe_class: normalizeNullableString(candidate.probe_class) || 'fast',
    probe_priority: normalizeNumber(candidate.probe_priority),
    cooldown_key: cooldownKey,
    cooldown_wait_ms:
      candidate.cooldown_wait_ms == null ? null : normalizeNumber(candidate.cooldown_wait_ms),
    jitter_wait_ms:
      candidate.jitter_wait_ms == null ? null : normalizeNumber(candidate.jitter_wait_ms),
    total_wait_ms:
      candidate.total_wait_ms == null ? null : normalizeNumber(candidate.total_wait_ms),
    adaptive_penalty_ms: normalizeNumber(candidate.adaptive_penalty_ms),
    status: candidate.status == null ? null : normalizeNumber(candidate.status),
    error: normalizeNullableString(candidate.error),
    reason: normalizeNullableString(candidate.reason),
    active_slots: normalizeNumber(candidate.active_slots),
    max_concurrent_per_host: normalizeNumber(candidate.max_concurrent_per_host, 1),
    queue_depth: normalizeNumber(candidate.queue_depth),
    response_elapsed_ms:
      candidate.response_elapsed_ms == null
        ? null
        : normalizeNumber(candidate.response_elapsed_ms),
    queued_at: queuedAt,
    scheduled_at: normalizeNullableString(candidate.scheduled_at),
    dispatch_started_at: normalizeNullableString(candidate.dispatch_started_at),
    finished_at: normalizeNullableString(candidate.finished_at),
    updated_at: updatedAt,
  }
}

export function normalizeActiveProbeQueueSnapshot(payload: unknown): ActiveProbeQueueSnapshot | null {
  if (!payload || typeof payload !== 'object') {
    return null
  }

  const candidate = payload as Record<string, unknown>
  const pending = Array.isArray(candidate.pending)
    ? candidate.pending.map(normalizeActiveProbeQueueEntry).filter(Boolean) as ActiveProbeQueueEntry[]
    : []
  const running = Array.isArray(candidate.running)
    ? candidate.running.map(normalizeActiveProbeQueueEntry).filter(Boolean) as ActiveProbeQueueEntry[]
    : []
  const recent = Array.isArray(candidate.recent)
    ? candidate.recent.map(normalizeActiveProbeQueueEntry).filter(Boolean) as ActiveProbeQueueEntry[]
    : []

  return { pending, running, recent }
}
