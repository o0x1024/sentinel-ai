import { describe, expect, it } from 'vitest'

import { stripDerivedAiConfigFields } from './settingsAiSupport'

describe('settingsAiSupport', () => {
  it('removes derived vision capability fields from model entries', () => {
    const input = {
      default_llm_model: 'openai/gpt-4o',
      providers: {
        OpenAI: {
          models: [
            {
              id: 'gpt-4o',
              name: 'gpt-4o',
              supports_vision: true,
              vision_capability_status: 'supported',
              vision_capability_source: 'runtime_probe',
              vision_capability_evidence: 'probe ok',
            },
          ],
        },
      },
    }

    const result = stripDerivedAiConfigFields(input)

    expect(result.providers.OpenAI.models[0]).toEqual({
      id: 'gpt-4o',
      name: 'gpt-4o',
      supports_vision: true,
    })
    expect(input.providers.OpenAI.models[0].vision_capability_status).toBe('supported')
  })
})
