import { parseStoredHeaderEntries } from '../../http/headers'
import type { HttpExchangeRequest, HttpReplayResponse } from '../../http/model'
import { endpointFromUrl } from '../../http/url'
import { normalizeProxyHistoryHttpVersion } from '../../proxyHistoryHttpSupport'
import { formatResponseRaw, getHarStatusText } from '../../proxyHistoryFormattingSupport'
import type { ProxyRequest } from '../../proxyHistoryTypes'
import type { TrafficWorkbenchSource } from '../../trafficWorkbenchTypes'
import type { HistorySnapshot, HistorySnapshotVariant } from '../model/historySnapshot'

function resolveVariantUrl(request: ProxyRequest, variant: HistorySnapshotVariant) {
  return variant === 'edited' && request.was_edited && request.edited_url
    ? request.edited_url
    : request.url
}

function resolveVariantMethod(request: ProxyRequest, variant: HistorySnapshotVariant) {
  return variant === 'edited' && request.was_edited && request.edited_method
    ? request.edited_method
    : request.method
}

function resolveVariantHeaders(request: ProxyRequest, variant: HistorySnapshotVariant) {
  return parseStoredHeaderEntries(
    variant === 'edited' && request.was_edited
      ? request.edited_request_headers
      : request.request_headers,
  )
}

function resolveVariantBody(request: ProxyRequest, variant: HistorySnapshotVariant) {
  const body = variant === 'edited' && request.was_edited
    ? request.edited_request_body
    : request.request_body
  return body || ''
}

function resolveVariantStatusCode(request: ProxyRequest, variant: HistorySnapshotVariant) {
  const statusCode = variant === 'edited' && request.was_edited && request.edited_status_code
    ? request.edited_status_code
    : request.status_code
  return Number.isFinite(statusCode) ? statusCode : null
}

function resolveVariantResponseHeaders(request: ProxyRequest, variant: HistorySnapshotVariant) {
  return variant === 'edited' && request.was_edited && request.edited_response_headers
    ? request.edited_response_headers
    : request.response_headers
}

function resolveVariantResponseBody(request: ProxyRequest, variant: HistorySnapshotVariant) {
  return variant === 'edited' && request.was_edited && request.edited_response_body
    ? request.edited_response_body
    : request.response_body
}

function buildPreviewResponse(
  request: ProxyRequest,
  variant: HistorySnapshotVariant,
): HttpReplayResponse | undefined {
  const statusCode = resolveVariantStatusCode(request, variant)
  if (statusCode === null) {
    return undefined
  }

  return {
    statusCode,
    versionObserved: normalizeProxyHistoryHttpVersion(request.http_version_observed) as HttpReplayResponse['versionObserved'],
    statusText: getHarStatusText(statusCode),
    headers: parseStoredHeaderEntries(resolveVariantResponseHeaders(request, variant)),
    bodyText: resolveVariantResponseBody(request, variant) || '',
    rawText: formatResponseRaw(request, variant),
    responseTimeMs: Number.isFinite(request.response_time) ? request.response_time : 0,
  }
}

function buildHttpExchangeRequest(request: ProxyRequest, variant: HistorySnapshotVariant): HttpExchangeRequest {
  const absoluteUrl = resolveVariantUrl(request, variant)
  const parsedUrl = new URL(absoluteUrl)
  const previewResponse = buildPreviewResponse(request, variant)

  return {
    endpoint: endpointFromUrl(absoluteUrl),
    absoluteUrl,
    sourceRequestId: request.db_request_id ?? null,
    preferredRequestView: 'pretty',
    ...(previewResponse ? { previewResponse } : {}),
    request: {
      method: resolveVariantMethod(request, variant),
      target: `${parsedUrl.pathname}${parsedUrl.search}`,
      versionPreference: normalizeProxyHistoryHttpVersion(request.http_version_observed),
      headers: resolveVariantHeaders(request, variant),
      bodyText: resolveVariantBody(request, variant),
    },
  }
}

export function createHistorySnapshotFromProxyRequest(
  request: ProxyRequest,
  variant: HistorySnapshotVariant,
  source: TrafficWorkbenchSource,
): HistorySnapshot {
  return {
    id: `history-snapshot:${request.id}:${variant}`,
    requestId: request.id,
    dbRequestId: request.db_request_id ?? null,
    trafficRequestId: request.traffic_request_id ?? null,
    variant,
    request: buildHttpExchangeRequest(request, variant),
    responseRawText: formatResponseRaw(request, variant),
    responseStatusCode: resolveVariantStatusCode(request, variant),
    source,
    createdAt: Date.now(),
  }
}
