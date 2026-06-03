import { describe, expect, it } from 'vitest'

import {
  buildToolsActivatedMessage,
  buildToolsPreview,
  normalizeToolsActivatedPayload,
} from './agentToolActivation'

describe('agentToolActivation', () => {
  it('builds preview with truncation after six tools', () => {
    expect(
      buildToolsPreview([
        'tool_search',
        'file_read',
        'grep',
        'glob',
        'lsp',
        'file_edit',
        'file_write',
      ]),
    ).toBe('tool_search, file_read, grep, glob, lsp, file_edit +1')
  })

  it('normalizes query, runtime hint, and tool ids', () => {
    expect(
      normalizeToolsActivatedPayload({
        tool_ids: [' file_read ', '', 1],
        tools: ['tool_search', 'file_read'],
        query: ' read changed file ',
        runtime_hint: ' prefer readback ',
      }),
    ).toEqual({
      toolIds: ['file_read'],
      tools: ['tool_search', 'file_read'],
      query: 'read changed file',
      runtimeHint: 'prefer readback',
      toolsPreview: 'tool_search, file_read',
    })
  })

  it('builds a user-facing multi-line activation message', () => {
    expect(
      buildToolsActivatedMessage({
        tool_ids: ['file_read', 'grep'],
        tools: ['tool_search', 'file_read', 'grep'],
        query: 'find recent code references',
        runtime_hint: 'recent file changes detected; prefer readback',
      }),
    ).toBe(
      [
        'Deferred tools activated: file_read, grep',
        'Query: find recent code references',
        'Active toolset: tool_search, file_read, grep',
        'Reason: recent file changes detected; prefer readback',
      ].join('\n'),
    )
  })
})
