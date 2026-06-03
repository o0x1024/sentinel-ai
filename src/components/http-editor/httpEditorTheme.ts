import { HighlightStyle, syntaxHighlighting } from '@codemirror/language'
import { EditorView } from 'codemirror'
import { tags as t } from '@lezer/highlight'
import {
  httpEditorCookieNameTag,
  httpEditorCookieValueTag,
  httpEditorHeaderNameTag,
  httpEditorHeaderValueTag,
  httpEditorMethodTag,
  httpEditorParameterKeyTag,
  httpEditorParameterValueTag,
  httpEditorProtocolTag,
} from './httpEditorHighlightTags'

interface HttpEditorThemeOptions {
  backgroundColor?: string
  color?: string
  gutterBackgroundColor?: string
  gutterColor?: string
  gutterBorderRight?: string
  activeLineBackgroundColor?: string
  activeLineGutterBackgroundColor?: string
  selectionBackgroundColor?: string
  caretColor?: string
}

interface HttpHighlightPalette {
  methodColor: string
  protocolColor: string
  pathColor: string
  headerKeyColor: string
  headerValueColor: string
  bodyKeyColor: string
  cookieKeyColor: string
  cookieValueColor: string
  parameterKeyColor: string
  parameterValueColor: string
  valueColor: string
  stringColor: string
  numberColor: string
  booleanColor: string
  subtleValueColor: string
  punctuationColor: string
  commentColor: string
  invalidColor: string
  tagBracketColor: string
  statusRedirectColor: string
  statusClientErrorColor: string
}

const createHttpHighlightStyle = (palette: HttpHighlightPalette) => HighlightStyle.define([
  { tag: t.meta, color: palette.subtleValueColor },
  { tag: t.url, color: palette.pathColor },
  { tag: t.string, color: palette.stringColor },
  { tag: t.number, color: palette.numberColor },
  { tag: t.bool, color: palette.booleanColor, fontWeight: '500' },
  { tag: t.atom, color: palette.booleanColor },
  { tag: t.variableName, color: palette.valueColor },
  { tag: t.name, color: palette.valueColor },
  { tag: t.standard(t.variableName), color: palette.valueColor },
  { tag: t.definition(t.variableName), color: palette.valueColor },
  { tag: t.local(t.variableName), color: palette.valueColor },
  { tag: t.modifier, color: palette.valueColor },
  { tag: t.typeName, color: palette.valueColor },
  { tag: t.controlKeyword, color: palette.valueColor },
  { tag: t.operatorKeyword, color: palette.bodyKeyColor },
  { tag: t.special(t.string), color: palette.stringColor },
  { tag: httpEditorMethodTag, color: palette.methodColor, fontWeight: '600' },
  { tag: httpEditorProtocolTag, color: palette.protocolColor },
  { tag: httpEditorHeaderNameTag, color: palette.headerKeyColor, fontWeight: '600' },
  { tag: httpEditorHeaderValueTag, color: palette.headerValueColor },
  { tag: httpEditorCookieNameTag, color: palette.cookieKeyColor, fontWeight: '600' },
  { tag: httpEditorCookieValueTag, color: palette.cookieValueColor },
  { tag: t.attributeName, color: palette.headerKeyColor, fontWeight: '600' },
  { tag: httpEditorParameterKeyTag, color: palette.parameterKeyColor, fontWeight: '700' },
  { tag: httpEditorParameterValueTag, color: palette.parameterValueColor },
  { tag: t.propertyName, color: palette.bodyKeyColor, fontWeight: '700' },
  { tag: t.tagName, color: palette.bodyKeyColor, fontWeight: '600' },
  { tag: t.keyword, color: palette.bodyKeyColor, fontWeight: '600' },
  { tag: t.special(t.atom), color: palette.statusRedirectColor, fontWeight: '600' },
  { tag: t.special(t.number), color: palette.statusClientErrorColor, fontWeight: '600' },
  { tag: t.angleBracket, color: palette.tagBracketColor },
  { tag: t.bracket, color: palette.tagBracketColor },
  { tag: t.paren, color: palette.tagBracketColor },
  { tag: t.squareBracket, color: palette.tagBracketColor },
  { tag: t.brace, color: palette.tagBracketColor },
  { tag: t.comment, color: palette.commentColor },
  { tag: t.lineComment, color: palette.commentColor },
  { tag: t.blockComment, color: palette.commentColor },
  { tag: t.punctuation, color: palette.punctuationColor },
  { tag: t.operator, color: palette.punctuationColor },
  { tag: t.strong, fontWeight: '600' },
  { tag: t.invalid, color: palette.invalidColor },
])

