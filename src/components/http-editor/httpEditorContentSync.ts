export interface TextChangeRange {
  from: number
  to: number
  insert: string
}

export function computeMinimalTextChange(previous: string, next: string): TextChangeRange | null {
  if (previous === next) return null

  let prefixLength = 0
  const maxPrefixLength = Math.min(previous.length, next.length)
  while (prefixLength < maxPrefixLength && previous.charCodeAt(prefixLength) === next.charCodeAt(prefixLength)) {
    prefixLength += 1
  }

  let previousSuffixIndex = previous.length
  let nextSuffixIndex = next.length
  while (
    previousSuffixIndex > prefixLength
    && nextSuffixIndex > prefixLength
    && previous.charCodeAt(previousSuffixIndex - 1) === next.charCodeAt(nextSuffixIndex - 1)
  ) {
    previousSuffixIndex -= 1
    nextSuffixIndex -= 1
  }

  return {
    from: prefixLength,
    to: previousSuffixIndex,
    insert: next.slice(prefixLength, nextSuffixIndex),
  }
}
