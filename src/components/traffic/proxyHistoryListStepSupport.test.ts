import { describe, expect, it } from 'vitest'
import {
  areProxyHistoryRequestIdsEqual,
  classifyProxyHistoryRequestIdStep,
} from './proxyHistoryListStepSupport'

describe('proxyHistoryListStepSupport', () => {
  it('detects stable request ids', () => {
    expect(areProxyHistoryRequestIdsEqual([3, 2, 1], [3, 2, 1])).toBe(true)
    expect(classifyProxyHistoryRequestIdStep([3, 2, 1], [3, 2, 1])).toEqual({
      type: 'stable',
      addedFrontCount: 0,
      addedTailStart: 3,
      preservedCount: 3,
    })
  })

  it('detects prepended rows and counts the added front items', () => {
    expect(classifyProxyHistoryRequestIdStep([24, 23, 22], [26, 25, 24, 23, 22])).toEqual({
      type: 'prepend',
      addedFrontCount: 2,
      addedTailStart: 5,
      preservedCount: 3,
    })
  })

  it('falls back to full recompute when ids are reordered in place', () => {
    expect(classifyProxyHistoryRequestIdStep([5, 4, 3], [5, 3, 4])).toEqual({
      type: 'full',
      addedFrontCount: 0,
      addedTailStart: 3,
      preservedCount: 0,
    })
  })
})
