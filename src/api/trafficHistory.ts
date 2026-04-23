import { invoke } from '@tauri-apps/api/core'
import type { ProxyRequest } from '@/components/traffic/proxyHistoryTypes'

export async function resolveProxyHistoryRequestIdByDbRequestId(
  dbRequestId: number,
): Promise<number | null> {
  const response = await invoke<{ success: boolean; data?: number | null; error?: string }>(
    'resolve_proxy_history_request_id_by_db_request_id',
    { dbRequestId },
  )

  if (!response.success) {
    throw new Error(response.error || 'Failed to resolve proxy history request')
  }

  return typeof response.data === 'number' ? response.data : null
}

export async function getProxyRequest(requestId: number): Promise<ProxyRequest | null> {
  const response = await invoke<{ success: boolean; data?: ProxyRequest | null; error?: string }>(
    'get_proxy_request',
    { id: requestId },
  )

  if (!response.success) {
    throw new Error(response.error || 'Failed to load proxy request')
  }

  return response.data || null
}
