import { describe, expect, it } from 'vitest'
import {
  findLatestCompletedReplayResponseForDraft,
  restoreRepeaterTabResponseFromReplayRuns,
} from './proxyRepeaterReplayResponseRestoreSupport'
import type { RepeaterTab } from './proxyRepeaterTypes'
import type { ReplayRun } from './workbench/model/replayRun'

function buildResponse(bodyText: string, rawText = '') {
  return {
    statusCode: 200,
    versionObserved: 'HTTP/2' as const,
    statusText: 'OK',
    headers: [{ name: 'Content-Type', value: 'application/json' }],
    bodyText,
    rawText,
    responseTimeMs: 42,
  }
}

function buildRun(overrides: Partial<ReplayRun>): ReplayRun {
  return {
    id: 'run-1',
    draftId: 'draft-1',
    draftRevisionId: 'revision-1',
    state: 'done',
    response: buildResponse('{"old":true}'),
    error: null,
    createdAt: 1,
    updatedAt: 1,
    ...overrides,
  }
}

function buildTab(): RepeaterTab {
  return {
    id: 'tab-1',
    draftId: 'draft-1',
    mode: 'draft',
    name: 'example.test',
    sourceRequestId: null,
    targetHost: 'example.test',
    targetPort: 443,
    useTls: true,
    overrideSni: false,
    sniHost: '',
    initialRawRequest: '',
    rawRequest: '',
    prettyRequest: '',
    lastCompletedRawResponse: '',
    previousRawResponse: 'stale',
    rawResponse: '',
    requestTab: 'pretty',
    responseTab: 'pretty',
    response: null,
    isSending: false,
    modified: false,
    userEdited: true,
  }
}

describe('proxyRepeaterReplayResponseRestoreSupport', () => {
  it('selects the latest completed replay response for a draft', () => {
    const response = findLatestCompletedReplayResponseForDraft([
      buildRun({ id: 'old', updatedAt: 10, response: buildResponse('{"old":true}') }),
      buildRun({ id: 'new', updatedAt: 20, response: buildResponse('{"new":true}') }),
      buildRun({ id: 'running', state: 'running', updatedAt: 30, response: buildResponse('{"skip":true}') }),
    ], 'draft-1')

    expect(response?.bodyText).toBe('{"new":true}')
  })

  it('restores tab response body and raw response after draft hydration', () => {
    const tab = buildTab()
    restoreRepeaterTabResponseFromReplayRuns(tab, [
      buildRun({
        response: buildResponse(
          '{"ok":true}',
          'HTTP/2 200 OK\r\nContent-Type: application/json\r\n\r\n{"ok":true}',
        ),
      }),
    ])

    expect(tab.response?.bodyText).toBe('{"ok":true}')
    expect(tab.rawResponse).toContain('{"ok":true}')
    expect(tab.lastCompletedRawResponse).toBe(tab.rawResponse)
  })

  it('keeps the existing response when replay hydration has not arrived yet', () => {
    const tab = buildTab()
    tab.response = buildResponse(
      '{"local":true}',
      'HTTP/2 200 OK\r\nContent-Type: application/json\r\n\r\n{"local":true}',
    )
    tab.rawResponse = tab.response.rawText
    tab.lastCompletedRawResponse = tab.response.rawText

    restoreRepeaterTabResponseFromReplayRuns(tab, [])

    expect(tab.response?.bodyText).toBe('{"local":true}')
    expect(tab.rawResponse).toContain('{"local":true}')
  })
})
