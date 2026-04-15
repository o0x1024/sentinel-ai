export type CookieHeaderValueMode = 'none' | 'cookie' | 'set-cookie'
export type CookieHeaderValuePhase = 'segment-name' | 'segment-equals' | 'segment-value' | 'segment-end'

export interface CookieHeaderTokenizerState {
  mode: CookieHeaderValueMode
  segmentIndex: number
  phase: CookieHeaderValuePhase
}

interface StreamLike {
  eatSpace(): boolean
  eol(): boolean
  next(): string | undefined
  peek(): string | undefined
}

const COOKIE_VALUE_SEPARATOR = ';'

const isSetCookieAttributeSegment = (state: CookieHeaderTokenizerState) =>
  state.mode === 'set-cookie' && state.segmentIndex > 0

const readUntilSegmentBoundary = (stream: StreamLike) => {
  let consumed = 0
  while (!stream.eol()) {
    const char = stream.peek()
    if (!char || char === '=' || char === COOKIE_VALUE_SEPARATOR) break
    stream.next()
    consumed += 1
  }
  return consumed
}

const readUntilValueBoundary = (stream: StreamLike) => {
  let consumed = 0
  while (!stream.eol()) {
    const char = stream.peek()
    if (!char || char === COOKIE_VALUE_SEPARATOR) break
    stream.next()
    consumed += 1
  }
  return consumed
}

const consumeSegmentSeparator = (stream: StreamLike, state: CookieHeaderTokenizerState) => {
  if (stream.peek() !== COOKIE_VALUE_SEPARATOR) return null

  stream.next()
  state.segmentIndex += 1
  state.phase = 'segment-name'
  return 'http-cookie-separator'
}

const getSegmentNameToken = (state: CookieHeaderTokenizerState) =>
  isSetCookieAttributeSegment(state) ? 'http-cookie-attribute' : 'http-cookie-name'

const getSegmentValueToken = (state: CookieHeaderTokenizerState) =>
  isSetCookieAttributeSegment(state) ? 'http-cookie-attribute-value' : 'http-cookie-value-sensitive'

export const getCookieHeaderValueMode = (headerName: string): CookieHeaderValueMode => {
  const normalized = headerName.trim().toLowerCase()
  if (normalized === 'cookie') return 'cookie'
  if (normalized === 'set-cookie') return 'set-cookie'
  return 'none'
}

export const createCookieHeaderTokenizerState = (
  mode: CookieHeaderValueMode,
): CookieHeaderTokenizerState => ({
  mode,
  segmentIndex: 0,
  phase: 'segment-name',
})

export const tokenizeCookieHeaderValue = (
  stream: StreamLike,
  state: CookieHeaderTokenizerState,
): string | null => {
  if (stream.eatSpace()) return null
  if (stream.eol()) return null

  switch (state.phase) {
    case 'segment-name': {
      const separatorToken = consumeSegmentSeparator(stream, state)
      if (separatorToken) return separatorToken

      const consumed = readUntilSegmentBoundary(stream)
      if (consumed === 0 && stream.peek() === '=') {
        state.phase = 'segment-value'
        stream.next()
        return 'http-cookie-separator'
      }
      if (stream.peek() === '=') {
        state.phase = 'segment-equals'
        return getSegmentNameToken(state)
      }

      state.phase = 'segment-end'
      return isSetCookieAttributeSegment(state) ? 'http-cookie-flag' : getSegmentNameToken(state)
    }
    case 'segment-equals':
      if (stream.peek() === '=') {
        stream.next()
      }
      state.phase = 'segment-value'
      return 'http-cookie-separator'
    case 'segment-value':
      if (readUntilValueBoundary(stream) === 0) {
        state.phase = 'segment-end'
        return tokenizeCookieHeaderValue(stream, state)
      }
      state.phase = 'segment-end'
      return getSegmentValueToken(state)
    case 'segment-end': {
      const separatorToken = consumeSegmentSeparator(stream, state)
      if (separatorToken) return separatorToken
      state.phase = 'segment-name'
      return tokenizeCookieHeaderValue(stream, state)
    }
    default:
      return null
  }
}
