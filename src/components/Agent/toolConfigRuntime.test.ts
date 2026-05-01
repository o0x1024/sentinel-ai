import { describe, expect, it } from 'vitest'

import {
  buildRuntimeToolConfigForExecution,
  buildRuntimeToolConfigForTeamRole,
} from '@/components/Agent/toolConfigRuntime'

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

  it('applies Team specialist tools as the intersection of profile tools and role tools', () => {
    const runtimeConfig = buildRuntimeToolConfigForTeamRole({
      enabled: true,
      selection_strategy: 'Manual',
      max_tools: 6,
      fixed_tools: ['interactive_shell'],
      disabled_tools: [],
      manual_tools: ['file_read', 'shell'],
      allowed_tools: [],
    }, {
      specialist: {
        tools: ['file_read', 'http_request', 'shell'],
      },
    }, 'specialist')

    expect(runtimeConfig).toMatchObject({
      enabled: true,
      selection_strategy: { Manual: ['file_read', 'shell'] },
      max_tools: 6,
      fixed_tools: [],
      allowed_tools: ['file_read', 'shell'],
    })
  })

  it('disables Team specialist tools when the role scope has no overlap with the profile tools', () => {
    const runtimeConfig = buildRuntimeToolConfigForTeamRole({
      enabled: true,
      selection_strategy: 'Manual',
      max_tools: 8,
      fixed_tools: ['interactive_shell', 'file_write'],
      disabled_tools: [],
      manual_tools: ['interactive_shell', 'file_write', 'http_request'],
      allowed_tools: [],
    }, {
      specialist: {
        tools: ['shell', 'browser_shell'],
      },
    }, 'specialist')

    expect(runtimeConfig.enabled).toBe(false)
    expect(runtimeConfig.selection_strategy).toEqual({ Manual: [] })
    expect(runtimeConfig.fixed_tools).toEqual([])
    expect(runtimeConfig.allowed_tools).toEqual([])
  })

  it('keeps role tools exact without alias expansion', () => {
    const runtimeConfig = buildRuntimeToolConfigForTeamRole({
      enabled: true,
      selection_strategy: 'Manual',
      max_tools: 8,
      fixed_tools: [],
      disabled_tools: [],
      manual_tools: ['shell', 'interactive_shell'],
      allowed_tools: [],
    }, {
      specialist: {
        tools: ['shell'],
      },
    }, 'specialist')

    expect(runtimeConfig.selection_strategy).toEqual({
      Manual: ['shell'],
    })
    expect(runtimeConfig.allowed_tools).toEqual(['shell'])
  })

  it('uses explicit agent allowed tools for Team scope even when profile strategy is not manual', () => {
    const runtimeConfig = buildRuntimeToolConfigForTeamRole({
      enabled: true,
      selection_strategy: 'Hybrid',
      max_tools: 8,
      fixed_tools: ['interactive_shell'],
      disabled_tools: [],
      manual_tools: [],
      allowed_tools: ['interactive_shell', 'shell', 'grep'],
    }, {
      specialist: {
        tools: ['shell', 'grep', 'http_request'],
      },
    }, 'specialist')

    expect(runtimeConfig.selection_strategy).toEqual({
      Manual: ['shell', 'grep'],
    })
    expect(runtimeConfig.allowed_tools).toEqual(['shell', 'grep'])
  })
})
