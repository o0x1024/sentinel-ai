import { HighlightStyle, syntaxHighlighting } from '@codemirror/language'
import { EditorView } from 'codemirror'
import { tags as t } from '@lezer/highlight'
import {
  httpEditorParameterKeyTag,
  httpEditorParameterValueTag,
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
  headerKeyColor: string
  bodyKeyColor: string
  cookieKeyColor: string
  parameterKeyColor: string
  parameterValueColor: string
  valueColor: string
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
  { tag: t.url, color: palette.valueColor },
  { tag: t.string, color: palette.valueColor },
  { tag: t.number, color: palette.valueColor },
  { tag: t.bool, color: palette.valueColor, fontWeight: '500' },
  { tag: t.atom, color: palette.valueColor },
  { tag: t.variableName, color: palette.valueColor },
  { tag: t.name, color: palette.valueColor },
  { tag: t.standard(t.variableName), color: palette.valueColor },
  { tag: t.definition(t.variableName), color: palette.valueColor },
  { tag: t.local(t.variableName), color: palette.valueColor },
  { tag: t.modifier, color: palette.valueColor },
  { tag: t.typeName, color: palette.valueColor },
  { tag: t.controlKeyword, color: palette.valueColor },
  { tag: t.operatorKeyword, color: palette.valueColor },
  { tag: t.special(t.string), color: palette.subtleValueColor, fontStyle: 'italic' },
  { tag: t.special(t.attributeName), color: palette.cookieKeyColor, fontWeight: '600' },
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
  { tag: t.strong, fontWeight: '600', color: palette.valueColor },
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
}, { dark: true })

export const lightHttpHighlightStyle = createHttpHighlightStyle({
  headerKeyColor: '#1d4ed8',
  bodyKeyColor: '#6d28d9',
  cookieKeyColor: '#0f766e',
  parameterKeyColor: '#1d4ed8',
  parameterValueColor: '#b42318',
  valueColor: '#111827',
  subtleValueColor: '#475569',
  punctuationColor: '#6b7280',
  commentColor: '#6b7280',
  invalidColor: '#dc2626',
  tagBracketColor: '#94a3b8',
  statusRedirectColor: '#b45309',
  statusClientErrorColor: '#c2410c',
})

export const darkHttpHighlightStyle = createHttpHighlightStyle({
  headerKeyColor: '#93c5fd',
  bodyKeyColor: '#ddd6fe',
  cookieKeyColor: '#5eead4',
  parameterKeyColor: '#7dd3fc',
  parameterValueColor: '#fca5a5',
  valueColor: '#e5e7eb',
  subtleValueColor: '#cbd5e1',
  punctuationColor: '#94a3b8',
  commentColor: '#94a3b8',
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
