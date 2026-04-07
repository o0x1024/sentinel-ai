import type {
  IntruderAttackPlan,
  IntruderAttackType,
  IntruderPayloadProcessingRule,
  IntruderPayloadSet,
  IntruderPosition,
} from './types'
import { applyPayloadProcessingRules } from './payloadProcessing'
import { expandPayloadSet } from './payloads'

const MARKER = '§'

interface TemplateParts {
  segments: string[]
  tokens: string[]
}

export interface IntruderPayloadResolutionContext {
  template: string
  positions: IntruderPosition[]
}

export interface IntruderPayloadPluginProcessingContext {
  payloadSet: IntruderPayloadSet | undefined
  originalPayload: string
  baseValue: string
  positionIndex: number
  template: string
  positions: IntruderPosition[]
}

export function clearIntruderMarkers(input: string): string {
  return input.split(MARKER).join('')
}

export function wrapSelectionWithMarkers(input: string, selectionStart: number, selectionEnd: number): string {
  if (selectionStart === selectionEnd) return input

  const start = Math.min(selectionStart, selectionEnd)
  const end = Math.max(selectionStart, selectionEnd)
  const selected = input.slice(start, end)

  if (!selected) return input

  return `${input.slice(0, start)}${MARKER}${selected}${MARKER}${input.slice(end)}`
}

export function extractIntruderPositions(input: string): IntruderPosition[] {
  const positions: IntruderPosition[] = []
  let cursor = 0
  let index = 0

  while (cursor < input.length) {
    const start = input.indexOf(MARKER, cursor)
    if (start === -1) break

    const end = input.indexOf(MARKER, start + 1)
    if (end === -1) break

    const value = input.slice(start + 1, end)
    positions.push({
      index,
      start,
      end,
      value,
      preview: value.length > 32 ? `${value.slice(0, 29)}...` : value,
    })

    cursor = end + 1
    index += 1
  }

  return positions
}

function splitTemplate(input: string): TemplateParts {
  const segments: string[] = []
  const tokens: string[] = []
  let cursor = 0

  while (cursor < input.length) {
    const start = input.indexOf(MARKER, cursor)
    if (start === -1) break

    const end = input.indexOf(MARKER, start + 1)
    if (end === -1) break

    segments.push(input.slice(cursor, start))
    tokens.push(input.slice(start + 1, end))
    cursor = end + 1
  }

  segments.push(input.slice(cursor))
  return { segments, tokens }
}

function joinTemplate(segments: string[], values: string[]): string {
  let output = ''

  for (let index = 0; index < values.length; index += 1) {
    output += segments[index] ?? ''
    output += values[index] ?? ''
  }

  output += segments[values.length] ?? ''
  return output
}

export function getRequiredPayloadSetCount(attackType: IntruderAttackType, positionCount: number): number {
  if (attackType === 'sniper' || attackType === 'batteringRam') {
    return 1
  }

  return Math.max(positionCount, 1)
}

export function encodeSelectedPayloadCharacters(payload: string, characters: string): string {
  if (!characters) return payload

  const selected = new Set(Array.from(characters))
  let output = ''

  for (const char of Array.from(payload)) {
    output += selected.has(char) ? encodeURIComponent(char) : char
  }

  return output
}

async function encodePayload(
  payload: string,
  payloadSet: IntruderPayloadSet | undefined,
  processingRules: IntruderPayloadProcessingRule[],
  baseValue: string,
  positionIndex: number,
  context: IntruderPayloadResolutionContext,
  pluginProcessor?: (payload: string, context: IntruderPayloadPluginProcessingContext) => Promise<string | null>,
): Promise<string | null> {
  const processed = await applyPayloadProcessingRules(payload, processingRules, {
    originalPayload: payload,
    baseValue,
  })
  if (processed == null) return null

  const pluginProcessed = pluginProcessor
    ? await pluginProcessor(processed, {
      payloadSet,
      originalPayload: payload,
      baseValue,
      positionIndex,
      template: context.template,
      positions: context.positions,
    })
    : processed
  if (pluginProcessed == null) return null
  if (!payloadSet?.urlEncode) return pluginProcessed
  return encodeSelectedPayloadCharacters(pluginProcessed, payloadSet.urlEncodeCharacters)
}

