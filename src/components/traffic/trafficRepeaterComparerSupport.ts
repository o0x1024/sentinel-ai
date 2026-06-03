import { buildSourceRequestFromRawRequest } from '@/components/traffic/intruder/http'
import type { TrafficComparePayload } from './transfers'

type RepeaterCompareLabels = {
  defaultName: string
  requestVersions: string
  responseVersions: string
  originalRequest: string
  currentRequest: string
  previousResponse: string
  currentResponse: string
}

type RepeaterCompareTab = {
  name: string
  targetHost: string
  targetPort: number
  useTls: boolean
  initialRawRequest: string
  rawRequest: string
  previousRawResponse: string
  rawResponse: string
}

const normalizeText = (text: string): string => text.replace(/\r\n/g, '\n').replace(/\r/g, '\n')

function buildCompareName(name: string, fallbackHost: string, suffix: string, defaultName: string): string {
  const base = name || fallbackHost || defaultName
  return `${base} · ${suffix}`
}

function buildTarget(tab: RepeaterCompareTab) {
  return {
    host: tab.targetHost,
    port: tab.targetPort || (tab.useTls ? 443 : 80),
    useTls: tab.useTls,
  }
}

export function canCompareRepeaterRequestVersions(tab: RepeaterCompareTab | null): boolean {
  if (!tab) return false
  return normalizeText(tab.initialRawRequest).trim() !== normalizeText(tab.rawRequest).trim()
}

export function canCompareRepeaterResponseVersions(tab: RepeaterCompareTab | null): boolean {
  if (!tab) return false
  return normalizeText(tab.previousRawResponse).trim().length > 0 && normalizeText(tab.rawResponse).trim().length > 0
}

export function buildRepeaterRequestVersionComparePayload(
  tab: RepeaterCompareTab,
  labels: RepeaterCompareLabels,
): TrafficComparePayload | null {
  if (!canCompareRepeaterRequestVersions(tab)) return null

  const protocol: 'http' | 'https' = tab.useTls ? 'https' : 'http'
  const target = buildTarget(tab)

  return {
    name: buildCompareName(tab.name, tab.targetHost, labels.requestVersions, labels.defaultName),
    leftLabel: labels.originalRequest,
    rightLabel: labels.currentRequest,
    leftText: tab.initialRawRequest,
    rightText: tab.rawRequest,
    compareMeta: {
      source: 'repeater',
      kind: 'requestVersions',
    },
    leftMeta: {
      messageType: 'request',
      protocol,
      repeaterRequest: buildSourceRequestFromRawRequest(tab.initialRawRequest, target) ?? undefined,
    },
    rightMeta: {
      messageType: 'request',
      protocol,
      repeaterRequest: buildSourceRequestFromRawRequest(tab.rawRequest, target) ?? undefined,
    },
  }
}

export function buildRepeaterResponseVersionComparePayload(
  tab: RepeaterCompareTab,
  labels: RepeaterCompareLabels,
): TrafficComparePayload | null {
  if (!canCompareRepeaterResponseVersions(tab)) return null

  const protocol: 'http' | 'https' = tab.useTls ? 'https' : 'http'

  return {
    name: buildCompareName(tab.name, tab.targetHost, labels.responseVersions, labels.defaultName),
    leftLabel: labels.previousResponse,
    rightLabel: labels.currentResponse,
    leftText: tab.previousRawResponse,
    rightText: tab.rawResponse,
    compareMeta: {
      source: 'repeater',
      kind: 'responseVersions',
    },
    leftMeta: {
      messageType: 'response',
      protocol,
    },
    rightMeta: {
      messageType: 'response',
      protocol,
    },
  }
}
