import { beforeEach, describe, expect, it, vi } from 'vitest'
import { nextTick, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useAgentModelAndToolConfig } from './useAgentModelAndToolConfig'

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}))

const mockedInvoke = vi.mocked(invoke)

const createModelConfig = () => useAgentModelAndToolConfig({
  localError: ref(null),
  getFailedToSaveToolConfigLabel: () => 'failed',
})

const mockAiConfig = () => {
  mockedInvoke.mockImplementation(async (command: string) => {
    if (command !== 'get_ai_config') return null
    return {
      default_llm_model: 'mimo/mimo-v2.5-pro',
      providers: {
        Mimo: {
          enabled: true,
          provider: 'Mimo',
          models: [{ id: 'mimo-v2.5-pro', name: 'mimo-v2.5-pro' }],
        },
        deepseek: {
          enabled: true,
          provider: 'deepseek',
          models: [{ id: 'deepseek-v4-pro', name: 'deepseek-v4-pro' }],
        },
      },
    }
  })
}

describe('useAgentModelAndToolConfig', () => {
  beforeEach(() => {
    mockedInvoke.mockReset()
    localStorage.clear()
  })

  it('uses the AI global default instead of the old assistant model cache', async () => {
    mockAiConfig()
    localStorage.setItem('sentinel:agent:assistant-model', 'deepseek/deepseek-v4-pro')

    const config = createModelConfig()
    await config.loadAssistantModelOptions()
    await nextTick()

    expect(config.assistantGlobalDefaultModel.value).toBe('mimo/mimo-v2.5-pro')
    expect(config.assistantSelectedModel.value).toBe('mimo/mimo-v2.5-pro')
  })

  it('keeps an existing conversation model selection when model options reload', async () => {
    mockAiConfig()
    const config = createModelConfig()
    config.setAssistantSelectedModel('deepseek/deepseek-v4-pro', { persist: false })

    await config.loadAssistantModelOptions()
    await nextTick()

    expect(config.assistantSelectedModel.value).toBe('deepseek/deepseek-v4-pro')
  })
})
