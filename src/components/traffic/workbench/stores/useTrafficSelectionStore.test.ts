import { beforeEach, describe, expect, it } from 'vitest'
import { useTrafficSelectionStore } from './useTrafficSelectionStore'
import { useHistorySnapshotStore } from './useHistorySnapshotStore'
import { useDraftStore } from './useDraftStore'
import { useAttackWorkspaceStore } from './useAttackWorkspaceStore'
import { useReplayStore } from './useReplayStore'

describe('useTrafficSelectionStore', () => {
  beforeEach(() => {
    useHistorySnapshotStore().resetHistorySnapshotStore()
    useDraftStore().resetDraftStore()
    useAttackWorkspaceStore().resetAttackWorkspaceStore()
    useReplayStore().resetReplayStore()
    useTrafficSelectionStore().clearSelection()
  })

  it('tracks explicit selection envelopes across snapshot draft and workspace', () => {
    const history = useHistorySnapshotStore()
    const drafts = useDraftStore()
    const attack = useAttackWorkspaceStore()
    const selection = useTrafficSelectionStore()

    const snapshot = history.createSnapshotFromProxyRequest(
      {
        id: 18,
        status_code: 200,
        response_size: 0,
        response_time: 0,
        timestamp: new Date().toISOString(),
        host: 'example.com',
        method: 'GET',
        url: 'https://example.com/demo',
        scheme: 'https',
        http_version_observed: 'HTTP/1.1',
        request_headers: 'Host: example.com',
        request_body: '',
        response_headers: 'Content-Type: text/plain',
        response_body: '',
        was_edited: false,
        db_request_id: 18,
        traffic_request_id: 'traffic-18',
        origin_kind: null,
        origin_ref_id: null,
        parent_request_id: null,
        source_draft_revision_id: null,
      },
      'original',
      { kind: 'history', label: '历史记录 #18', requestId: 18 },
    )
    selection.selectHistorySnapshot(snapshot)
    expect(selection.activeSelection.value?.kind).toBe('history-snapshot')
    expect(snapshot.request.previewResponse).toMatchObject({
      statusCode: 200,
      headers: [{ name: 'Content-Type', value: 'text/plain' }],
      bodyText: '',
    })

    const draft = drafts.createDraftFromExchangeRequest({
      request: snapshot.request,
      source: snapshot.source,
      sourceSnapshotId: snapshot.id,
      title: 'example.com',
    })
    selection.selectDraft(draft)
    expect(selection.activeSelection.value).toMatchObject({
      kind: 'draft',
      draftId: draft.id,
      sourceSnapshotId: snapshot.id,
    })

    const revision = drafts.appendRevision(draft.id, 'send')
    expect(revision).not.toBeNull()
    const workspace = attack.createWorkspaceFromDraft({
      draft,
      revision: revision!,
    })
    selection.selectAttackWorkspace(workspace)
    expect(selection.activeSelection.value).toMatchObject({
      kind: 'attack-workspace',
      workspaceId: workspace.id,
      sourceDraftId: draft.id,
    })
  })

  it('can clear only one kind without dropping unrelated selection state transitions', () => {
    const selection = useTrafficSelectionStore()
    selection.selectDraft({
      id: 'draft-1',
      title: 'draft-1',
      source: null,
      sourceSnapshotId: null,
      endpoint: { scheme: 'https', host: 'example.com', port: 443, sniHost: 'example.com' },
      endpointMode: 'manual',
      rawRequest: 'GET / HTTP/1.1\r\nHost: example.com\r\n\r\n',
      preferredView: 'raw',
      pinned: false,
      activeRevisionId: 'rev-1',
      createdAt: 1,
      updatedAt: 1,
    })
    selection.clearSelectionKind('history-snapshot')
    expect(selection.activeSelection.value?.kind).toBe('draft')
    selection.clearSelectionKind('draft')
    expect(selection.activeSelection.value).toBeNull()
  })
})
