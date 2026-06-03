import type { ProxyRequest } from './proxyHistoryTypes'

function textLength(value?: string | null): number {
  return value?.length ?? 0
}

export function buildProxyHistorySelectionChangeKey(request: ProxyRequest | null): string {
  if (!request) {
    return 'empty'
  }

  return [
    request.id,
    request.method,
    request.url,
    request.http_version_observed || '',
    request.request_body_loaded === false ? 'request-pending' : 'request-ready',
    request.edited_request_body_loaded === false ? 'edited-request-pending' : 'edited-request-ready',
    textLength(request.request_headers),
    textLength(request.request_body),
    textLength(request.edited_request_headers),
    textLength(request.edited_request_body),
    request.edited_method || '',
    request.edited_url || '',
  ].join('|')
}

export function buildProxyHistoryRangeSelectionIds(
  orderedRequests: ProxyRequest[],
  anchorRequestId: number,
  targetRequestId: number,
): number[] {
  const anchorIndex = orderedRequests.findIndex(request => request.id === anchorRequestId)
  const targetIndex = orderedRequests.findIndex(request => request.id === targetRequestId)

  if (anchorIndex === -1 || targetIndex === -1) {
    return [targetRequestId]
  }

  const startIndex = Math.min(anchorIndex, targetIndex)
  const endIndex = Math.max(anchorIndex, targetIndex)
  return orderedRequests.slice(startIndex, endIndex + 1).map(request => request.id)
}
