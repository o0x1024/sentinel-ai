import type { TrafficMessageType } from './trafficDisplaySettings'
import type { HttpExchangeRequest } from './http/model'

export interface TrafficComparerDraftRequestInput {
  request?: HttpExchangeRequest
  text?: string
  messageType?: TrafficMessageType
  name?: string
  label?: string
  side?: 'auto' | 'left' | 'right'
}

interface TransferEnvelope<T> {
  id: string
  payload: T
}

export const REPEATER_TRANSFER_STORAGE_KEY = 'trafficAnalysis.transfer.repeater'
export const INTRUDER_TRANSFER_STORAGE_KEY = 'trafficAnalysis.transfer.intruder'
export const COMPARER_TRANSFER_STORAGE_KEY = 'trafficAnalysis.transfer.comparer'

export type TrafficCompareSource = 'history' | 'repeater' | 'intruder' | 'generic'
export type TrafficCompareKind =
  | 'requestVersions'
  | 'responseVersions'
  | 'responseDiff'
  | 'baselineDiff'
  | 'generic'

export interface TrafficCompareMeta {
  source: TrafficCompareSource
  kind: TrafficCompareKind
}

export interface TrafficCompareSideMeta {
  messageType: TrafficMessageType
  protocol?: 'http' | 'https'
  repeaterRequest?: HttpExchangeRequest
}

export interface TrafficComparePayload {
  name: string
  leftLabel: string
  rightLabel: string
  leftText: string
  rightText: string
  leftMeta?: TrafficCompareSideMeta
  rightMeta?: TrafficCompareSideMeta
  compareMeta?: TrafficCompareMeta
}

export function queueRepeaterTransfer(request: HttpExchangeRequest): void {
  const envelope: TransferEnvelope<HttpExchangeRequest> = {
    id: `transfer-${Date.now()}-${Math.random().toString(36).slice(2, 10)}`,
    payload: request,
  }
  localStorage.setItem(REPEATER_TRANSFER_STORAGE_KEY, JSON.stringify(envelope))
}

export function queueIntruderTransfer(request: HttpExchangeRequest): void {
  const envelope: TransferEnvelope<HttpExchangeRequest> = {
    id: `transfer-${Date.now()}-${Math.random().toString(36).slice(2, 10)}`,
    payload: request,
  }
  localStorage.setItem(INTRUDER_TRANSFER_STORAGE_KEY, JSON.stringify(envelope))
}

export function queueComparerTransfer(payload: TrafficComparePayload): void {
  const envelope: TransferEnvelope<TrafficComparePayload> = {
    id: `transfer-${Date.now()}-${Math.random().toString(36).slice(2, 10)}`,
    payload,
  }
  localStorage.setItem(COMPARER_TRANSFER_STORAGE_KEY, JSON.stringify(envelope))
}

export function parseTransferEnvelope<T>(raw: string | null): TransferEnvelope<T> | null {
  if (!raw) return null

  try {
    return JSON.parse(raw) as TransferEnvelope<T>
  } catch {
    return null
  }
}
