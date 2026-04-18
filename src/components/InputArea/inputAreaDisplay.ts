import type { TrafficSendType } from '@/components/Agent/agentDraftTypes'
import type { MentionTokenKind } from '@/components/InputArea/mentionTokenSupport'

export const getAssetRiskBadgeClass = (level?: string) => {
  switch (String(level || 'unknown').toLowerCase()) {
    case 'critical':
      return 'badge-error'
    case 'high':
      return 'badge-warning'
    case 'medium':
      return 'badge-info'
    case 'low':
      return 'badge-success'
    default:
      return 'badge-ghost'
  }
}

export const getMentionBadgeClass = (kind: MentionTokenKind) => {
  switch (kind) {
    case 'file':
      return 'badge-secondary'
    case 'asset':
      return 'badge-primary'
    case 'message':
      return 'badge-info'
    case 'traffic':
      return 'badge-accent'
    default:
      return 'badge-ghost'
  }
}

export const getMentionBadgeLabel = (kind: MentionTokenKind) => {
  switch (kind) {
    case 'file':
      return 'FILE'
    case 'asset':
      return 'ASSET'
    case 'message':
      return 'MSG'
    case 'traffic':
      return 'HTTP'
    default:
      return 'REF'
  }
}

export const getMentionInlineClass = (kind: MentionTokenKind) => {
  switch (kind) {
    case 'file':
      return 'mention-inline-file'
    case 'asset':
      return 'mention-inline-asset'
    case 'message':
      return 'mention-inline-message'
    case 'traffic':
      return 'mention-inline-traffic'
    default:
      return ''
  }
}

export const getMethodBadgeClass = (method: string): string => {
  switch (method?.toUpperCase()) {
    case 'GET':
      return 'badge-info'
    case 'POST':
      return 'badge-success'
    case 'PUT':
      return 'badge-warning'
    case 'DELETE':
      return 'badge-error'
    case 'PATCH':
      return 'badge-accent'
    default:
      return 'badge-ghost'
  }
}

export const getStatusBadgeClass = (status: number): string => {
  if (!status || status === 0) return 'badge-ghost'
  if (status >= 200 && status < 300) return 'badge-success'
  if (status >= 300 && status < 400) return 'badge-info'
  if (status >= 400 && status < 500) return 'badge-warning'
  if (status >= 500) return 'badge-error'
  return 'badge-ghost'
}

export const getUrlPath = (url: string): string => {
  try {
    const urlObj = new URL(url)
    const path = urlObj.pathname + urlObj.search
    return path.length > 30 ? path.substring(0, 30) + '...' : path
  } catch {
    return url.length > 30 ? url.substring(0, 30) + '...' : url
  }
}

export const getTypeBadgeClass = (type?: TrafficSendType): string => {
  switch (type) {
    case 'request':
      return 'badge-primary'
    case 'response':
      return 'badge-secondary'
    default:
      return 'badge-accent'
  }
}

export const getTypeLabel = (type?: TrafficSendType): string => {
  switch (type) {
    case 'request':
      return 'REQ'
    case 'response':
      return 'RES'
    default:
      return 'ALL'
  }
}

export const formatReferencedFileSize = (size: number): string => {
  if (!Number.isFinite(size) || size <= 0) return '0 B'
  if (size >= 1024 * 1024) return `${(size / (1024 * 1024)).toFixed(1)} MB`
  if (size >= 1024) return `${(size / 1024).toFixed(1)} KB`
  return `${size} B`
}
