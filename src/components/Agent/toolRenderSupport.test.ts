import { describe, expect, it } from 'vitest'

import {
  isFileToolName,
  isSearchToolName,
  isShellLikeToolName,
  parseStructuredToolPayload,
  shouldRenderSpecializedShellTool,
} from '@/components/Agent/toolRenderSupport'

describe('toolRenderSupport', () => {
  it('detects shell-like tool names', () => {
    expect(isShellLikeToolName('shell')).toBe(true)
    expect(isShellLikeToolName('interactive_shell')).toBe(false)
    expect(isShellLikeToolName('powershell')).toBe(true)
    expect(isShellLikeToolName('file_read')).toBe(false)
  })

  it('detects file and search tool names', () => {
    expect(isFileToolName('file_read')).toBe(true)
    expect(isFileToolName('grep')).toBe(false)
    expect(isSearchToolName('glob')).toBe(true)
    expect(isSearchToolName('file_edit')).toBe(false)
  })

  it('keeps specialized shell rendering for normal shell output', () => {
    expect(
      shouldRenderSpecializedShellTool({
        toolName: 'shell',
        result: { stdout: 'ok', stderr: '' },
      }),
    ).toBe(true)
  })

  it('disables specialized shell rendering when shell is missing from the toolset', () => {
    expect(
      shouldRenderSpecializedShellTool({
        toolName: 'shell',
        result: 'Toolset error: ToolNotFoundError: shell',
      }),
    ).toBe(false)
  })

  it('parses structured tool payloads from nested text arrays', () => {
    expect(
      parseStructuredToolPayload([
        {
          type: 'text',
          text: JSON.stringify({
            file_path: 'src/components/Agent/MessageBlock.vue',
            start_line: 1,
            end_line: 10,
          }),
        },
      ]),
    ).toEqual({
      file_path: 'src/components/Agent/MessageBlock.vue',
      start_line: 1,
      end_line: 10,
    })
  })
})
