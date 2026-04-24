import type { HttpReplayResponse } from '../../http/model'

export type ReplayRunState = 'running' | 'done' | 'error' | 'cancelled'

export interface ReplayRun {
  id: string
  draftId: string
  draftRevisionId: string
  state: ReplayRunState
  response: HttpReplayResponse | null
  error: string | null
  createdAt: number
  updatedAt: number
}
