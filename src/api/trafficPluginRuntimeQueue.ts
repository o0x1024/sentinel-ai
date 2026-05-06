import { invoke } from '@tauri-apps/api/core'
import {
  normalizeTrafficPluginRuntimeQueueStatsSnapshot,
  type TrafficPluginRuntimeQueueStatsSnapshot,
} from '@/components/traffic/trafficPluginRuntimeQueueTypes'

interface CommandResponse<T> {
  success: boolean
  data?: T
  error?: string
}

export async function getTrafficPluginRuntimeQueueStats(): Promise<TrafficPluginRuntimeQueueStatsSnapshot> {
  const response = await invoke<CommandResponse<unknown>>('get_traffic_plugin_runtime_queue_stats')
  if (!response.success) {
    throw new Error(response.error || 'Failed to load traffic plugin runtime queue stats')
  }

  const snapshot = normalizeTrafficPluginRuntimeQueueStatsSnapshot(response.data)
  if (!snapshot) {
    throw new Error('Failed to normalize traffic plugin runtime queue stats')
  }

  return snapshot
}
