import { describe, expect, it } from 'vitest'
import {
  canMovePinnedSearchShortcut,
  getPinnedSearchShortcutMoveTarget,
} from '@/services/pinnedSearchShortcutOrdering'

const group = {
  items: [
    { id: 'one' },
    { id: 'two' },
    { id: 'three' },
  ],
}

describe('pinnedSearchShortcutOrdering', () => {
  it('returns adjacent move target inside a group', () => {
    expect(getPinnedSearchShortcutMoveTarget(group, 'two', -1)).toBe('one')
    expect(getPinnedSearchShortcutMoveTarget(group, 'two', 1)).toBe('three')
  })

  it('returns null when moving past the group boundary', () => {
    expect(getPinnedSearchShortcutMoveTarget(group, 'one', -1)).toBeNull()
    expect(getPinnedSearchShortcutMoveTarget(group, 'three', 1)).toBeNull()
  })

  it('checks whether a shortcut can move within its group', () => {
    expect(canMovePinnedSearchShortcut(group, 'one', -1)).toBe(false)
    expect(canMovePinnedSearchShortcut(group, 'one', 1)).toBe(true)
  })
})
