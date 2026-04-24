import { beforeEach, describe, expect, it } from 'vitest'
import { useReplayStore } from './useReplayStore'

describe('useReplayStore', () => {
  beforeEach(() => {
    useReplayStore().resetReplayStore()
  })

  it('tracks replay lifecycle transitions', () => {
    const store = useReplayStore()
    const run = store.startReplayRun('draft-1', 'revision-1')

    expect(store.runningReplayCount.value).toBe(1)

    store.completeReplayRun(run.id, {
      statusCode: 200,
      versionObserved: 'HTTP/1.1',
      statusText: 'OK',
      headers: [{ name: 'Content-Type', value: 'application/json' }],
      bodyText: '{"ok":true}',
      bodyBytesBase64: 'eyJvayI6dHJ1ZX0=',
      rawText: 'HTTP/1.1 200 OK\r\n\r\n{"ok":true}',
      responseTimeMs: 52,
    })

    expect(store.runningReplayCount.value).toBe(0)
    expect(store.replayRuns.value[0]?.state).toBe('done')
    expect(store.replayRuns.value[0]?.response?.statusCode).toBe(200)
  })

  it('replaces the replay store state from persistence', () => {
    const store = useReplayStore()

    store.replaceState([
      {
        id: 'run-1',
        draftId: 'draft-1',
        draftRevisionId: 'revision-1',
        state: 'cancelled',
        response: null,
        error: 'cancelled',
        createdAt: 1,
        updatedAt: 2,
      },
    ])

    expect(store.replayRuns.value).toHaveLength(1)
    expect(store.replayRuns.value[0]?.state).toBe('cancelled')
    expect(store.runningReplayCount.value).toBe(0)
  })
})
