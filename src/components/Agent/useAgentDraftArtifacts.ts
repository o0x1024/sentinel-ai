import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { AgentMessage, PendingDocumentAttachment, ProcessedDocumentResult } from '@/types/agent'
import { restoreDraftArtifactsFromMessage } from './agentMessageReplaySupport'
import type { ReferencedAsset, ReferencedFile, ReferencedTraffic, TrafficSendType } from './agentDraftTypes'

export const useAgentDraftArtifacts = () => {
  const pendingAttachments = ref<any[]>([])
  const pendingDocuments = ref<PendingDocumentAttachment[]>([])
  const processedDocuments = ref<ProcessedDocumentResult[]>([])
  const referencedFiles = ref<ReferencedFile[]>([])
  const referencedTraffic = ref<ReferencedTraffic[]>([])
  const referencedAssets = ref<ReferencedAsset[]>([])

  const clearDraftArtifacts = () => {
    pendingAttachments.value = []
    pendingDocuments.value = []
    processedDocuments.value = []
    referencedFiles.value = []
    referencedTraffic.value = []
    referencedAssets.value = []
  }

  const handleAddAttachments = async (filePaths: string[]) => {
    if (!filePaths || filePaths.length === 0) return
    try {
      const attachments = await invoke<any[]>('upload_multiple_images', { filePaths })
      if (!attachments || attachments.length === 0) return
      pendingAttachments.value.push(...attachments)
      console.log('[useAgentDraftArtifacts] Uploaded', attachments.length, 'attachments')
    } catch (error) {
      console.error('[useAgentDraftArtifacts] Upload failed:', error)
    }
  }

  const handleRemoveAttachment = (index: number) => {
    if (index >= 0 && index < pendingAttachments.value.length) {
      pendingAttachments.value.splice(index, 1)
    }
  }

  const handleAddDocuments = (docs: PendingDocumentAttachment[]) => {
    pendingDocuments.value.push(...docs)
    console.log('[useAgentDraftArtifacts] Added', docs.length, 'document(s) for processing')
  }

  const handleRemoveDocument = (index: number) => {
    if (index < 0 || index >= pendingDocuments.value.length) return
    const removed = pendingDocuments.value.splice(index, 1)
    if (!removed[0]) return
    const processedIndex = processedDocuments.value.findIndex((item) => item.id === removed[0].id)
    if (processedIndex >= 0) {
      processedDocuments.value.splice(processedIndex, 1)
    }
  }

  const handleDocumentProcessed = (result: ProcessedDocumentResult) => {
    const existingIdx = processedDocuments.value.findIndex((item) => item.id === result.id)
    if (existingIdx >= 0) {
      processedDocuments.value[existingIdx] = result
    } else {
      processedDocuments.value.push(result)
    }

    const pendingIdx = pendingDocuments.value.findIndex((item) => item.id === result.id)
    if (pendingIdx >= 0) {
      pendingDocuments.value[pendingIdx].status = result.status
      pendingDocuments.value[pendingIdx].file_id = result.file_id
      pendingDocuments.value[pendingIdx].file_path = result.file_path
      pendingDocuments.value[pendingIdx].error_message = result.error_message
    }

    console.log('[useAgentDraftArtifacts] Document processed:', result.original_filename, result.file_id)
  }

  const restoreArtifactsFromMessage = (message: AgentMessage) => {
    const restored = restoreDraftArtifactsFromMessage(message)
    pendingAttachments.value = restored.pendingAttachments
    pendingDocuments.value = restored.pendingDocuments
    processedDocuments.value = restored.processedDocuments
    referencedFiles.value = restored.referencedFiles
    referencedTraffic.value = restored.referencedTraffic
    referencedAssets.value = restored.referencedAssets
  }

  const handleRemoveTraffic = (index: number) => {
    if (index >= 0 && index < referencedTraffic.value.length) {
      referencedTraffic.value.splice(index, 1)
    }
  }

  const handleRemoveFile = (index: number) => {
    if (index >= 0 && index < referencedFiles.value.length) {
      referencedFiles.value.splice(index, 1)
    }
  }

  const handleClearTraffic = () => {
    referencedTraffic.value = []
  }

  const handleClearFiles = () => {
    referencedFiles.value = []
  }

  const handleRemoveAsset = (index: number) => {
    if (index >= 0 && index < referencedAssets.value.length) {
      referencedAssets.value.splice(index, 1)
    }
  }

  const handleClearAssets = () => {
    referencedAssets.value = []
  }

  const addReferencedTraffic = (traffic: ReferencedTraffic[], type: TrafficSendType = 'both') => {
    const existingIds = new Set(referencedTraffic.value.map((item) => item.id))
    const newTraffic = traffic
      .filter((item) => !existingIds.has(item.id))
      .map((item) => ({ ...item, sendType: type }))
    referencedTraffic.value.push(...newTraffic)
  }

  const addReferencedAssets = (assets: ReferencedAsset[]) => {
    const existingIds = new Set(referencedAssets.value.map((item) => item.id))
    const newAssets = assets.filter((item) => item?.id && !existingIds.has(item.id))
    referencedAssets.value.push(...newAssets)
  }

  const addReferencedFiles = (files: ReferencedFile[]) => {
    const existingIds = new Set(referencedFiles.value.map((item) => item.id))
    const newFiles = files.filter((item) => item?.id && !existingIds.has(item.id))
    referencedFiles.value.push(...newFiles)
  }

  const syncReferencedTraffic = (traffic: ReferencedTraffic[]) => {
    referencedTraffic.value = [...traffic]
  }

  const syncReferencedAssets = (assets: ReferencedAsset[]) => {
    referencedAssets.value = [...assets]
  }

  const syncReferencedFiles = (files: ReferencedFile[]) => {
    referencedFiles.value = [...files]
  }

  return {
    addReferencedAssets,
    addReferencedFiles,
    addReferencedTraffic,
    clearDraftArtifacts,
    handleAddAttachments,
    handleAddDocuments,
    handleClearAssets,
    handleClearFiles,
    handleClearTraffic,
    handleDocumentProcessed,
    handleRemoveAsset,
    handleRemoveAttachment,
    handleRemoveDocument,
    handleRemoveFile,
    handleRemoveTraffic,
    pendingAttachments,
    pendingDocuments,
    processedDocuments,
    referencedAssets,
    referencedFiles,
    referencedTraffic,
    restoreArtifactsFromMessage,
    syncReferencedAssets,
    syncReferencedFiles,
    syncReferencedTraffic,
  }
}
