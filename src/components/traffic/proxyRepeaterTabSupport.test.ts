import { describe, expect, it } from 'vitest'
import { createRepeaterTab } from './proxyRepeaterTabSupport'
import type { HttpExchangeRequest } from './http/model'

function createHistoryRequest(): HttpExchangeRequest {
  return {
    endpoint: {
      scheme: 'https',
      host: 'example.com',
      port: 443,
    },
    absoluteUrl: 'https://example.com/api/users?id=7',
    sourceRequestId: 42,
    preferredRequestView: 'pretty',
    request: {
      method: 'POST',
      target: '/api/users?id=7',
      versionPreference: 'HTTP/1.1',
      headers: [
        { name: 'Content-Type', value: 'application/json' },
        { name: 'X-Test', value: '1' },
      ],
      bodyText: '{"alpha":true}',
    },
  }
}

describe('proxyRepeaterTabSupport', () => {
  it('uses the preferred request view from the source request', () => {
    const tab = createRepeaterTab({
      request: createHistoryRequest(),
      fallbackNameIndex: 1,
      generateId: () => 'tab-1',
      defaultRequestTab: 'raw',
      defaultResponseTab: 'pretty',
    })

    expect(tab.requestTab).toBe('pretty')
    expect(tab.sourceRequestId).toBe(42)
  })

  it('formats the initial pretty request with the repeater pretty formatter', () => {
    const tab = createRepeaterTab({
      request: createHistoryRequest(),
      fallbackNameIndex: 1,
      generateId: () => 'tab-1',
      defaultRequestTab: 'pretty',
      defaultResponseTab: 'pretty',
    })

    expect(tab.prettyRequest).toBe([
      'POST /api/users?id=7 HTTP/1.1',
      'Host: example.com',
      'Content-Type: application/json',
      'X-Test: 1',
      '',
      '{',
      '  "alpha":true',
      '}',
    ].join('\n'))
  })

  it('hydrates preview response data from a history request', () => {
    const request = createHistoryRequest()
    request.previewResponse = {
      statusCode: 200,
      versionObserved: 'HTTP/1.1',
      statusText: 'OK',
      headers: [{ name: 'Content-Type', value: 'application/json' }],
      bodyText: '{"ok":true}',
      rawText: 'HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{"ok":true}',
      responseTimeMs: 24,
    }

    const tab = createRepeaterTab({
      request,
      fallbackNameIndex: 1,
      generateId: () => 'tab-1',
      defaultRequestTab: 'pretty',
      defaultResponseTab: 'pretty',
    })

    expect(tab.response?.bodyText).toBe('{"ok":true}')
    expect(tab.rawResponse).toContain('{"ok":true}')
    expect(tab.lastCompletedRawResponse).toBe(tab.rawResponse)
  })
})
