import type { HttpHeaderEntry, HttpReplayResponse } from './model'
import { normalizeHeaderEntries } from './headers'
import { normalizeHttpVersionToken } from './version'

export interface RawReplayRedirectHop {
  url: string
  status_code: number
  location?: string | null
  set_cookie_count: number
}

export interface RawReplayCommandResult {
  raw_response: string
  response_time_ms: number
  final_url: string
  redirect_chain: RawReplayRedirectHop[]
  status_code: number
  version_observed?: string | null
  status_text?: string | null
  headers: HttpHeaderEntry[]
  body_text: string
}

export function getHttpStatusText(statusCode: number): string {
  const statusTexts: Record<number, string> = {
    101: 'Switching Protocols',
    200: 'OK',
    201: 'Created',
    202: 'Accepted',
    204: 'No Content',
    206: 'Partial Content',
    301: 'Moved Permanently',
    302: 'Found',
    303: 'See Other',
    304: 'Not Modified',
    307: 'Temporary Redirect',
    308: 'Permanent Redirect',
    400: 'Bad Request',
    401: 'Unauthorized',
    403: 'Forbidden',
    404: 'Not Found',
    405: 'Method Not Allowed',
    409: 'Conflict',
    429: 'Too Many Requests',
    500: 'Internal Server Error',
    502: 'Bad Gateway',
    503: 'Service Unavailable',
  }
  if (statusCode === -1) return 'TUNNEL'
  if (statusCode === 0) return 'TLS ERR'
  return statusTexts[statusCode] || 'Unknown'
}

export function buildHttpReplayResponseFromCommandResult(
  result: RawReplayCommandResult,
): HttpReplayResponse {
  return {
    statusCode: result.status_code,
    versionObserved: result.version_observed
      ? normalizeHttpVersionToken(result.version_observed)
      : undefined,
    statusText: result.status_text || getHttpStatusText(result.status_code),
    headers: normalizeHeaderEntries(result.headers || []),
    bodyText: result.body_text || '',
    rawText: result.raw_response,
    responseTimeMs: result.response_time_ms,
  }
}

export function createRawResponseFromReplayResponse(response: HttpReplayResponse): string {
  const version = response.versionObserved || 'HTTP/1.1'
  const statusText = response.statusText || getHttpStatusText(response.statusCode)
  const lines = [
    `${version} ${response.statusCode} ${statusText}`.trimEnd(),
    ...normalizeHeaderEntries(response.headers).map((header) => `${header.name}: ${header.value}`),
    '',
    response.bodyText ?? '',
  ]

  return lines.join('\r\n')
}
