import type { TrafficComparePayload } from './transfers'
import type { HttpExchangeRequest } from './http/model'
import { parseStoredHeaderEntries } from './http/headers'
import { endpointFromUrl } from './http/url'
import { normalizeProxyHistoryHttpVersion } from './proxyHistoryHttpSupport'
import type { ProxyRequest } from './proxyHistoryTypes'
import {
  formatRequestRaw,
  formatResponseRaw,
  hasEditedResponse,
} from './proxyHistoryFormattingSupport'

type CompareVersionKind = 'request' | 'response'
type CompareVersionLabels = {
  requestVersions: string
  responseVersions: string
  originalRequest: string
  editedRequest: string
  originalResponse: string
  editedResponse: string
}

function inferProtocol(url: string, fallbackProtocol?: string): 'http' | 'https' {
  try {
    return new URL(url).protocol === 'http:' ? 'http' : 'https'
  } catch {
    return fallbackProtocol === 'http' ? 'http' : 'https'
  }
}

function buildRequestTransferRequest(request: ProxyRequest, viewMode: 'original' | 'edited'): HttpExchangeRequest {
  const useEdited = viewMode === 'edited' && request.was_edited
  const url = useEdited && request.edited_url ? request.edited_url : request.url
  const method = useEdited && request.edited_method ? request.edited_method : request.method
  const headers = parseStoredHeaderEntries(useEdited ? request.edited_request_headers : request.request_headers)
  const body = useEdited && request.edited_request_body != null ? request.edited_request_body : request.request_body
  const endpoint = endpointFromUrl(url)
  let target = '/'
  try {
    const parsedUrl = new URL(url)
    target = `${parsedUrl.pathname}${parsedUrl.search}`
  } catch {
    target = '/'
  }

  return {
    endpoint,
    absoluteUrl: url,
    request: {
      method,
      target,
      versionPreference: normalizeProxyHistoryHttpVersion(request.http_version_observed),
      headers,
      bodyText: body || '',
    },
  }
}

function buildCompareName(request: ProxyRequest, kind: CompareVersionKind, labels: CompareVersionLabels): string {
  const host = request.host || request.url
  return kind === 'request'
    ? `${host} · ${labels.requestVersions}`
    : `${host} · ${labels.responseVersions}`
}

export function buildRequestVersionComparePayload(
  request: ProxyRequest,
  labels: CompareVersionLabels,
): TrafficComparePayload | null {
  if (!request.was_edited) return null

  const originalRequest = buildRequestTransferRequest(request, 'original')
  const editedRequest = buildRequestTransferRequest(request, 'edited')

  return {
    name: buildCompareName(request, 'request', labels),
    leftLabel: labels.originalRequest,
    rightLabel: labels.editedRequest,
    leftText: formatRequestRaw(request, 'original'),
    rightText: formatRequestRaw(request, 'edited'),
    compareMeta: {
      source: 'history',
      kind: 'requestVersions',
    },
    leftMeta: {
      messageType: 'request',
      protocol: inferProtocol(originalRequest.absoluteUrl, request.scheme),
      repeaterRequest: originalRequest,
    },
    rightMeta: {
      messageType: 'request',
      protocol: inferProtocol(editedRequest.absoluteUrl, request.scheme),
      repeaterRequest: editedRequest,
    },
  }
}

export function buildResponseVersionComparePayload(
  request: ProxyRequest,
  labels: CompareVersionLabels,
): TrafficComparePayload | null {
  if (!request.was_edited || !hasEditedResponse(request)) return null

  const protocol = inferProtocol(request.url, request.scheme)

  return {
    name: buildCompareName(request, 'response', labels),
    leftLabel: labels.originalResponse,
    rightLabel: labels.editedResponse,
    leftText: formatResponseRaw(request, 'original'),
    rightText: formatResponseRaw(request, 'edited'),
    compareMeta: {
      source: 'history',
      kind: 'responseVersions',
    },
    leftMeta: {
      messageType: 'response',
      protocol,
    },
    rightMeta: {
      messageType: 'response',
      protocol,
    },
  }
}

export function canCompareRequestVersions(request: ProxyRequest | null): boolean {
  return !!request?.was_edited
}

export function canCompareResponseVersions(request: ProxyRequest | null): boolean {
  return !!request?.was_edited && hasEditedResponse(request)
}
