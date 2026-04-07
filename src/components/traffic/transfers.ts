import type { TrafficMessageType } from './trafficDisplaySettings'

export interface TrafficTransferRequest {
  method: string
  url: string
  headers: Record<string, string>
  body?: string
}

export interface TrafficComparerDraftRequestInput {
  request?: TrafficTransferRequest
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
export const COMPARER_TRANSFER_STORAGE_KEY = 'trafficAnalysis.transfer.comparer'

export type TrafficCompareSource = 'history' | 'repeater' | 'intruder' | 'generic'
export type TrafficCompareKind = 'requestVersions' | 'responseVersions' | 'responseDiff' | 'baselineDiff' | 'generic'

export interface TrafficCompareMeta {
  source: TrafficCompareSource
  kind: TrafficCompareKind
}

export interface TrafficCompareSideMeta {
  messageType: TrafficMessageType
  protocol?: 'http' | 'https'
  repeaterRequest?: TrafficTransferRequest
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

export function queueRepeaterTransfer(request: TrafficTransferRequest): void {
  const envelope: TransferEnvelope<TrafficTransferRequest> = {
    id: `transfer-${Date.now()}-${Math.random().toString(36).slice(2, 10)}`,
    payload: request,
  }
  localStorage.setItem(REPEATER_TRANSFER_STORAGE_KEY, JSON.stringify(envelope))
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
