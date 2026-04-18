import type { Tag } from '@lezer/highlight'

export type UrlEncodedToken =
  | 'http-parameter-key'
  | 'http-parameter-value'
  | 'punctuation'
  | 'http-query'
  | null

export interface UrlEncodedTokenizerState {
  phase: 'key' | 'value' | 'fragment'
}

export interface UrlEncodedTokenizerOptions {
  stopAtWhitespace?: boolean
}

export interface UrlEncodedStreamModeConfig {
  name: string
  tokenTable: Record<string, Tag | readonly Tag[]>
  stopAtWhitespace?: boolean
}

const isWhitespace = (char: string) => /\s/.test(char)

export const createUrlEncodedTokenizerState = (): UrlEncodedTokenizerState => ({
  phase: 'key',
})

export const readUrlEncodedToken = (
  text: string,
  from: number,
  state: UrlEncodedTokenizerState,
  options: UrlEncodedTokenizerOptions = {},
): { to: number; token: UrlEncodedToken } => {
  const stopAtWhitespace = options.stopAtWhitespace ?? false

  if (from >= text.length) {
    return { to: from, token: null }
  }

  const current = text[from]

  if (stopAtWhitespace && isWhitespace(current)) {
    let to = from + 1
    while (to < text.length && isWhitespace(text[to])) to += 1
    return { to, token: null }
  }

  if (current === '?' || current === '&') {
    state.phase = 'key'
    return { to: from + 1, token: 'punctuation' }
  }

  if (current === '=') {
    state.phase = 'value'
    return { to: from + 1, token: 'punctuation' }
  }

  if (current === '#') {
    state.phase = 'fragment'
    return { to: from + 1, token: 'punctuation' }
  }

  let to = from

  if (state.phase === 'fragment') {
    while (to < text.length && !(stopAtWhitespace && isWhitespace(text[to]))) {
      to += 1
    }
    return { to, token: 'http-query' }
  }

  while (to < text.length) {
    const char = text[to]
    if (char === '&' || char === '#' || (state.phase === 'key' && char === '=')) break
    if (stopAtWhitespace && isWhitespace(char)) break
    to += 1
  }

  return {
    to,
    token: state.phase === 'value' ? 'http-parameter-value' : 'http-parameter-key',
  }
}

export const createUrlEncodedStreamMode = (config: UrlEncodedStreamModeConfig) => ({
  name: config.name,
  tokenTable: config.tokenTable,
  startState(): UrlEncodedTokenizerState {
    return createUrlEncodedTokenizerState()
  },
  copyState(state: unknown): UrlEncodedTokenizerState {
    return { ...(state as UrlEncodedTokenizerState) }
  },
  token(stream, state: unknown) {
    const result = readUrlEncodedToken(
      stream.string,
      stream.pos,
      state as UrlEncodedTokenizerState,
      { stopAtWhitespace: config.stopAtWhitespace },
    )
    if (result.to <= stream.pos) {
      stream.pos += 1
      return null
    }
    stream.pos = result.to
    return result.token
  },
})
