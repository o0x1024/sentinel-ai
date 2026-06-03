import type { HttpExchangeRequest } from '../../http/model'
import type { TrafficWorkbenchSource } from '../../trafficWorkbenchTypes'

export type HistorySnapshotVariant = 'original' | 'edited'

export interface HistorySnapshot {
  id: string
  requestId: number | null
  dbRequestId: number | null
  trafficRequestId: string | null
  variant: HistorySnapshotVariant
  request: HttpExchangeRequest
  responseRawText: string
  responseStatusCode: number | null
  source: TrafficWorkbenchSource
  createdAt: number
}
