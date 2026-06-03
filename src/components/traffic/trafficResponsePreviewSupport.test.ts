import { describe, expect, it } from 'vitest'
import {
  buildImagePreviewSrc,
  getDisplayResponseBody,
  extractBase64BodyPayload,
} from './trafficResponsePreviewSupport'

describe('trafficResponsePreviewSupport', () => {
  it('builds image preview data urls from base64 bodies', () => {
    const payload = 'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+aR6QAAAAASUVORK5CYII='
    const src = buildImagePreviewSrc(payload, 'image/png')

    expect(src).toBe(`data:image/png;base64,${payload}`)
  })

  it('extracts prefixed base64 payloads', () => {
    expect(extractBase64BodyPayload('[BASE64] aGVsbG8=')).toBe('aGVsbG8=')
  })

  it('decodes image bodies for editor display without exposing the base64 marker', () => {
    expect(getDisplayResponseBody('[BASE64]aGVsbG8=', 'image/png')).toBe('hello')
  })
})
