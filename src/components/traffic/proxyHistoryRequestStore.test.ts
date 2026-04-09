import { describe, expect, it } from 'vitest'
import { dedupeProxyHistoryRequests, mergeProxyHistoryRequests } from './proxyHistoryRequestStore'
import type { ProxyRequest } from './proxyHistoryTypes'

const createRequest = (id: number, overrides: Partial<ProxyRequest> = {}): ProxyRequest => ({
  id,
  url: `https://example.com/${id}`,
  host: 'example.com',
  protocol: 'https',
  method: 'GET',
  status_code: 200,
  response_size: 128,
  response_time: 25,
  timestamp: `2026-04-09T00:00:${String(id).padStart(2, '0')}Z`,
  ...overrides,
})

describe('proxyHistoryRequestStore', () => {
  it('deduplicates repeated request ids while keeping the first position', () => {
    const deduped = dedupeProxyHistoryRequests([
      createRequest(10, { status_code: 0 }),
      createRequest(11),
      createRequest(10, { status_code: 200, title: 'updated' }),
    ])

    expect(deduped).toHaveLength(2)
    expect(deduped.map((request) => request.id)).toEqual([10, 11])
    expect(deduped[0]).toMatchObject({
      id: 10,
      status_code: 200,
      title: 'updated',
    })
  })

  it('appends only new ids and merges overlapping page results', () => {
    const result = mergeProxyHistoryRequests(
      [
        createRequest(12, { status_code: 201 }),
        createRequest(11),
      ],
      [
        createRequest(11, { title: 'overlap' }),
        createRequest(10),
        createRequest(10, { response_time: 50 }),
      ],
      'append',
    )

    expect(result.requests.map((request) => request.id)).toEqual([12, 11, 10])
    expect(result.addedRequests.map((request) => request.id)).toEqual([10])
    expect(result.requests[1]).toMatchObject({
      id: 11,
      title: 'overlap',
    })
    expect(result.requests[2]).toMatchObject({
      id: 10,
      response_time: 50,
    })
  })

  it('prepends only one request when a batch contains duplicate event ids', () => {
    const result = mergeProxyHistoryRequests(
      [createRequest(9)],
      [
        createRequest(10, { status_code: 0 }),
        createRequest(10, { status_code: 200, title: 'final' }),
      ],
      'prepend',
    )

    expect(result.requests.map((request) => request.id)).toEqual([10, 9])
    expect(result.addedRequests.map((request) => request.id)).toEqual([10])
    expect(result.requests[0]).toMatchObject({
      id: 10,
      status_code: 200,
      title: 'final',
    })
  })
})
