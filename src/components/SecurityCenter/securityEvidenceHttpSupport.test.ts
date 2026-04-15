import { describe, expect, it } from 'vitest'
import type { WorkbenchEvidenceExchange } from './securityWorkbenchSystemAgentContent'
import {
  buildSecurityEvidenceRawRequest,
  buildSecurityEvidenceRawResponse,
} from './securityEvidenceHttpSupport'

describe('securityEvidenceHttpSupport', () => {
  it('builds a raw request with origin-form target and injected host header', () => {
    const exchange: WorkbenchEvidenceExchange = {
      requestMethod: 'POST',
      requestUrl: 'https://example.com/api/users?id=7',
      requestHeaders: JSON.stringify([{ name: 'Content-Type', value: 'application/json' }]),
      requestBody: '{"name":"alice"}',
      responseStatus: null,
    }

    expect(buildSecurityEvidenceRawRequest(exchange)).toBe(
      [
        'POST /api/users?id=7 HTTP/1.1',
        'Host: example.com',
        'Content-Type: application/json',
        '',
        '{"name":"alice"}',
      ].join('\r\n'),
    )
  })

  it('resolves relative request urls against the evidence url', () => {
    const exchange: WorkbenchEvidenceExchange = {
      requestMethod: 'GET',
      requestUrl: '/tenant/list?page=1',
      responseStatus: null,
    }

    expect(buildSecurityEvidenceRawRequest(exchange, 'https://app.example.com/root')).toBe(
      ['GET /tenant/list?page=1 HTTP/1.1', 'Host: app.example.com', '', ''].join('\r\n'),
    )
  })

  it('builds a raw response with status line, headers, and body', () => {
    const exchange: WorkbenchEvidenceExchange = {
      requestMethod: 'GET',
      requestUrl: 'https://example.com/api/users',
      responseStatus: 404,
      responseHeaders: JSON.stringify([
        { name: 'Content-Type', value: 'application/json' },
        { name: 'X-Trace', value: 'abc' },
      ]),
      responseBody: '{"error":"not found"}',
    }

    expect(buildSecurityEvidenceRawResponse(exchange)).toBe(
      [
        'HTTP/1.1 404 Not Found',
        'Content-Type: application/json',
        'X-Trace: abc',
        '',
        '{"error":"not found"}',
      ].join('\r\n'),
    )
  })

  it('accepts structured headers and body values without throwing', () => {
    const exchange = {
      requestMethod: 'POST',
      requestUrl: '/api/debug',
      requestHeaders: { 'Content-Type': 'application/json', 'X-Flags': ['a', 'b'] },
      requestBody: { ok: true, count: 3 },
      responseStatus: 200,
      responseHeaders: [{ name: 'Content-Type', value: 'application/json' }],
      responseBody: { success: true },
    } as unknown as WorkbenchEvidenceExchange

    expect(buildSecurityEvidenceRawRequest(exchange, 'https://example.com/base')).toContain(
      'Content-Type: application/json',
    )
    expect(buildSecurityEvidenceRawRequest(exchange, 'https://example.com/base')).toContain(
      '"ok": true',
    )
    expect(buildSecurityEvidenceRawResponse(exchange)).toContain('HTTP/1.1 200 OK')
    expect(buildSecurityEvidenceRawResponse(exchange)).toContain('"success": true')
  })
})
