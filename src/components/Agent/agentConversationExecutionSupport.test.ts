import { describe, expect, it } from 'vitest'

import {
  ensureConversationForExecution,
  resolveAgentHarnessMode,
  resolveTenthManRuleForExecution,
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
          host_shell: 'bash',
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
          host_shell: 'bash',
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
          host_shell: 'bash',
        },
        workingDirectory: '/tmp/project-b',
      }),
    ).toBe(false)
  })

  it('rejects a host session when the configured host shell changes', () => {
    expect(
      shouldReuseTerminalSession({
        currentSessionId: 'session-host',
        currentSessionFingerprint: 'host|sentinel-sandbox:latest|bash|/tmp/project',
        terminalConfig: {
          default_execution_mode: 'host',
          docker_image: 'sentinel-sandbox:latest',
          host_shell: '/bin/zsh',
        },
        workingDirectory: '/tmp/project',
      }),
    ).toBe(false)
  })

  it('uses direct harness mode for short requests without tools or a task plan', () => {
    expect(resolveAgentHarnessMode({
      forceTaskPlanContract: false,
      runtimeToolConfig: { enabled: false },
    })).toBe('direct')
  })

  it('uses tool_run harness mode when tools are enabled without a task plan', () => {
    expect(resolveAgentHarnessMode({
      forceTaskPlanContract: false,
      runtimeToolConfig: { enabled: true },
    })).toBe('tool_run')
  })

  it('uses planned harness mode when task planning is forced', () => {
    expect(resolveAgentHarnessMode({
      forceTaskPlanContract: true,
      runtimeToolConfig: { enabled: false },
    })).toBe('planned')
  })

  it('disables tenth man rule when runtime tool scope excludes tenth_man_review', () => {
    expect(resolveTenthManRuleForExecution({
      enabled: true,
      runtimeToolConfig: {
        enabled: true,
        selection_strategy: { Manual: ['file_read', 'grep'] },
        max_tools: 4,
        preselected_tools: [],
        disabled_tools: [],
        allowed_tools: ['file_read', 'grep'],
      },
    })).toBe(false)
  })

  it('keeps tenth man rule enabled when runtime tool scope includes tenth_man_review', () => {
    expect(resolveTenthManRuleForExecution({
      enabled: true,
      runtimeToolConfig: {
        enabled: true,
        selection_strategy: { Manual: ['tenth_man_review'] },
        max_tools: 1,
        preselected_tools: [],
        disabled_tools: [],
        allowed_tools: ['tenth_man_review'],
      },
    })).toBe(true)
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
        harnessMaxContinuations: 8,
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
        harnessMaxContinuations: 8,
      },
    })
  })
})
