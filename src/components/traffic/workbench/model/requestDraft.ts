import type { HttpEndpoint } from '../../http/model'
import type { TrafficWorkbenchSource } from '../../trafficWorkbenchTypes'

export type DraftEndpointMode = 'auto' | 'manual'
export type DraftPreferredView = 'pretty' | 'raw'

export interface RequestDraft {
  id: string
  title: string
  source: TrafficWorkbenchSource | null
  sourceSnapshotId: string | null
  endpoint: HttpEndpoint
  endpointMode: DraftEndpointMode
  rawRequest: string
  preferredView: DraftPreferredView
  pinned: boolean
  activeRevisionId: string
  createdAt: number
  updatedAt: number
}
