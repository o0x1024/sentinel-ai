import { computed, ref } from 'vue'
import type { HttpReplayResponse } from '../../http/model'
import type { ReplayRun } from '../model/replayRun'
import { createWorkbenchEntityId } from '../services/id'

const replayRunsState = ref<ReplayRun[]>([])

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
}

function resetReplayStore() {
  replayRunsState.value = []
}

export function useReplayStore() {
  const runningReplayCount = computed(() =>
    replayRunsState.value.filter(run => run.state === 'running').length,
  )

  return {
    replayRuns: replayRunsState,
    runningReplayCount,
    startReplayRun,
    completeReplayRun,
    failReplayRun,
    cancelReplayRun,
    replaceState,
    resetReplayStore,
  }
}
