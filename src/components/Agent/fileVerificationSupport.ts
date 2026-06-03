import type { AgentMessage, MessageMetadata } from '@/types/agent'

const FILE_MUTATION_TOOLS = new Set(['file_edit', 'file_write'])

export function applyFileVerificationStatuses(messages: AgentMessage[]): AgentMessage[] {
  const verifiedReads = new Set<string>()

  for (let index = messages.length - 1; index >= 0; index -= 1) {
    const message = messages[index]
    const metadata = message.metadata
    if (!metadata) continue

    const toolName = String(metadata.tool_name || '').trim().toLowerCase()
    if (!toolName) continue

    if (toolName === 'file_read') {
      const readRecord = extractFileRecord(message)
      if (readRecord) {
        verifiedReads.add(buildVerificationKey(readRecord.path, readRecord.contentHash))
      }
      continue
    }

    if (!FILE_MUTATION_TOOLS.has(toolName)) continue

    if (metadata.status === 'failed' || metadata.status === 'cancelled' || metadata.error) {
      metadata.file_verification_status = 'failed'
      continue
    }

    const fileRecord = extractFileRecord(message)
    if (!fileRecord) {
      delete metadata.file_verification_status
      continue
    }

    metadata.file_verification_status = verifiedReads.has(
      buildVerificationKey(fileRecord.path, fileRecord.contentHash),
    )
      ? 'verified'
      : 'pending'
  }

  return messages
}

function extractFileRecord(
  message: AgentMessage,
): { path: string; contentHash: string } | null {
  const metadata = message.metadata
  if (!metadata) return null

  const payload = parseToolResultPayload(message)
  if (!payload || typeof payload !== 'object') return null

  const record = payload as Record<string, unknown>
  const contentHash =
    typeof record.content_hash === 'string' ? record.content_hash.trim() : ''
  if (!contentHash) return null

  const path =
    extractStoredArtifactPath(record) ||
    (typeof record.file_path === 'string' ? record.file_path.trim() : '')
  if (!path) return null

  return { path, contentHash }
}

function parseToolResultPayload(message: AgentMessage): unknown {
  const metadata = message.metadata as MessageMetadata | undefined
  const raw =
    metadata?.tool_result ??
    (message.type === 'tool_result' ? message.content : '')
  return parseStructuredValue(raw)
}

function parseStructuredValue(raw: unknown): unknown {
  if (typeof raw === 'string') {
    const trimmed = raw.trim()
    if (!trimmed) return null
    try {
      return JSON.parse(trimmed)
    } catch {
      return trimmed
    }
  }

  if (Array.isArray(raw)) {
    const textPayload = raw
      .filter((item) => item && typeof item === 'object' && (item as Record<string, unknown>).type === 'text')
      .map((item) => (item as Record<string, unknown>).text)
      .find((value) => typeof value === 'string')
    if (typeof textPayload === 'string') {
      return parseStructuredValue(textPayload)
    }
  }

  return raw
}

function extractStoredArtifactPath(record: Record<string, unknown>): string {
  const artifacts = record.stored_artifacts
  if (!Array.isArray(artifacts)) return ''
  const first = artifacts.find((item) => item && typeof item === 'object') as
    | Record<string, unknown>
    | undefined
  return typeof first?.path === 'string' ? first.path.trim() : ''
}

function buildVerificationKey(path: string, contentHash: string): string {
  return `${path}::${contentHash}`
}
