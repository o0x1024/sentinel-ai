import { computed, ref } from 'vue'
import type { AttackWorkspace } from '../model/attackWorkspace'
import type { HistorySnapshot } from '../model/historySnapshot'
import type { ReplayRun } from '../model/replayRun'
import type { RequestDraft } from '../model/requestDraft'
import type { TrafficSelectionEnvelope, TrafficSelectionKind } from '../model/selectionEnvelope'
import {
  createAttackWorkspaceSelectionEnvelope,
  createDraftSelectionEnvelope,
  createHistorySnapshotSelectionEnvelope,
  createReplayRunSelectionEnvelope,
} from '../model/selectionEnvelope'

const activeSelectionState = ref<TrafficSelectionEnvelope | null>(null)

function selectHistorySnapshot(snapshot: HistorySnapshot | null) {
  activeSelectionState.value = snapshot ? createHistorySnapshotSelectionEnvelope(snapshot) : null
}

function selectDraft(draft: RequestDraft | null) {
  activeSelectionState.value = draft ? createDraftSelectionEnvelope(draft) : null
}

function selectReplayRun(run: ReplayRun | null, draft: RequestDraft | null) {
  activeSelectionState.value = run ? createReplayRunSelectionEnvelope(run, draft) : null
}

function selectAttackWorkspace(workspace: AttackWorkspace | null) {
  activeSelectionState.value = workspace ? createAttackWorkspaceSelectionEnvelope(workspace) : null
}

function clearSelection() {
  activeSelectionState.value = null
}

function clearSelectionKind(kind: TrafficSelectionKind) {
  if (activeSelectionState.value?.kind === kind) {
    activeSelectionState.value = null
  }
}

export function useTrafficSelectionStore() {
  const activeKind = computed(() => activeSelectionState.value?.kind ?? null)

  return {
    activeSelection: activeSelectionState,
    activeKind,
    selectHistorySnapshot,
    selectDraft,
    selectReplayRun,
    selectAttackWorkspace,
    clearSelection,
    clearSelectionKind,
  }
}
