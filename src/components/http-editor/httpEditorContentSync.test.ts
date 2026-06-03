import { describe, expect, it } from 'vitest'
import { computeMinimalTextChange } from './httpEditorContentSync'

describe('httpEditorContentSync', () => {
  it('returns null when the text is unchanged', () => {
    expect(computeMinimalTextChange('alpha', 'alpha')).toBeNull()
  })

  it('computes an insertion near the cursor instead of replacing the whole document', () => {
    expect(computeMinimalTextChange('{"a":1}', '{"ab":1}')).toEqual({
      from: 3,
      to: 3,
      insert: 'b',
    })
  })

  it('computes a deletion without touching the unchanged prefix and suffix', () => {
    expect(computeMinimalTextChange('{"ab":1}', '{"a":1}')).toEqual({
      from: 3,
      to: 4,
      insert: '',
    })
  })

  it('handles middle replacements', () => {
    expect(computeMinimalTextChange('header\r\n\r\nfalse', 'header\r\n\r\ntrue')).toEqual({
      from: 10,
      to: 14,
      insert: 'tru',
    })
  })
})
