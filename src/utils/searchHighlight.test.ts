import { describe, expect, it } from 'vitest'
import { buildHighlightedParts } from '@/utils/searchHighlight'

describe('searchHighlight', () => {
  it('returns a plain segment for empty query', () => {
    expect(buildHighlightedParts('Sentinel AI', '')).toEqual([
      { text: 'Sentinel AI', matched: false },
    ])
  })

  it('highlights case-insensitive matches', () => {
    expect(buildHighlightedParts('Sentinel AI', 'sent')).toEqual([
      { text: 'Sent', matched: true },
      { text: 'inel AI', matched: false },
    ])
  })

  it('highlights multiple tokens', () => {
    expect(buildHighlightedParts('漏洞消息中心', '漏洞 消息')).toEqual([
      { text: '漏洞消息', matched: true },
      { text: '中心', matched: false },
    ])
  })

  it('merges overlapping matches', () => {
    expect(buildHighlightedParts('dashboard', 'dash ash')).toEqual([
      { text: 'dash', matched: true },
      { text: 'board', matched: false },
    ])
  })
})
