import { defineComponent, h, nextTick } from 'vue'
import { flushPromises, mount } from '@vue/test-utils'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { useProxyConfiguration } from './useProxyConfiguration'

vi.mock('@tauri-apps/plugin-dialog', () => ({
  open: vi.fn(),
}))

vi.mock('@/composables/useDialog', async () => {
  const { createDialogConfirmAndToastMock } = await import('./trafficMessageViewTestMocks')
  return createDialogConfirmAndToastMock()
})

function createCommandResponse<T>(data: T) {
  return {
    success: true,
    data,
  }
}

const TestHarness = defineComponent({
  setup(_, { expose }) {
    const state = useProxyConfiguration({
      t: (key: string, fallback?: string) => fallback ?? key,
      emitFilterRuleAdded: () => {},
    })
    expose(state)
    return () => h('div')
  },
})

describe('useProxyConfiguration', () => {
  beforeEach(() => {
    vi.useFakeTimers()
    global.testUtils.mockInvoke.mockReset()
    global.testUtils.mockListen.mockReset()
    global.testUtils.mockListen.mockResolvedValue(() => {})
    window.localStorage.clear()

    global.testUtils.mockInvoke.mockImplementation(async (command: string, payload?: any) => {
      switch (command) {
        case 'get_proxy_config':
          return createCommandResponse({
            start_port: 8080,
            max_port_attempts: 10,
            mitm_enabled: true,
            max_request_body_size: 2 * 1024 * 1024,
            max_response_body_size: 2 * 1024 * 1024,
            upstream_proxy: null,
            exclude_self_traffic: true,
            scope_include_rules: [],
            scope_exclude_rules: [],
            match_replace_rules: [
              {
                enabled: true,
                type: 'Request header',
                match: 'User-Agent',
                replace: 'Sentinel',
                scope: 'In scope',
                item: '1',
                comment: 'rewrite ua',
              },
            ],
          })
        case 'get_proxy_auto_start':
        case 'get_intercept_enabled':
        case 'get_request_intercept_enabled':
        case 'get_response_intercept_enabled':
          return createCommandResponse(false)
        case 'get_traffic_analysis_plugin_enabled':
          return createCommandResponse(true)
        case 'get_traffic_behavior_signal_settings':
          return createCommandResponse({
            mode: 'proxy_inferred',
            browserExtensionConnected: false,
            browserExtensionLastSeenAt: null,
          })
        case 'get_traffic_plugin_runtime_settings':
          return createCommandResponse({
            activeProbe: {
              maxQueueDepth: 1000,
              maxPendingPerRun: 250,
              maxPendingPerPlugin: 500,
              maxGlobalConcurrent: 20,
              jitterRange: [300, 1000],
              minHostCooldownMs: 1000,
              maxConcurrentPerHost: 2,
              maxConcurrentPerRun: 6,
              maxConcurrentPerPlugin: 10,
              timeoutMs: 8000,
            },
            bountyFetch: {
              maxQueueDepth: 1000,
              maxPendingPerRun: 250,
              maxPendingPerPlugin: 500,
              maxGlobalConcurrent: 20,
              maxConcurrentPerHost: 2,
              maxConcurrentPerRun: 6,
              maxConcurrentPerPlugin: 10,
              minHostDelayMs: 1000,
              jitterRange: [300, 1000],
              timeoutMs: 8000,
            },
            monitorFetch: {
              maxQueueDepth: 500,
              maxPendingPerRun: 100,
              maxPendingPerPlugin: 250,
              maxGlobalConcurrent: 8,
              maxConcurrentPerHost: 1,
              maxConcurrentPerRun: 3,
              maxConcurrentPerPlugin: 4,
              minHostDelayMs: 2000,
              jitterRange: [500, 2000],
              timeoutMs: 15000,
            },
            agentFetch: {
              maxQueueDepth: 300,
              maxPendingPerRun: 75,
              maxPendingPerPlugin: 150,
              maxGlobalConcurrent: 10,
              maxConcurrentPerHost: 2,
              maxConcurrentPerRun: 4,
              maxConcurrentPerPlugin: 6,
              minHostDelayMs: 500,
              jitterRange: [100, 500],
              timeoutMs: 15000,
            },
            pluginTestFetch: {
              maxQueueDepth: 50,
              maxPendingPerRun: 20,
              maxPendingPerPlugin: 30,
              maxGlobalConcurrent: 2,
              maxConcurrentPerHost: 1,
              maxConcurrentPerRun: 2,
              maxConcurrentPerPlugin: 2,
              minHostDelayMs: 100,
              jitterRange: [0, 100],
              timeoutMs: 5000,
            },
          })
        case 'get_traffic_oast_config':
          return createCommandResponse({
            enabled: false,
            serverBaseUrl: '',
            apiKey: '',
            pollIntervalSecs: 15,
            requestTimeoutSecs: 10,
          })
        case 'get_traffic_behavior_extension_installation':
          return createCommandResponse({
            bridgeUrl: 'http://127.0.0.1:18931',
            extensionDirectory: '',
            directorySource: 'bundled',
            bundledWithApp: true,
          })
        case 'get_proxy_status':
          return createCommandResponse({
            running: false,
            port: 8080,
          })
        case 'save_proxy_config':
          return createCommandResponse(payload?.config ?? null)
        case 'update_runtime_filter_rules':
          return createCommandResponse(null)
        default:
          return createCommandResponse(null)
      }
    })
  })

  afterEach(() => {
    vi.runOnlyPendingTimers()
    vi.useRealTimers()
  })

  it('persists match-replace enabled changes when toggled off', async () => {
    const wrapper = mount(TestHarness)
    await flushPromises()
    await vi.advanceTimersByTimeAsync(500)

    const viewModel = wrapper.vm as unknown as {
      matchReplaceRules: Array<{
        enabled: boolean
        type: string
        match: string
        replace: string
        scope: string
        item: string
        comment: string
      }>
    }

    expect(viewModel.matchReplaceRules).toHaveLength(1)
    expect(viewModel.matchReplaceRules[0].enabled).toBe(true)

    viewModel.matchReplaceRules[0].enabled = false
    await nextTick()
    await vi.advanceTimersByTimeAsync(1000)
    await flushPromises()

    expect(global.testUtils.mockInvoke).toHaveBeenCalledWith('save_proxy_config', {
      config: expect.objectContaining({
        match_replace_rules: [
          expect.objectContaining({
            enabled: false,
            type: 'Request header',
            match: 'User-Agent',
            replace: 'Sentinel',
          }),
        ],
      }),
    })

    wrapper.unmount()
  })
})
