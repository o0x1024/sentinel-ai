import { describe, expect, it } from 'vitest'
import {
  createCookieHeaderTokenizerState,
  getCookieHeaderValueMode,
  tokenizeCookieHeaderValue,
  type CookieHeaderValueMode,
} from './httpEditorCookieHighlight'

class MockStream {
  private readonly source: string
  private position = 0
  private tokenStart = 0

  constructor(source: string) {
    this.source = source
  }

  beginToken() {
    this.tokenStart = this.position
  }

  current() {
    return this.source.slice(this.tokenStart, this.position)
  }

  eatSpace() {
    const start = this.position
    while (this.position < this.source.length && /\s/.test(this.source[this.position])) {
      this.position += 1
    }
    return this.position > start
  }

  eol() {
    return this.position >= this.source.length
  }

  next() {
    if (this.eol()) return undefined
    const char = this.source[this.position]
    this.position += 1
    return char
  }

  peek() {
    return this.source[this.position]
  }
}

const collectTokens = (value: string, mode: CookieHeaderValueMode) => {
  const stream = new MockStream(value)
  const state = createCookieHeaderTokenizerState(mode)
  const tokens: Array<{ text: string; token: string }> = []

  while (!stream.eol()) {
    stream.beginToken()
    const token = tokenizeCookieHeaderValue(stream, state)
    const text = stream.current()
    if (token && text) {
      tokens.push({ text, token })
    }
  }

  return tokens
}

describe('httpEditorCookieHighlight', () => {
  it('detects cookie-like header names', () => {
    expect(getCookieHeaderValueMode('Cookie')).toBe('cookie')
    expect(getCookieHeaderValueMode('Set-Cookie')).toBe('set-cookie')
    expect(getCookieHeaderValueMode('Authorization')).toBe('none')
  })

  it('tokenizes cookie request headers into names, separators, and values', () => {
    expect(collectTokens('session=abc123; locale=zh-CN', 'cookie')).toEqual([
      { text: 'session', token: 'http-cookie-name' },
      { text: '=', token: 'http-cookie-separator' },
      { text: 'abc123', token: 'http-cookie-value-sensitive' },
      { text: ';', token: 'http-cookie-separator' },
      { text: 'locale', token: 'http-cookie-name' },
      { text: '=', token: 'http-cookie-separator' },
      { text: 'zh-CN', token: 'http-cookie-value-sensitive' },
    ])
  })

  it('tokenizes set-cookie attributes separately from the primary cookie pair', () => {
    expect(
      collectTokens('session=abc123; Path=/; HttpOnly; SameSite=Lax', 'set-cookie'),
    ).toEqual([
      { text: 'session', token: 'http-cookie-name' },
      { text: '=', token: 'http-cookie-separator' },
      { text: 'abc123', token: 'http-cookie-value-sensitive' },
      { text: ';', token: 'http-cookie-separator' },
      { text: 'Path', token: 'http-cookie-attribute' },
      { text: '=', token: 'http-cookie-separator' },
      { text: '/', token: 'http-cookie-attribute-value' },
      { text: ';', token: 'http-cookie-separator' },
      { text: 'HttpOnly', token: 'http-cookie-flag' },
      { text: ';', token: 'http-cookie-separator' },
      { text: 'SameSite', token: 'http-cookie-attribute' },
      { text: '=', token: 'http-cookie-separator' },
      { text: 'Lax', token: 'http-cookie-attribute-value' },
    ])
  })

  it('keeps expires attributes intact even when they contain commas and spaces', () => {
    expect(
      collectTokens('session=abc123; Expires=Wed, 21 Oct 2015 07:28:00 GMT; Secure', 'set-cookie'),
    ).toEqual([
      { text: 'session', token: 'http-cookie-name' },
      { text: '=', token: 'http-cookie-separator' },
      { text: 'abc123', token: 'http-cookie-value-sensitive' },
      { text: ';', token: 'http-cookie-separator' },
      { text: 'Expires', token: 'http-cookie-attribute' },
      { text: '=', token: 'http-cookie-separator' },
      { text: 'Wed, 21 Oct 2015 07:28:00 GMT', token: 'http-cookie-attribute-value' },
      { text: ';', token: 'http-cookie-separator' },
      { text: 'Secure', token: 'http-cookie-flag' },
    ])
  })
})
