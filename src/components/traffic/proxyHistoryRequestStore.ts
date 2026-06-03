import type { ProxyRequest } from './proxyHistoryTypes'

export type ProxyHistoryInsertPosition = 'prepend' | 'append'

export type MergeProxyHistoryRequestsResult = {
  requests: ProxyRequest[]
  addedRequests: ProxyRequest[]
}

export const dedupeProxyHistoryRequests = (
  requests: ProxyRequest[],
): ProxyRequest[] => {
  const dedupedRequests: ProxyRequest[] = []
  const indexById = new Map<number, number>()

  requests.forEach((request) => {
    const existingIndex = indexById.get(request.id)
    if (existingIndex === undefined) {
      indexById.set(request.id, dedupedRequests.length)
      dedupedRequests.push(request)
      return
    }

    dedupedRequests[existingIndex] = {
      ...dedupedRequests[existingIndex],
      ...request,
    }
  })

  return dedupedRequests
}

export const mergeProxyHistoryRequests = (
  currentRequests: ProxyRequest[],
  incomingRequests: ProxyRequest[],
  position: ProxyHistoryInsertPosition,
): MergeProxyHistoryRequestsResult => {
  const mergedCurrentRequests = dedupeProxyHistoryRequests(currentRequests)
  const dedupedIncomingRequests = dedupeProxyHistoryRequests(incomingRequests)
  const nextCurrentRequests = mergedCurrentRequests.slice()
  const indexById = new Map(
    nextCurrentRequests.map((request, index) => [request.id, index] as const),
  )
  const addedRequests: ProxyRequest[] = []

  dedupedIncomingRequests.forEach((request) => {
    const existingIndex = indexById.get(request.id)
    if (existingIndex === undefined) {
      addedRequests.push(request)
      return
    }

    nextCurrentRequests[existingIndex] = {
      ...nextCurrentRequests[existingIndex],
      ...request,
    }
  })

  return {
    requests: position === 'prepend'
      ? [...addedRequests, ...nextCurrentRequests]
      : [...nextCurrentRequests, ...addedRequests],
    addedRequests,
  }
}
