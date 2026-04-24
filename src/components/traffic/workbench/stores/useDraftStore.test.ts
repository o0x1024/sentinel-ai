import { beforeEach, describe, expect, it } from 'vitest'
import { useDraftStore } from './useDraftStore'
import type { HttpExchangeRequest } from '../../http/model'

function createRequest(): HttpExchangeRequest {
  return {
    endpoint: {
      scheme: 'https',
      host: 'example.com',
      port: 443,
    },
    absoluteUrl: 'https://example.com/api/login',
    sourceRequestId: 101,
    preferredRequestView: 'pretty',
    request: {
      method: 'POST',
      target: '/api/login',
      versionPreference: 'HTTP/1.1',
      headers: [
        { name: 'Host', value: 'example.com' },
        { name: 'Content-Type', value: 'application/json' },
      ],
      bodyText: '{"username":"admin"}',
    },
  }
}

describe('useDraftStore', () => {
  beforeEach(() => {
    useDraftStore().resetDraftStore()
  })

  it('creates a draft with an initial clone revision from an exchange request', () => {
    const store = useDraftStore()
    const draft = store.createDraftFromExchangeRequest({
      request: createRequest(),
      source: {
        kind: 'history',
        label: '历史记录 #101',
        requestId: 101,
      },
    })

    expect(store.drafts.value).toHaveLength(1)
    expect(store.activeDraft.value?.id).toBe(draft.id)
    expect(store.activeDraftRevisions.value).toHaveLength(1)
    expect(store.activeDraftRevisions.value[0]?.reason).toBe('clone')
    expect(store.activeDraft.value?.rawRequest).toContain('POST /api/login HTTP/1.1')
  })

  it('creates immutable revisions from the current draft content', () => {
    const store = useDraftStore()
    const draft = store.createDraftFromExchangeRequest({
      request: createRequest(),
      source: null,
    })

    store.updateDraftRequest(draft.id, 'GET /health HTTP/1.1\r\nHost: example.com\r\n\r\n')
    const revision = store.appendRevision(draft.id, 'manualSave')

    expect(revision).not.toBeNull()
    expect(store.activeDraft.value?.activeRevisionId).toBe(revision?.id)
    expect(store.revisions.value[draft.id]).toHaveLength(2)
    expect(store.revisions.value[draft.id]?.[1]?.rawRequest).toContain('GET /health')
  })
})
