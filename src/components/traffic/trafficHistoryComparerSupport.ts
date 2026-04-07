import type { TrafficComparePayload, TrafficTransferRequest } from './transfers'
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

function parseHeaderRecord(rawHeaders?: string): Record<string, string> {
  if (!rawHeaders) return {}

  try {
    const parsed = JSON.parse(rawHeaders)
    return parsed && typeof parsed === 'object' && !Array.isArray(parsed)
      ? parsed as Record<string, string>
      : {}
  } catch {
    return {}
  }
}

function inferProtocol(url: string, fallbackProtocol?: string): 'http' | 'https' {
  try {
    return new URL(url).protocol === 'http:' ? 'http' : 'https'
  } catch {
    return fallbackProtocol === 'http' ? 'http' : 'https'
  }
}

function buildRequestTransferRequest(request: ProxyRequest, viewMode: 'original' | 'edited'): TrafficTransferRequest {
  const useEdited = viewMode === 'edited' && request.was_edited
  const url = useEdited && request.edited_url ? request.edited_url : request.url
  const method = useEdited && request.edited_method ? request.edited_method : request.method
  const headers = parseHeaderRecord(useEdited ? request.edited_request_headers : request.request_headers)
  const body = useEdited && request.edited_request_body != null ? request.edited_request_body : request.request_body

  return {
    method,
    url,
    headers,
    body: body || undefined,
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
      protocol: inferProtocol(originalRequest.url, request.protocol),
      repeaterRequest: originalRequest,
    },
    rightMeta: {
      messageType: 'request',
      protocol: inferProtocol(editedRequest.url, request.protocol),
      repeaterRequest: editedRequest,
    },
  }
}

export function buildResponseVersionComparePayload(
  request: ProxyRequest,
  labels: CompareVersionLabels,
): TrafficComparePayload | null {
  if (!request.was_edited || !hasEditedResponse(request)) return null

  const protocol = inferProtocol(request.url, request.protocol)

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
