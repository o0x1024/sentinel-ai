import { Tag, tags as t } from '@lezer/highlight'

export const httpEditorParameterKeyTag = Tag.define('http-editor-parameter-key', t.propertyName)
export const httpEditorParameterValueTag = Tag.define('http-editor-parameter-value', t.string)