export function estimateAttackCount(
  attackType: IntruderAttackType,
  positionCount: number,
  payloadSets: IntruderPayloadSet[],
): number {
  if (!positionCount) return 0

  const payloadLists = payloadSets.map((payloadSet) => expandPayloadSet(payloadSet))

  if (attackType === 'sniper') {
    return positionCount * (payloadLists[0]?.length ?? 0)
  }

  if (attackType === 'batteringRam') {
    return payloadLists[0]?.length ?? 0
  }

  if (attackType === 'pitchfork') {
    const relevant = payloadLists.slice(0, positionCount)
    if (!relevant.length || relevant.some((payloads) => payloads.length === 0)) return 0
    return Math.min(...relevant.map((payloads) => payloads.length))
  }

  const relevant = payloadLists.slice(0, positionCount)
  if (!relevant.length || relevant.some((payloads) => payloads.length === 0)) return 0
  return relevant.reduce((total, payloads) => total * payloads.length, 1)
}

export function autoMarkIntruderPositions(rawRequest: string): string {
  const normalized = clearIntruderMarkers(rawRequest).replace(/\r\n/g, '\n').replace(/\r/g, '\n')
  const separatorIndex = normalized.indexOf('\n\n')
  const headerPart = separatorIndex === -1 ? normalized : normalized.slice(0, separatorIndex)
  const bodyPart = separatorIndex === -1 ? '' : normalized.slice(separatorIndex + 2)
  const headerLines = headerPart.split('\n')
  const requestLine = headerLines[0] ?? ''
  const headers = headerLines.slice(1)

  const markedRequestLine = markQueryStringValues(requestLine)
  const contentTypeHeader = headers.find((line) => line.toLowerCase().startsWith('content-type:')) ?? ''
  const contentType = contentTypeHeader.split(':').slice(1).join(':').trim().toLowerCase()

  let markedBody = bodyPart
  if (contentType.includes('application/x-www-form-urlencoded')) {
    markedBody = markFormUrlEncodedValues(bodyPart)
  } else if (contentType.includes('application/json')) {
    markedBody = markJsonPrimitiveValues(bodyPart)
  }

  const rebuilt = [markedRequestLine, ...headers].join('\n')
  return separatorIndex === -1 ? rebuilt : `${rebuilt}\n\n${markedBody}`.replace(/\n/g, '\r\n')
}

function markQueryStringValues(requestLine: string): string {
  const match = requestLine.match(/^(\S+\s+)(\S+)(\s+HTTP\/[\d.]+)$/)
  if (!match) return requestLine

  const [, prefix, target, suffix] = match
  const questionIndex = target.indexOf('?')
  if (questionIndex === -1) return requestLine

  const path = target.slice(0, questionIndex)
  const query = target.slice(questionIndex + 1)
  const params = query.split('&').map((entry) => {
    const equalIndex = entry.indexOf('=')
    if (equalIndex === -1) return entry
    return `${entry.slice(0, equalIndex + 1)}${MARKER}${entry.slice(equalIndex + 1)}${MARKER}`
  })

  return `${prefix}${path}?${params.join('&')}${suffix}`
}

function markFormUrlEncodedValues(body: string): string {
  return body
    .split('&')
    .map((entry) => {
      const equalIndex = entry.indexOf('=')
      if (equalIndex === -1) return entry
      return `${entry.slice(0, equalIndex + 1)}${MARKER}${entry.slice(equalIndex + 1)}${MARKER}`
    })
    .join('&')
}

function markJsonPrimitiveValues(body: string): string {
  return body.replace(
    /:\s*("(?:\\.|[^"])*"|true|false|null|-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?)/g,
    (_, value: string) => `: ${MARKER}${value}${MARKER}`,
  )
}

