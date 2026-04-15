import { StreamLanguage } from '@codemirror/language'
import { css } from '@codemirror/legacy-modes/mode/css'
import { javascript, json } from '@codemirror/legacy-modes/mode/javascript'
import { html, xml } from '@codemirror/legacy-modes/mode/xml'
import { tags as t } from '@lezer/highlight'
import type { Extension } from '@codemirror/state'
import {
  createCookieHeaderTokenizerState,
  getCookieHeaderValueMode,
  tokenizeCookieHeaderValue,
  type CookieHeaderTokenizerState,
} from './httpEditorCookieHighlight'
import { detectHttpBodyLanguage, parseHttpMessageDocument } from './httpDocument'

type BodyModeKey = 'none' | 'html' | 'xml' | 'json' | 'css' | 'javascript'
type LegacyMode = any

type BurpHttpState = {
  phase: 'start' | 'requestPath' | 'requestQuery' | 'requestProtocol' | 'responseStatus' | 'responseText' | 'header' | 'headerColon' | 'headerValue' | 'body'
  inBody: boolean
  bodyState: unknown
  headerNameToken: string
  headerValueToken: string
  cookieTokenizer: CookieHeaderTokenizerState
}

const BURP_HTTP_TOKEN_TABLE = {
  variable: t.variableName,
  'variable-2': t.special(t.variableName),
  'string-2': t.special(t.string),
  'string property': t.propertyName,
  'string-2 property': t.propertyName,
  'variable property': t.propertyName,
  def: t.definition(t.variableName),
  tag: t.tagName,
  attribute: t.attributeName,
  type: t.typeName,
  builtin: t.standard(t.variableName),
  qualifier: t.modifier,
  'property error': [t.propertyName, t.invalid],
  error: t.invalid,
  header: t.heading,
  property: t.propertyName,
  'http-method': t.attributeName,
  'http-protocol': t.meta,
  'http-path': t.url,
  'http-query': [t.url, t.meta],
  'http-status': [t.number, t.strong],
  'http-status-success': [t.number, t.bool, t.strong],
  'http-status-redirect': [t.number, t.special(t.atom), t.strong],
  'http-status-client-error': [t.number, t.special(t.number), t.strong],
  'http-status-server-error': [t.number, t.invalid, t.strong],
  'http-status-text': t.name,
  'http-header-name': t.attributeName,
  'http-header-name-important': t.attributeName,
  'http-header-name-meta': t.attributeName,
  'http-header-name-sensitive': t.attributeName,
  'http-header-value': t.string,
  'http-header-value-important': t.string,
  'http-header-value-meta': t.string,
  'http-header-value-sensitive': t.string,
  'http-cookie-name': t.special(t.attributeName),
  'http-cookie-attribute': t.special(t.attributeName),
  'http-cookie-attribute-value': t.string,
  'http-cookie-value-sensitive': t.string,
  'http-cookie-flag': t.special(t.attributeName),
  'http-cookie-separator': t.punctuation,
} as const

const IMPORTANT_HEADERS = new Set([
  'host',
  'content-type',
  'content-length',
  'user-agent',
])

const META_HEADERS = new Set([
  'accept',
  'accept-language',
  'accept-encoding',
  'cache-control',
  'pragma',
  'referer',
  'origin',
])

const SENSITIVE_HEADERS = new Set([
  'authorization',
  'proxy-authorization',
  'cookie',
  'set-cookie',
  'x-api-key',
  'x-auth-token',
  'token',
])

const getHeaderNameToken = (name: string): string => {
  const normalized = name.trim().toLowerCase()
  if (SENSITIVE_HEADERS.has(normalized)) return 'http-header-name-sensitive'
  if (IMPORTANT_HEADERS.has(normalized)) return 'http-header-name-important'
  if (META_HEADERS.has(normalized)) return 'http-header-name-meta'
  return 'http-header-name'
}

const getHeaderValueToken = (name: string): string => {
  const normalized = name.trim().toLowerCase()
  if (SENSITIVE_HEADERS.has(normalized)) return 'http-header-value-sensitive'
  if (IMPORTANT_HEADERS.has(normalized)) return 'http-header-value-important'
  if (META_HEADERS.has(normalized)) return 'http-header-value-meta'
  return 'http-header-value'
}

const cloneLegacyState = (value: unknown) => {
  if (value == null) return value
  if (typeof structuredClone === 'function') {
    try {
      return structuredClone(value)
    } catch {
      // Some legacy mode states include functions or parser internals that
      // aren't structured-cloneable. Fall back to the same shallow-copy
      // behavior CodeMirror uses for stream parser state objects.
    }
  }
  if (Array.isArray(value)) return [...value]
  if (typeof value === 'object') return { ...(value as Record<string, unknown>) }
  return value
}

const withBurpTokenTable = (mode: LegacyMode): LegacyMode => ({
  ...mode,
  tokenTable: BURP_HTTP_TOKEN_TABLE,
})

const detectBodyMode = (content: string): { key: BodyModeKey; mode: LegacyMode | null } => {
  const parsed = parseHttpMessageDocument(content)

  switch (detectHttpBodyLanguage(parsed.body, parsed.contentType)) {
    case 'html':
      return { key: 'html', mode: withBurpTokenTable(html as LegacyMode) }
    case 'xml':
      return { key: 'xml', mode: withBurpTokenTable(xml as LegacyMode) }
    case 'json':
      return { key: 'json', mode: withBurpTokenTable(json as LegacyMode) }
    case 'css':
      return { key: 'css', mode: withBurpTokenTable(css as LegacyMode) }
    case 'javascript':
      return { key: 'javascript', mode: withBurpTokenTable(javascript as LegacyMode) }
    default:
      return { key: 'none', mode: null }
  }
}

