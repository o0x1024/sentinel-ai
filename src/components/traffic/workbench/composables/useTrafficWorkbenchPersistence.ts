import { ref, type Ref } from 'vue'
import type { useTrafficWorkbenchStore } from '../stores/useTrafficWorkbenchStore'
import {
  loadAttackWorkspaceStore,
  loadReplayRunStore,
  loadTrafficDraftStore,
  saveAttackWorkspaceStore,
  saveReplayRunStore,
  saveTrafficDraftStore,
} from '@/api/trafficWorkbench'
import {
  buildAttackWorkspaceFromPersistence,
  buildPersistedAttackWorkspaceStore,
  buildPersistedDraftStore,
  buildPersistedReplayRunStore,
  buildReplayRunFromPersistence,
  restorePersistedDraftRecords,
} from '../services/persistenceSupport'

type TrafficWorkbenchStore = ReturnType<typeof useTrafficWorkbenchStore>

export function useTrafficWorkbenchPersistence(workbenchState: TrafficWorkbenchStore) {
  const ready = ref(false)
  const persistedDraftStoreFingerprint = ref('')
  const persistedAttackWorkspaceStoreFingerprint = ref('')
  const persistedReplayRunStoreFingerprint = ref('')
  let persistTimer: number | null = null

  async function hydrate() {
    try {
      const [draftStore, attackStore, replayStore] = await Promise.all([
        loadTrafficDraftStore(),
        loadAttackWorkspaceStore(),
        loadReplayRunStore(),
      ])
      const restoredDraftState = restorePersistedDraftRecords(draftStore.drafts)
      workbenchState.drafts.replaceState(
        restoredDraftState.drafts,
        restoredDraftState.revisionsByDraftId,
        draftStore.activeDraftId,
      )
      workbenchState.attack.replaceState(
        attackStore.workspaces.map(buildAttackWorkspaceFromPersistence),
        attackStore.activeWorkspaceId,
      )
      workbenchState.replay.replaceState(
        replayStore.replayRuns.map(buildReplayRunFromPersistence),
      )
      persistedDraftStoreFingerprint.value = JSON.stringify(draftStore)
      persistedAttackWorkspaceStoreFingerprint.value = JSON.stringify(attackStore)
      persistedReplayRunStoreFingerprint.value = JSON.stringify(replayStore)
    } catch (error) {
      console.error('Failed to hydrate traffic workbench persistence', error)
    } finally {
      ready.value = true
    }
  }

  async function persist() {
    const draftStore = buildPersistedDraftStore(
      workbenchState.drafts.drafts.value,
      workbenchState.drafts.revisions.value,
      workbenchState.drafts.activeDraftId.value,
    )
    const attackStore = buildPersistedAttackWorkspaceStore(
      workbenchState.attack.workspaces.value,
      workbenchState.attack.activeWorkspaceId.value,
    )
    const replayStore = buildPersistedReplayRunStore(workbenchState.replay.replayRuns.value)
    const nextDraftFingerprint = JSON.stringify(draftStore)
    const nextAttackFingerprint = JSON.stringify(attackStore)
    const nextReplayFingerprint = JSON.stringify(replayStore)

    if (persistedDraftStoreFingerprint.value !== nextDraftFingerprint) {
      await saveTrafficDraftStore(draftStore)
      persistedDraftStoreFingerprint.value = nextDraftFingerprint
    }

    if (persistedAttackWorkspaceStoreFingerprint.value !== nextAttackFingerprint) {
      await saveAttackWorkspaceStore(attackStore)
      persistedAttackWorkspaceStoreFingerprint.value = nextAttackFingerprint
    }

    if (persistedReplayRunStoreFingerprint.value !== nextReplayFingerprint) {
      await saveReplayRunStore(replayStore)
      persistedReplayRunStoreFingerprint.value = nextReplayFingerprint
    }
  }

  function schedulePersist() {
    if (!ready.value) {
      return
    }
    if (persistTimer !== null) {
      window.clearTimeout(persistTimer)
    }
    persistTimer = window.setTimeout(() => {
      persistTimer = null
      void persist().catch(error => {
        console.error('Failed to persist traffic workbench state', error)
      })
    }, 180)
  }

  function stopPersistTimer() {
    if (persistTimer !== null) {
      window.clearTimeout(persistTimer)
      persistTimer = null
    }
  }

  return {
    ready,
    hydrate,
    persist,
    schedulePersist,
    stopPersistTimer,
  }
}
