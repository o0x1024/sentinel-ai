import { describe, expect, it } from 'vitest'
import { extractTargetFromRequest } from './http'

describe('intruder http target extraction', () => {
  it('preserves https from fallback url on non-default ports', () => {
    const target = extractTargetFromRequest(
      'GET /v1/memories HTTP/1.1\r\nHost: demo.example.com:8000\r\n\r\n',
      'https://demo.example.com:8000/v1/memories',
    )

    expect(target).toEqual({
      host: 'demo.example.com',
      port: 8000,
      useTls: true,
    })
  })

  it('falls back to host header heuristics when no fallback url is available', () => {
    const target = extractTargetFromRequest(
      'GET /health HTTP/1.1\r\nHost: demo.example.com:8080\r\n\r\n',
    )

    expect(target).toEqual({
      host: 'demo.example.com',
      port: 8080,
      useTls: false,
    })
  })
})
