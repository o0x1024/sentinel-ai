import { StreamLanguage } from '@codemirror/language'
import { javascript } from '@codemirror/lang-javascript'
import { json } from '@codemirror/lang-json'
import { css as legacyCss } from '@codemirror/legacy-modes/mode/css'
import { html as legacyHtml, xml as legacyXml } from '@codemirror/legacy-modes/mode/xml'
import { tags as t } from '@lezer/highlight'
import type { Extension } from '@codemirror/state'
import type { HttpBodyLanguage } from './httpDocument'

const LEGACY_BODY_TOKEN_TABLE = {
  variable: t.variableName,
  'variable-2': t.special(t.variableName),
  'variable-3': t.local(t.variableName),
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
  property: t.propertyName,
  word: t.name,
  atom: t.atom,
  error: t.invalid,
  header: t.heading,
} as const

const htmlBodyLanguage = StreamLanguage.define({
  ...legacyHtml,
  tokenTable: LEGACY_BODY_TOKEN_TABLE,
})

const xmlBodyLanguage = StreamLanguage.define({
  ...legacyXml,
  tokenTable: LEGACY_BODY_TOKEN_TABLE,
})

const cssBodyLanguage = StreamLanguage.define({
  ...legacyCss,
  tokenTable: LEGACY_BODY_TOKEN_TABLE,
})

export const getHttpBodyLanguageExtensions = (language: HttpBodyLanguage): Extension[] => {
  switch (language) {
    case 'json':
      return [json()]
    case 'javascript':
      return [javascript()]
    case 'html':
      return [htmlBodyLanguage]
    case 'xml':
      return [xmlBodyLanguage]
    case 'css':
      return [cssBodyLanguage]
    default:
      return []
  }
}
