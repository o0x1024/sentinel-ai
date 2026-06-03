import { describe, expect, it } from 'vitest'
import { parseStoredHeaderEntries } from './headers'

describe('parseStoredHeaderEntries', () => {
  it('preserves repeated set-cookie values stored as continued raw lines', () => {
    const headers = parseStoredHeaderEntries(
      'set-cookie: session=abc; Path=/; HttpOnly\r\ntheme=dark; Path=/\r\ncontent-type: text/html',
    )

    expect(headers).toEqual([
      {
        name: 'set-cookie',
        value: 'session=abc; Path=/; HttpOnly\ntheme=dark; Path=/',
      },
      {
        name: 'content-type',
        value: 'text/html',
      },
    ])
  })
})
