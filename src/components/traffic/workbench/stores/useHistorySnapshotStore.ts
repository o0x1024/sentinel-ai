import { computed, ref } from 'vue'
import type { TrafficWorkbenchSource } from '../../trafficWorkbenchTypes'
import type { ProxyRequest } from '../../proxyHistoryTypes'
import type { HistorySnapshot, HistorySnapshotVariant } from '../model/historySnapshot'
import { createHistorySnapshotFromProxyRequest } from '../services/snapshotFactory'

const snapshotsState = ref<Record<string, HistorySnapshot>>({})
const activeSnapshotIdState = ref<string | null>(null)

function upsertSnapshot(snapshot: HistorySnapshot) {
  snapshotsState.value = {
    ...snapshotsState.value,
    [snapshot.id]: snapshot,
  }
  activeSnapshotIdState.value = snapshot.id
  return snapshot
}

function createSnapshotFromProxyRequest(
  request: ProxyRequest,
  variant: HistorySnapshotVariant,
  source: TrafficWorkbenchSource,
) {
  return upsertSnapshot(createHistorySnapshotFromProxyRequest(request, variant, source))
}

function selectSnapshot(snapshotId: string | null) {
  activeSnapshotIdState.value = snapshotId
}

function resetHistorySnapshotStore() {
  snapshotsState.value = {}
  activeSnapshotIdState.value = null
}

export function useHistorySnapshotStore() {
  const snapshots = computed(() => Object.values(snapshotsState.value))
  const activeSnapshot = computed(() =>
    activeSnapshotIdState.value ? snapshotsState.value[activeSnapshotIdState.value] ?? null : null,
  )

  return {
    snapshots,
    activeSnapshotId: activeSnapshotIdState,
    activeSnapshot,
    upsertSnapshot,
    createSnapshotFromProxyRequest,
    selectSnapshot,
    resetHistorySnapshotStore,
  }
}
