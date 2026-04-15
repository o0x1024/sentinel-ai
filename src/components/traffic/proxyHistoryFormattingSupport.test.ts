import { describe, expect, it } from 'vitest'
import {
  formatRequestRaw,
  formatResponseRaw,
} from './proxyHistoryFormattingSupport'
import type { ProxyRequest } from './proxyHistoryTypes'

const createRequest = (overrides: Partial<ProxyRequest> = {}): ProxyRequest => ({
  id: 1,
  url: 'https://example.com/login?next=%2Fhome',
  host: 'example.com',
  scheme: 'https',
  http_version_observed: 'HTTP/2',
  method: 'POST',
  request_headers: JSON.stringify([
    { name: 'content-type', value: 'application/x-www-form-urlencoded' },
  ]),
  request_body: 'username=alice',
  response_headers: JSON.stringify([
    { name: 'location', value: '/home' },
  ]),
  response_body: '',
  status_code: 302,
  response_size: 0,
  response_time: 25,
  timestamp: '2026-04-13T08:00:00.000Z',
  ...overrides,
})

describe('proxyHistoryFormattingSupport', () => {
  it('formats request raw text with the observed HTTP version', () => {
    const text = formatRequestRaw(createRequest())

    expect(text.startsWith('POST /login?next=%2Fhome HTTP/2\n')).toBe(true)
    expect(text).toContain('Host: example.com\n')
    expect(text).toContain('content-type: application/x-www-form-urlencoded\n')
    expect(text.endsWith('\nusername=alice')).toBe(true)
  })

  it('formats response raw text with the observed HTTP version and reason phrase', () => {
    const text = formatResponseRaw(createRequest())

    expect(text.startsWith('HTTP/2 302 Found\n')).toBe(true)
    expect(text).toContain('location: /home\n')
  })
})
