import { describe, expect, it } from 'vitest'
import { detectHttpBodyLanguage } from './httpDocument'

describe('httpDocument', () => {
  it('treats incomplete json objects as json-like while editing', () => {
    expect(detectHttpBodyLanguage('{', '')).toBe('json')
    expect(detectHttpBodyLanguage('{"suspended"', '')).toBe('json')
    expect(detectHttpBodyLanguage('{"suspended": false,', '')).toBe('json')
    expect(detectHttpBodyLanguage('[', '')).toBe('json')
  })

  it('still respects explicit content types first', () => {
    expect(detectHttpBodyLanguage('a=1&b=2', 'application/x-www-form-urlencoded')).toBe('form')
    expect(detectHttpBodyLanguage('alpha', 'application/json')).toBe('json')
    expect(detectHttpBodyLanguage('<div></div>', 'text/html')).toBe('html')
  })
})
