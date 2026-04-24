import type { TrafficWorkbenchSource } from '../../trafficWorkbenchTypes'
import type { IntruderPosition, IntruderTarget } from '../../intruder/types'

export type AttackWorkspaceRunState = 'idle' | 'running' | 'done' | 'cancelled'

export interface AttackWorkspaceProgress {
  total: number
  completed: number
  failed: number
  active: number
  truncated: boolean
}

export interface AttackWorkspace {
  id: string
  title: string
  source: TrafficWorkbenchSource | null
  sourceDraftId: string | null
  sourceDraftRevisionId: string | null
  requestText: string
  target: IntruderTarget
  positions: IntruderPosition[]
  runState: AttackWorkspaceRunState
  resultCount: number
  progress: AttackWorkspaceProgress
  createdAt: number
  updatedAt: number
}
