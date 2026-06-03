import type { AgentTrackedArtifact } from '@/types/agent'

export interface StoredArtifactItem {
  slot: string
  path: string
  storage_backend: string
  size: number
  lines: number
}

export interface StoredArtifactView extends StoredArtifactItem {
  fullyRead: boolean
  nextCommand: string
  nextLabel: string
  progressLabel: string
  progressPercent: number | null
  sizeLabel: string
  lineLabel: string
}

const DEFAULT_READBACK_CHUNK_LINES = 200

export function extractStoredArtifacts(raw: unknown): StoredArtifactItem[] {
  const value = parseStructuredValue(raw)
  return extractStoredArtifactsFromValue(value)
}

export function buildStoredArtifactViews(
  raw: unknown,
  trackedRaw?: AgentTrackedArtifact[] | unknown,
): StoredArtifactView[] {
  const trackedArtifacts = normalizeTrackedArtifacts(trackedRaw)

  return extractStoredArtifacts(raw).map((artifact) => {
    const tracked = trackedArtifacts.find((item) => item.artifact_id === artifact.path)
    return {
      ...artifact,
      fullyRead: tracked?.fully_read === true,
      nextCommand: buildNextReadbackCommand(artifact, tracked),
      nextLabel: tracked?.fully_read === true
        ? 'Readback complete'
        : artifact.storage_backend === 'host'
          ? 'Next: file_read'
          : 'Next: shell',
      progressLabel: buildProgressLabel(artifact, tracked),
      progressPercent: buildProgressPercent(artifact, tracked),
      sizeLabel: formatBytes(artifact.size),
      lineLabel: artifact.lines > 0 ? `${artifact.lines} lines` : 'line count unavailable',
    }
  })
}

function parseStructuredValue(raw: unknown): unknown {
  if (typeof raw === 'string') {
    const trimmed = raw.trim()
    if (!trimmed) return trimmed
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

function extractStoredArtifactsFromValue(value: unknown): StoredArtifactItem[] {
  if (!value || typeof value !== 'object') {
    return []
  }

  const candidate = value as Record<string, unknown>
  const direct = normalizeStoredArtifacts(candidate.stored_artifacts)
  if (direct.length > 0) {
    return direct
  }

  if (candidate.output && typeof candidate.output === 'object') {
    return normalizeStoredArtifacts((candidate.output as Record<string, unknown>).stored_artifacts)
  }

  return []
}

function normalizeStoredArtifacts(value: unknown): StoredArtifactItem[] {
  if (!Array.isArray(value)) {
    return []
  }

  return value
    .map((item) => {
      if (!item || typeof item !== 'object') {
        return null
      }
      const record = item as Record<string, unknown>
      const path = typeof record.path === 'string' ? record.path : ''
      if (!path) {
        return null
      }
      return {
        slot: typeof record.slot === 'string' ? record.slot : 'artifact',
        path,
        storage_backend:
          typeof record.storage_backend === 'string' ? record.storage_backend : 'unknown',
        size: typeof record.size === 'number' ? record.size : 0,
        lines: typeof record.lines === 'number' ? record.lines : 0,
      } satisfies StoredArtifactItem
    })
    .filter((item): item is StoredArtifactItem => item !== null)
}

function normalizeTrackedArtifacts(raw: AgentTrackedArtifact[] | unknown): AgentTrackedArtifact[] {
  return Array.isArray(raw) ? raw as AgentTrackedArtifact[] : []
}

function buildNextReadbackCommand(
  artifact: StoredArtifactItem,
  tracked?: AgentTrackedArtifact,
): string {
  if (tracked?.fully_read) {
    return 'No further readback required'
  }

  const startLine = Math.max(1, Number(tracked?.contiguous_read_through_line || 0) + 1)
  const totalLines = Number(tracked?.total_lines || artifact.lines || 0)
  const remaining = totalLines > 0
    ? Math.max(1, totalLines - startLine + 1)
    : DEFAULT_READBACK_CHUNK_LINES
  const lineLimit = Math.min(remaining, DEFAULT_READBACK_CHUNK_LINES)

  if (artifact.storage_backend === 'host') {
    return `file_read { "file_path": "${artifact.path}", "offset": ${startLine}, "limit": ${lineLimit} }`
  }

  const endLine = startLine + lineLimit - 1
  return `sed -n '${startLine},${endLine}p' ${artifact.path}`
}

function buildProgressLabel(
  artifact: StoredArtifactItem,
  tracked?: AgentTrackedArtifact,
): string {
  if (!tracked) {
    return 'Unread'
  }
  if (tracked.fully_read) {
    return 'Sequential readback complete'
  }

  const covered = Number(tracked.contiguous_read_through_line || 0)
  const total = Number(tracked.total_lines || artifact.lines || 0)
  if (covered > 0 && total > 0) {
    return `${covered} / ${total} lines covered`
  }
  if (total > 0) {
    return `0 / ${total} lines covered`
  }
  if (Array.isArray(tracked.read_ranges) && tracked.read_ranges.length > 0) {
    return `${tracked.read_ranges.length} read range(s) recorded`
  }
  return 'Unread'
}

function buildProgressPercent(
  artifact: StoredArtifactItem,
  tracked?: AgentTrackedArtifact,
): number | null {
  if (!tracked) {
    return artifact.lines > 0 ? 0 : null
  }
  if (tracked.fully_read) {
    return 100
  }

  const total = Number(tracked.total_lines || artifact.lines || 0)
  if (total <= 0) {
    return null
  }

  const covered = Number(tracked.contiguous_read_through_line || 0)
  const percent = Math.max(0, Math.min(100, (covered / total) * 100))
  return Number(percent.toFixed(percent >= 10 ? 0 : 1))
}

function formatBytes(size: number): string {
  if (!Number.isFinite(size) || size <= 0) {
    return 'size unavailable'
  }
  if (size < 1024) {
    return `${size} B`
  }
  if (size < 1024 * 1024) {
    return `${(size / 1024).toFixed(1)} KB`
  }
  return `${(size / (1024 * 1024)).toFixed(1)} MB`
}
