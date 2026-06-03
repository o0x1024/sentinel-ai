import { describe, expect, it } from 'vitest'
import {
  createUrlEncodedTokenizerState,
  readUrlEncodedToken,
  type UrlEncodedToken,
} from './httpEditorUrlEncodedTokenizer'

function tokenizeAll(text: string, stopAtWhitespace = false): UrlEncodedToken[] {
  const state = createUrlEncodedTokenizerState()
  const tokens: UrlEncodedToken[] = []
  let from = 0

  while (from < text.length) {
    const result = readUrlEncodedToken(text, from, state, { stopAtWhitespace })
    if (result.to <= from) {
      from += 1
      continue
    }
    tokens.push(result.token)
    from = result.to
  }

  return tokens
}

describe('httpEditorUrlEncodedTokenizer', () => {
  it('marks query parameter keys and values with dedicated parameter tokens', () => {
    expect(tokenizeAll('?page=1&sort=desc', true)).toEqual([
      'punctuation',
      'http-parameter-key',
      'punctuation',
      'http-parameter-value',
      'punctuation',
      'http-parameter-key',
      'punctuation',
      'http-parameter-value',
    ])
  })

  it('marks form-urlencoded body keys and values with dedicated parameter tokens', () => {
    expect(tokenizeAll('username=alice&token=abc123')).toEqual([
      'http-parameter-key',
      'punctuation',
      'http-parameter-value',
      'punctuation',
      'http-parameter-key',
      'punctuation',
      'http-parameter-value',
    ])
  })
})
