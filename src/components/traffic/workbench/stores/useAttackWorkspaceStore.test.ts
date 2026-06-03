import { beforeEach, describe, expect, it } from 'vitest'
import { useAttackWorkspaceStore } from './useAttackWorkspaceStore'
import { useDraftStore } from './useDraftStore'
import type { HttpExchangeRequest } from '../../http/model'

function createRequest(): HttpExchangeRequest {
  return {
    endpoint: {
      scheme: 'https',
      host: 'example.com',
      port: 443,
    },
    absoluteUrl: 'https://example.com/search?q=test',
    sourceRequestId: 88,
    preferredRequestView: 'pretty',
    request: {
      method: 'GET',
      target: '/search?q=test',
      versionPreference: 'HTTP/1.1',
      headers: [
        { name: 'Host', value: 'example.com' },
      ],
      bodyText: '',
    },
  }
}

describe('useAttackWorkspaceStore', () => {
  beforeEach(() => {
    useAttackWorkspaceStore().resetAttackWorkspaceStore()
    useDraftStore().resetDraftStore()
  })

  it('creates an attack workspace directly from an exchange request', () => {
    const store = useAttackWorkspaceStore()
    const workspace = store.createWorkspaceFromExchangeRequest({
      request: createRequest(),
      source: {
        kind: 'history',
        label: '历史记录 #88',
        requestId: 88,
      },
    })

    expect(store.workspaces.value).toHaveLength(1)
    expect(store.activeWorkspace.value?.id).toBe(workspace.id)
    expect(store.activeWorkspace.value?.target.host).toBe('example.com')
    expect(store.activeWorkspace.value?.requestText).toContain('GET /search?q=test HTTP/1.1')
  })

  it('freezes the draft revision when creating a workspace from a draft', () => {
    const draftStore = useDraftStore()
    const attackStore = useAttackWorkspaceStore()
    const draft = draftStore.createDraftFromExchangeRequest({
      request: createRequest(),
      source: null,
    })

    draftStore.updateDraftRequest(draft.id, 'GET /search?q=$admin$ HTTP/1.1\r\nHost: example.com\r\n\r\n')
    const revision = draftStore.appendRevision(draft.id, 'attack')

    const workspace = attackStore.createWorkspaceFromDraft({
      draft: draftStore.activeDraft.value!,
      revision: revision!,
    })

    expect(workspace.sourceDraftId).toBe(draft.id)
    expect(workspace.sourceDraftRevisionId).toBe(revision?.id)
    expect(workspace.requestText).toContain('$admin$')
    expect(workspace.positions).toHaveLength(1)
  })

  it('updates workspace content after creation', () => {
    const store = useAttackWorkspaceStore()
    const workspace = store.createWorkspaceFromExchangeRequest({
      request: createRequest(),
      source: null,
    })

    store.updateWorkspaceRequestText(workspace.id, 'POST /login HTTP/1.1\r\nHost: example.com\r\n\r\nx=1')
    store.updateWorkspaceTarget(workspace.id, {
      host: 'api.example.com',
      port: 8443,
      useTls: true,
    })
    store.updateWorkspacePositions(workspace.id, [
      {
        index: 1,
        start: 5,
        end: 10,
        value: 'admin',
        preview: 'admin',
      },
    ])
    store.updateWorkspaceTitle(workspace.id, 'login attack')

    expect(store.activeWorkspace.value?.requestText).toContain('POST /login HTTP/1.1')
    expect(store.activeWorkspace.value?.target.host).toBe('api.example.com')
    expect(store.activeWorkspace.value?.positions).toHaveLength(1)
    expect(store.activeWorkspace.value?.title).toBe('login attack')
  })
})
