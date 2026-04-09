export interface OrderedPinnedSearchShortcutLike {
  id: string
}

export interface OrderedPinnedSearchShortcutGroup<T extends OrderedPinnedSearchShortcutLike> {
  items: T[]
}

export type PinnedSearchShortcutMoveDirection = -1 | 1

export function getPinnedSearchShortcutMoveTarget<T extends OrderedPinnedSearchShortcutLike>(
  group: OrderedPinnedSearchShortcutGroup<T>,
  activeId: string,
  direction: PinnedSearchShortcutMoveDirection,
) {
  const activeIndex = group.items.findIndex(item => item.id === activeId)
  if (activeIndex < 0) {
    return null
  }

  const targetIndex = activeIndex + direction
  if (targetIndex < 0 || targetIndex >= group.items.length) {
    return null
  }

  return group.items[targetIndex]?.id || null
}

export function canMovePinnedSearchShortcut<T extends OrderedPinnedSearchShortcutLike>(
  group: OrderedPinnedSearchShortcutGroup<T>,
  activeId: string,
  direction: PinnedSearchShortcutMoveDirection,
) {
  return getPinnedSearchShortcutMoveTarget(group, activeId, direction) !== null
}
