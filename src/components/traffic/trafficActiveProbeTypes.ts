export type ActiveProbePhase =
  | 'queued'
  | 'scheduled'
  | 'dispatching'
  | 'completed'
  | 'failed'
  | 'skipped'
  | 'scan_started'
  | 'plugin_invoked'
  | 'plugin_completed'
  | 'plugin_failed'

export interface ActiveProbeEventPayload {
  plugin_id?: string | null
  traffic_request_id?: string | null
  request_id: string
  phase: ActiveProbePhase | string
  method?: string | null
  url?: string | null
  probe_label?: string | null
  target_name?: string | null
  target_path?: string | null
  target_location?: 'query' | 'body' | string | null
  probe_value?: string | null
  technique?: string | null
  probe_class?: string | null
  probe_priority?: number | null
  cooldown_key?: string | null
  cooldown_wait_ms?: number | null
  jitter_wait_ms?: number | null
  total_wait_ms?: number | null
  adaptive_penalty_ms?: number | null
  status?: number | null
  error?: string | null
  reason?: string | null
  target_count?: number | null
  active_slots?: number | null
  max_concurrent_per_host?: number | null
  queue_depth?: number | null
  response_elapsed_ms?: number | null
  timestamp?: string | null
}

export interface ActiveProbeEntry extends ActiveProbeEventPayload {
  phase: ActiveProbePhase | string
  request_id: string
  url: string
  method: string
  lastUpdatedAt: number
  startedAt: number
}
