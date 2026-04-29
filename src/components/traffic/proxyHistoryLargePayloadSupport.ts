import { estimateResponseBodySize } from './trafficResponsePreviewSupport'
import { hasEditedRequest, hasEditedResponse } from './proxyHistoryFormattingSupport'
import type { ProxyHistoryViewMode, ProxyRequest } from './proxyHistoryTypes'

export const LARGE_HISTORY_DETAIL_THRESHOLD_BYTES = 256 * 1024

function estimateTextBytes(value?: string | null): number {
  if (!value) {
    return 0
  }

  return new Blob([value]).size
}

function resolveRequestHeaders(request: ProxyRequest, viewMode: ProxyHistoryViewMode): string {
  return viewMode === 'edited' && hasEditedRequest(request) && request.edited_request_headers
    ? request.edited_request_headers
    : request.request_headers || ''
}

function resolveRequestBody(request: ProxyRequest, viewMode: ProxyHistoryViewMode): string {
  return viewMode === 'edited' && hasEditedRequest(request) && request.edited_request_body
    ? request.edited_request_body
    : request.request_body || ''
}

function resolveResponseHeaders(request: ProxyRequest, viewMode: ProxyHistoryViewMode): string {
  return viewMode === 'edited' && hasEditedResponse(request) && request.edited_response_headers
    ? request.edited_response_headers
    : request.response_headers || ''
}

function resolveResponseBody(request: ProxyRequest, viewMode: ProxyHistoryViewMode): string {
  return viewMode === 'edited' && hasEditedResponse(request) && request.edited_response_body
    ? request.edited_response_body
    : request.response_body || ''
}

export function isLargeHistoryRequestPayload(
  request: ProxyRequest | null,
  viewMode: ProxyHistoryViewMode,
): boolean {
  if (!request) {
    return false
  }

  const totalBytes = (
    estimateTextBytes(request.method)
    + estimateTextBytes(request.url)
    + estimateTextBytes(request.http_version_observed)
    + estimateTextBytes(resolveRequestHeaders(request, viewMode))
    + estimateTextBytes(resolveRequestBody(request, viewMode))
  )

  return totalBytes >= LARGE_HISTORY_DETAIL_THRESHOLD_BYTES
}

export function isLargeHistoryResponsePayload(
  request: ProxyRequest | null,
  viewMode: ProxyHistoryViewMode,
): boolean {
  if (!request) {
    return false
  }

  const responseBody = resolveResponseBody(request, viewMode)
  const estimatedBodyBytes = responseBody ? estimateResponseBodySize(responseBody) : 0
  const totalBytes = Math.max(
    request.response_size || 0,
    estimateTextBytes(resolveResponseHeaders(request, viewMode)) + estimatedBodyBytes,
  )

  return totalBytes >= LARGE_HISTORY_DETAIL_THRESHOLD_BYTES
}
