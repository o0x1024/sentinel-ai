import type { ProcessedDocumentResult } from '@/types/agent'
import { invoke } from '@tauri-apps/api/core'
import type {
  ReferencedAsset,
  ReferencedConversationMessage,
  ReferencedFile,
  ReferencedTraffic,
} from '@/types/agentReferences'
import type { AssistantConversationBinding } from './agentDraftTypes'
import { runtimeToolConfigAllowsTool, TENTH_MAN_REVIEW_TOOL_ID } from './toolConfigRuntime'
import { buildTerminalSessionFingerprint, useTerminal } from '@/composables/useTerminal'
import { useBrowserShell } from '@/composables/useBrowserShell'

type ExecutionMode = 'docker' | 'host'

interface AgentRuntimeTerminalConfig {
  docker_image?: string | null
  default_execution_mode?: ExecutionMode | null
  host_shell?: string | null
  docker_shell?: string | null
}

interface AgentRuntimeConfigResponse {
  terminal?: AgentRuntimeTerminalConfig | null
  working_directory?: string | null
}

const DEFAULT_DOCKER_IMAGE = 'sentinel-sandbox:latest'
const DEFAULT_HOST_TERMINAL_SHELL =
  typeof navigator !== 'undefined' && navigator.platform.toLowerCase().includes('mac')
    ? '/bin/zsh'
    : '/bin/bash'
const DEFAULT_DOCKER_TERMINAL_SHELL = 'bash'
const DEFAULT_DOCKER_WORKING_DIRECTORY = '/workspace'
const CONTAINER_CONTEXT_DIR = '/workspace/context'
const EXECUTION_DIR_PREFIX = 'session_'

function dockerSessionWorkingDir(executionId?: string | null): string {
  const raw = (executionId || '').trim()
  if (!raw) return DEFAULT_DOCKER_WORKING_DIRECTORY
  const sanitized = raw.replace(/[^a-zA-Z0-9\-_]/g, '').slice(0, 12)
  if (!sanitized) return DEFAULT_DOCKER_WORKING_DIRECTORY
  return `${CONTAINER_CONTEXT_DIR}/${EXECUTION_DIR_PREFIX}${sanitized}`
}

const normalizeExecutionMode = (value?: string | null): ExecutionMode =>
  String(value || '')
    .trim()
    .toLowerCase() === 'host'
    ? 'host'
    : 'docker'

const normalizeDockerImage = (value?: string | null): string => {
  const normalized = String(value || '').trim()
  return normalized || DEFAULT_DOCKER_IMAGE
}

const resolveTerminalShell = (
  terminalConfig?: AgentRuntimeTerminalConfig | null,
  explicitShell?: string
): string => {
  const explicit = String(explicitShell || '').trim()
  if (explicit) return explicit

  const executionMode = normalizeExecutionMode(terminalConfig?.default_execution_mode)
  if (executionMode === 'docker') {
    return String(terminalConfig?.docker_shell || '').trim() || DEFAULT_DOCKER_TERMINAL_SHELL
  }
  return String(terminalConfig?.host_shell || '').trim() || DEFAULT_HOST_TERMINAL_SHELL
}

const normalizeWorkingDirectory = (value?: string | null): string => String(value || '').trim()

const resolveTerminalWorkingDirectory = (params: {
  executionMode: ExecutionMode
  requestedWorkingDirectory?: string | null
  defaultWorkingDirectory?: string | null
  executionId?: string | null
}): string => {
  if (params.executionMode === 'docker') {
    return dockerSessionWorkingDir(params.executionId)
  }
  return (
    normalizeWorkingDirectory(params.requestedWorkingDirectory) ||
    normalizeWorkingDirectory(params.defaultWorkingDirectory)
  )
}

