import { ref } from 'vue'
import { getProxyRequestByTrafficRequestId } from '@/api/trafficHistory'
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

  async function ensurePreview(trafficRequestId: string) {
    if (!trafficRequestId || previewRequests.value[trafficRequestId]) {
      return previewRequests.value[trafficRequestId] ?? null
    }

    previewLoadingId.value = trafficRequestId
    try {
      const request = await getProxyRequestByTrafficRequestId(trafficRequestId)
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
