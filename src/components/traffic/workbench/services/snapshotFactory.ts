import { parseStoredHeaderEntries } from '../../http/headers'
import type { HttpExchangeRequest } from '../../http/model'
import { endpointFromUrl } from '../../http/url'
import { normalizeProxyHistoryHttpVersion } from '../../proxyHistoryHttpSupport'
import { formatResponseRaw } from '../../proxyHistoryFormattingSupport'
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

function buildHttpExchangeRequest(request: ProxyRequest, variant: HistorySnapshotVariant): HttpExchangeRequest {
  const absoluteUrl = resolveVariantUrl(request, variant)
  const parsedUrl = new URL(absoluteUrl)

  return {
    endpoint: endpointFromUrl(absoluteUrl),
    absoluteUrl,
    sourceRequestId: request.db_request_id ?? null,
    preferredRequestView: 'pretty',
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
