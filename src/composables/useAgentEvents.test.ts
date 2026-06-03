import { flushPromises, mount } from '@vue/test-utils'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { defineComponent, h, ref, type Ref } from 'vue'
import { useAgentEvents } from './useAgentEvents'
import { useTerminal } from './useTerminal'
import type { UseAgentEventsReturn } from './useAgentEventTypes'

const eventListeners: Record<string, Array<(event: { payload: any }) => void>> = {}

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}))

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(async (eventName: string, handler: (event: { payload: any }) => void) => {
    eventListeners[eventName] ||= []
    eventListeners[eventName].push(handler)
    return () => {
      eventListeners[eventName] = (eventListeners[eventName] || []).filter(item => item !== handler)
    }
  }),
}))

const emitTauriEvent = async (eventName: string, payload: any) => {
  for (const handler of eventListeners[eventName] || []) {
    handler({ payload })
  }
  await flushPromises()
}

describe('useAgentEvents', () => {
  let api: UseAgentEventsReturn | null = null
  let executionId: Ref<string>

  beforeEach(() => {
    Object.keys(eventListeners).forEach((key) => {
      delete eventListeners[key]
    })
    executionId = ref('conversation-1')
    useTerminal().resetTerminal()
    api = null
  })

  afterEach(() => {
    api?.stopListening()
    api = null
  })

  const mountHarness = async () => {
    const Harness = defineComponent({
      setup() {
        api = useAgentEvents(executionId)
        return () => h('div')
      },
    })
    const wrapper = mount(Harness)
    await flushPromises()
    return wrapper
  }

  it('accepts a newer generation after the previous execution has finished', async () => {
    const wrapper = await mountHarness()

    await emitTauriEvent('agent:user_message', {
      execution_id: 'conversation-1',
      generation: 1,
      message_id: 'user-1',
      content: 'first message',
      timestamp: 1000,
    })
    expect(api?.messages.value.map(message => message.content)).toEqual(['first message'])

    await emitTauriEvent('agent:execution_finished', {
      execution_id: 'conversation-1',
      generation: 1,
      outcome: 'succeeded',
      success: true,
    })
    expect(api?.isExecuting.value).toBe(false)

    await emitTauriEvent('agent:user_message', {
      execution_id: 'conversation-1',
      generation: 2,
      message_id: 'user-2',
      content: 'resent message',
      timestamp: 2000,
    })

    expect(api?.messages.value.map(message => message.content)).toEqual([
      'first message',
      'resent message',
    ])

    wrapper.unmount()
  })

  it('rejects older generation events after a newer generation is active', async () => {
    const wrapper = await mountHarness()

    await emitTauriEvent('agent:user_message', {
      execution_id: 'conversation-1',
      generation: 2,
      message_id: 'user-2',
      content: 'current message',
      timestamp: 2000,
    })

    await emitTauriEvent('agent:user_message', {
      execution_id: 'conversation-1',
      generation: 1,
      message_id: 'user-1',
      content: 'stale message',
      timestamp: 1000,
    })

    expect(api?.messages.value.map(message => message.content)).toEqual(['current message'])

    wrapper.unmount()
  })

  it('tracks context compression only for the active execution generation', async () => {
    const wrapper = await mountHarness()

    await emitTauriEvent('agent:user_message', {
      execution_id: 'conversation-1',
      generation: 2,
      message_id: 'user-2',
      content: 'current message',
      timestamp: 2000,
    })

    await emitTauriEvent('agent:context_compression_started', {
      execution_id: 'conversation-1',
      generation: 1,
      reason: 'token_threshold',
      recent_tokens: 90000,
      threshold_tokens: 80000,
      message_count: 24,
      recent_message_count: 20,
    })

    expect(api?.contextCompression.value).toBeNull()

    await emitTauriEvent('agent:context_compression_started', {
      execution_id: 'conversation-1',
      generation: 2,
      reason: 'token_threshold',
      recent_tokens: 90000,
      threshold_tokens: 80000,
      message_count: 24,
      recent_message_count: 20,
    })

    expect(api?.contextCompression.value).toMatchObject({
      active: true,
      executionId: 'conversation-1',
      generation: 2,
      reason: 'token_threshold',
      recentTokens: 90000,
      thresholdTokens: 80000,
      messageCount: 24,
      recentMessageCount: 20,
    })

    await emitTauriEvent('agent:context_compression_finished', {
      execution_id: 'conversation-1',
      generation: 2,
      status: 'completed',
    })

    expect(api?.contextCompression.value).toBeNull()

    wrapper.unmount()
  })

  it('clears active context compression when execution finishes', async () => {
    const wrapper = await mountHarness()

    await emitTauriEvent('agent:user_message', {
      execution_id: 'conversation-1',
      generation: 3,
      message_id: 'user-3',
      content: 'current message',
      timestamp: 3000,
    })
    await emitTauriEvent('agent:context_compression_started', {
      execution_id: 'conversation-1',
      generation: 3,
      reason: 'message_count',
    })

    expect(api?.contextCompression.value?.active).toBe(true)

    await emitTauriEvent('agent:execution_finished', {
      execution_id: 'conversation-1',
      generation: 3,
      outcome: 'succeeded',
      success: true,
    })

    expect(api?.contextCompression.value).toBeNull()

    wrapper.unmount()
  })

  it('keeps compression status when the matching user message arrives after compression started', async () => {
    const wrapper = await mountHarness()

    await emitTauriEvent('agent:context_compression_started', {
      execution_id: 'conversation-1',
      generation: 4,
      reason: 'message_count',
    })

    await emitTauriEvent('agent:user_message', {
      execution_id: 'conversation-1',
      generation: 4,
      message_id: 'user-4',
      content: 'current message',
      timestamp: 4000,
    })

    expect(api?.contextCompression.value?.active).toBe(true)
    expect(api?.contextCompression.value?.generation).toBe(4)

    wrapper.unmount()
  })

  it('tracks request context pressure details for the active execution generation', async () => {
    const wrapper = await mountHarness()

    await emitTauriEvent('agent:user_message', {
      execution_id: 'conversation-1',
      generation: 5,
      message_id: 'user-5',
      content: 'current message',
      timestamp: 5000,
    })

    await emitTauriEvent('agent:context_pressure', {
      execution_id: 'conversation-1',
      generation: 4,
      phase: 'pre_stream_request',
      used_tokens: 90000,
      remaining_tokens: 10000,
      usage_percentage: 90,
      context_pressure: 'AutoCompact',
      should_compact: true,
      should_block: false,
      max_context_tokens: 128000,
      effective_context_tokens: 108800,
      warning_threshold_tokens: 88600,
      auto_compact_threshold_tokens: 95800,
      blocking_threshold_tokens: 105800,
      output_reserve_tokens: 19200,
    })

    expect(api?.contextUsage.value).toBeNull()

    await emitTauriEvent('agent:context_pressure', {
      execution_id: 'conversation-1',
      generation: 5,
      phase: 'pre_stream_request',
      used_tokens: 96000,
      remaining_tokens: 12800,
      usage_percentage: 88.24,
      context_pressure: 'AutoCompact',
      should_compact: true,
      should_block: false,
      system_prompt_tokens: 20000,
      task_tokens: 1000,
      history_tokens: 75000,
      history_count: 32,
      max_context_tokens: 128000,
      effective_context_tokens: 108800,
      warning_threshold_tokens: 88600,
      auto_compact_threshold_tokens: 95800,
      blocking_threshold_tokens: 105800,
      output_reserve_tokens: 19200,
    })

    expect(api?.contextUsage.value).toMatchObject({
      usedTokens: 96000,
      maxTokens: 128000,
      remainingTokens: 12800,
      contextPressure: 'AutoCompact',
      shouldCompact: true,
      shouldBlock: false,
      pressurePhase: 'pre_stream_request',
      systemPromptTokens: 20000,
      taskTokens: 1000,
      historyTokens: 75000,
      historyCount: 32,
      effectiveContextTokens: 108800,
      autoCompactThresholdTokens: 95800,
    })

    wrapper.unmount()
  })

  it('shows compression status when request compaction is requested', async () => {
    const wrapper = await mountHarness()

    await emitTauriEvent('agent:user_message', {
      execution_id: 'conversation-1',
      generation: 6,
      message_id: 'user-6',
      content: 'current message',
      timestamp: 6000,
    })

    await emitTauriEvent('agent:context_compaction_requested', {
      execution_id: 'conversation-1',
      generation: 6,
      phase: 'pre_stream_request',
      used_tokens: 98000,
      remaining_tokens: 10800,
      context_pressure: 'AutoCompact',
    })

    expect(api?.contextCompression.value).toMatchObject({
      active: true,
      executionId: 'conversation-1',
      generation: 6,
      reason: 'pre_stream_request',
      recentTokens: 98000,
    })

    wrapper.unmount()
  })

  it('keeps empty shell continuation calls out of the visible message flow', async () => {
    const wrapper = await mountHarness()

    await emitTauriEvent('agent:tool_call_complete', {
      execution_id: 'conversation-1',
      tool_call_id: 'call-poll-1',
      tool_name: 'shell',
      arguments: JSON.stringify({
        session_id: 'session-1',
        action: 'poll',
      }),
    })

    expect(api?.messages.value).toHaveLength(0)

    await emitTauriEvent('agent:tool_result', {
      execution_id: 'conversation-1',
      tool_call_id: 'call-poll-1',
      result: JSON.stringify({
        action: 'poll',
        stdout: '',
        status: 'running',
      }),
      success: false,
    })

    expect(api?.messages.value).toHaveLength(0)

    await emitTauriEvent('agent:tool_call_complete', {
      execution_id: 'conversation-1',
      tool_call_id: 'call-key-1',
      tool_name: 'shell',
      arguments: JSON.stringify({
        session_id: 'session-1',
        action: 'key',
        key: 'ArrowDown',
      }),
    })

    expect(api?.messages.value).toHaveLength(0)

    await emitTauriEvent('agent:tool_result', {
      execution_id: 'conversation-1',
      tool_call_id: 'call-key-1',
      result: JSON.stringify({
        action: 'key',
        stdout: '',
        status: 'running',
      }),
      success: false,
    })

    expect(api?.messages.value).toHaveLength(0)

    wrapper.unmount()
  })

  it('shows shell continuation results when they contain output', async () => {
    const wrapper = await mountHarness()

    await emitTauriEvent('agent:tool_call_complete', {
      execution_id: 'conversation-1',
      tool_call_id: 'call-submit-1',
      tool_name: 'shell',
      arguments: JSON.stringify({
        session_id: 'session-1',
        action: 'submit',
      }),
    })

    expect(api?.messages.value).toHaveLength(0)

    await emitTauriEvent('agent:tool_result', {
      execution_id: 'conversation-1',
      tool_call_id: 'call-submit-1',
      result: JSON.stringify({
        action: 'submit',
        stdout: 'Scaffolding project...',
        status: 'running',
      }),
      success: false,
    })

    expect(api?.messages.value).toHaveLength(1)
    expect(api?.messages.value[0]?.metadata?.tool_result).toContain('Scaffolding project')

    wrapper.unmount()
  })

  it('inserts late shell continuation output before the following final answer', async () => {
    const wrapper = await mountHarness()

    await emitTauriEvent('agent:tool_call_complete', {
      execution_id: 'conversation-1',
      tool_call_id: 'call-shell-start',
      tool_name: 'shell',
      arguments: JSON.stringify({
        command: 'npm create vite@latest .',
        yield_time_ms: 1000,
      }),
    })

    await emitTauriEvent('agent:tool_call_complete', {
      execution_id: 'conversation-1',
      tool_call_id: 'call-submit-late',
      tool_name: 'shell',
      arguments: JSON.stringify({
        session_id: 'session-1',
        action: 'submit',
      }),
    })

    await emitTauriEvent('agent:chunk', {
      execution_id: 'conversation-1',
      chunk_type: 'text',
      content: '命令执行成功',
    })

    expect(api?.messages.value.map(message => message.type)).toEqual(['tool_call', 'final'])

    await emitTauriEvent('agent:tool_result', {
      execution_id: 'conversation-1',
      tool_call_id: 'call-submit-late',
      result: JSON.stringify({
        action: 'submit',
        stdout: 'VITE v8.0.11 ready in 429 ms',
        status: 'running',
      }),
      success: false,
    })

    expect(api?.messages.value.map(message => message.type)).toEqual(['tool_call', 'tool_call', 'final'])
    expect(api?.messages.value[1]?.metadata?.tool_result).toContain('VITE v8.0.11')
    expect(api?.messages.value[2]?.content).toBe('命令执行成功')

    wrapper.unmount()
  })

  it('keeps streaming thinking unlimited until text closes the segment', async () => {
    const wrapper = await mountHarness()

    await emitTauriEvent('agent:chunk', {
      execution_id: 'conversation-1',
      generation: 1,
      chunk_type: 'reasoning',
      content: 'first thought',
    })

    expect(api?.messages.value).toHaveLength(1)
    expect(api?.messages.value[0]?.type).toBe('thinking')
    expect(api?.messages.value[0]?.metadata?.status).toBe('streaming')

    await emitTauriEvent('agent:chunk', {
      execution_id: 'conversation-1',
      generation: 1,
      chunk_type: 'text',
      content: 'answer',
    })

    expect(api?.messages.value.map(message => message.type)).toEqual(['thinking', 'final'])
    expect(api?.messages.value[0]?.metadata?.status).toBe('complete')
    expect(api?.messages.value[1]?.content).toBe('answer')

    wrapper.unmount()
  })

  it('splits thinking into separate segments around assistant text', async () => {
    const wrapper = await mountHarness()

    await emitTauriEvent('agent:chunk', {
      execution_id: 'conversation-1',
      generation: 1,
      chunk_type: 'reasoning',
      content: 'plan A',
    })
    await emitTauriEvent('agent:chunk', {
      execution_id: 'conversation-1',
      generation: 1,
      chunk_type: 'text',
      content: 'partial answer',
    })
    await emitTauriEvent('agent:chunk', {
      execution_id: 'conversation-1',
      generation: 1,
      chunk_type: 'reasoning',
      content: 'plan B',
    })

    expect(api?.messages.value.map(message => message.type)).toEqual([
      'thinking',
      'final',
      'thinking',
    ])
    expect(api?.messages.value[0]?.content).toBe('plan A')
    expect(api?.messages.value[0]?.metadata?.status).toBe('complete')
    expect(api?.messages.value[2]?.content).toBe('plan B')
    expect(api?.messages.value[2]?.metadata?.status).toBe('streaming')

    wrapper.unmount()
  })

  it('accepts live chunks for the current conversation after refresh when execution id differs', async () => {
    const wrapper = await mountHarness()

    await emitTauriEvent('agent:chunk', {
      execution_id: 'run-after-refresh',
      conversation_id: 'conversation-1',
      generation: 1,
      chunk_type: 'text',
      content: 'continued after refresh',
    })

    expect(api?.isExecuting.value).toBe(true)
    expect(api?.currentExecutionId.value).toBe('run-after-refresh')
    expect(api?.messages.value).toHaveLength(1)
    expect(api?.messages.value[0]?.content).toBe('continued after refresh')

    wrapper.unmount()
  })

  it('syncs shell sessions without opening the terminal panel', async () => {
    const wrapper = await mountHarness()
    const terminal = useTerminal()

    await emitTauriEvent('agent:tool_call_complete', {
      execution_id: 'conversation-1',
      tool_call_id: 'call-shell-session',
      tool_name: 'shell',
      arguments: JSON.stringify({
        command: 'npm run dev',
        yield_time_ms: 1000,
      }),
    })

    await emitTauriEvent('agent:tool_result', {
      execution_id: 'conversation-1',
      tool_call_id: 'call-shell-session',
      result: JSON.stringify({
        session_id: 'session-dev',
        stdout: 'ready',
        status: 'running',
      }),
      success: false,
    })

    expect(terminal.currentSessionId.value).toBe('session-dev')
    expect(terminal.isTerminalActive.value).toBe(false)

    wrapper.unmount()
  })

  it('still shows non-poll shell calls in the visible message flow', async () => {
    const wrapper = await mountHarness()

    await emitTauriEvent('agent:tool_call_complete', {
      execution_id: 'conversation-1',
      tool_call_id: 'call-shell-start',
      tool_name: 'shell',
      arguments: JSON.stringify({
        command: 'npm create vite@latest .',
        yield_time_ms: 1000,
      }),
    })

    expect(api?.messages.value).toHaveLength(1)
    expect(api?.messages.value[0]?.type).toBe('tool_call')
    expect(api?.messages.value[0]?.metadata?.tool_name).toBe('shell')

    wrapper.unmount()
  })
})
