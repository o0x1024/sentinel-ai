import { nextTick, type Ref } from 'vue'
import type { TrafficContextCandidateEvidenceSelection } from '../../trafficContextCandidateTypes'
import type { ProxyRequest } from '../../proxyHistoryTypes'

interface HistoryPaneHandle {
  openRequestById?: (
    requestId: number,
    matchedLocations: string[],
    pane: 'request' | 'response',
    searchTerms: string[],
  ) => Promise<void> | void
}

export function useTrafficWorkbenchHistoryNavigation(options: {
  proxyHistoryRef: Ref<HistoryPaneHandle | null>
  closeBasketDrawer: () => void
  closeWorkbench: () => void
  ensurePreview: (trafficRequestId: string) => Promise<ProxyRequest | null>
}) {
  const { proxyHistoryRef, closeBasketDrawer, closeWorkbench, ensurePreview } = options

  async function openHistoryRequest(payload: TrafficContextCandidateEvidenceSelection) {
    if (!Number.isFinite(payload.requestId)) {
      return
    }

    closeBasketDrawer()
    await nextTick()
    await proxyHistoryRef.value?.openRequestById?.(
      payload.requestId,
      payload.matchedLocations,
      payload.pane || 'request',
      payload.searchTerms || [],
    )
  }

  async function openHistoryRequestById(requestId: number) {
    await openHistoryRequest({
      requestId,
      pane: 'request',
      matchedLocations: [],
      searchTerms: [],
    })
  }

  async function openHistoryRequestByTrafficRequestId(trafficRequestId: string) {
    if (!trafficRequestId) {
      return
    }

    const request = await ensurePreview(trafficRequestId)
    if (!request) {
      return
    }

    await openHistoryRequestById(request.id)
  }

  async function openHistoryRequestFromBasket(requestId: number) {
    closeBasketDrawer()
    await openHistoryRequestById(requestId)
  }

  async function openHistoryRequestFromOast(requestId: number) {
    closeWorkbench()
    await nextTick()
    await openHistoryRequestById(requestId)
  }

  return {
    openHistoryRequest,
    openHistoryRequestById,
    openHistoryRequestByTrafficRequestId,
    openHistoryRequestFromBasket,
    openHistoryRequestFromOast,
  }
}
