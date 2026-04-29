import { describe, expect, it, vi } from 'vitest'
import { createDefaultGrepPayloadSettings } from './analysis'
import { createDefaultResultFilter, createDefaultResultSort, saveIntruderResultsWindowState } from './results'
import type { IntruderResultsWindowState } from './types'

function createResultsWindowState(): IntruderResultsWindowState {
  return {
    workspaceId: 'workspace-1',
    workspaceName: 'Workspace 1',
    target: {
      host: 'example.com',
      port: 443,
      useTls: true,
    },
    requestText: 'GET / HTTP/1.1\r\nHost: example.com\r\n\r\n',
    positions: [],
    results: [],
    selectedResultId: null,
    progress: {
      total: 0,
      completed: 0,
      failed: 0,
      active: 0,
      truncated: false,
    },
    isRunning: false,
    captureFilter: createDefaultResultFilter(),
    viewFilter: createDefaultResultFilter(),
    sort: createDefaultResultSort(),
    grepMatchRules: [],
    grepExtractRules: [],
    grepPayloadSettings: createDefaultGrepPayloadSettings(),
    visibleColumns: [],
  }
}

describe('intruder results window state persistence', () => {
  it('does not throw when browser storage quota is exceeded', () => {
    const spy = vi.spyOn(window.localStorage.__proto__, 'setItem').mockImplementation(() => {
      throw new DOMException('The quota has been exceeded.', 'QuotaExceededError')
    })

    expect(() => saveIntruderResultsWindowState(createResultsWindowState())).not.toThrow()

    spy.mockRestore()
  })
})
