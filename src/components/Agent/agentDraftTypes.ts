import type { PendingDocumentAttachment, ProcessedDocumentResult } from '@/types/agent'
import type {
  ReferencedConversationMessage,
  ReferencedAsset,
  ReferencedFile,
  ReferencedTraffic,
  TrafficSendType,
} from '@/types/agentReferences'
import type { UiToolConfigPayload } from './toolConfigRuntime'
import type { ModelVisionCapability } from '@/services/aiModelCapabilities'
export type {
  ReferencedConversationMessage,
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
  referencedMessages: ReferencedConversationMessage[]
  referencedTraffic: ReferencedTraffic[]
  referencedAssets: ReferencedAsset[]
}

export interface AssistantModelOption {
  value: string
  label: string
  description?: string
  supportsVision?: boolean
  visionCapability?: ModelVisionCapability
}

export type AssistantContextMode = 'claude-like' | 'codex-like' | 'sentinel-like'

export type AssistantRunMode = 'assistant' | 'team'

export interface AssistantSessionSettings {
  profileId: string
  contextMode: AssistantContextMode
  runMode: AssistantRunMode
  workingDirectoryOverride: string
  ragEnabled: boolean
  webSearchEnabled: boolean
  tenthManEnabled: boolean
  harnessMaxContinuations: number
}

export interface AssistantConversationBinding extends AssistantSessionSettings {
  schemaVersion: number
  browserShellDirectWriteEnabled?: boolean
  browserShellSessionId?: string | null
  selectedModel?: string | null
  toolsEnabled?: boolean
  toolConfig?: UiToolConfigPayload | null
}
