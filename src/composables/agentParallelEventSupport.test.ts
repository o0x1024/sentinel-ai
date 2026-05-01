import { describe, expect, it } from 'vitest'
import {
  appendParallelChunk,
  appendParallelToolCall,
  appendParallelToolResult,
  normalizeParallelRun,
  summarizeParallelRun,
  type ParallelModelState,
} from './agentParallelEventSupport'

describe('agentParallelEventSupport', () => {
  it('normalizes backend parallel run detail for UI rendering', () => {
    const run = normalizeParallelRun({
      id: 'run-1',
      parent_conversation_id: 'conv-1',
      task: 'review this',
      status: 'succeeded',
      aggregation_mode: 'judge',
      judge_status: 'succeeded',
      judge_content: 'best answer',
      items: [
        {
          model_run_id: 'child-1',
          provider: 'openai',
          model: 'gpt',
          status: 'succeeded',
          content: 'answer',
          tool_count: 2,
        },
      ],
    })

    expect(run.parentConversationId).toBe('conv-1')
    expect(run.aggregationMode).toBe('judge')
    expect(run.judgeContent).toBe('best answer')
    expect(run.items[0].toolCount).toBe(2)
    expect(summarizeParallelRun(run)).toContain('1/1')
  })

  it('counts tool calls as first model response', () => {
    const item: ParallelModelState = {
      modelRunId: 'child-1',
      provider: 'openai',
      model: 'gpt',
      status: 'pending',
      content: '',
      toolCount: 0,
      startedAtMs: Date.now() - 250,
      events: [],
    }

    appendParallelToolCall(item, 'shell', { command: 'pwd' }, 'call-1')

    expect(item.firstResponseMs || 0).toBeGreaterThan(0)
    expect(item.events[0].type).toBe('tool_call')
  })

  it('upserts tool calls and results by tool call id', () => {
    const item: ParallelModelState = {
      modelRunId: 'child-1',
      provider: 'openai',
      model: 'gpt',
      status: 'pending',
      content: '',
      toolCount: 0,
      startedAtMs: Date.now() - 250,
      events: [],
    }

    appendParallelToolCall(item, 'shell', { command: 'pwd' }, 'call-1')
    appendParallelToolCall(item, 'shell', { command: 'pwd' }, 'call-1')
    appendParallelToolResult(item, 'shell', 'ok', true, 'call-1')
    appendParallelToolResult(item, 'shell', 'ok', true, 'call-1')

    expect(item.events).toHaveLength(1)
    expect(item.events[0].type).toBe('tool_result')
    expect(item.events[0].id).toBe('parallel-event:child-1:tool:call-1')
    expect(item.toolCount).toBe(1)
  })

  it('keeps a single growing text event for streamed chunks', () => {
    const item: ParallelModelState = {
      modelRunId: 'child-1',
      provider: 'openai',
      model: 'gpt',
      status: 'pending',
      content: '',
      toolCount: 0,
      startedAtMs: Date.now() - 250,
      events: [],
    }

    appendParallelChunk(item, 'text', 'hello')
    appendParallelChunk(item, 'text', ' world')

    expect(item.events).toHaveLength(1)
    expect(item.events[0].id).toBe('parallel-event:child-1:text:0')
    expect(item.events[0].content).toBe('hello world')
  })

  it('deduplicates persisted duplicate events during normalization', () => {
    const run = normalizeParallelRun({
      id: 'run-1',
      parent_conversation_id: 'conv-1',
      task: 'review this',
      status: 'succeeded',
      items: [
        {
          model_run_id: 'child-1',
          provider: 'openai',
          model: 'gpt',
          status: 'succeeded',
          events: [
            { id: 'a', type: 'text', title: '输出', content: 'same', timestamp_ms: 1 },
            { id: 'b', type: 'text', title: '输出', content: 'same', timestamp_ms: 2 },
            {
              id: 'c',
              type: 'tool_result',
              title: '工具结果: shell',
              content: 'ok',
              tool_name: 'shell',
              tool_call_id: 'call-1',
              timestamp_ms: 3,
            },
            {
              id: 'd',
              type: 'tool_result',
              title: '工具结果: shell',
              content: 'ok',
              tool_name: 'shell',
              tool_call_id: 'call-1',
              timestamp_ms: 4,
            },
          ],
        },
      ],
    })

    expect(run.items[0].events).toHaveLength(2)
    expect(run.items[0].events.map(event => event.id)).toEqual(['b', 'd'])
  })
})
