import { describe, expect, it } from 'vitest'
import { getNextCommandSuggestionIndex } from '@/services/commandSuggestionNavigation'

describe('commandSuggestionNavigation', () => {
  it('returns -1 when no suggestions exist', () => {
    expect(getNextCommandSuggestionIndex(0, 0, 1)).toBe(-1)
  })

  it('starts from the first suggestion when moving forward without a selection', () => {
    expect(getNextCommandSuggestionIndex(-1, 3, 1)).toBe(0)
  })

  it('starts from the last suggestion when moving backward without a selection', () => {
    expect(getNextCommandSuggestionIndex(-1, 3, -1)).toBe(2)
  })

  it('cycles through suggestions in both directions', () => {
    expect(getNextCommandSuggestionIndex(1, 3, 1)).toBe(2)
    expect(getNextCommandSuggestionIndex(0, 3, -1)).toBe(2)
  })
})
