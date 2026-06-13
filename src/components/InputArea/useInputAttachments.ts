import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
import type { UnlistenFn } from '@tauri-apps/api/event'
import type { PendingDocumentAttachment, ProcessedDocumentResult } from '@/types/agent'
import { dialog } from '@/composables/useDialog'

export const useInputAttachments = (params: {
  conversationId: () => string | null
  isActive?: () => boolean
  emitAddAttachments: (files: string[]) => void
  emitAddDocuments: (files: PendingDocumentAttachment[]) => void
  emitDocumentProcessed: (result: ProcessedDocumentResult) => void
  emitRemoveAttachment: (index: number) => void
  emitRemoveDocument: (index: number) => void
}) => {
  const fileInputRef = ref<HTMLInputElement | null>(null)
  const isDragOver = ref(false)
  let unlistenDragDrop: UnlistenFn | null = null

  const toMimeType = (mediaType?: string): string => {
    if (!mediaType) return 'image/jpeg'
    const normalized = mediaType.toLowerCase()
    if (normalized === 'jpeg' || normalized === 'jpg') return 'image/jpeg'
    if (normalized === 'png') return 'image/png'
    if (normalized === 'gif') return 'image/gif'
    if (normalized === 'webp') return 'image/webp'
    return normalized.startsWith('image/') ? normalized : `image/${normalized}`
  }

  const getAttachmentPreview = (attachment: any): string => {
    try {
      const image = attachment.image ?? attachment
      const mediaTypeRaw: string | undefined = image?.media_type
      const mime = toMimeType(mediaTypeRaw)
      const dataField = image?.data
      const base64 = typeof dataField === 'string' ? dataField : dataField?.data
      if (!base64) return ''
      return `data:${mime};base64,${base64}`
    } catch (error) {
      console.error('[useInputAttachments] Failed to build attachment preview:', error, attachment)
      return ''
    }
  }

  const formatFileSize = (bytes: number): string => {
    if (bytes < 1024) return `${bytes} B`
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
  }

  const removeAttachment = (index: number) => {
    params.emitRemoveAttachment(index)
  }

  const removeDocument = (index: number) => {
    params.emitRemoveDocument(index)
  }

  const getMimeTypeFromExt = (ext: string): string => {
    const mimeMap: Record<string, string> = {
      docx: 'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
      doc: 'application/msword',
      xlsx: 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet',
      xls: 'application/vnd.ms-excel',
      pptx: 'application/vnd.openxmlformats-officedocument.presentationml.presentation',
      ppt: 'application/vnd.ms-powerpoint',
      pdf: 'application/pdf',
      rtf: 'application/rtf',
      eml: 'message/rfc822',
      msg: 'application/vnd.ms-outlook',
      txt: 'text/plain',
      md: 'text/markdown',
      json: 'application/json',
      xml: 'application/xml',
      csv: 'text/csv',
      html: 'text/html',
      htm: 'text/html',
      css: 'text/css',
      js: 'text/javascript',
      ts: 'text/typescript',
      jsx: 'text/javascript',
      tsx: 'text/typescript',
      py: 'text/x-python',
      java: 'text/x-java',
      c: 'text/x-c',
      cpp: 'text/x-c++',
      h: 'text/x-c',
      hpp: 'text/x-c++',
      rs: 'text/x-rust',
      go: 'text/x-go',
      rb: 'text/x-ruby',
      php: 'text/x-php',
      sh: 'text/x-shellscript',
      bash: 'text/x-shellscript',
      zsh: 'text/x-shellscript',
      sql: 'text/x-sql',
      yaml: 'text/yaml',
      yml: 'text/yaml',
      toml: 'text/x-toml',
      ini: 'text/x-ini',
      conf: 'text/plain',
      cfg: 'text/plain',
      log: 'text/plain',
      zip: 'application/zip',
      tar: 'application/x-tar',
      gz: 'application/gzip',
      rar: 'application/vnd.rar',
      '7z': 'application/x-7z-compressed',
    }
    return mimeMap[ext] || 'text/plain'
  }

  const processDroppedFiles = async (paths: string[]) => {
    const imageFiles: string[] = []

    for (const filePath of paths) {
      const fileName = filePath.split('/').pop() || filePath.split('\\').pop() || 'unknown'
      const ext = fileName.split('.').pop()?.toLowerCase() || ''

      if (['jpg', 'jpeg', 'png', 'gif', 'webp'].includes(ext)) {
        imageFiles.push(filePath)
        continue
      }

      let fileSize = 0
      try {
        const stat = await invoke<{ size: number; is_file: boolean }>('get_file_stat', { path: filePath })
        if (stat.is_file === false) {
          continue
        }
        fileSize = stat.size
      } catch {
        console.warn('[useInputAttachments] Could not get file size for:', filePath)
      }

      const queuedId = crypto.randomUUID()
      const queuedDoc: PendingDocumentAttachment = {
        id: queuedId,
        original_path: filePath,
        original_filename: fileName,
        file_size: fileSize,
        mime_type: getMimeTypeFromExt(ext),
        status: 'processing',
      }
      params.emitAddDocuments([queuedDoc])

      try {
        const uploaded = await invoke<ProcessedDocumentResult>('upload_document_attachment', {
          filePath,
          clientId: queuedId,
          conversationId: params.conversationId(),
        })
        params.emitDocumentProcessed(uploaded)
      } catch (error) {
        console.error('[useInputAttachments] Failed to upload document:', error)
        params.emitDocumentProcessed({
          id: queuedId,
          file_id: queuedId,
          original_filename: fileName,
          file_size: fileSize,
          mime_type: getMimeTypeFromExt(ext),
          status: 'failed',
          error_message: String(error),
        } as ProcessedDocumentResult)
      }
    }

    if (imageFiles.length > 0) {
      params.emitAddAttachments(imageFiles)
    }
  }

  const triggerFileSelect = async () => {
    try {
      const { open } = await import('@tauri-apps/plugin-dialog')
      const selected = await open({ multiple: true })
      if (!selected) return
      const filePaths = Array.isArray(selected) ? selected : [selected]
      await processDroppedFiles(filePaths)
    } catch (error) {
      console.error('[useInputAttachments] Tauri 文件选择失败:', error)
    }
  }

  const onFilesSelected = (event: Event) => {
    const input = event.target as HTMLInputElement
    if (!input.files || input.files.length === 0) return
    console.warn('[useInputAttachments] 收到 File 对象，当前默认按 Tauri 模式运行，建议通过对话框选择文件')
    input.value = ''
  }

  const onDragOver = (event: DragEvent) => {
    if (params.isActive && !params.isActive()) return
    if (event.dataTransfer?.types.includes('Files')) {
      isDragOver.value = true
    }
  }

  const onDragLeave = () => {
    isDragOver.value = false
  }

  const onDrop = async (_event: DragEvent) => {
    isDragOver.value = false
  }

  const setupNativeDragDrop = async () => {
    if (unlistenDragDrop) {
      return
    }
    try {
      const webview = getCurrentWebviewWindow()
      unlistenDragDrop = await webview.onDragDropEvent(async (event) => {
        if (params.isActive && !params.isActive()) {
          isDragOver.value = false
          return
        }
        const payload = event.payload as any
        const paths = Array.isArray(payload?.paths) ? payload.paths : []
        const hasFilePaths = paths.length > 0

        if (event.payload.type === 'over' || event.payload.type === 'enter') {
          if (hasFilePaths) {
            isDragOver.value = true
          }
          return
        }

        if (event.payload.type === 'leave') {
          isDragOver.value = false
          return
        }

        if (event.payload.type === 'drop') {
          isDragOver.value = false
          if (paths.length > 0) {
            await processDroppedFiles(paths)
          }
        }
      })
    } catch (error) {
      console.error('[useInputAttachments] Failed to setup Tauri drag-drop:', error)
    }
  }

  const teardownNativeDragDrop = () => {
    if (unlistenDragDrop) {
      unlistenDragDrop()
      unlistenDragDrop = null
    }
  }

  const retryUploadDocument = async (document: PendingDocumentAttachment) => {
    try {
      const uploaded = await invoke<ProcessedDocumentResult>('upload_document_attachment', {
        filePath: document.original_path,
        clientId: document.id,
        conversationId: params.conversationId(),
      })
      params.emitDocumentProcessed(uploaded)
    } catch (error) {
      console.error('[useInputAttachments] Retry upload document failed:', error)
    }
  }

  const runSecurityAnalysis = async (document: PendingDocumentAttachment) => {
    const fileId = document.file_id || document.id
    if (!fileId) return
    try {
      const message = await invoke<string>('run_file_security_analysis', { fileId })
      dialog.toast.success(message)
    } catch (error) {
      dialog.toast.error(String(error))
    }
  }

  return {
    fileInputRef,
    formatFileSize,
    getAttachmentPreview,
    isDragOver,
    onDragLeave,
    onDragOver,
    onDrop,
    onFilesSelected,
    removeAttachment,
    removeDocument,
    retryUploadDocument,
    runSecurityAnalysis,
    setupNativeDragDrop,
    teardownNativeDragDrop,
    triggerFileSelect,
  }
}
