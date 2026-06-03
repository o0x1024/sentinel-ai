import type { IntruderPayloadSet } from './types'

const MAX_CHARACTER_SUBSTITUTION_VARIANTS = 512

export function parsePayloadLines(payloadsText: string): string[] {
  return payloadsText
    .split(/\r\n|\r|\n/)
    .map((line) => line.trimEnd())
    .filter((line) => line.length > 0)
}

export function countPayloadSet(payloadSet: IntruderPayloadSet): number {
  switch (payloadSet.payloadType) {
    case 'bruteForcer':
      return countBruteForcerPayloads(payloadSet)
    default:
      return expandPayloadSet(payloadSet).length
  }
}

export function expandPayloadSet(payloadSet: IntruderPayloadSet, limit?: number): string[] {
  switch (payloadSet.payloadType) {
    case 'bruteForcer':
      return buildBruteForcerPayloads(payloadSet, limit)
    case 'appDictionary':
      return takePayloadLimit(parsePayloadLines(payloadSet.payloadsText), limit)
    case 'extensionGenerated':
      return takePayloadLimit(parsePayloadLines(payloadSet.payloadsText), limit)
    case 'characterSubstitution':
      return takePayloadLimit(buildCharacterSubstitutionPayloads(payloadSet), limit)
    case 'characterList':
      return takePayloadLimit(buildCharacterListPayloads(payloadSet), limit)
    case 'nullPayloads':
      return takePayloadLimit(buildNullPayloads(payloadSet), limit)
    case 'usernameGenerator':
      return takePayloadLimit(buildUsernameGeneratorPayloads(payloadSet), limit)
    case 'runtimeFile':
      return takePayloadLimit(parsePayloadLines(payloadSet.payloadsText), limit)
    case 'numbers':
      return takePayloadLimit(buildNumberPayloads(payloadSet), limit)
    case 'dates':
      return takePayloadLimit(buildDatePayloads(payloadSet), limit)
    case 'simpleList':
    default:
      return takePayloadLimit(parsePayloadLines(payloadSet.payloadsText), limit)
  }
}

function takePayloadLimit(values: string[], limit?: number): string[] {
  if (limit == null) return values
  const normalizedLimit = normalizePayloadLimit(limit)
  if (!Number.isFinite(normalizedLimit)) return values
  return values.slice(0, normalizedLimit)
}

function normalizePayloadLimit(limit?: number): number {
  if (limit == null || !Number.isFinite(limit)) return Number.POSITIVE_INFINITY
  return Math.max(0, Math.floor(limit))
}

function normalizeBruteForcerConfiguration(payloadSet: IntruderPayloadSet) {
  const characters = Array.from(new Set(Array.from(payloadSet.bruteForceCharacterSet || '')))
  const minLength = Math.max(0, Math.floor(Number(payloadSet.bruteForceMinLength) || 0))
  const rawMaxLength = Math.max(0, Math.floor(Number(payloadSet.bruteForceMaxLength) || 0))
  const maxLength = Math.max(minLength, rawMaxLength)

  return {
    characters,
    minLength,
    maxLength,
  }
}

function countBruteForcerPayloads(payloadSet: IntruderPayloadSet): number {
  const { characters, minLength, maxLength } = normalizeBruteForcerConfiguration(payloadSet)
  const base = characters.length

  if (!base) {
    return minLength === 0 && maxLength === 0 ? 1 : 0
  }

  let total = 0
  for (let length = minLength; length <= maxLength; length += 1) {
    const count = base ** length
    if (!Number.isFinite(count)) {
      return Number.MAX_SAFE_INTEGER
    }
    total += count
    if (!Number.isSafeInteger(total)) {
      return Number.MAX_SAFE_INTEGER
    }
  }

  return total
}

function buildBruteForcerPayloads(payloadSet: IntruderPayloadSet, limit?: number): string[] {
  const { characters, minLength, maxLength } = normalizeBruteForcerConfiguration(payloadSet)
  const normalizedLimit = normalizePayloadLimit(limit)
  const values: string[] = []

  if (!characters.length) {
    if (minLength === 0 && maxLength === 0 && normalizedLimit > 0) {
      return ['']
    }
    return values
  }

  const pushValue = (value: string): boolean => {
    if (values.length >= normalizedLimit) return true
    values.push(value)
    return values.length >= normalizedLimit
  }

  const buildLength = (targetLength: number, depth: number, current: string): boolean => {
    if (values.length >= normalizedLimit) return true
    if (depth >= targetLength) {
      return pushValue(current)
    }

    for (const character of characters) {
      if (buildLength(targetLength, depth + 1, `${current}${character}`)) {
        return true
      }
    }

    return false
  }

  for (let length = minLength; length <= maxLength; length += 1) {
    if (length === 0) {
      if (pushValue('')) break
      continue
    }

    if (buildLength(length, 0, '')) {
      break
    }
  }

  return values
}

function buildCharacterSubstitutionPayloads(payloadSet: IntruderPayloadSet): string[] {
  const sourceValues = parsePayloadLines(payloadSet.substitutionSource)
  const substitutionMap = parseSubstitutionRules(payloadSet.substitutionRules)
  if (!sourceValues.length) return []
  if (!substitutionMap.size) return sourceValues

  return sourceValues.flatMap((value) => generateSubstitutionVariants(value, substitutionMap))
}

