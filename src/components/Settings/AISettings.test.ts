import { flushPromises, mount } from '@vue/test-utils'
import { describe, expect, it, vi } from 'vitest'
import AISettings from './AISettings.vue'

vi.mock('vue-i18n', () => ({
  useI18n: () => ({
    t: (key: string, fallback?: string) => fallback || key,
  }),
}))

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(async (command: string) => {
    if (command === 'get_config') return []
    if (command === 'get_detailed_ai_usage_stats') return []
    return null
  }),
}))

describe('AISettings', () => {
  it('renders provider configuration when mounted with release-style props', async () => {
    const wrapper = mount(AISettings, {
      props: {
        aiServiceStatus: [],
        selectedAiProvider: 'OpenAI',
        settings: {
          ai: {
            temperature: 0.7,
            maxTokens: 2000,
            outputStorageThreshold: 50000,
            maxTurns: 100,
          },
        },
        customProvider: {
          name: '',
          display_name: '',
          api_key: '',
          api_base: '',
          model_id: '',
          compat_mode: 'openai',
          rig_provider: 'openai',
          extra_headers_json: '',
          extra_body_json: '',
          timeout: 120,
          max_retries: 3,
        },
        aiUsageStats: {},
        saving: false,
        aiConfig: {
          default_llm_provider: 'volcengine',
          default_llm_model: 'volcengine/doubao-seed-2-0-pro-260215',
          providers: {
            OpenAI: {
              id: 'openai',
              provider: 'openai',
              name: 'OpenAI',
              api_key: 'test-key',
              api_base: 'https://ark.cn-beijing.volces.com/api/coding/v3',
              enabled: false,
              default_model: 'ark-code-latest',
              models: [
                {
                  id: 'doubao-seed-2-0-pro-260215',
                  name: 'doubao-seed-2-0-pro-260215',
                  description: 'OpenAI model',
                  is_available: true,
                  context_length: 4096,
                  supports_streaming: true,
                  supports_tools: true,
                },
              ],
              rig_provider: 'openai',
              max_context_length: null,
            },
            volcengine: {
              id: 'volcengine',
              provider: 'volcengine',
              name: 'volcengine',
              api_key: 'test-key',
              api_base: 'https://ark.cn-beijing.volces.com/api/v3',
              enabled: true,
              default_model: 'doubao-seed-2-0-pro-260215',
              models: [],
              rig_provider: 'openai',
              max_context_length: null,
            },
          },
        },
      },
    })

    await flushPromises()

    expect(wrapper.text()).toContain('settings.ai.defaultConfig')
    expect(wrapper.text()).toContain('OpenAI')
    expect(wrapper.find('.ai-settings').exists()).toBe(true)
  })
})
