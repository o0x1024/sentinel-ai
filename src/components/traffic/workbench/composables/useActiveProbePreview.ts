import { invoke } from '@tauri-apps/api/core'
import { ref } from 'vue'
import type { ProxyRequest } from '../../proxyHistoryTypes'

export function useActiveProbePreview() {
  const previewRequests = ref<Record<string, ProxyRequest | undefined>>({})
  const previewLoadingId = ref<string | null>(null)

  function rememberPreviewRequest(request: ProxyRequest) {
    if (!request.traffic_request_id) {
      return
    }

    previewRequests.value = {
      ...previewRequests.value,
      [request.traffic_request_id]: request,
    }
  }

  async function loadRequestByTrafficRequestId(trafficRequestId: string): Promise<ProxyRequest | null> {
    const listResponse = await invoke<any>('list_proxy_requests', {
      limit: 200,
      offset: 0,
    })

    const summary = listResponse?.success
      ? (listResponse.data as ProxyRequest[]).find(
          request => request.traffic_request_id === trafficRequestId,
        )
      : null

    if (!summary) {
      return null
    }

    const detailResponse = await invoke<any>('get_proxy_request', { id: summary.id })
    if (!detailResponse?.success || !detailResponse.data) {
      return null
    }

    return detailResponse.data as ProxyRequest
  }

  async function ensurePreview(trafficRequestId: string) {
    if (!trafficRequestId || previewRequests.value[trafficRequestId]) {
      return previewRequests.value[trafficRequestId] ?? null
    }

    previewLoadingId.value = trafficRequestId
    try {
      const request = await loadRequestByTrafficRequestId(trafficRequestId)
      if (request) {
        rememberPreviewRequest(request)
      }
      return request
    } catch (error) {
      console.error(`Failed to load active probe preview for ${trafficRequestId}:`, error)
      return null
    } finally {
      if (previewLoadingId.value === trafficRequestId) {
        previewLoadingId.value = null
      }
    }
  }

  return {
    previewRequests,
    previewLoadingId,
    rememberPreviewRequest,
    ensurePreview,
  }
}
