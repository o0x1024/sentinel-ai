import { HighlightStyle, syntaxHighlighting } from '@codemirror/language'
import { EditorView } from 'codemirror'
import { tags as t } from '@lezer/highlight'

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

export const lightHttpHighlightStyle = HighlightStyle.define([
  { tag: t.attributeName, color: '#b45309' },
  { tag: t.propertyName, color: '#b45309' },
  { tag: t.keyword, color: '#b45309' },
  { tag: t.controlKeyword, color: '#111827' },
  { tag: t.operatorKeyword, color: '#111827' },
  { tag: t.meta, color: '#5f6b7a' },
  { tag: t.url, color: '#1f2937' },
  { tag: t.string, color: '#111827' },
  { tag: t.special(t.string), color: '#475569' },
  { tag: t.number, color: '#0f172a' },
  { tag: t.bool, color: '#15803d', fontWeight: '600' },
  { tag: t.atom, color: '#111827' },
  { tag: t.special(t.atom), color: '#b45309', fontWeight: '600' },
  { tag: t.special(t.number), color: '#c2410c', fontWeight: '600' },
  { tag: t.variableName, color: '#1f2937' },
  { tag: t.name, color: '#1f2937' },
  { tag: t.standard(t.variableName), color: '#1f2937' },
  { tag: t.definition(t.variableName), color: '#1f2937' },
  { tag: t.local(t.variableName), color: '#1f2937' },
  { tag: t.modifier, color: '#1f2937' },
  { tag: t.tagName, color: '#c218a1' },
  { tag: t.angleBracket, color: '#c218a1' },
  { tag: t.bracket, color: '#c218a1' },
  { tag: t.paren, color: '#c218a1' },
  { tag: t.squareBracket, color: '#c218a1' },
  { tag: t.brace, color: '#c218a1' },
  { tag: t.comment, color: '#6b7280' },
  { tag: t.lineComment, color: '#6b7280' },
  { tag: t.blockComment, color: '#6b7280' },
  { tag: t.punctuation, color: '#6b7280' },
  { tag: t.operator, color: '#6b7280' },
  { tag: t.strong, fontWeight: '600', color: '#0f172a' },
  { tag: t.invalid, color: '#dc2626' },
])

export const darkHttpHighlightStyle = HighlightStyle.define([
  { tag: t.attributeName, color: '#f59e0b' },
  { tag: t.propertyName, color: '#f59e0b' },
  { tag: t.keyword, color: '#f59e0b' },
  { tag: t.controlKeyword, color: '#e5e7eb' },
  { tag: t.operatorKeyword, color: '#e5e7eb' },
  { tag: t.meta, color: '#94a3b8' },
  { tag: t.url, color: '#e5e7eb' },
  { tag: t.string, color: '#e5e7eb' },
  { tag: t.special(t.string), color: '#cbd5e1' },
  { tag: t.number, color: '#f8fafc' },
  { tag: t.bool, color: '#4ade80', fontWeight: '600' },
  { tag: t.atom, color: '#e5e7eb' },
  { tag: t.special(t.atom), color: '#fbbf24', fontWeight: '600' },
  { tag: t.special(t.number), color: '#fb923c', fontWeight: '600' },
  { tag: t.variableName, color: '#e5e7eb' },
  { tag: t.name, color: '#e5e7eb' },
  { tag: t.standard(t.variableName), color: '#e5e7eb' },
  { tag: t.definition(t.variableName), color: '#e5e7eb' },
  { tag: t.local(t.variableName), color: '#e5e7eb' },
  { tag: t.modifier, color: '#e5e7eb' },
  { tag: t.tagName, color: '#f472d0' },
  { tag: t.angleBracket, color: '#f472d0' },
  { tag: t.bracket, color: '#f472d0' },
  { tag: t.paren, color: '#f472d0' },
  { tag: t.squareBracket, color: '#f472d0' },
  { tag: t.brace, color: '#f472d0' },
  { tag: t.comment, color: '#94a3b8' },
  { tag: t.lineComment, color: '#94a3b8' },
  { tag: t.blockComment, color: '#94a3b8' },
  { tag: t.punctuation, color: '#94a3b8' },
  { tag: t.operator, color: '#94a3b8' },
  { tag: t.strong, fontWeight: '600', color: '#f8fafc' },
  { tag: t.invalid, color: '#f87171' },
])

export const isDarkHttpEditorTheme = (): boolean => document.documentElement.getAttribute('data-theme') === 'dark'

export const getHttpCodeThemeExtensions = (highlightEnabled: boolean, options: HttpEditorThemeOptions = {}) => {
  const dark = isDarkHttpEditorTheme()
  const theme = dark ? createDarkHttpCodeTheme(options) : createLightHttpCodeTheme(options)

  if (!highlightEnabled) return [theme]

  return [
    theme,
    syntaxHighlighting(dark ? darkHttpHighlightStyle : lightHttpHighlightStyle),
  ]
}
