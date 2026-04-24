import { computed } from 'vue'
import { useAttackWorkspaceStore } from './useAttackWorkspaceStore'
import { useDraftStore } from './useDraftStore'
import { useHistorySnapshotStore } from './useHistorySnapshotStore'
import { useReplayStore } from './useReplayStore'
import { useTrafficSelectionStore } from './useTrafficSelectionStore'

export function useTrafficWorkbenchStore() {
  const historySnapshots = useHistorySnapshotStore()
  const drafts = useDraftStore()
  const replay = useReplayStore()
  const attack = useAttackWorkspaceStore()
  const selection = useTrafficSelectionStore()

  const counts = computed(() => ({
    snapshots: historySnapshots.snapshots.value.length,
    drafts: drafts.drafts.value.length,
    replayRuns: replay.replayRuns.value.length,
    attackWorkspaces: attack.workspaces.value.length,
  }))

  return {
    historySnapshots,
    drafts,
    replay,
    attack,
    selection,
    counts,
  }
}
