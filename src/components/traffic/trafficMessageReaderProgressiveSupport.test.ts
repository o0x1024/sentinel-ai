import { describe, expect, it } from 'vitest'
import {
  shouldProgressivelyRenderTrafficMessage,
  TRAFFIC_MESSAGE_READER_PROGRESSIVE_THRESHOLD_BYTES,
} from './trafficMessageReaderProgressiveSupport'

describe('trafficMessageReaderProgressiveSupport', () => {
  it('keeps small messages on direct render path', () => {
    expect(shouldProgressivelyRenderTrafficMessage('hello')).toBe(false)
  })

  it('moves large messages to progressive render path', () => {
    expect(
      shouldProgressivelyRenderTrafficMessage('a'.repeat(TRAFFIC_MESSAGE_READER_PROGRESSIVE_THRESHOLD_BYTES)),
    ).toBe(true)
  })
})
