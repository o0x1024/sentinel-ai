import {
  parseMentionTokens,
  type MentionTokenKind,
  snapCursorToMentionBoundary,
} from '@/components/InputArea/mentionTokenSupport'

export type MirrorTextSegment = {
  start: number
  type: 'text'
  value: string
}

export type MirrorMentionSegment = {
  end: number
  id: string
  kind: MentionTokenKind
  label: string
  start: number
  type: 'mention'
  value: string
}

export type MirrorCaretSegment = {
  start: number
  type: 'caret'
}

export type MirrorRenderSegment = MirrorTextSegment | MirrorMentionSegment | MirrorCaretSegment

export const buildHighlightedMentionSegments = (
  text: string,
): Array<MirrorTextSegment | MirrorMentionSegment> => {
  const segments: Array<MirrorTextSegment | MirrorMentionSegment> = []
  let cursor = 0

  for (const token of parseMentionTokens(text)) {
    if (token.start > cursor) {
      segments.push({
        start: cursor,
        type: 'text',
        value: text.slice(cursor, token.start),
      })
    }
    segments.push({
      end: token.end,
      id: token.id,
      kind: token.kind,
      label: token.label,
      start: token.start,
      type: 'mention',
      value: token.text,
    })
    cursor = token.end
  }

  if (cursor < text.length) {
    segments.push({
      start: cursor,
      type: 'text',
      value: text.slice(cursor),
    })
  }

  return segments
}

export const resolveMirrorCaretIndex = (
  text: string,
  isFocused: boolean,
  selectionStart: number,
  selectionEnd: number,
): number | null => {
  if (!text || !isFocused || selectionStart !== selectionEnd) return null
  return snapCursorToMentionBoundary(text, selectionStart, 'nearest')
}

const pushMirrorTextSegments = (
  segments: MirrorRenderSegment[],
  value: string,
  start: number,
  caretIndex: number | null,
) => {
  if (!value) return
  if (caretIndex == null || caretIndex < start || caretIndex > start + value.length) {
    segments.push({
      start,
      type: 'text',
      value,
    })
    return
  }

  const relativeCaretIndex = caretIndex - start
  const prefix = value.slice(0, relativeCaretIndex)
  const suffix = value.slice(relativeCaretIndex)
  if (prefix) {
    segments.push({
      start,
      type: 'text',
      value: prefix,
    })
  }
  segments.push({
    start: caretIndex,
    type: 'caret',
  })
  if (suffix) {
    segments.push({
      start: caretIndex,
      type: 'text',
      value: suffix,
    })
  }
}

export const buildMirrorRenderSegments = (
  text: string,
  highlightedSegments: Array<MirrorTextSegment | MirrorMentionSegment>,
  caretIndex: number | null,
): MirrorRenderSegment[] => {
  const segments: MirrorRenderSegment[] = []
  let cursor = 0

  for (const segment of highlightedSegments) {
    if (segment.type === 'text') {
      pushMirrorTextSegments(segments, segment.value, segment.start, caretIndex)
      cursor = segment.start + segment.value.length
      continue
    }

    if (caretIndex === segment.start) {
      segments.push({
        start: caretIndex,
        type: 'caret',
      })
    }

    segments.push(segment)
    cursor = segment.end

    if (caretIndex === segment.end) {
      segments.push({
        start: caretIndex,
        type: 'caret',
      })
    }
  }

  if (cursor < text.length) {
    pushMirrorTextSegments(segments, text.slice(cursor), cursor, caretIndex)
    cursor = text.length
  }

  if (caretIndex === text.length && cursor === text.length && segments[segments.length - 1]?.type !== 'caret') {
    segments.push({
      start: caretIndex,
      type: 'caret',
    })
  }

  return segments
}
