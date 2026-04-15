import { describe, expect, it } from 'vitest'

import { buildRuntimeToolConfigForExecution } from '@/components/Agent/toolConfigRuntime'

describe('toolConfigRuntime', () => {
  it('serializes manual selection strategy into Rust enum payload for agent execution', () => {
    const runtimeConfig = buildRuntimeToolConfigForExecution({
      enabled: true,
      selection_strategy: 'Manual',
      max_tools: 3,
      fixed_tools: ['interactive_shell'],
      disabled_tools: [],
      manual_tools: ['browser__open', 'http_request'],
      allowed_tools: [],
    })

    expect(runtimeConfig.selection_strategy).toEqual({
      Manual: ['browser__open', 'http_request'],
    })
  })

  it('keeps manual selection serialized when web search is injected', () => {
    const runtimeConfig = buildRuntimeToolConfigForExecution({
      enabled: true,
      selection_strategy: 'Manual',
      max_tools: 3,
      fixed_tools: ['interactive_shell'],
      disabled_tools: [],
      manual_tools: ['http_request'],
      allowed_tools: [],
    }, {
      webSearchEnabled: true,
    })

    expect(runtimeConfig.selection_strategy).toEqual({
      Manual: ['http_request', 'web_search'],
    })
  })
})
