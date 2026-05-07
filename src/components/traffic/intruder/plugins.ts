import { invoke } from '@tauri-apps/api/core'
import { buildSourceRequestFromRawRequest } from './http'
import type {
  IntruderPayloadSet,
  IntruderPluginProcessorBinding,
  IntruderPluginProcessorCategory,
  IntruderPosition,
  IntruderTarget,
} from './types'

interface CommandResponse<T> {
  success: boolean
  data?: T
  error?: string
}

export interface IntruderPluginSummary {
  id: string
  name: string
  category: string
  description?: string
}

interface IntruderPayloadGenerationResult {
  payloads: string[]
  output?: unknown
}

interface IntruderPayloadProcessorResult {
  payload?: string | null
  skipped: boolean
  output?: unknown
}

interface IntruderRequestProcessorResult {
  raw_request: string
  output?: unknown
}

export interface IntruderRequestProcessorTrace {
  pluginId: string
  requestText: string
  output?: unknown
}

export async function listIntruderPlugins(
  category: 'payload_generator' | IntruderPluginProcessorCategory,
): Promise<IntruderPluginSummary[]> {
  const response = await invoke<CommandResponse<IntruderPluginSummary[]>>('intruder_list_plugins', {
    category,
  })

  if (!response.success) {
    throw new Error(response.error || 'Failed to list Intruder plugins')
  }

  return response.data || []
}

export async function listIntruderPayloadGeneratorPlugins(): Promise<IntruderPluginSummary[]> {
  return listIntruderPlugins('payload_generator')
}

export async function getIntruderPluginInputSchema(pluginId: string): Promise<Record<string, any> | null> {
  const response = await invoke<CommandResponse<Record<string, any>>>('get_plugin_input_schema', { pluginId })

  if (!response.success) {
    throw new Error(response.error || `Failed to load schema for plugin ${pluginId}`)
  }

  return response.data || null
}

export function parseIntruderPluginConfig(configText: string): Record<string, any> {
  const trimmed = configText.trim()
  if (!trimmed) return {}

  const parsed = JSON.parse(trimmed)
  if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) {
    throw new Error('Plugin configuration must be a JSON object')
  }

  return parsed as Record<string, any>
}

export async function generateIntruderPluginPayloads(options: {
  requestText: string
  target: IntruderTarget
  positions: IntruderPosition[]
  payloadSet: IntruderPayloadSet
}): Promise<string[]> {
  const { requestText, target, positions, payloadSet } = options
  if (!payloadSet.pluginId.trim()) {
    throw new Error('Select an Intruder payload plugin first')
  }

  const response = await invoke<CommandResponse<IntruderPayloadGenerationResult>>('intruder_generate_payloads', {
    pluginId: payloadSet.pluginId,
    input: {
      request: buildSourceRequestFromRawRequest(requestText, target),
      rawRequest: requestText,
      target,
      positions,
      payloadSet: {
        id: payloadSet.id,
        name: payloadSet.name,
        payloadType: payloadSet.payloadType,
      },
      config: parseIntruderPluginConfig(payloadSet.pluginConfig),
      options: {},
    },
  })

  if (!response.success || !response.data) {
    throw new Error(response.error || `Plugin ${payloadSet.pluginId} did not return payloads`)
  }

  return response.data.payloads || []
}

export async function processIntruderPayloadWithPlugin(options: {
  requestText: string
  target: IntruderTarget
  positions: IntruderPosition[]
  binding: IntruderPluginProcessorBinding
  payload: string
  originalPayload: string
  baseValue: string
  positionIndex: number
}): Promise<string | null> {
  const { requestText, target, positions, binding, payload, originalPayload, baseValue, positionIndex } = options
  if (!binding.pluginId.trim()) {
    throw new Error('Select an Intruder payload processor first')
  }

  const response = await invoke<CommandResponse<IntruderPayloadProcessorResult>>('intruder_process_payload', {
    pluginId: binding.pluginId,
    input: {
      request: buildSourceRequestFromRawRequest(requestText, target),
      rawRequest: requestText,
      target,
      positions,
      payload,
      originalPayload,
      baseValue,
      positionIndex,
      config: parseIntruderPluginConfig(binding.config),
    },
  })

  if (!response.success || !response.data) {
    throw new Error(response.error || `Plugin ${binding.pluginId} failed to process payload`)
  }

  return response.data.skipped ? null : response.data.payload ?? payload
}

export async function transformIntruderRequestWithPlugin(options: {
  requestText: string
  target: IntruderTarget
  positions: IntruderPosition[]
  binding: IntruderPluginProcessorBinding
  payloadValues: string[]
  payloadSummary: string
  requestIndex: number
}): Promise<IntruderRequestProcessorTrace> {
  const { requestText, target, positions, binding, payloadValues, payloadSummary, requestIndex } = options
  if (!binding.pluginId.trim()) {
    throw new Error('Select an Intruder request processor first')
  }

  const response = await invoke<CommandResponse<IntruderRequestProcessorResult>>('intruder_transform_request', {
    pluginId: binding.pluginId,
    input: {
      request: buildSourceRequestFromRawRequest(requestText, target),
      rawRequest: requestText,
      target,
      positions,
      payloadValues,
      payloadSummary,
      requestIndex,
      config: parseIntruderPluginConfig(binding.config),
    },
  })

  if (!response.success || !response.data?.raw_request) {
    throw new Error(response.error || `Plugin ${binding.pluginId} failed to transform request`)
  }

  return {
    pluginId: binding.pluginId,
    requestText: response.data.raw_request,
    output: response.data.output,
  }
}
