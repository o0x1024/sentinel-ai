import type { AttackWorkspaceRunState } from '../model/attackWorkspace'
import type { ReplayRunState } from '../model/replayRun'

export function replayBadgeLabel(state: ReplayRunState | null | undefined) {
  if (state === 'running') return 'Running'
  if (state === 'done') return 'Done'
  if (state === 'error') return 'Error'
  if (state === 'cancelled') return 'Stopped'
  return 'Idle'
}

export function replayBadgeClass(state: ReplayRunState | null | undefined) {
  if (state === 'running') return 'bg-warning/18 text-warning'
  if (state === 'done') return 'bg-success/16 text-success'
  if (state === 'error') return 'bg-error/14 text-error'
  if (state === 'cancelled') return 'bg-base-200 text-base-content/55'
  return 'bg-base-200 text-base-content/55'
}

export function attackBadgeLabel(state: AttackWorkspaceRunState) {
  if (state === 'running') return 'Running'
  if (state === 'done') return 'Done'
  if (state === 'cancelled') return 'Stopped'
  return 'Idle'
}

export function attackBadgeClass(state: AttackWorkspaceRunState) {
  if (state === 'running') return 'bg-warning/18 text-warning'
  if (state === 'done') return 'bg-success/16 text-success'
  if (state === 'cancelled') return 'bg-base-200 text-base-content/55'
  return 'bg-base-200 text-base-content/55'
}
