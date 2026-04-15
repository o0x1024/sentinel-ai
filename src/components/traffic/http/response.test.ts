import { describe, expect, it } from 'vitest'
import {
  buildHttpReplayResponseFromCommandResult,
  createRawResponseFromReplayResponse,
} from './response'

describe('http response helpers', () => {
  it('builds structured replay responses from command results', () => {
    const response = buildHttpReplayResponseFromCommandResult({
      raw_response: 'HTTP/2 201 Created\r\nContent-Type: application/json\r\n\r\n{"ok":true}',
      response_time_ms: 42,
      final_url: 'https://example.com/api',
      redirect_chain: [],
      status_code: 201,
      version_observed: 'HTTP/2.0',
      status_text: 'Created',
      headers: [{ name: 'Content-Type', value: 'application/json' }],
      body_text: '{"ok":true}',
    })

    expect(response).toMatchObject({
      statusCode: 201,
      versionObserved: 'HTTP/2',
      statusText: 'Created',
      bodyText: '{"ok":true}',
      responseTimeMs: 42,
    })
    expect(response.headers).toEqual([
      { name: 'Content-Type', value: 'application/json' },
    ])
  })

  it('recreates raw response text from structured responses', () => {
    const raw = createRawResponseFromReplayResponse({
      statusCode: 302,
      versionObserved: 'HTTP/2',
      statusText: 'Found',
      headers: [{ name: 'Location', value: '/home' }],
      bodyText: '',
      rawText: '',
      responseTimeMs: 10,
    })

    expect(raw).toBe('HTTP/2 302 Found\r\nLocation: /home\r\n\r\n')
  })
})
