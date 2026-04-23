import { invoke } from '@tauri-apps/api/core'
import type {
  DeleteTrafficOastEventsResult,
  DeleteTrafficOastRecordResult,
  TrafficOastConfig,
  TrafficOastEventKey,
  TrafficOastRecord,
  TrafficOastTestResult,
} from '@/components/traffic/proxyConfigurationTypes'

export interface CreateTrafficOastTokenInput {
  label?: string
  sourceTool?: string
  sourceRequestId?: number | null
}

export async function getTrafficOastConfig(): Promise<TrafficOastConfig> {
  const response = await invoke<{ success: boolean; data?: TrafficOastConfig; error?: string }>('get_traffic_oast_config')
  if (!response.success || !response.data) {
    throw new Error(response.error || 'Failed to load traffic OAST config')
  }
  return response.data
}

export async function createTrafficOastToken(
  payload: CreateTrafficOastTokenInput,
): Promise<TrafficOastRecord> {
  const response = await invoke<{ success: boolean; data?: TrafficOastRecord; error?: string }>(
    'create_traffic_oast_token',
    {
      payload: {
        label: payload.label ?? null,
        sourceTool: payload.sourceTool ?? null,
        sourceRequestId: payload.sourceRequestId ?? null,
      },
    },
  )
  if (!response.success || !response.data) {
    throw new Error(response.error || 'Failed to create OAST token')
  }
  return response.data
}

export async function listTrafficOastRecords(): Promise<TrafficOastRecord[]> {
  const response = await invoke<{ success: boolean; data?: TrafficOastRecord[]; error?: string }>(
    'list_traffic_oast_records',
  )
  if (!response.success || !response.data) {
    throw new Error(response.error || 'Failed to list OAST records')
  }
  return response.data
}

export async function syncTrafficOastRecords(): Promise<TrafficOastRecord[]> {
  const response = await invoke<{ success: boolean; data?: TrafficOastRecord[]; error?: string }>(
    'sync_traffic_oast_records',
  )
  if (!response.success || !response.data) {
    throw new Error(response.error || 'Failed to sync OAST records')
  }
  return response.data
}

export async function deleteTrafficOastRecord(token: string): Promise<DeleteTrafficOastRecordResult> {
  const response = await invoke<{ success: boolean; data?: DeleteTrafficOastRecordResult; error?: string }>(
    'delete_traffic_oast_record',
    { token },
  )
  if (!response.success || !response.data) {
    throw new Error(response.error || 'Failed to delete OAST record')
  }
  return response.data
}

export async function deleteTrafficOastEvents(
  token: string,
  eventKeys: TrafficOastEventKey[],
): Promise<DeleteTrafficOastEventsResult> {
  const response = await invoke<{ success: boolean; data?: DeleteTrafficOastEventsResult; error?: string }>(
    'delete_traffic_oast_events_command',
    {
      payload: {
        token,
        eventKeys,
      },
    },
  )
  if (!response.success || !response.data) {
    throw new Error(response.error || 'Failed to delete OAST events')
  }
  return response.data
}

export async function testTrafficOastConfig(
  config: TrafficOastConfig,
): Promise<TrafficOastTestResult> {
  const response = await invoke<{ success: boolean; data?: TrafficOastTestResult; error?: string }>(
    'test_traffic_oast_config_command',
    {
      payload: { config },
    },
  )
  if (!response.success || !response.data) {
    throw new Error(response.error || 'Failed to test OAST config')
  }
  return response.data
}
