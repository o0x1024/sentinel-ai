export type TrafficSendType = 'request' | 'response' | 'both'

export interface ReferencedTraffic {
  id: number
  url: string
  method: string
  host: string
  status_code: number
  mentionText?: string
  request_headers?: string
  request_body?: string
  response_headers?: string
  response_body?: string
  sendType?: TrafficSendType
}

export interface ReferencedAsset {
  id: string
  name: string
  value: string
  asset_type: string
  mentionText?: string
  risk_level?: string
  status?: string
  description?: string
  tags?: string[]
  metadata?: Record<string, any>
}

export interface ReferencedFile {
  id: string
  path: string
  relativePath: string
  mentionText?: string
  preview: string
  truncated: boolean
  size: number
}

export interface ReferencedConversationMessage {
  id: string
  content: string
  mentionText?: string
  roleLabel: string
  timestamp: number
  type: string
}
