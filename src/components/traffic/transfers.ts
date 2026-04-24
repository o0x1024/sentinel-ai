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