export const shouldReuseTerminalSession = (params: {
  currentSessionId?: string | null
  currentSessionFingerprint?: string | null
  shell?: string
  terminalConfig?: AgentRuntimeTerminalConfig | null
  workingDirectory?: string | null
  defaultWorkingDirectory?: string | null
  executionId?: string | null
}): boolean => {
  const sessionId = String(params.currentSessionId || '').trim()
  if (!sessionId) {
    return false
  }

  const currentFingerprint = String(params.currentSessionFingerprint || '').trim()
  const terminalConfig = params.terminalConfig
  if (!terminalConfig) {
    return true
  }

  if (!currentFingerprint) {
    return false
  }

  const expectedFingerprint = buildTerminalSessionFingerprint(
    normalizeExecutionMode(terminalConfig.default_execution_mode),
    normalizeDockerImage(terminalConfig.docker_image),
    resolveTerminalShell(terminalConfig, params.shell),
    resolveTerminalWorkingDirectory({
      executionMode: normalizeExecutionMode(terminalConfig.default_execution_mode),
      requestedWorkingDirectory: params.workingDirectory,
      defaultWorkingDirectory: params.defaultWorkingDirectory,
      executionId: params.executionId,
    })
  )

  return currentFingerprint === expectedFingerprint
}

const resolveActiveTerminalBinding = async (params: {
  currentSessionId?: string | null
  currentSessionFingerprint?: string | null
  workingDirectory?: string | null
}): Promise<{
  currentTerminalSessionId?: string
  currentTerminalSessionFingerprint?: string
}> => {
  const sessionId = String(params.currentSessionId || '').trim()
  const sessionFingerprint = String(params.currentSessionFingerprint || '').trim()
  if (!sessionId) {
    return {}
  }

  try {
    const config = await invoke<AgentRuntimeConfigResponse>('get_agent_config')
    if (
      !shouldReuseTerminalSession({
        currentSessionId: sessionId,
        currentSessionFingerprint: sessionFingerprint,
        terminalConfig: config?.terminal,
        workingDirectory: params.workingDirectory,
        defaultWorkingDirectory: config?.working_directory,
      })
    ) {
      return {}
    }
  } catch (error) {
    console.warn(
      '[AgentView] Failed to verify active terminal binding, reusing existing session:',
      error
    )
  }

  return {
    currentTerminalSessionId: sessionId,
    currentTerminalSessionFingerprint: sessionFingerprint || undefined,
  }
}

export const takeOverConversationExecution = async (params: {
  appendPartialAssistantMessage: (message: {
    content: string
    id: string
    timestamp: number
  }) => void
  conversationId?: string | null
  createMessageId: () => string
  currentTime: () => number
  isExecuting: boolean
  savePartialAssistantMessage: (request: {
    content: string
    conversation_id: string
    id: string
    role: string
  }) => Promise<void>
  stopExecution: () => Promise<void>
  streamingContent?: string | null
  waitForStop: () => Promise<void>
}): Promise<void> => {
  if (!params.isExecuting || !params.conversationId) {
    return
  }

  console.log('[AgentView] Takeover: stopping current execution to handle new message')
  try {
    const partial = (params.streamingContent || '').trim()
    if (partial) {
      const partialMsgId = params.createMessageId()
      console.log('[AgentView] Takeover: saving partial response:', partial.substring(0, 100))
      params.appendPartialAssistantMessage({
        content: partial,
        id: partialMsgId,
        timestamp: params.currentTime(),
      })
      await params.savePartialAssistantMessage({
        content: partial,
        conversation_id: params.conversationId,
        id: partialMsgId,
        role: 'assistant',
      })
    }

    await params.stopExecution()
    await params.waitForStop()
    console.log('[AgentView] Takeover: previous execution stopped, proceeding with new message')
  } catch (e) {
    console.warn('[AgentView] Takeover stop failed, continuing:', e)
  }
}

export const ensureConversationForExecution = async (params: {
  conversationId?: string | null
  conversationBinding?: AssistantConversationBinding | null
  createConversation: (request: {
    service_name: string
    title: string
    conversation_binding?: AssistantConversationBinding | null
  }) => Promise<string>
  getConversationTitle: () => string
  getDisplayTitle: () => string
  loadConversationList?: () => void
  onConversationReady: (conversationId: string, title: string) => void
}): Promise<string | null | undefined> => {
  if (params.conversationId) {
    return params.conversationId
  }

  console.log('[AgentView] No conversation ID, creating new conversation')
  const conversationId = await params.createConversation({
    conversation_binding: params.conversationBinding,
    service_name: 'default',
    title: params.getConversationTitle(),
  })
  params.onConversationReady(conversationId, params.getDisplayTitle())
  params.loadConversationList?.()
  console.log('[AgentView] Created new conversation:', conversationId)
  return conversationId
}

