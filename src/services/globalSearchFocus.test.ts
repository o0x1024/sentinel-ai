import { describe, expect, it } from 'vitest'
import { isGlobalSearchShortcut } from '@/services/globalSearchFocus'

describe('globalSearchFocus', () => {
  it('matches ctrl+k', () => {
    expect(isGlobalSearchShortcut({
      key: 'k',
      ctrlKey: true,
      metaKey: false,
      altKey: false,
      shiftKey: false,
    })).toBe(true)
  })

  it('matches cmd+k', () => {
    expect(isGlobalSearchShortcut({
      key: 'K',
      ctrlKey: false,
      metaKey: true,
      altKey: false,
      shiftKey: false,
    })).toBe(true)
  })

  it('rejects modified variants', () => {
    expect(isGlobalSearchShortcut({
      key: 'k',
      ctrlKey: true,
      metaKey: false,
      altKey: true,
      shiftKey: false,
    })).toBe(false)
  })
})
