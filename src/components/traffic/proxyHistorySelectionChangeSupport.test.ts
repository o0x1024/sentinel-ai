import { describe, expect, it } from 'vitest'
import {
  buildProxyHistoryRangeSelectionIds,
  buildProxyHistorySelectionChangeKey,
} from './proxyHistorySelectionChangeSupport'
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
    response_size: 0,
    response_time: 12,
    timestamp: '2026-04-28T10:00:00Z',
    has_full_details: false,
    request_body_loaded: true,
    response_body_loaded: false,
    edited_request_body_loaded: true,
    edited_response_body_loaded: false,
    ...overrides,
  }
}

describe('buildProxyHistorySelectionChangeKey', () => {
  it('changes when request-side preview details become available', () => {
    const summary = buildRequest({
      request_body: '',
      request_body_loaded: false,
    })
    const preview = buildRequest({
      request_body: '{"id":1}',
      request_body_loaded: true,
    })

    expect(buildProxyHistorySelectionChangeKey(preview)).not.toBe(
      buildProxyHistorySelectionChangeKey(summary),
    )
  })
})

describe('buildProxyHistoryRangeSelectionIds', () => {
  it('returns ids between the anchor and target in current order', () => {
    const requests = [
      buildRequest({ id: 11 }),
      buildRequest({ id: 12 }),
      buildRequest({ id: 13 }),
      buildRequest({ id: 14 }),
    ]

    expect(buildProxyHistoryRangeSelectionIds(requests, 13, 11)).toEqual([11, 12, 13])
  })

  it('uses the target when the anchor is not in the current order', () => {
    const requests = [
      buildRequest({ id: 21 }),
      buildRequest({ id: 22 }),
    ]

    expect(buildProxyHistoryRangeSelectionIds(requests, 99, 22)).toEqual([22])
  })
})
