import { describe, expect, it } from 'vitest'

import { decodeUnicodeEscapes, encodeUnicodeEscapes } from './cyberchefUnicodeSupport'

describe('cyberchefUnicodeSupport', () => {
  it('encodes basic multilingual plane characters into unicode escapes', () => {
    expect(encodeUnicodeEscapes('A中')).toBe('\\u0041\\u4e2d')
  })

  it('encodes astral plane characters as surrogate pairs', () => {
    expect(encodeUnicodeEscapes('😀')).toBe('\\ud83d\\ude00')
  })

  it('decodes unicode escapes into text', () => {
    expect(decodeUnicodeEscapes('\\u0041\\u4e2d')).toBe('A中')
  })

  it('decodes surrogate pairs into astral plane characters', () => {
    expect(decodeUnicodeEscapes('\\ud83d\\ude00')).toBe('😀')
  })

  it('preserves non-escaped text during decode', () => {
    expect(decodeUnicodeEscapes('prefix:\\u4f60\\u597d')).toBe('prefix:你好')
  })

  it('rejects malformed unicode escape sequences', () => {
    expect(() => decodeUnicodeEscapes('\\u12')).toThrow('Invalid Unicode escape sequence')
    expect(() => decodeUnicodeEscapes('\\ud83d\\u0041')).toThrow('Invalid Unicode surrogate pair')
  })
})
