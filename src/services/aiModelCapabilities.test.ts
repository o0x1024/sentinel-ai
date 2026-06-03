import { describe, expect, it } from 'vitest'

import {
  getModelSupportsVision,
  getModelVisionCapability,
  inferModelSupportsVision,
} from './aiModelCapabilities'

describe('aiModelCapabilities', () => {
  it('infers common multimodal models', () => {
    expect(inferModelSupportsVision('openai', 'gpt-4o')).toBe(true)
    expect(inferModelSupportsVision('anthropic', 'claude-3-5-sonnet')).toBe(true)
    expect(inferModelSupportsVision('gemini', 'gemini-2.5-pro')).toBe(true)
    expect(inferModelSupportsVision('deepseek', 'deepseek-vl2')).toBe(true)
  })

  it('rejects obvious non-visual models', () => {
    expect(inferModelSupportsVision('openai', 'text-embedding-3-large')).toBe(false)
    expect(inferModelSupportsVision('ollama', 'qwen2.5-coder:7b')).toBe(false)
  })

  it('upgrades stale model metadata with heuristic support', () => {
    expect(
      getModelSupportsVision('openai', {
        id: 'gpt-4o',
        supports_vision: false,
      }),
    ).toBe(true)
  })

  it('returns unsupported only for explicit non-vision metadata', () => {
    expect(
      getModelVisionCapability('ollama', {
        id: 'qwen2.5-coder:7b',
        supports_vision: false,
      }),
    ).toBe('unsupported')
  })

  it('prefers backend-resolved vision capability status when present', () => {
    expect(
      getModelVisionCapability('ollama', {
        id: 'qwen2.5-coder:7b',
        supports_vision: true,
        vision_capability_status: 'unsupported',
      }),
    ).toBe('unsupported')
  })

  it('returns unknown when there is no explicit metadata and no heuristic match', () => {
    expect(
      getModelVisionCapability('ollama', {
        id: 'qwen2.5-coder:7b',
      }),
    ).toBe('unknown')
  })
})
