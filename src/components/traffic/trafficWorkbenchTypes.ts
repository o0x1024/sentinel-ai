import type { HttpExchangeRequest } from './http/model'

export type TrafficWorkbenchRequestVariant = 'original' | 'edited'

export type TrafficWorkbenchSourceKind =
  | 'history'
  | 'intercept'
  | 'basket'
  | 'capture'
  | 'repeater'
  | 'intruder'
  | 'comparer'
  | 'oast'

export interface TrafficWorkbenchSource {
  kind: TrafficWorkbenchSourceKind
  label: string
  requestId?: number
}

export interface TrafficWorkbenchRequestContext {
  sourceKind: TrafficWorkbenchSourceKind
  sourceLabel: string
  requestId: number | null
  sourceRequestId: number | null
  method: string
  host: string
  path: string
  statusCode: number | null
  variant: TrafficWorkbenchRequestVariant
  hasEditedVariant: boolean
  mode: 'preview' | 'draft' | 'workspace'
  modeLabel: string
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
  tool: 'capture' | 'repeater' | 'intruder' | 'comparer' | 'oast'
  title: string
  source: TrafficWorkbenchSource | null
  updatedAt: number
  count: number
}
