import type { PendingDocumentAttachment, ProcessedDocumentResult } from '@/types/agent'

export type TrafficSendType = 'request' | 'response' | 'both'

export interface ReferencedTraffic {
  id: number
  url: string
  method: string
  host: string
  status_code: number
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
  risk_level?: string
  status?: string
  description?: string
  tags?: string[]
  metadata?: Record<string, any>
}

export interface DraftArtifactState {
  pendingAttachments: any[]
  pendingDocuments: PendingDocumentAttachment[]
  processedDocuments: ProcessedDocumentResult[]
  referencedTraffic: ReferencedTraffic[]
  referencedAssets: ReferencedAsset[]
}

export interface AssistantModelOption {
  value: string
  label: string
  description?: string
}