const createBurpLikeHttpMode = (bodyMode: LegacyMode | null): LegacyMode => ({
  name: 'burp-http',
  tokenTable: BURP_HTTP_TOKEN_TABLE,
  startState(): BurpHttpState {
    return {
      phase: 'start',
      inBody: false,
      bodyState: bodyMode?.startState ? bodyMode.startState() : null,
      headerNameToken: 'http-header-name',
      headerValueToken: 'http-header-value',
      cookieTokenizer: createCookieHeaderTokenizerState('none'),
    }
  },
  copyState(state: unknown): BurpHttpState {
    const current = state as BurpHttpState
    return {
      phase: current.phase,
      inBody: current.inBody,
      bodyState: bodyMode?.copyState ? bodyMode.copyState(current.bodyState) : cloneLegacyState(current.bodyState),
      headerNameToken: current.headerNameToken,
      headerValueToken: current.headerValueToken,
      cookieTokenizer: { ...current.cookieTokenizer },
    }
  },
  blankLine(state: unknown) {
    const current = state as BurpHttpState
    if (current.inBody) {
      return bodyMode?.blankLine ? bodyMode.blankLine(current.bodyState) : null
    }
    current.inBody = true
    current.phase = 'body'
    current.cookieTokenizer = createCookieHeaderTokenizerState('none')
    return bodyMode?.blankLine ? bodyMode.blankLine(current.bodyState) : null
  },
  token(stream, state: unknown) {
    const current = state as BurpHttpState
    if (current.inBody) {
      if (!bodyMode) {
        stream.skipToEnd()
        return null
      }
      return bodyMode.token(stream, current.bodyState)
    }

    switch (current.phase) {
      case 'start':
        if (stream.eatSpace()) return null
        if (stream.match(/^HTTP\/\d(?:\.\d+)?/)) {
          current.phase = 'responseStatus'
          return 'http-protocol'
        }
        if (stream.match(/^[A-Z]+/)) {
          current.phase = 'requestPath'
          return 'http-method'
        }
        stream.skipToEnd()
        current.phase = 'header'
        return 'error'
      case 'requestPath':
        if (stream.eatSpace()) return null
        stream.eatWhile(/[^\s?#]/)
        if (stream.peek() === '?' || stream.peek() === '#') {
          current.phase = 'requestQuery'
        } else {
          current.phase = 'requestProtocol'
        }
        return 'http-path'
      case 'requestQuery':
        if (stream.eatSpace()) return null
        stream.eatWhile(/[^\s]/)
        current.phase = 'requestProtocol'
        return 'http-query'
      case 'requestProtocol':
        if (stream.eatSpace()) return null
        if (stream.match(/^HTTP\/\d(?:\.\d+)?/)) {
          current.phase = 'header'
          return 'http-protocol'
        }
        stream.skipToEnd()
        current.phase = 'header'
        return 'error'
      case 'responseStatus':
        if (stream.eatSpace()) return null
        if (stream.match(/^\d+/)) {
          current.phase = 'responseText'
          const statusCode = Number(stream.current())
          if (statusCode >= 200 && statusCode < 300) return 'http-status-success'
          if (statusCode >= 300 && statusCode < 400) return 'http-status-redirect'
          if (statusCode >= 400 && statusCode < 500) return 'http-status-client-error'
          if (statusCode >= 500 && statusCode < 600) return 'http-status-server-error'
          return 'http-status'
        }
        stream.skipToEnd()
        current.phase = 'header'
        return 'error'
      case 'responseText':
        if (stream.eatSpace()) return null
        stream.skipToEnd()
        current.phase = 'header'
        return 'http-status-text'
      case 'header':
        if (stream.sol() && !stream.eat(/[ \t]/)) {
          if (stream.match(/^[^:\s][^:]*(?=:)/)) {
            const headerName = stream.current()
            current.headerNameToken = getHeaderNameToken(headerName)
            current.headerValueToken = getHeaderValueToken(headerName)
            current.cookieTokenizer = createCookieHeaderTokenizerState(getCookieHeaderValueMode(headerName))
            current.phase = 'headerColon'
            return current.headerNameToken
          }
          stream.skipToEnd()
          return 'error'
        }
        stream.skipToEnd()
        return 'http-header-value'
      case 'headerColon':
        if (stream.eat(':')) {
          current.phase = 'headerValue'
          return current.headerNameToken
        }
        current.phase = 'headerValue'
        return null
      case 'headerValue':
        if (stream.eatSpace()) return null
        if (current.cookieTokenizer.mode !== 'none') {
          const token = tokenizeCookieHeaderValue(stream, current.cookieTokenizer)
          if (stream.eol()) {
            current.phase = 'header'
            current.cookieTokenizer = createCookieHeaderTokenizerState('none')
          }
          return token
        }
        stream.skipToEnd()
        current.phase = 'header'
        return current.headerValueToken
      default:
        stream.skipToEnd()
        return null
    }
  },
})

export const getHttpLanguageSignature = (content: string): string => detectBodyMode(content).key

export const getHttpEditorLanguageExtensions = (content: string): Extension[] => {
  const { mode } = detectBodyMode(content)
  return [StreamLanguage.define(createBurpLikeHttpMode(mode) as never)]
}
