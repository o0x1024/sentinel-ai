import { describe, expect, it } from 'vitest'
import {
  TRAFFIC_TEXT_CODEC_ACTIONS,
  replaceTrafficTextSelection,
  transformTrafficTextCodec,
} from './trafficTextCodecSupport'

const actionByKey = Object.fromEntries(TRAFFIC_TEXT_CODEC_ACTIONS.map(action => [action.key, action]))

describe('trafficTextCodecSupport', () => {
  it('encodes and decodes utf-8 base64', () => {
    const encoded = transformTrafficTextCodec('A中', actionByKey.base64Encode)

    expect(encoded).toBe('QeS4rQ==')
    expect(transformTrafficTextCodec(encoded, actionByKey.base64Decode)).toBe('A中')
  })

  it('encodes and decodes url text', () => {
    expect(transformTrafficTextCodec('a b&中', actionByKey.urlEncode)).toBe('a%20b%26%E4%B8%AD')
    expect(transformTrafficTextCodec('a+b%26%E4%B8%AD', actionByKey.urlDecode)).toBe('a b&中')
  })

  it('encodes and decodes utf-8 hex', () => {
    const encoded = transformTrafficTextCodec('A中', actionByKey.hexEncode)

    expect(encoded).toBe('41e4b8ad')
    expect(transformTrafficTextCodec('41 e4 b8 ad', actionByKey.hexDecode)).toBe('A中')
  })

  it('encodes and decodes unicode escapes', () => {
    const encoded = transformTrafficTextCodec('A😀', actionByKey.unicodeEncode)

    expect(encoded).toBe('\\u0041\\ud83d\\ude00')
    expect(transformTrafficTextCodec(encoded, actionByKey.unicodeDecode)).toBe('A😀')
  })

  it('replaces the selected text and returns the new selection', () => {
    expect(replaceTrafficTextSelection('abc123xyz', { from: 3, to: 6 }, '456')).toEqual({
      content: 'abc456xyz',
      selectionStart: 3,
      selectionEnd: 6,
    })
  })
})
