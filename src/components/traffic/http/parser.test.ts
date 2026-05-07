import { describe, expect, it } from 'vitest'
import { parseRawHttpRequest } from './parser'

describe('http parser', () => {
  it('preserves request body line endings while normalizing only the header block', () => {
    const parsed = parseRawHttpRequest(
      'POST /upload HTTP/1.1\r\nHost: example.com\r\nContent-Type: multipart/form-data; boundary=x\r\n\r\n--x\r\nfield\r\n--x--\r\n',
    )

    expect(parsed?.method).toBe('POST')
    expect(parsed?.headers.find((header) => header.name === 'Host')?.value).toBe('example.com')
    expect(parsed?.bodyText).toBe('--x\r\nfield\r\n--x--\r\n')
  })

  it('supports LF-only header separators without rewriting body text', () => {
    const parsed = parseRawHttpRequest('POST /api HTTP/1.1\nHost: example.com\n\nline1\r\nline2')

    expect(parsed?.bodyText).toBe('line1\r\nline2')
  })
})