const createLightHttpCodeTheme = (options: HttpEditorThemeOptions = {}) => EditorView.theme({
  '&': {
    height: '100%',
    backgroundColor: options.backgroundColor ?? 'transparent',
    color: options.color ?? '#111827',
  },
  '.cm-content': {
    fontFamily: 'var(--traffic-editor-font-family)',
    caretColor: options.caretColor ?? options.color ?? '#111827',
  },
  '.cm-cursor': {
    borderLeftColor: options.caretColor ?? options.color ?? '#111827',
  },
  '&.cm-focused .cm-selectionBackground, .cm-selectionBackground': {
    backgroundColor: options.selectionBackgroundColor ?? '#d7e8ff',
  },
  '.cm-gutters': {
    backgroundColor: options.gutterBackgroundColor ?? 'transparent',
    color: options.gutterColor ?? '#8b93a1',
    borderRight: options.gutterBorderRight ?? 'none',
  },
  '.cm-activeLine': {
    backgroundColor: options.activeLineBackgroundColor ?? 'transparent',
  },
  '.cm-activeLineGutter': {
    backgroundColor: options.activeLineGutterBackgroundColor ?? options.gutterBackgroundColor ?? 'transparent',
  },
  '.cm-searchMatch': {
    backgroundColor: '#fff3a3',
    boxShadow: 'inset 0 0 0 1px #d9b74d',
    color: '#111111',
  },
  '.cm-searchMatch.cm-searchMatch-selected': {
    backgroundColor: '#ffcc73',
    boxShadow: 'inset 0 0 0 1px #cf8a18',
    color: '#111111',
  },
  '.cm-selectionMatch': {
    backgroundColor: '#e6f0ff',
  },
}, { dark: false })

const createDarkHttpCodeTheme = (options: HttpEditorThemeOptions = {}) => EditorView.theme({
  '&': {
    height: '100%',
    backgroundColor: options.backgroundColor ?? 'transparent',
    color: options.color ?? '#e5e7eb',
  },
  '.cm-content': {
    fontFamily: 'var(--traffic-editor-font-family)',
    caretColor: options.caretColor ?? options.color ?? '#e5e7eb',
  },
  '.cm-cursor': {
    borderLeftColor: options.caretColor ?? options.color ?? '#e5e7eb',
  },
  '&.cm-focused .cm-selectionBackground, .cm-selectionBackground': {
    backgroundColor: options.selectionBackgroundColor ?? '#1f3a5f',
  },
  '.cm-gutters': {
    backgroundColor: options.gutterBackgroundColor ?? 'transparent',
    color: options.gutterColor ?? '#64748b',
    borderRight: options.gutterBorderRight ?? 'none',
  },
  '.cm-activeLine': {
    backgroundColor: options.activeLineBackgroundColor ?? 'transparent',
  },
  '.cm-activeLineGutter': {
    backgroundColor: options.activeLineGutterBackgroundColor ?? options.gutterBackgroundColor ?? 'transparent',
  },
  '.cm-searchMatch': {
    backgroundColor: '#6c5312',
    boxShadow: 'inset 0 0 0 1px #9f7b1b',
    color: '#f8fafc',
  },
  '.cm-searchMatch.cm-searchMatch-selected': {
    backgroundColor: '#8b5e14',
    boxShadow: 'inset 0 0 0 1px #d7a13d',
    color: '#f8fafc',
  },
  '.cm-selectionMatch': {
    backgroundColor: '#243349',
  },
}, { dark: true })

export const lightHttpHighlightStyle = createHttpHighlightStyle({
  methodColor: '#111111',
  protocolColor: '#111111',
  pathColor: '#111111',
  headerKeyColor: '#2b63c6',
  headerValueColor: '#111111',
  bodyKeyColor: '#2b63c6',
  cookieKeyColor: '#2b63c6',
  cookieValueColor: '#c53030',
  parameterKeyColor: '#2b63c6',
  parameterValueColor: '#c55a11',
  valueColor: '#111111',
  stringColor: '#2f8f2f',
  numberColor: '#111111',
  booleanColor: '#2b63c6',
  subtleValueColor: '#5f6b7a',
  punctuationColor: '#4a5565',
  commentColor: '#7b8694',
  invalidColor: '#cc1f1a',
  tagBracketColor: '#6b7280',
  statusRedirectColor: '#b7791f',
  statusClientErrorColor: '#c53030',
})

export const darkHttpHighlightStyle = createHttpHighlightStyle({
  methodColor: '#f5f7fa',
  protocolColor: '#cbd5e1',
  pathColor: '#f5f7fa',
  headerKeyColor: '#7fb0ff',
  headerValueColor: '#e5e7eb',
  bodyKeyColor: '#7fb0ff',
  cookieKeyColor: '#7fb0ff',
  cookieValueColor: '#ff8a8a',
  parameterKeyColor: '#7fb0ff',
  parameterValueColor: '#f1a45b',
  valueColor: '#e5e7eb',
  stringColor: '#7ddc6f',
  numberColor: '#f5f7fa',
  booleanColor: '#7fb0ff',
  subtleValueColor: '#a9b6c5',
  punctuationColor: '#94a3b8',
  commentColor: '#7c8aa0',
  invalidColor: '#f87171',
  tagBracketColor: '#94a3b8',
  statusRedirectColor: '#fbbf24',
  statusClientErrorColor: '#fb923c',
})

const DARK_HTTP_EDITOR_THEMES = new Set([
  'dark',
  'synthwave',
  'halloween',
  'forest',
  'black',
  'luxury',
  'dracula',
])

export const isDarkHttpEditorTheme = (): boolean =>
  DARK_HTTP_EDITOR_THEMES.has(document.documentElement.getAttribute('data-theme') || '')

export const getHttpCodeThemeExtensions = (highlightEnabled: boolean, options: HttpEditorThemeOptions = {}) => {
  const dark = isDarkHttpEditorTheme()
  const theme = dark ? createDarkHttpCodeTheme(options) : createLightHttpCodeTheme(options)

  if (!highlightEnabled) return [theme]

  return [
    theme,
    syntaxHighlighting(dark ? darkHttpHighlightStyle : lightHttpHighlightStyle),
  ]
}
