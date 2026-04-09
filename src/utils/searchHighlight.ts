export interface HighlightPart {
  text: string
  matched: boolean
}

function escapeRegExp(value: string) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
}

function getQueryTokens(query: string) {
  return Array.from(
    new Set(
      query
        .trim()
        .split(/\s+/)
        .map(token => token.trim().toLowerCase())
        .filter(Boolean),
    ),
  ).sort((left, right) => right.length - left.length)
}

export function buildHighlightedParts(text: string, query: string): HighlightPart[] {
  const source = String(text || '')
  if (!source) {
    return [{ text: '', matched: false }]
  }

  const tokens = getQueryTokens(query)
  if (tokens.length === 0) {
    return [{ text: source, matched: false }]
  }

  const ranges: Array<{ start: number; end: number }> = []
  const lowerSource = source.toLowerCase()

  for (const token of tokens) {
    const matcher = new RegExp(escapeRegExp(token), 'gi')
    for (const match of lowerSource.matchAll(matcher)) {
      const start = match.index ?? -1
      if (start < 0) {
        continue
      }

      ranges.push({
        start,
        end: start + token.length,
      })
    }
  }

  if (ranges.length === 0) {
    return [{ text: source, matched: false }]
  }

  ranges.sort((left, right) => left.start - right.start || left.end - right.end)

  const mergedRanges: Array<{ start: number; end: number }> = []
  for (const current of ranges) {
    const previous = mergedRanges[mergedRanges.length - 1]
    if (!previous || current.start > previous.end) {
      mergedRanges.push({ ...current })
      continue
    }

    previous.end = Math.max(previous.end, current.end)
  }

  const parts: HighlightPart[] = []
  let cursor = 0

  for (const range of mergedRanges) {
    if (range.start > cursor) {
      parts.push({
        text: source.slice(cursor, range.start),
        matched: false,
      })
    }

    parts.push({
      text: source.slice(range.start, range.end),
      matched: true,
    })
    cursor = range.end
  }

  if (cursor < source.length) {
    parts.push({
      text: source.slice(cursor),
      matched: false,
    })
  }

  return parts.filter(part => part.text.length > 0)
}
