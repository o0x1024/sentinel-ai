import type { HttpEndpoint } from '../../http/model'

export type DraftRevisionReason = 'clone' | 'manualSave' | 'send' | 'attack'

export interface DraftRevision {
  id: string
  draftId: string
  endpoint: HttpEndpoint
  rawRequest: string
  reason: DraftRevisionReason
  createdAt: number
}
