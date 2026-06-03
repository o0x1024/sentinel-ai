import type { HttpExchangeRequest } from './http/model'
import { parseStoredHeaderEntries } from './http/headers'
import { endpointFromUrl } from './http/url'
import { DEFAULT_HTTP_VERSION, normalizeHttpVersionToken } from './http/version'
import type { ProxyRequest } from './proxyHistoryTypes'

export const DEFAULT_PROXY_HISTORY_HTTP_VERSION = DEFAULT_HTTP_VERSION

export const normalizeProxyHistoryHttpVersion = (
  value?: string | null,
) => normalizeHttpVersionToken(value, DEFAULT_PROXY_HISTORY_HTTP_VERSION)

export const formatProxyHistorySchemeLabel = (scheme?: string | null) =>
  (scheme || 'http').trim().toUpperCase()

export const buildHttpExchangeRequestFromHistory = (
  request: ProxyRequest,
): HttpExchangeRequest => {
  const endpoint = endpointFromUrl(request.url)
  const parsedUrl = new URL(request.url)

  return {
    endpoint,
    absoluteUrl: request.url,
    sourceRequestId: request.db_request_id ?? null,
    preferredRequestView: 'pretty',
    request: {
      method: request.method,
      target: `${parsedUrl.pathname}${parsedUrl.search}`,
      versionPreference: normalizeProxyHistoryHttpVersion(request.http_version_observed),
      headers: parseStoredHeaderEntries(request.request_headers),
      bodyText: request.request_body || '',
    },
  }
}
