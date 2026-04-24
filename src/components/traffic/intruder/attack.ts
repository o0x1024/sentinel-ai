import type {
  IntruderAttackPlan,
  IntruderAttackType,
  IntruderPayloadProcessingRule,
  IntruderPayloadSet,
  IntruderPosition,
} from './types'
import { applyPayloadProcessingRules } from './payloadProcessing'
import { countPayloadSet, expandPayloadSet } from './payloads'
import {
  INTRUDER_PRIMARY_MARKER,
  clearIntruderMarkers,
  extractIntruderPositions,
  splitIntruderTemplate,
  wrapSelectionWithMarkers,
} from './intruderMarkers'
export {
  clearIntruderMarkers,
  extractIntruderPositions,
  wrapSelectionWithMarkers,
} from './intruderMarkers'

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
  if (!positionCount) {
    return payloadSets[0] ? countPayloadSet(payloadSets[0]) : 0
  }

  const payloadCounts = payloadSets.map((payloadSet) => countPayloadSet(payloadSet))

  if (attackType === 'sniper') {
    return positionCount * (payloadCounts[0] ?? 0)
  }

  if (attackType === 'batteringRam') {
    return payloadCounts[0] ?? 0
  }

  return estimateAttackCountFromLengths(attackType, positionCount, payloadCounts)
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
    return `${entry.slice(0, equalIndex + 1)}${INTRUDER_PRIMARY_MARKER}${entry.slice(equalIndex + 1)}${INTRUDER_PRIMARY_MARKER}`
  })

  return `${prefix}${path}?${params.join('&')}${suffix}`
}

function markFormUrlEncodedValues(body: string): string {
  return body
    .split('&')
    .map((entry) => {
      const equalIndex = entry.indexOf('=')
      if (equalIndex === -1) return entry
      return `${entry.slice(0, equalIndex + 1)}${INTRUDER_PRIMARY_MARKER}${entry.slice(equalIndex + 1)}${INTRUDER_PRIMARY_MARKER}`
    })
    .join('&')
}

function markJsonPrimitiveValues(body: string): string {
  return body.replace(
    /:\s*("(?:\\.|[^"])*"|true|false|null|-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?)/g,
    (_, value: string) => `: ${INTRUDER_PRIMARY_MARKER}${value}${INTRUDER_PRIMARY_MARKER}`,
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
  const payloadContext = { template, positions }
  const payloadLists = await Promise.all(
    payloadSets.map(async (payloadSet) => {
      if (
        payloadResolver
        && (payloadSet.payloadType === 'extensionGenerated' || payloadSet.payloadType === 'appDictionary')
      ) {
        return payloadResolver(payloadSet, payloadContext)
      }
      if (payloadSet.payloadType === 'bruteForcer') {
        return expandPayloadSet(payloadSet, maxRequests)
      }
      return expandPayloadSet(payloadSet)
    }),
  )
  const { segments, tokens } = splitIntruderTemplate(template)
  const requests: IntruderAttackPlan['requests'] = []
  const totalGenerated = estimateAttackCountFromResolvedPayloads(
    attackType,
    positions.length,
    payloadSets,
    payloadLists,
  )
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

  if (!positions.length) {
    const payloads = payloadLists[0] ?? []
    const requestText = clearIntruderMarkers(template)

    for (const payload of payloads) {
      if (requests.length >= maxRequests) {
        truncated = true
        break
      }

      requests.push({
        requestText,
        payloadValues: [payload],
        payloadSummary: `P1=${payload}`,
      })
    }

    return { requests, totalGenerated, truncated }
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
  return estimateAttackCountFromLengths(
    attackType,
    positionCount,
    payloadLists.map((payloads) => payloads.length),
  )
}

function estimateAttackCountFromResolvedPayloads(
  attackType: IntruderAttackType,
  positionCount: number,
  payloadSets: IntruderPayloadSet[],
  payloadLists: string[][],
): number {
  return estimateAttackCountFromLengths(
    attackType,
    positionCount,
    payloadSets.map((payloadSet, index) =>
      payloadSet.payloadType === 'extensionGenerated' || payloadSet.payloadType === 'appDictionary'
        ? payloadLists[index]?.length ?? 0
        : countPayloadSet(payloadSet),
    ),
  )
}

function estimateAttackCountFromLengths(
  attackType: IntruderAttackType,
  positionCount: number,
  payloadCounts: number[],
): number {
  if (!positionCount) {
    return payloadCounts[0] ?? 0
  }

  if (attackType === 'sniper') {
    return positionCount * (payloadCounts[0] ?? 0)
  }

  if (attackType === 'batteringRam') {
    return payloadCounts[0] ?? 0
  }

  const relevant = payloadCounts.slice(0, positionCount)
  if (!relevant.length || relevant.some((count) => count === 0)) return 0

  if (attackType === 'pitchfork') {
    return Math.min(...relevant)
  }

  let total = 1
  for (const count of relevant) {
    total *= count
    if (!Number.isSafeInteger(total)) {
      return Number.MAX_SAFE_INTEGER
    }
  }

  return total
}
