import { StringStream } from '@codemirror/language'
import { json } from '@codemirror/legacy-modes/mode/javascript'

type LegacyMode = any

const JSON_PROPERTY_STYLE = 'string property'
const PROPERTY_SEPARATOR_PATTERN = /^\s*:/

function isJsonPropertyStringToken(stream: StringStream, tokenText: string) {
  const trimmedToken = tokenText.trimStart()
  if (!trimmedToken.startsWith('"')) return false

  return PROPERTY_SEPARATOR_PATTERN.test(stream.string.slice(stream.pos))
}

export function createHttpJsonStreamMode(options: { tokenTable: LegacyMode['tokenTable'] }): LegacyMode {
  const baseMode = json as LegacyMode

  return {
    ...baseMode,
    tokenTable: options.tokenTable,
    token(stream: StringStream, state: unknown) {
      const style = baseMode.token(stream, state)

      if (style === 'string' && isJsonPropertyStringToken(stream, stream.current())) {
        return JSON_PROPERTY_STYLE
      }

      return style
    },
  }
}
