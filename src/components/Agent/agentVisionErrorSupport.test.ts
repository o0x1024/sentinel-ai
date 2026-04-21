import { describe, expect, it } from 'vitest'
import {
  buildVisionModelUnsupportedError,
  isVisionModelUnsupportedError,
} from './agentVisionErrorSupport'

describe('isVisionModelUnsupportedError', () => {
  it('matches the backend unsupported vision guidance', () => {
    expect(
      isVisionModelUnsupportedError(
        'Current model does not support image understanding: openai/gpt-4.1. Switch to a vision-capable model in the conversation work config, or explicitly change image handling to local OCR.',
      ),
    ).toBe(true)
  })

  it('returns false for unrelated execution errors', () => {
    expect(isVisionModelUnsupportedError('Agent execution failed')).toBe(false)
  })

  it('returns false when only one phrase is present', () => {
    expect(isVisionModelUnsupportedError('Current model does not support image understanding')).toBe(false)
  })

  it('builds the same unsupported vision guidance used by the frontend guard', () => {
    expect(buildVisionModelUnsupportedError('openai', 'gpt-4.1')).toContain(
      'Current model does not support image understanding: openai/gpt-4.1.',
    )
  })
})
