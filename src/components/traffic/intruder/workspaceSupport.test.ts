import { describe, expect, it } from 'vitest'
import {
  normalizePersistedIntruderWorkspaceList,
  serializeIntruderWorkspaceSessionStore,
  type IntruderWorkspace,
} from './workspaceSupport'

function createWorkspace(): IntruderWorkspace {
  return {
    id: 'workspace-1',
    name: 'example.com',
    sourceRequestId: 101,
    requestText: 'GET / HTTP/1.1\r\nHost: example.com\r\n\r\n',
    requestViewTab: 'raw',
    target: {
      host: 'example.com',
      port: 443,
      useTls: true,
    },
    positions: [],
    attackType: 'sniper',
    payloadSets: [],
    payloadProcessingRules: [],
    payloadProcessorPlugins: [],
    requestProcessorPlugins: [],
    grepMatchRules: [],
    grepExtractRules: [],
    grepPayloadSettings: {
      enabled: false,
      caseSensitive: false,
      excludeHeaders: false,
      matchUrlEncoded: false,
    },
    selectedResourcePoolId: 'default',
    attackOptions: {
      concurrency: 10,
      delayMs: 0,
      randomDelayMs: 0,
      delayIncrementMs: 0,
      autoThrottleEnabled: false,
      autoThrottleStatusCodes: [429, 503],
      timeoutSecs: 30,
      maxRequests: 500,
      updateHostHeader: true,
      updateContentLength: true,
      setConnectionClose: true,
      followRedirects: false,
      maxRedirects: 5,
      processCookiesInRedirects: true,
      retryCount: 3,
      retryPauseMs: 2000,
      storeRequests: true,
      storeResponses: true,
      storeFullPayloads: false,
      denialOfServiceMode: false,
      makeUnmodifiedBaseline: true,
      autoPauseEnabled: false,
      autoPauseMode: 'contains',
      autoPauseExpression: '',
      autoPauseExpressions: [],
    },
    results: [
      {
        id: 'result-1',
        index: 1,
        payloadSummary: 'admin',
        payloadValues: ['admin'],
        statusCode: 200,
        responseLength: 128,
        wordCount: 4,
        lineCount: 2,
        responseTimeMs: 50,
        rawRequest: 'GET / HTTP/1.1',
        rawResponse: 'HTTP/1.1 200 OK',
        responseHeaders: [{ name: 'Content-Type', value: 'text/plain' }],
        responseBodyText: 'ok',
        redirectCount: 0,
        finalUrl: 'https://example.com/',
        redirectChain: [],
        payloadReflectionCount: 0,
        grepMatches: {},
        grepExtracts: {},
      },
    ],
    selectedResultId: 'result-1',
    progress: {
      total: 1,
      completed: 1,
      failed: 0,
      active: 0,
      truncated: false,
    },
    isRunning: false,
    captureFilter: {
      enabled: false,
      query: '',
      invert: false,
      statusCode: '',
      onlyErrors: false,
      hideBaseline: false,
      grepMatchRuleIds: [],
      columnFilters: [],
    },
    viewFilter: {
      enabled: false,
      query: '',
      invert: false,
      statusCode: '',
      onlyErrors: false,
      hideBaseline: false,
      grepMatchRuleIds: [],
      columnFilters: [],
    },
    sort: {
      key: 'index',
      direction: 'asc',
    },
    visibleColumns: ['index', 'statusCode'],
  }
}

describe('intruder workspace session persistence', () => {
  it('serializes and restores results and selection', () => {
    const store = serializeIntruderWorkspaceSessionStore('workspace-1', [createWorkspace()])
    const restored = normalizePersistedIntruderWorkspaceList(store, {
      attackLabel: '攻击',
      payloadLabel: 'Payload',
    })

    expect(restored?.activeWorkspaceId).toBe('workspace-1')
    expect(restored?.workspaces[0]?.results).toHaveLength(1)
    expect(restored?.workspaces[0]?.selectedResultId).toBe('result-1')
    expect(restored?.workspaces[0]?.progress.completed).toBe(1)
    expect(restored?.workspaces[0]?.isRunning).toBe(false)
  })
})
