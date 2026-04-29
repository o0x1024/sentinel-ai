import { describe, expect, it } from 'vitest'
import { formatTrafficJsonBody } from './trafficJsonFormattingSupport'

describe('trafficJsonFormattingSupport', () => {
  it('pretty prints JSON without inserting spaces before values', () => {
    expect(formatTrafficJsonBody('{"MeasureCode":"","Count":1,"Nested":{"Name":"x"}}')).toBe([
      '{',
      '  "MeasureCode":"",',
      '  "Count":1,',
      '  "Nested":{',
      '    "Name":"x"',
      '  }',
      '}',
    ].join('\n'))
  })

  it('keeps invalid JSON unchanged', () => {
    expect(formatTrafficJsonBody('{"MeasureCode":')).toBe('{"MeasureCode":')
  })

  it('does not alter spaces inside string values', () => {
    expect(formatTrafficJsonBody('{"Text":"prefix \\": value"}')).toBe([
      '{',
      '  "Text":"prefix \\": value"',
      '}',
    ].join('\n'))
  })
})