export const buildAssistantModelOverride = (
  assistantSelectedModel?: string | null
): string | undefined =>
  assistantSelectedModel && assistantSelectedModel.includes('/')
    ? assistantSelectedModel
    : undefined

const buildModelTarget = (modelKey?: string | null): { provider: string; model: string } | null => {
  const [provider = '', ...modelParts] = String(modelKey || '')
    .trim()
    .split('/')
  const model = modelParts.join('/').trim()
  if (!provider.trim() || !model) return null
  return {
    provider: provider.trim(),
    model,
  }
}

const buildParallelModelTargets = (modelKeys: string[]) =>
  modelKeys
    .map(buildModelTarget)
    .filter((item): item is { provider: string; model: string } => !!item)

export type AgentHarnessMode = 'direct' | 'tool_run' | 'planned' | 'team'

const isRuntimeToolConfigEnabled = (value: unknown): boolean =>
  !!value && typeof value === 'object' && (value as { enabled?: unknown }).enabled === true

export const resolveAgentHarnessMode = (params: {
  forceTaskPlanContract: boolean
  runtimeToolConfig: unknown
}): AgentHarnessMode => {
  if (params.forceTaskPlanContract) return 'planned'
  if (isRuntimeToolConfigEnabled(params.runtimeToolConfig)) return 'tool_run'
  return 'direct'
}

export const resolveTenthManRuleForExecution = (params: {
  enabled: boolean
  runtimeToolConfig: unknown
}): boolean =>
  params.enabled === true &&
  runtimeToolConfigAllowsTool(params.runtimeToolConfig, TENTH_MAN_REVIEW_TOOL_ID)

