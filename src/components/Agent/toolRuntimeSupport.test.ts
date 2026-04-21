import { describe, expect, it } from 'vitest'

import { extractToolRuntimeMetadata } from './toolRuntimeSupport'

describe('toolRuntimeSupport', () => {
  it('extracts direct runtime metadata from structured output', () => {
    expect(
      extractToolRuntimeMetadata({
        runtime: {
          execution_environment: 'docker',
          working_dir: '/workspace/context',
          container_ref: 'sentinel-sandbox-main',
        },
      })
    ).toEqual({
      executionEnvironment: 'docker',
      workingDir: '/workspace/context',
      containerRef: 'sentinel-sandbox-main',
    })
  })

  it('extracts runtime metadata from nested tool text payloads', () => {
    expect(
      extractToolRuntimeMetadata([
        {
          type: 'text',
          text: JSON.stringify({
            runtime: {
              execution_environment: 'host',
              working_dir: '/Users/like/code/sentinel/sentinel-ai',
            },
          }),
        },
      ])
    ).toEqual({
      executionEnvironment: 'host',
      workingDir: '/Users/like/code/sentinel/sentinel-ai',
      containerRef: null,
    })
  })

  it('returns null when runtime metadata is absent', () => {
    expect(
      extractToolRuntimeMetadata({
        matches: ['src/main.rs'],
      })
    ).toBeNull()
  })
})
