import type { HttpExchangeRequest } from './http/model'

export type TrafficWorkbenchSourceKind =
  | 'history'
  | 'intercept'
  | 'basket'
  | 'repeater'
  | 'intruder'
  | 'comparer'

export interface TrafficWorkbenchSource {
  kind: TrafficWorkbenchSourceKind
  label: string
  requestId?: number
}

export interface TrafficWorkbenchBasketItem {
  id: string
  name: string
  host: string
  request: HttpExchangeRequest
  requestId?: number
  source: TrafficWorkbenchSource
  createdAt: number
}

export interface TrafficWorkbenchBasketCandidateInput {
  request: HttpExchangeRequest
  requestId?: number
  title: string
  host: string
  sourceLabel?: string
}

export interface TrafficWorkbenchToolSession {
  tool: 'repeater' | 'intruder' | 'comparer'
  title: string
  source: TrafficWorkbenchSource | null
  updatedAt: number
  count: number
}