function buildCharacterListPayloads(payloadSet: IntruderPayloadSet): string[] {
  return Array.from(new Set(Array.from(payloadSet.characterList || ''))).filter((char) => char.length > 0)
}

function buildNumberPayloads(payloadSet: IntruderPayloadSet): string[] {
  const start = Number.isFinite(payloadSet.numberFrom) ? payloadSet.numberFrom : 0
  const end = Number.isFinite(payloadSet.numberTo) ? payloadSet.numberTo : start
  const step = Math.max(1, Math.abs(payloadSet.numberStep || 1))
  const values: string[] = []

  if (start <= end) {
    for (let value = start; value <= end; value += step) {
      values.push(String(value).padStart(payloadSet.numberPadWidth || 0, '0'))
    }
    return values
  }

  for (let value = start; value >= end; value -= step) {
    values.push(String(value).padStart(payloadSet.numberPadWidth || 0, '0'))
  }
  return values
}

function buildNullPayloads(payloadSet: IntruderPayloadSet): string[] {
  const count = Math.max(0, payloadSet.nullCount || 0)
  return Array.from({ length: count }, () => payloadSet.nullValue ?? '')
}

function buildUsernameGeneratorPayloads(payloadSet: IntruderPayloadSet): string[] {
  const firstNames = parsePayloadLines(payloadSet.usernameFirstNames)
  const lastNames = parsePayloadLines(payloadSet.usernameLastNames)
  const formats = parsePayloadLines(payloadSet.usernameFormats)

  if (!firstNames.length || !lastNames.length) return []

  const resolvedFormats = formats.length ? formats : ['{first}.{last}', '{f}{last}', '{first}{l}']
  const values = new Set<string>()

  firstNames.forEach((firstName) => {
    lastNames.forEach((lastName) => {
      resolvedFormats.forEach((format) => {
        values.add(applyUsernameFormat(format, firstName, lastName))
      })
    })
  })

  return Array.from(values).filter((value) => value.length > 0)
}

function buildDatePayloads(payloadSet: IntruderPayloadSet): string[] {
  const startDate = parseDate(payloadSet.dateFrom)
  const endDate = parseDate(payloadSet.dateTo)
  if (!startDate || !endDate) return []

  const stepDays = Math.max(1, Math.abs(payloadSet.dateStepDays || 1))
  const values: string[] = []
  const direction = startDate.getTime() <= endDate.getTime() ? 1 : -1
  const cursor = new Date(startDate)

  while ((direction === 1 && cursor <= endDate) || (direction === -1 && cursor >= endDate)) {
    values.push(formatDate(cursor, payloadSet.dateFormat))
    cursor.setDate(cursor.getDate() + stepDays * direction)
  }

  return values
}

function parseDate(value: string): Date | null {
  if (!value) return null
  const parsed = new Date(value)
  if (Number.isNaN(parsed.getTime())) return null
  parsed.setHours(0, 0, 0, 0)
  return parsed
}

function parseSubstitutionRules(rulesText: string): Map<string, string[]> {
  const rules = new Map<string, string[]>()

  parsePayloadLines(rulesText).forEach((line) => {
    const match = line.match(/^(.+?)(?:=>|=|:)(.+)$/)
    if (!match) return

    const key = match[1]?.trim()
    const replacements = match[2]
      ?.split(/[,|]/)
      .map((value) => value.trim())
      .filter((value) => value.length > 0) ?? []

    if (!key || !replacements.length) return
    rules.set(key, Array.from(new Set(replacements)))
  })

  return rules
}

function generateSubstitutionVariants(input: string, substitutionMap: Map<string, string[]>): string[] {
  const variants = new Set<string>()
  const characters = Array.from(input)

  const expand = (index: number, current: string) => {
    if (variants.size >= MAX_CHARACTER_SUBSTITUTION_VARIANTS) return

    if (index >= characters.length) {
      variants.add(current)
      return
    }

    const char = characters[index]
    const replacements = substitutionMap.get(char) || substitutionMap.get(char.toLowerCase()) || []
    const nextValues = Array.from(new Set([char, ...replacements]))

    nextValues.forEach((value) => {
      expand(index + 1, `${current}${value}`)
    })
  }

  expand(0, '')
  return Array.from(variants)
}

function applyUsernameFormat(format: string, firstName: string, lastName: string): string {
  const first = normalizeUsernamePart(firstName)
  const last = normalizeUsernamePart(lastName)
  const replacements: Record<string, string> = {
    '{first}': first,
    '{last}': last,
    '{f}': first.slice(0, 1),
    '{l}': last.slice(0, 1),
  }

  return Object.entries(replacements).reduce(
    (value, [token, replacement]) => value.split(token).join(replacement),
    format,
  )
}

function normalizeUsernamePart(value: string): string {
  return value.trim().replace(/\s+/g, '').toLowerCase()
}

function formatDate(value: Date, format: IntruderPayloadSet['dateFormat']): string {
  const year = value.getFullYear()
  const month = String(value.getMonth() + 1).padStart(2, '0')
  const day = String(value.getDate()).padStart(2, '0')

  switch (format) {
    case 'yyyyMMdd':
      return `${year}${month}${day}`
    case 'MM/dd/yyyy':
      return `${month}/${day}/${year}`
    case 'yyyy-MM-dd':
    default:
      return `${year}-${month}-${day}`
  }
}
