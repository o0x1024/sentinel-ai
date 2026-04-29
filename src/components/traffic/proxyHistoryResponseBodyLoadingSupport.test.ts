import { describe, expect, it } from 'vitest'
import { resolveProxyHistoryResponseBodyLoadVariant } from './proxyHistoryResponseBodyLoadingSupport'
import type { ProxyRequest } from './proxyHistoryTypes'

function buildRequest(overrides: Partial<ProxyRequest> = {}): ProxyRequest {
  return {
    id: 1,
    url: 'https://example.test/api',
    host: 'example.test',
    scheme: 'https',
    method: 'GET',
    status_code: 200,
    request_headers: 'Host: example.test',
    request_body: '',
    response_headers: 'Content-Type: text/plain',
    response_body: '',
    response_size: 128,
    response_time: 12,
    timestamp: '2026-04-28T10:00:00Z',
    has_full_details: false,
    request_body_loaded: true,
    response_body_loaded: false,
    edited_request_body_loaded: true,
    edited_response_body_loaded: true,
    ...overrides,
  }
}

describe('resolveProxyHistoryResponseBodyLoadVariant', () => {
  it('loads original body for edited metadata when edited body does not exist', () => {
    const request = buildRequest({
      edited_status_code: 201,
      edited_response_body_loaded: true,
      response_body_loaded: false,
    })

    expect(resolveProxyHistoryResponseBodyLoadVariant(request, 'edited')).toBe('original')
  })

  it('loads edited body when the edited body is pending', () => {
    const request = buildRequest({
      edited_response_headers: 'Content-Type: text/plain',
      edited_response_body_loaded: false,
      response_body_loaded: false,
    })

    expect(resolveProxyHistoryResponseBodyLoadVariant(request, 'edited')).toBe('edited')
  })

  it('does not load when the visible body is already loaded', () => {
    const request = buildRequest({
      response_body: 'ok',
      response_body_loaded: true,
    })

    expect(resolveProxyHistoryResponseBodyLoadVariant(request, 'original')).toBeNull()
  })
})
