import { describe, expect, it } from 'vitest'
import {
  buildHttpExchangeRequestFromHistory,
  normalizeProxyHistoryHttpVersion,
} from './proxyHistoryHttpSupport'
import type { ProxyRequest } from './proxyHistoryTypes'

const createRequest = (overrides: Partial<ProxyRequest> = {}): ProxyRequest => ({
  id: 1,
  url: 'https://example.com/products?id=7',
  host: 'example.com',
  scheme: 'https',
  http_version_observed: 'HTTP/2',
  method: 'POST',
  request_headers: JSON.stringify([
    { name: 'content-type', value: 'application/json' },
    { name: 'x-test', value: '1' },
  ]),
  request_body: '{"ok":true}',
  status_code: 200,
  response_size: 32,
  response_time: 18,
  timestamp: '2026-04-13T08:00:00.000Z',
  ...overrides,
})

describe('proxyHistoryHttpSupport', () => {
  it('normalizes observed versions to supported values', () => {
    expect(normalizeProxyHistoryHttpVersion('HTTP/2.0')).toBe('HTTP/2')
    expect(normalizeProxyHistoryHttpVersion('http/1.0')).toBe('HTTP/1.0')
    expect(normalizeProxyHistoryHttpVersion('spdy/3')).toBe('HTTP/1.1')
    expect(normalizeProxyHistoryHttpVersion(undefined)).toBe('HTTP/1.1')
  })

  it('builds structured exchange requests from history records', () => {
    const exchange = buildHttpExchangeRequestFromHistory(createRequest())

    expect(exchange.endpoint).toMatchObject({
      scheme: 'https',
      host: 'example.com',
      port: 443,
    })
    expect(exchange.absoluteUrl).toBe('https://example.com/products?id=7')
    expect(exchange.request).toMatchObject({
      method: 'POST',
      target: '/products?id=7',
      versionPreference: 'HTTP/2',
      bodyText: '{"ok":true}',
    })
    expect(exchange.request.headers).toEqual([
      { name: 'content-type', value: 'application/json' },
      { name: 'x-test', value: '1' },
    ])
  })
})
