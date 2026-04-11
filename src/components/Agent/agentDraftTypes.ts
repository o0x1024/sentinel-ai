import type { PendingDocumentAttachment, ProcessedDocumentResult } from '@/types/agent'
import type {
  ReferencedAsset,
  ReferencedFile,
  ReferencedTraffic,
  TrafficSendType,
} from '@/types/agentReferences'
export type {
  ReferencedAsset,
  ReferencedFile,
  ReferencedTraffic,
  TrafficSendType,
} from '@/types/agentReferences'

export interface DraftArtifactState {
  pendingAttachments: any[]
  pendingDocuments: PendingDocumentAttachment[]
  processedDocuments: ProcessedDocumentResult[]
  referencedFiles: ReferencedFile[]
  referencedTraffic: ReferencedTraffic[]
  referencedAssets: ReferencedAsset[]
}

export interface AssistantModelOption {
  value: string
  label: string
  description?: string
}

export type AssistantContextMode = 'claude-like' | 'codex-like'

export type AssistantRunMode = 'assistant' | 'team'

export interface AssistantSessionSettings {
  profileId: string
  contextMode: AssistantContextMode
  runMode: AssistantRunMode
  ragEnabled: boolean
  webSearchEnabled: boolean
  tenthManEnabled: boolean
}

export interface AssistantConversationBinding extends AssistantSessionSettings {
  schemaVersion: number
  selectedModel?: string | null
  toolsEnabled?: boolean
}
