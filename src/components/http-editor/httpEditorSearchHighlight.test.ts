import { describe, expect, it } from 'vitest'
import { SearchQuery } from '@codemirror/search'
import { collectSearchHighlightRanges } from './httpEditorSearchHighlight'

describe('httpEditorSearchHighlight', () => {
  it('collects all literal matches and marks the first match selected by default', () => {
    const query = new SearchQuery({ search: 'token' })
    const ranges = collectSearchHighlightRanges('token alpha token beta', query)

    expect(ranges).toEqual([
      { from: 0, to: 5, selected: true },
      { from: 12, to: 17, selected: false },
    ])
  })

  it('marks the range overlapping the current selection as selected', () => {
    const query = new SearchQuery({ search: 'token' })
    const ranges = collectSearchHighlightRanges('token alpha token beta', query, {
      from: 12,
      to: 17,
    })

    expect(ranges).toEqual([
      { from: 0, to: 5, selected: false },
      { from: 12, to: 17, selected: true },
    ])
  })

  it('returns no ranges for invalid regex queries', () => {
    const query = new SearchQuery({ search: '(', regexp: true })
    const ranges = collectSearchHighlightRanges('token alpha token beta', query)

    expect(ranges).toEqual([])
  })
})
