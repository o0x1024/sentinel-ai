export type ProxyHistoryListStep = {
  type: 'stable' | 'prepend' | 'append' | 'full'
  addedFrontCount: number
  addedTailStart: number
  preservedCount: number
}

export const areProxyHistoryRequestIdsEqual = (left: number[], right: number[]) =>
  left.length === right.length && left.every((id, index) => id === right[index])

export const classifyProxyHistoryRequestIdStep = (
  previousIds: number[],
  nextIds: number[],
): ProxyHistoryListStep => {
  if (areProxyHistoryRequestIdsEqual(previousIds, nextIds)) {
    return {
      type: 'stable',
      addedFrontCount: 0,
      addedTailStart: nextIds.length,
      preservedCount: nextIds.length,
    }
  }

  for (let addedFrontCount = 1; addedFrontCount <= nextIds.length; addedFrontCount += 1) {
    const preservedCount = nextIds.length - addedFrontCount
    if (preservedCount > previousIds.length) continue
    if (preservedCount === 0 && previousIds.length > 0) continue

    let matches = true
    for (let index = 0; index < preservedCount; index += 1) {
      if (nextIds[addedFrontCount + index] !== previousIds[index]) {
        matches = false
        break
      }
    }

    if (matches) {
      return {
        type: 'prepend',
        addedFrontCount,
        addedTailStart: nextIds.length,
        preservedCount,
      }
    }
  }

  if (nextIds.length >= previousIds.length) {
    let matchesPrefix = true
    for (let index = 0; index < previousIds.length; index += 1) {
      if (nextIds[index] !== previousIds[index]) {
        matchesPrefix = false
        break
      }
    }

    if (matchesPrefix) {
      return {
        type: 'append',
        addedFrontCount: 0,
        addedTailStart: previousIds.length,
        preservedCount: previousIds.length,
      }
    }
  }

  return {
    type: 'full',
    addedFrontCount: 0,
    addedTailStart: nextIds.length,
    preservedCount: 0,
  }
}
