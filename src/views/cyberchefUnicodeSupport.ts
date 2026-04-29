const UNICODE_ESCAPE_PREFIX = '\\u'
const HEX_SEQUENCE_LENGTH = 4
const HIGH_SURROGATE_START = 0xd800
const HIGH_SURROGATE_END = 0xdbff
const LOW_SURROGATE_START = 0xdc00
const LOW_SURROGATE_END = 0xdfff

function toUnicodeEscape(codeUnit: number): string {
  return `${UNICODE_ESCAPE_PREFIX}${codeUnit.toString(16).padStart(HEX_SEQUENCE_LENGTH, '0')}`
}

function isHighSurrogate(codeUnit: number): boolean {
  return codeUnit >= HIGH_SURROGATE_START && codeUnit <= HIGH_SURROGATE_END
}

function isLowSurrogate(codeUnit: number): boolean {
  return codeUnit >= LOW_SURROGATE_START && codeUnit <= LOW_SURROGATE_END
}

export function encodeUnicodeEscapes(input: string): string {
  let output = ''

  for (const char of input) {
    const codePoint = char.codePointAt(0)
    if (codePoint == null) {
      continue
    }

    if (codePoint <= 0xffff) {
      output += toUnicodeEscape(codePoint)
      continue
    }

    const adjustedCodePoint = codePoint - 0x10000
    const highSurrogate = HIGH_SURROGATE_START + (adjustedCodePoint >> 10)
    const lowSurrogate = LOW_SURROGATE_START + (adjustedCodePoint & 0x3ff)
    output += `${toUnicodeEscape(highSurrogate)}${toUnicodeEscape(lowSurrogate)}`
  }

  return output
}

function readUnicodeEscape(input: string, index: number): { value: number; nextIndex: number } {
  const prefix = input.slice(index, index + 2)
  if (prefix !== UNICODE_ESCAPE_PREFIX) {
    throw new Error('Invalid Unicode escape sequence')
  }

  const hex = input.slice(index + 2, index + 2 + HEX_SEQUENCE_LENGTH)
  if (!/^[0-9a-fA-F]{4}$/.test(hex)) {
    throw new Error('Invalid Unicode escape sequence')
  }

  return {
    value: parseInt(hex, 16),
    nextIndex: index + 2 + HEX_SEQUENCE_LENGTH
  }
}

export function decodeUnicodeEscapes(input: string): string {
  let output = ''
  let index = 0

  while (index < input.length) {
    if (input[index] !== '\\') {
      output += input[index]
      index += 1
      continue
    }

    const current = readUnicodeEscape(input, index)

    if (isLowSurrogate(current.value)) {
      throw new Error('Invalid Unicode surrogate pair')
    }

    if (!isHighSurrogate(current.value)) {
      output += String.fromCharCode(current.value)
      index = current.nextIndex
      continue
    }

    if (current.nextIndex >= input.length) {
      throw new Error('Invalid Unicode surrogate pair')
    }

    const next = readUnicodeEscape(input, current.nextIndex)
    if (!isLowSurrogate(next.value)) {
      throw new Error('Invalid Unicode surrogate pair')
    }

    const codePoint = 0x10000 + ((current.value - HIGH_SURROGATE_START) << 10) + (next.value - LOW_SURROGATE_START)
    output += String.fromCodePoint(codePoint)
    index = next.nextIndex
  }

  return output
}
