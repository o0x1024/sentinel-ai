import { describe, expect, it } from 'vitest'
import {
  CHUNKED_HEX_VIEW_THRESHOLD_BYTES,
  convertTextToHexChunk,
  shouldChunkHexView,
} from './trafficHexViewSupport'

describe('trafficHexViewSupport', () => {
  it('converts text chunks to hex with stable global line breaks', () => {
    const prefix = 'a'.repeat(15)
    const source = `${prefix}BC`

    const tailHex = convertTextToHexChunk(source, 15, source.length)

    expect(tailHex).toBe('42 \n43 ')
  })

  it('does not chunk small hex payloads', () => {
    expect(shouldChunkHexView('hello')).toBe(false)
  })

  it('chunks large hex payloads', () => {
    expect(shouldChunkHexView('a'.repeat(CHUNKED_HEX_VIEW_THRESHOLD_BYTES))).toBe(true)
  })
})
