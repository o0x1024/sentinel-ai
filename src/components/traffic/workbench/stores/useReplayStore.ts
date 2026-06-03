import { computed, ref } from 'vue'
import type { HttpReplayResponse } from '../../http/model'
import type { ReplayRun } from '../model/replayRun'
import { createWorkbenchEntityId } from '../services/id'

const replayRunsState = ref<ReplayRun[]>([])
const mutationVersionState = ref(0)

function markMutated() {
  mutationVersionState.value += 1
}

function startReplayRun(draftId: string, draftRevisionId: string) {
  const now = Date.now()
  const run: ReplayRun = {
    id: createWorkbenchEntityId('traffic-replay-run'),
    draftId,
    draftRevisionId,
    state: 'running',
    response: null,
    error: null,
    createdAt: now,
    updatedAt: now,
  }
  replayRunsState.value = [...replayRunsState.value, run]
  markMutated()
  return run
}

function completeReplayRun(runId: string, response: HttpReplayResponse) {
  const now = Date.now()
  replayRunsState.value = replayRunsState.value.map(run =>
    run.id === runId
      ? {
        ...run,
        state: 'done',
        response,
        error: null,
        updatedAt: now,
      }
      : run,
  )
  markMutated()
}

function failReplayRun(runId: string, error: string) {
  const now = Date.now()
  replayRunsState.value = replayRunsState.value.map(run =>
    run.id === runId
      ? {
        ...run,
        state: 'error',
        error,
        updatedAt: now,
      }
      : run,
  )
  markMutated()
}

function cancelReplayRun(runId: string) {
  const now = Date.now()
  replayRunsState.value = replayRunsState.value.map(run =>
    run.id === runId
      ? {
        ...run,
        state: 'cancelled',
        updatedAt: now,
      }
      : run,
  )
  markMutated()
}

function replaceState(replayRuns: ReplayRun[]) {
  replayRunsState.value = replayRuns.map(run => ({
    ...run,
    response: run.response
      ? {
        ...run.response,
        headers: [...run.response.headers],
      }
      : null,
  }))
  markMutated()
}

function resetReplayStore() {
  replayRunsState.value = []
  markMutated()
}

export function useReplayStore() {
  const runningReplayCount = computed(() =>
    replayRunsState.value.filter(run => run.state === 'running').length,
  )

  return {
    replayRuns: replayRunsState,
    mutationVersion: mutationVersionState,
    runningReplayCount,
    startReplayRun,
    completeReplayRun,
    failReplayRun,
    cancelReplayRun,
    replaceState,
    resetReplayStore,
  }
}
