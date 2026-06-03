import { describe, expect, it } from 'vitest'

import {
  buildRuntimeToolConfigForExecution,
  buildRuntimeToolConfigForTeamRole,
  normalizeUiToolConfigPayload,
  runtimeToolConfigAllowsTool,
} from '@/components/Agent/toolConfigRuntime'

describe('toolConfigRuntime', () => {
  it('serializes manual selection strategy into Rust enum payload for agent execution', () => {
    const runtimeConfig = buildRuntimeToolConfigForExecution({
      enabled: true,
      selection_strategy: 'Manual',
      max_tools: 3,
      preselected_tools: ['interactive_shell'],
      disabled_tools: [],
      manual_tools: ['browser_shell__list', 'http_request'],
      allowed_tools: [],
    })

    expect(runtimeConfig.selection_strategy).toEqual({
      Manual: ['browser_shell__list', 'http_request'],
    })
    expect(runtimeConfig.allowed_tools).toEqual(['browser_shell__list', 'http_request', 'shell'])
  })

  it('keeps manual selection serialized when web search is injected', () => {
    const runtimeConfig = buildRuntimeToolConfigForExecution({
      enabled: true,
      selection_strategy: 'Manual',
      max_tools: 3,
      preselected_tools: ['interactive_shell'],
      disabled_tools: [],
      manual_tools: ['http_request'],
      allowed_tools: [],
    }, {
      webSearchEnabled: true,
    })

    expect(runtimeConfig.selection_strategy).toEqual({
      Manual: ['http_request', 'web_search'],
    })
    expect(runtimeConfig.allowed_tools).toEqual(['http_request', 'shell', 'web_search'])
  })

  it('uses manual selection as the runtime allowlist so unselected tools stay blocked after reloads', () => {
    const runtimeConfig = buildRuntimeToolConfigForExecution({
      enabled: true,
      selection_strategy: 'Manual',
      max_tools: 4,
      preselected_tools: [],
      disabled_tools: [],
      manual_tools: ['file_read', 'grep'],
      allowed_tools: [],
    })

    expect(runtimeConfig.allowed_tools).toEqual(['file_read', 'grep'])
    expect(runtimeConfig.allowed_tools).not.toContain('tenth_man_review')
    expect(runtimeToolConfigAllowsTool(runtimeConfig, 'tenth_man_review')).toBe(false)
    expect(runtimeToolConfigAllowsTool(runtimeConfig, 'file_read')).toBe(true)
  })

  it('blocks tenth man when it is disabled in non-manual runtime config', () => {
    const runtimeConfig = buildRuntimeToolConfigForExecution({
      enabled: true,
      selection_strategy: 'Keyword',
      max_tools: 4,
      preselected_tools: [],
      disabled_tools: ['tenth_man_review'],
      manual_tools: [],
      allowed_tools: [],
    })

    expect(runtimeToolConfigAllowsTool(runtimeConfig, 'tenth_man_review')).toBe(false)
  })

  it('normalizes removed Skills selection strategy to Keyword', () => {
    const normalized = normalizeUiToolConfigPayload({
      enabled: true,
      selection_strategy: 'Skills',
      max_tools: 8,
      preselected_tools: [],
      disabled_tools: [],
      allowed_tools: [],
    })

    expect(normalized.selection_strategy).toBe('Keyword')
  })

  it('applies Team specialist tools as the intersection of profile tools and role tools', () => {
    const runtimeConfig = buildRuntimeToolConfigForTeamRole({
      enabled: true,
      selection_strategy: 'Manual',
      max_tools: 6,
      preselected_tools: ['interactive_shell'],
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
      preselected_tools: [],
      allowed_tools: ['file_read', 'shell'],
    })
  })

  it('maps legacy interactive shell ids to shell in Team specialist scope', () => {
    const runtimeConfig = buildRuntimeToolConfigForTeamRole({
      enabled: true,
      selection_strategy: 'Manual',
      max_tools: 8,
      preselected_tools: ['interactive_shell', 'file_write'],
      disabled_tools: [],
      manual_tools: ['interactive_shell', 'file_write', 'http_request'],
      allowed_tools: [],
    }, {
      specialist: {
        tools: ['shell', 'browser_shell'],
      },
    }, 'specialist')

    expect(runtimeConfig.enabled).toBe(true)
    expect(runtimeConfig.selection_strategy).toEqual({ Manual: ['shell'] })
    expect(runtimeConfig.preselected_tools).toEqual([])
    expect(runtimeConfig.allowed_tools).toEqual(['shell'])
  })

  it('keeps role tools exact without alias expansion', () => {
    const runtimeConfig = buildRuntimeToolConfigForTeamRole({
      enabled: true,
      selection_strategy: 'Manual',
      max_tools: 8,
      preselected_tools: [],
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
      preselected_tools: ['interactive_shell'],
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

  it('falls back to Team role tools when the profile has no explicit preselected tools', () => {
    const runtimeConfig = buildRuntimeToolConfigForTeamRole({
      enabled: true,
      selection_strategy: 'Hybrid',
      max_tools: 8,
      preselected_tools: [],
      disabled_tools: [],
      manual_tools: [],
      allowed_tools: [],
    }, {
      orchestrator: {
        tools: ['ask_user_question', 'spawn_agent'],
      },
    }, 'orchestrator')

    expect(runtimeConfig.enabled).toBe(true)
    expect(runtimeConfig.selection_strategy).toEqual({
      Manual: ['ask_user_question', 'spawn_agent'],
    })
    expect(runtimeConfig.allowed_tools).toEqual(['ask_user_question', 'spawn_agent'])
  })
})
