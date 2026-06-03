import { invoke } from '@tauri-apps/api/core'
import {
  normalizeActiveProbeQueueSnapshot,
  type ActiveProbeQueueSnapshot,
} from '@/components/traffic/trafficActiveProbeTypes'

interface CommandResponse<T> {
  success: boolean
  data?: T
  error?: string
}

export async function getActiveProbeQueueSnapshot(): Promise<ActiveProbeQueueSnapshot> {
  const response = await invoke<CommandResponse<unknown>>('get_active_probe_queue_snapshot')
  if (!response.success) {
    throw new Error(response.error || 'Failed to load active probe queue snapshot')
  }

  const snapshot = normalizeActiveProbeQueueSnapshot(response.data)
  if (!snapshot) {
    throw new Error('Failed to normalize active probe queue snapshot')
  }

  return snapshot
}
