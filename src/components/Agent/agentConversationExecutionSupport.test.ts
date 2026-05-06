import { describe, expect, it } from 'vitest'

import {
  ensureConversationForExecution,
  shouldReuseTerminalSession,
} from '@/components/Agent/agentConversationExecutionSupport'

describe('agentConversationExecutionSupport', () => {
  it('reuses the session when fingerprint matches the current host runtime config', () => {
    expect(
      shouldReuseTerminalSession({
        currentSessionId: 'session-host',
        currentSessionFingerprint: 'host|sentinel-sandbox:latest|bash|/tmp/project',
        terminalConfig: {
          default_execution_mode: 'host',
          docker_image: 'sentinel-sandbox:latest',
        },
        workingDirectory: '/tmp/project',
      }),
    ).toBe(true)
  })

  it('rejects a stale docker session after the runtime config switches to host', () => {
    expect(
      shouldReuseTerminalSession({
        currentSessionId: 'session-docker',
        currentSessionFingerprint: 'docker|sentinel-sandbox:latest|bash|/workspace',
        terminalConfig: {
          default_execution_mode: 'host',
          docker_image: 'sentinel-sandbox:latest',
        },
        workingDirectory: '/tmp/project',
      }),
    ).toBe(false)
  })

  it('rejects a docker session when the configured docker image changes', () => {
    expect(
      shouldReuseTerminalSession({
        currentSessionId: 'session-docker',
        currentSessionFingerprint: 'docker|sentinel-sandbox:latest|bash|/workspace',
        terminalConfig: {
          default_execution_mode: 'docker',
          docker_image: 'custom-sandbox:dev',
        },
      }),
    ).toBe(false)
  })

  it('rejects unverifiable sessions when config exists but fingerprint is missing', () => {
    expect(
      shouldReuseTerminalSession({
        currentSessionId: 'session-unknown',
        currentSessionFingerprint: '',
        terminalConfig: {
          default_execution_mode: 'docker',
          docker_image: 'sentinel-sandbox:latest',
        },
      }),
    ).toBe(false)
  })

  it('rejects a host session when the conversation working directory changes', () => {
    expect(
      shouldReuseTerminalSession({
        currentSessionId: 'session-host',
        currentSessionFingerprint: 'host|sentinel-sandbox:latest|bash|/tmp/project-a',
        terminalConfig: {
          default_execution_mode: 'host',
          docker_image: 'sentinel-sandbox:latest',
        },
        workingDirectory: '/tmp/project-b',
      }),
    ).toBe(false)
  })

  it('passes the current conversation binding when execution creates a new conversation', async () => {
    let capturedRequest: Record<string, unknown> | null = null

    const conversationId = await ensureConversationForExecution({
      conversationBinding: {
        schemaVersion: 4,
        profileId: 'assistant.default',
        contextMode: 'sentinel-like',
        runMode: 'assistant',
        workingDirectoryOverride: '',
        ragEnabled: false,
        webSearchEnabled: true,
        tenthManEnabled: true,
        selectedModel: 'openai/gpt-5.5',
        toolsEnabled: true,
        toolConfig: null,
      },
      createConversation: async (request) => {
        capturedRequest = request
        return 'conv-exec'
      },
      getConversationTitle: () => 'Execution conversation',
      getDisplayTitle: () => 'Execution conversation',
      onConversationReady: () => {},
    })

    expect(conversationId).toBe('conv-exec')
    expect(capturedRequest).toMatchObject({
      service_name: 'default',
      title: 'Execution conversation',
      conversation_binding: {
        contextMode: 'sentinel-like',
        selectedModel: 'openai/gpt-5.5',
        tenthManEnabled: true,
      },
    })
  })
})
