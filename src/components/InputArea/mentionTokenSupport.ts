export type MentionTokenKind = 'file' | 'asset' | 'traffic'

export interface MentionTokenMatch {
  end: number
  id: string
  kind: MentionTokenKind
  label: string
  start: number
  text: string
}

const TOKEN_REGEX = /@(file|asset|http)\[([^\]\n]+)\]/g

const sanitizeTokenPart = (value: string) => value.replace(/[\]\n\r]/g, ' ').replace(/\|/g, '/').trim()

const splitTokenPayload = (payload: string) => {
  const separatorIndex = payload.indexOf('|')
  if (separatorIndex === -1) {
    const trimmed = payload.trim()
    return { id: trimmed, label: trimmed }
  }
  const id = payload.slice(0, separatorIndex).trim()
  const label = payload.slice(separatorIndex + 1).trim() || id
  return { id, label }
}

export const buildMentionToken = (params: {
  id: string
  kind: MentionTokenKind
  label: string
}) => {
  const safeId = sanitizeTokenPart(params.id)
  const safeLabel = sanitizeTokenPart(params.label)
  if (params.kind === 'file') {
    return `@file[${safeId}]`
  }
  if (params.kind === 'asset') {
    return `@asset[${safeId}|${safeLabel}]`
  }
  return `@http[${safeId}|${safeLabel}]`
}

export const parseMentionTokens = (text: string): MentionTokenMatch[] => {
  const matches: MentionTokenMatch[] = []
  for (const match of text.matchAll(TOKEN_REGEX)) {
    const full = match[0]
    const rawKind = match[1]
    const payload = match[2]
    const start = match.index ?? -1
    if (start < 0) continue
    const end = start + full.length
    const { id, label } = splitTokenPayload(payload)
    const kind: MentionTokenKind =
      rawKind === 'http'
        ? 'traffic'
        : rawKind === 'asset'
          ? 'asset'
          : 'file'
    matches.push({
      end,
      id,
      kind,
      label,
      start,
      text: full,
    })
  }
  return matches
}

export const findMentionTokenAtCursor = (
  text: string,
  cursor: number,
): MentionTokenMatch | null => {
  for (const token of parseMentionTokens(text)) {
    if (cursor > token.start && cursor < token.end) {
      return token
    }
  }
  return null
}

export const snapCursorToMentionBoundary = (
  text: string,
  cursor: number,
  preference: 'left' | 'right' | 'nearest' = 'nearest',
): number => {
  const token = findMentionTokenAtCursor(text, cursor)
  if (!token) return cursor
  if (preference === 'left') return token.start
  if (preference === 'right') return token.end
  return cursor - token.start <= token.end - cursor ? token.start : token.end
}

export const findMentionTokenAdjacentToCursor = (
  text: string,
  cursor: number,
  direction: 'left' | 'right',
): MentionTokenMatch | null => {
  const probe = direction === 'left' ? cursor - 1 : cursor + 1
  if (probe < 0 || probe > text.length) return null
  return findMentionTokenAtCursor(text, probe)
}

export const expandSelectionToMentionBoundaries = (
  text: string,
  selectionStart: number,
  selectionEnd: number,
): { end: number; start: number } => {
  if (selectionStart >= selectionEnd) {
    return { end: selectionEnd, start: selectionStart }
  }

  let nextStart = selectionStart
  let nextEnd = selectionEnd
  for (const token of parseMentionTokens(text)) {
    if (selectionStart < token.end && selectionEnd > token.start) {
      nextStart = Math.min(nextStart, token.start)
      nextEnd = Math.max(nextEnd, token.end)
    }
  }

  return {
    end: nextEnd,
    start: nextStart,
  }
}

export const findMentionTokenForDeletion = (
  text: string,
  selectionStart: number,
  selectionEnd: number,
  key: 'Backspace' | 'Delete',
): MentionTokenMatch | null => {
  const tokens = parseMentionTokens(text)

  if (selectionStart !== selectionEnd) {
    return tokens.find((token) => selectionStart < token.end && selectionEnd > token.start) || null
  }

  if (key === 'Backspace') {
    const probe = Math.max(0, selectionStart - 1)
    return tokens.find((token) => probe >= token.start && probe < token.end) || null
  }

  return tokens.find((token) => selectionStart >= token.start && selectionStart < token.end) || null
}

export const removeMentionTokenText = (text: string, tokenText?: string) => {
  if (!tokenText || !text.includes(tokenText)) return text
  return text
    .replace(tokenText, '')
    .replace(/[ \t]{2,}/g, ' ')
    .replace(/\s+\n/g, '\n')
    .replace(/\n\s+/g, '\n')
    .trim()
}

export const removeTextRange = (
  text: string,
  start: number,
  end: number,
): { cursor: number; value: string } => {
  const left = text.slice(0, start)
  const right = text.slice(end)
  const value = `${left}${right}`
    .replace(/[ \t]{2,}/g, ' ')
    .replace(/\s+\n/g, '\n')
    .replace(/\n\s+/g, '\n')
    .trim()
  return {
    cursor: Math.min(value.length, left.trimEnd().length),
    value,
  }
}
