import { mount } from '@vue/test-utils'
import { describe, expect, it, vi } from 'vitest'

import FileToolResult from './FileToolResult.vue'

vi.mock('vue-i18n', () => ({
  useI18n: () => ({
    t: (key: string) => ({
      'agent.toolCardFileReadTitle': 'Read file',
      'agent.toolCardErrorOutput': 'Error output',
      'agent.toolCardPath': 'Path',
      'agent.statusFailed': 'Failed',
      'agent.toolCardViewDetails': 'View structured payload',
      'agent.inputParameters': 'Input parameters',
      'agent.toolCardRawPayload': 'Raw payload',
    }[key] || key),
  }),
}))

const mountResult = (toolResult: string) => mount(FileToolResult, {
  props: {
    message: {
      id: 'tool-1',
      role: 'tool',
      content: '',
      metadata: {
        kind: 'tool_call',
        tool_name: 'file_read',
        tool_args: {
          file_path: '/Users/like/code/SecExample/pom.xml',
        },
        tool_result: toolResult,
        status: 'failed',
      },
      timestamp: new Date().toISOString(),
    } as any,
  },
  global: {
    stubs: {
      StoredArtifactPanel: true,
      ToolRuntimeMeta: true,
    },
  },
})

describe('FileToolResult', () => {
  it('renders failed file tool error output instead of hiding it in raw payload', async () => {
    const wrapper = mountResult(
      'File read failed: failed to read file: No such file or directory (os error 2)',
    )

    await wrapper.get('button').trigger('click')

    expect(wrapper.text()).toContain('Error output')
    expect(wrapper.text()).toContain('File read failed: failed to read file')
    expect(wrapper.text()).toContain('No such file or directory')
  })
})
