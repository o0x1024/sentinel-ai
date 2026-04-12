import type { IntruderPosition } from './types'

export const INTRUDER_PRIMARY_MARKER = '$'
export const INTRUDER_LEGACY_MARKER = '§'
const SUPPORTED_INTRUDER_MARKERS = new Set([INTRUDER_PRIMARY_MARKER, INTRUDER_LEGACY_MARKER])

export interface IntruderMarkerRange {
  marker: string
  from: number
  to: number
  contentFrom: number
  contentTo: number
  value: string
}

export function parseIntruderMarkerRanges(input: string): IntruderMarkerRange[] {
  const ranges: IntruderMarkerRange[] = []
  let cursor = 0

  while (cursor < input.length) {
    const marker = input[cursor]
    if (!SUPPORTED_INTRUDER_MARKERS.has(marker)) {
      cursor += 1
      continue
    }

    const end = input.indexOf(marker, cursor + 1)
    if (end === -1) {
      cursor += 1
      continue
    }

    ranges.push({
      marker,
      from: cursor,
      to: end + 1,
      contentFrom: cursor + 1,
      contentTo: end,
      value: input.slice(cursor + 1, end),
    })
    cursor = end + 1
  }

  return ranges
}

export function clearIntruderMarkers(input: string): string {
  const ranges = parseIntruderMarkerRanges(input)
  if (!ranges.length) return input

  let output = ''
  let cursor = 0

  for (const range of ranges) {
    output += input.slice(cursor, range.from)
    output += range.value
    cursor = range.to
  }

  output += input.slice(cursor)
  return output
}

export function wrapSelectionWithMarkers(input: string, selectionStart: number, selectionEnd: number): string {
  if (selectionStart === selectionEnd) return input

  const start = Math.min(selectionStart, selectionEnd)
  const end = Math.max(selectionStart, selectionEnd)
  const selected = input.slice(start, end)
  if (!selected) return input

  return `${input.slice(0, start)}${INTRUDER_PRIMARY_MARKER}${selected}${INTRUDER_PRIMARY_MARKER}${input.slice(end)}`
}

export function extractIntruderPositions(input: string): IntruderPosition[] {
  return parseIntruderMarkerRanges(input).map((range, index) => ({
    index,
    start: range.from,
    end: range.to - 1,
    value: range.value,
    preview: range.value.length > 32 ? `${range.value.slice(0, 29)}...` : range.value,
  }))
}

export function splitIntruderTemplate(input: string): { segments: string[]; tokens: string[] } {
  const ranges = parseIntruderMarkerRanges(input)
  if (!ranges.length) {
    return {
      segments: [input],
      tokens: [],
    }
  }

  const segments: string[] = []
  const tokens: string[] = []
  let cursor = 0

  for (const range of ranges) {
    segments.push(input.slice(cursor, range.from))
    tokens.push(range.value)
    cursor = range.to
  }

  segments.push(input.slice(cursor))
  return { segments, tokens }
}