export const executeConversationTask = async (params: {
  assistantContextMode: 'claude-like' | 'codex-like' | 'sentinel-like'
  assistantExecutionMode?: 'single' | 'parallel'
  assistantParallelJudgeModel?: string | null
  assistantParallelSelectedModels?: string[]
  assistantSelectedModel?: string | null
  conversationId: string
  defaultConversationTitle: string
  displayContent?: string
  enableRag: boolean
  enableTenthManRule: boolean
  firstMessage: string
  forceTaskPlanContract: boolean
  harnessMode?: AgentHarnessMode
  harnessMaxContinuations: number
  fullTask: string
  workingDirectory?: string | null
  maybeAutoRenameConversation: (params: {
    convId: string
    currentConversationId: string
    defaultTitle: string
    firstMessage: string
    onConversationListRefresh: () => void
    onCurrentConversationTitleChange: (title: string) => void
  }) => void
  skipAutoRename?: boolean
  onConversationListRefresh: () => void
  onCurrentConversationTitleChange: (title: string) => void
  runAgentExecute: (request: {
    config: {
      attachments?: unknown[]
      conversation_id: string
      context_mode: 'claude-like' | 'codex-like' | 'sentinel-like'
      current_browser_shell_direct_write_enabled?: boolean
      current_browser_shell_session_id?: string
      current_terminal_session_fingerprint?: string
      current_terminal_session_id?: string
      display_content?: string
      document_attachments?: ProcessedDocumentResult[]
      enable_rag: boolean
      enable_tenth_man_rule: boolean
      execution_id: string
      force_tasks: boolean
      harness_max_continuations: number
      harness_mode: AgentHarnessMode
      message_id: null
      model_override?: string
      persist_messages?: boolean
      referenced_assets?: ReferencedAsset[]
      referenced_files?: ReferencedFile[]
      referenced_messages?: ReferencedConversationMessage[]
      referenced_traffic?: ReferencedTraffic[]
      timeout_secs: number
      tool_config: unknown
      working_directory?: string
    }
    task: string
  }) => Promise<any>
  runAgentExecuteParallel?: (request: {
    request: {
      config: Record<string, unknown>
      aggregation_mode: 'manual' | 'judge'
      judge_model?: { provider: string; model: string }
      models: Array<{ provider: string; model: string }>
      task: string
    }
  }) => Promise<any>
  runtimeToolConfig: unknown
  executionId?: string
  persistMessages?: boolean
  usedAssets: ReferencedAsset[]
  usedAttachments: unknown[]
  usedDocuments: ProcessedDocumentResult[]
  usedFiles: ReferencedFile[]
  usedMessages: ReferencedConversationMessage[]
  usedTraffic: ReferencedTraffic[]
}): Promise<any> => {
  const terminal = useTerminal()
  const browserShell = useBrowserShell()
  const terminalBinding = await resolveActiveTerminalBinding({
    currentSessionId: terminal.currentSessionId.value,
    currentSessionFingerprint: terminal.currentSessionFingerprint.value,
    workingDirectory: params.workingDirectory,
  })
  const currentBrowserShellSessionId = browserShell.currentSessionId.value?.trim() || undefined
  const currentBrowserShellDirectWriteEnabled =
    currentBrowserShellSessionId && browserShell.directWriteEnabled.value === true
      ? true
      : undefined
  const harnessMode = params.harnessMode || resolveAgentHarnessMode({
    forceTaskPlanContract: params.forceTaskPlanContract,
    runtimeToolConfig: params.runtimeToolConfig,
  })
  const enableTenthManRule = resolveTenthManRuleForExecution({
    enabled: params.enableTenthManRule,
    runtimeToolConfig: params.runtimeToolConfig,
  })
  const executionId = params.executionId?.trim() || crypto.randomUUID()

  if (!params.skipAutoRename) {
    params.maybeAutoRenameConversation({
      convId: params.conversationId,
      currentConversationId: params.conversationId,
      defaultTitle: params.defaultConversationTitle,
      firstMessage: params.firstMessage,
      onConversationListRefresh: params.onConversationListRefresh,
      onCurrentConversationTitleChange: params.onCurrentConversationTitleChange,
    })
  }

  const config = {
    attachments: params.usedAttachments.length > 0 ? params.usedAttachments : undefined,
    conversation_id: params.conversationId,
    context_mode: params.assistantContextMode,
    current_browser_shell_direct_write_enabled: currentBrowserShellDirectWriteEnabled,
    current_browser_shell_session_id: currentBrowserShellSessionId,
    current_terminal_session_fingerprint: terminalBinding.currentTerminalSessionFingerprint,
    current_terminal_session_id: terminalBinding.currentTerminalSessionId,
    display_content: params.displayContent,
    document_attachments: params.usedDocuments.length > 0 ? params.usedDocuments : undefined,
    enable_rag: params.enableRag,
    enable_tenth_man_rule: enableTenthManRule,
    execution_id: executionId,
    force_tasks: params.forceTaskPlanContract,
    harness_max_continuations: params.harnessMaxContinuations,
    harness_mode: harnessMode,
    message_id: null,
    model_override: buildAssistantModelOverride(params.assistantSelectedModel),
    persist_messages: params.persistMessages,
    referenced_assets: params.usedAssets.length > 0 ? params.usedAssets : undefined,
    referenced_files: params.usedFiles.length > 0 ? params.usedFiles : undefined,
    referenced_messages: params.usedMessages.length > 0 ? params.usedMessages : undefined,
    referenced_traffic: params.usedTraffic.length > 0 ? params.usedTraffic : undefined,
    timeout_secs: 300,
    tool_config: params.runtimeToolConfig,
    working_directory: normalizeWorkingDirectory(params.workingDirectory) || undefined,
  }

  const parallelTargets = buildParallelModelTargets(params.assistantParallelSelectedModels || [])
  if (params.assistantExecutionMode === 'parallel') {
    if (parallelTargets.length < 2) {
      throw new Error('多模型并行执行至少需要选择两个模型。')
    }
    if (!params.runAgentExecuteParallel) {
      throw new Error('Parallel agent execution command is not available.')
    }
    const judgeModel = buildModelTarget(params.assistantParallelJudgeModel)
    return params.runAgentExecuteParallel({
      request: {
        task: params.fullTask,
        config: {
          ...config,
          model_override: undefined,
        },
        models: parallelTargets,
        aggregation_mode: judgeModel ? 'judge' : 'manual',
        judge_model: judgeModel || undefined,
      },
    })
  }

  return params.runAgentExecute({
    task: params.fullTask,
    config,
  })
}
