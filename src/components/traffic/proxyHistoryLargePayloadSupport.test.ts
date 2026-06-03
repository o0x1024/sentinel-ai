import { describe, expect, it } from 'vitest'
import {
  LARGE_HISTORY_DETAIL_THRESHOLD_BYTES,
  isLargeHistoryRequestPayload,
  isLargeHistoryResponsePayload,
} from './proxyHistoryLargePayloadSupport'
import type { ProxyRequest } from './proxyHistoryTypes'

const createRequest = (overrides: Partial<ProxyRequest> = {}): ProxyRequest => ({
  id: 1,
  url: 'https://example.com/api',
  host: 'example.com',
  scheme: 'https',
  http_version_observed: 'HTTP/2',
  method: 'POST',
  request_headers: 'content-type: application/json',
  request_body: '',
  response_headers: 'content-type: application/json',
  response_body: '',
  status_code: 200,
  response_size: 0,
  response_time: 25,
  timestamp: '2026-04-13T08:00:00.000Z',
  ...overrides,
})

describe('proxyHistoryLargePayloadSupport', () => {
  it('marks oversized request payloads as large', () => {
    const request = createRequest({
      request_body: 'a'.repeat(LARGE_HISTORY_DETAIL_THRESHOLD_BYTES),
    })

    expect(isLargeHistoryRequestPayload(request, 'edited')).toBe(true)
  })

  it('marks oversized response payloads as large from response_size', () => {
    const request = createRequest({
      response_size: LARGE_HISTORY_DETAIL_THRESHOLD_BYTES,
    })

    expect(isLargeHistoryResponsePayload(request, 'edited')).toBe(true)
  })

  it('keeps small payloads on the normal detail path', () => {
    const request = createRequest({
      request_body: '{"ok":true}',
      response_body: '{"ok":true}',
      response_size: 32,
    })

    expect(isLargeHistoryRequestPayload(request, 'edited')).toBe(false)
    expect(isLargeHistoryResponsePayload(request, 'edited')).toBe(false)
  })
})
