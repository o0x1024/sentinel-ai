import type { HttpExchangeRequest } from './http/model'
import type { RawReplayCommandResult } from './http/response'
import type { ProxyRequest } from './proxyHistoryTypes'
import type { InterceptedRequest, InterceptedResponse } from './proxyInterceptSupport'
import type { TrafficComparePayload } from './transfers'

export function createSelectedProxyRequest(): ProxyRequest {
  return {
    id: 101,
    scheme: 'https',
    http_version_observed: 'HTTP/2',
    was_edited: false,
    response_headers: '',
    response_body: '',
  } as unknown as ProxyRequest
}

export function createReplayResult(bodyText: string, rawResponse: string): RawReplayCommandResult {
  return {
    raw_response: rawResponse,
    response_time_ms: 120,
    final_url: 'https://example.com/api/test',
    redirect_chain: [],
    status_code: 200,
    version_observed: 'HTTP/1.1',
    status_text: 'OK',
    headers: [
      { name: 'Content-Type', value: 'text/plain' },
    ],
    body_text: bodyText,
  }
}

export function createInitialHttpExchangeRequest(): HttpExchangeRequest {
  return {
    endpoint: {
      scheme: 'https',
      host: 'example.com',
      port: 443,
    },
    absoluteUrl: 'https://example.com/api/test',
    request: {
      method: 'GET',
      target: '/api/test',
      versionPreference: 'HTTP/1.1',
      headers: [
        { name: 'Host', value: 'example.com' },
      ],
      bodyText: '',
    },
  }
}

export function createInterceptedRequest(): InterceptedRequest {
  return {
    id: 'req-1',
    timestamp: 1710000000000,
    method: 'GET',
    url: 'https://example.com/api/test',
    path: '/api/test',
    protocol: 'HTTP/2',
    headers: {
      Host: 'example.com',
      Accept: '*/*',
    },
    body: '',
  }
}

export function createInterceptedResponse(): InterceptedResponse {
  return {
    id: 'res-1',
    request_id: 'req-1',
    timestamp: 1710000001000,
    status: 200,
    headers: {
      'Content-Type': 'application/json',
    },
    body: '{"ok":true}',
  }
}

export function createTrafficComparePayload(): TrafficComparePayload {
  return {
    name: 'Comparison 1',
    leftLabel: 'Left',
    rightLabel: 'Right',
    leftText: 'GET /left HTTP/1.1\nHost: example.com\n\n',
    rightText: 'GET /right HTTP/1.1\nHost: example.com\n\n',
    leftMeta: { messageType: 'request' },
    rightMeta: { messageType: 'request' },
    compareMeta: { source: 'generic', kind: 'generic' },
  }
}
