import { Tag, tags as t } from '@lezer/highlight'

export const httpEditorParameterKeyTag = Tag.define('http-editor-parameter-key', t.propertyName)
export const httpEditorParameterValueTag = Tag.define('http-editor-parameter-value', t.string)
export const httpEditorMethodTag = Tag.define('http-editor-method')
export const httpEditorProtocolTag = Tag.define('http-editor-protocol', t.meta)
export const httpEditorHeaderNameTag = Tag.define('http-editor-header-name', t.attributeName)
export const httpEditorHeaderValueTag = Tag.define('http-editor-header-value', t.string)
export const httpEditorCookieNameTag = Tag.define('http-editor-cookie-name', t.attributeName)
export const httpEditorCookieValueTag = Tag.define('http-editor-cookie-value', t.string)
