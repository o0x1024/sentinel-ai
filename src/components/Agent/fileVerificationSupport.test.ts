import { describe, expect, it } from 'vitest'

import type { AgentMessage } from '@/types/agent'
import { applyFileVerificationStatuses } from './fileVerificationSupport'

describe('fileVerificationSupport', () => {
  it('marks file mutations as verified when a later file_read matches the same hash', () => {
    const messages: AgentMessage[] = [
      {
        id: 'write-1',
        type: 'tool_call',
        content: 'completed',
        timestamp: 1,
        metadata: {
          tool_name: 'file_write',
          tool_result: JSON.stringify({
            file_path: '/tmp/a.txt',
            content_hash: 'hash-a',
            stored_artifacts: [{ path: '/tmp/a.txt' }],
            change_summary: { first_changed_line: 1 },
          }),
        },
      },
      {
        id: 'read-1',
        type: 'tool_call',
        content: 'completed',
        timestamp: 2,
        metadata: {
          tool_name: 'file_read',
          tool_result: JSON.stringify({
            file_path: '/tmp/a.txt',
            content_hash: 'hash-a',
            start_line: 1,
            end_line: 10,
          }),
        },
      },
      {
        id: 'edit-1',
        type: 'tool_call',
        content: 'completed',
        timestamp: 3,
        metadata: {
          tool_name: 'file_edit',
          tool_result: JSON.stringify({
            file_path: '/tmp/b.txt',
            content_hash: 'hash-b',
            stored_artifacts: [{ path: '/tmp/b.txt' }],
            change_summary: { first_changed_line: 4 },
          }),
        },
      },
    ]

    applyFileVerificationStatuses(messages)

    expect(messages[0].metadata?.file_verification_status).toBe('verified')
    expect(messages[2].metadata?.file_verification_status).toBe('pending')
  })

  it('marks failed file mutations as failed instead of pending verification', () => {
    const messages: AgentMessage[] = [
      {
        id: 'write-failed-1',
        type: 'tool_call',
        content: 'failed',
        timestamp: 1,
        metadata: {
          tool_name: 'file_write',
          status: 'failed',
          error: 'file must be read with file_read before editing or overwriting: /tmp/a.txt',
          tool_result: 'file must be read with file_read before editing or overwriting: /tmp/a.txt',
        },
      },
    ]

    applyFileVerificationStatuses(messages)

    expect(messages[0].metadata?.file_verification_status).toBe('failed')
  })
})
