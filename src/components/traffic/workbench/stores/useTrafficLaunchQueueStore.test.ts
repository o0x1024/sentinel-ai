import { beforeEach, describe, expect, it } from 'vitest'
import { useTrafficLaunchQueueStore } from './useTrafficLaunchQueueStore'

describe('useTrafficLaunchQueueStore', () => {
  const store = useTrafficLaunchQueueStore()

  beforeEach(() => {
    store.consumeLaunchQueue()
  })

  it('queues and consumes pending launch actions without leaking references', () => {
    const request = {
      endpoint: { scheme: 'https' as const, host: 'api.example.com', port: 443 },
      absoluteUrl: 'https://api.example.com/login',
      sourceRequestId: 101,
      request: {
        method: 'POST',
        target: '/login',
        versionPreference: 'HTTP/1.1' as const,
        headers: [{ name: 'Host', value: 'api.example.com' }],
        bodyText: '{"name":"alice"}',
      },
    }

    store.queueDraftRequest(request)
    request.request.headers[0].value = 'mutated.example.com'
    store.queueAttackWorkspaceRequest(request)
    store.queueComparePayload({
      name: 'login compare',
      leftLabel: 'A',
      rightLabel: 'B',
      leftText: 'left',
      rightText: 'right',
      leftMeta: {
        messageType: 'request',
        repeaterRequest: request,
      },
    })

    const snapshot = store.consumeLaunchQueue()
    expect(snapshot.draftRequests).toHaveLength(1)
    expect(snapshot.attackWorkspaceRequests).toHaveLength(1)
    expect(snapshot.comparePayloads).toHaveLength(1)
    expect(snapshot.draftRequests[0]?.request.headers[0]?.value).toBe('api.example.com')
    expect(snapshot.attackWorkspaceRequests[0]?.request.headers[0]?.value).toBe('mutated.example.com')
    expect(snapshot.comparePayloads[0]?.leftMeta?.repeaterRequest?.request.headers[0]?.value).toBe(
      'mutated.example.com',
    )
    expect(store.consumeLaunchQueue()).toEqual({
      draftRequests: [],
      attackWorkspaceRequests: [],
      comparePayloads: [],
    })
  })
})