export async function buildIntruderAttackPlan(options: {
  template: string
  attackType: IntruderAttackType
  payloadSets: IntruderPayloadSet[]
  payloadProcessingRules?: IntruderPayloadProcessingRule[]
  maxRequests: number
  payloadResolver?: (
    payloadSet: IntruderPayloadSet,
    context: IntruderPayloadResolutionContext,
  ) => Promise<string[]>
  payloadPluginProcessor?: (
    payload: string,
    context: IntruderPayloadPluginProcessingContext,
  ) => Promise<string | null>
}): Promise<IntruderAttackPlan> {
  const {
    template,
    attackType,
    payloadSets,
    payloadProcessingRules = [],
    maxRequests,
    payloadResolver,
    payloadPluginProcessor,
  } = options
  const positions = extractIntruderPositions(template)

  if (!positions.length) {
    return { requests: [], totalGenerated: 0, truncated: false }
  }

  const payloadContext = { template, positions }
  const payloadLists = await Promise.all(
    payloadSets.map(async (payloadSet) => {
      if (payloadSet.payloadType === 'extensionGenerated' && payloadResolver) {
        return payloadResolver(payloadSet, payloadContext)
      }
      return expandPayloadSet(payloadSet)
    }),
  )
  const { segments, tokens } = splitTemplate(template)
  const requests: IntruderAttackPlan['requests'] = []
  const totalGenerated = estimateAttackCountFromPayloadLists(attackType, positions.length, payloadLists)
  let truncated = false

  const pushRequest = (values: string[], payloadValues: string[]) => {
    if (requests.length >= maxRequests) {
      truncated = true
      return
    }

    requests.push({
      requestText: joinTemplate(segments, values),
      payloadValues,
      payloadSummary: payloadValues.map((value, index) => `P${index + 1}=${value}`).join(' | '),
    })
  }

  if (attackType === 'sniper') {
    const payloadSet = payloadSets[0]
    const payloads = payloadLists[0] ?? []

    for (const [positionIndex] of positions.entries()) {
      for (const payload of payloads) {
        const values = [...tokens]
        const encoded = await encodePayload(payload, payloadSet, payloadProcessingRules, tokens[positionIndex] ?? '', positionIndex, payloadContext, payloadPluginProcessor)
        if (encoded == null) continue
        values[positionIndex] = encoded
        pushRequest(values, [payload])
      }
    }

    return { requests, totalGenerated, truncated }
  }

  if (attackType === 'batteringRam') {
    const payloadSet = payloadSets[0]
    const payloads = payloadLists[0] ?? []

    for (const payload of payloads) {
      const values: string[] = []
      let shouldSkip = false

      for (const token of tokens) {
        const encoded = await encodePayload(payload, payloadSet, payloadProcessingRules, token, values.length, payloadContext, payloadPluginProcessor)
        if (encoded == null) {
          shouldSkip = true
          break
        }
        values.push(encoded)
      }

      if (shouldSkip) continue
      pushRequest(values, [payload])
    }

    return { requests, totalGenerated, truncated }
  }

  if (attackType === 'pitchfork') {
    const relevantSets = payloadSets.slice(0, positions.length)
    const relevantLists = payloadLists.slice(0, positions.length)
    if (!relevantLists.length || relevantLists.some((payloads) => payloads.length === 0)) {
      return { requests: [], totalGenerated: 0, truncated: false }
    }

    const iterations = Math.min(...relevantLists.map((payloads) => payloads.length))
    for (let iteration = 0; iteration < iterations; iteration += 1) {
      const rawPayloads = relevantLists.map((payloads) => payloads[iteration] ?? '')
      const values: string[] = []
      let shouldSkip = false

      for (const [index, payload] of rawPayloads.entries()) {
        const encoded = await encodePayload(payload, relevantSets[index], payloadProcessingRules, tokens[index] ?? '', index, payloadContext, payloadPluginProcessor)
        if (encoded == null) {
          shouldSkip = true
          break
        }
        values.push(encoded)
      }

      if (shouldSkip) continue
      pushRequest(values, rawPayloads)
    }

    return { requests, totalGenerated, truncated }
  }

  const relevantSets = payloadSets.slice(0, positions.length)
  const relevantLists = payloadLists.slice(0, positions.length)
  if (!relevantLists.length || relevantLists.some((payloads) => payloads.length === 0)) {
    return { requests: [], totalGenerated: 0, truncated: false }
  }

  const walk = async (depth: number, values: string[], rawValues: string[]): Promise<void> => {
    if (truncated && requests.length >= maxRequests) {
      return
    }

    if (depth === relevantLists.length) {
      pushRequest(values, rawValues)
      return
    }

    const payloads = relevantLists[depth]
    for (const payload of payloads) {
      const encoded = await encodePayload(payload, relevantSets[depth], payloadProcessingRules, tokens[depth] ?? '', depth, payloadContext, payloadPluginProcessor)
      if (encoded == null) continue
      await walk(depth + 1, [...values, encoded], [...rawValues, payload])
    }
  }

  await walk(0, [], [])
  return { requests, totalGenerated, truncated }
}

function estimateAttackCountFromPayloadLists(
  attackType: IntruderAttackType,
  positionCount: number,
  payloadLists: string[][],
): number {
  if (!positionCount) return 0

  if (attackType === 'sniper') {
    return positionCount * (payloadLists[0]?.length ?? 0)
  }

  if (attackType === 'batteringRam') {
    return payloadLists[0]?.length ?? 0
  }

  const relevant = payloadLists.slice(0, positionCount)
  if (!relevant.length || relevant.some((payloads) => payloads.length === 0)) return 0

  if (attackType === 'pitchfork') {
    return Math.min(...relevant.map((payloads) => payloads.length))
  }

  return relevant.reduce((total, payloads) => total * payloads.length, 1)
}
