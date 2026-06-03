import type { ProcessedDocumentResult } from '@/types/agent'

export interface SubmissionReferencedTraffic {
  db_request_id?: number | null
  host: string
  id?: number
  method: string
  request_body?: string
  request_headers?: string
  response_body?: string
  response_body_available?: boolean
  response_body_preview?: string
  response_body_truncated?: boolean
  response_content_type?: string
  response_headers?: string
  response_size?: number
  response_time?: number
  sendType?: 'request' | 'response' | 'both'
  status_code: number
  url: string
}

export interface SubmissionReferencedAsset {
  asset_type: string
  description?: string
  id: string
  metadata?: Record<string, any>
  name: string
  risk_level?: string
  status?: string
  tags?: string[]
  value: string
}

export interface SubmissionReferencedFile {
  path: string
  preview: string
  relativePath: string
  size: number
  truncated: boolean
}

export interface SubmissionReferencedMessage {
  content: string
  roleLabel: string
  timestamp: number
  type: string
}

export interface SubmissionResourceSnapshot<TAttachment, TTraffic, TAsset, TFile, TMessage> {
  usedAssets: TAsset[]
  usedAttachments: TAttachment[]
  usedDocuments: ProcessedDocumentResult[]
  usedFiles: TFile[]
  usedMessages: TMessage[]
  usedTraffic: TTraffic[]
}

export interface PreparedSubmission<TAttachment, TTraffic, TAsset, TFile, TMessage>
  extends SubmissionResourceSnapshot<TAttachment, TTraffic, TAsset, TFile, TMessage> {
  displayContent?: string
  fullTask: string
}

export const collectSubmissionResourceSnapshot = <TAttachment, TTraffic, TAsset, TFile, TMessage>(params: {
  pendingAttachments: TAttachment[]
  processedDocuments: ProcessedDocumentResult[]
  referencedAssets: TAsset[]
  referencedFiles: TFile[]
  referencedMessages: TMessage[]
  referencedTraffic: TTraffic[]
}): SubmissionResourceSnapshot<TAttachment, TTraffic, TAsset, TFile, TMessage> => ({
  usedAssets: [...params.referencedAssets],
  usedAttachments: [...params.pendingAttachments],
  usedDocuments: params.processedDocuments.filter((doc) => doc.status === 'ready'),
  usedFiles: [...params.referencedFiles],
  usedMessages: [...params.referencedMessages],
  usedTraffic: [...params.referencedTraffic],
})

export const buildTrafficContext = (traffic: SubmissionReferencedTraffic[]): string => {
  const parts: string[] = ['Referenced HTTP traffic:\n']

  traffic.forEach((entry, index) => {
    const sendType = entry.sendType || 'both'
    const typeLabel = sendType === 'request'
      ? 'Request'
      : sendType === 'response'
        ? 'Response'
        : 'Traffic'
    parts.push(`\n--- ${typeLabel} #${index + 1} ---`)
    parts.push(`URL: ${entry.url}`)
    parts.push(`Replay: use http_request with referenced_traffic_index=${index + 1}${typeof entry.id === 'number' ? ` or referenced_traffic_id=${entry.id}` : ''} to preserve captured request headers including Cookie. Use traffic_response_read with the same reference to read the full captured response body in chunks.`)
    parts.push(`Method: ${entry.method}`)
    parts.push(`Host: ${entry.host}`)

    const showRequest = sendType === 'request' || sendType === 'both'
    const showResponse = sendType === 'response' || sendType === 'both'

    if (showResponse) {
      parts.push(`Status: ${entry.status_code || 'N/A'}`)
    }
    if (!showResponse && typeof entry.status_code === 'number') {
      parts.push(`Response Status: ${entry.status_code || 'N/A'}`)
    }
    if (entry.response_content_type) {
      parts.push(`Response Content-Type: ${entry.response_content_type}`)
    }
    if (typeof entry.response_size === 'number') {
      parts.push(`Response Size: ${entry.response_size} bytes`)
    }
    if (typeof entry.response_time === 'number') {
      parts.push(`Response Time: ${entry.response_time} ms`)
    }

    if (showRequest && entry.request_headers) {
      try {
        const headers = JSON.parse(entry.request_headers)
        const headerStr = Object.entries(headers)
          .map(([key, value]) => `  ${key}: ${value}`)
          .join('\n')
        parts.push(`\nRequest Headers:\n${headerStr}`)
      } catch {
        parts.push(`\nRequest Headers: ${entry.request_headers}`)
      }
    }

    if (showRequest && entry.request_body) {
      const body = entry.request_body.length > 2000
        ? `${entry.request_body.substring(0, 2000)}... [truncated]`
        : entry.request_body
      parts.push(`\nRequest Body:\n${body}`)
    }

    if (entry.response_headers) {
      try {
        const headers = JSON.parse(entry.response_headers)
        const headerStr = Object.entries(headers)
          .map(([key, value]) => `  ${key}: ${value}`)
          .join('\n')
        parts.push(`\nResponse Headers${showResponse ? '' : ' Summary'}:\n${headerStr}`)
      } catch {
        parts.push(`\nResponse Headers${showResponse ? '' : ' Summary'}: ${entry.response_headers}`)
      }
    }

    const responsePreview = entry.response_body_preview || (showResponse ? entry.response_body : undefined)
    if (responsePreview) {
      const body = responsePreview.length > 3000
        ? `${responsePreview.substring(0, 3000)}... [truncated]`
        : responsePreview
      const label = showResponse ? 'Response Body Preview' : 'Response Preview'
      parts.push(`\n${label}${entry.response_body_truncated ? ' [truncated]' : ''}:\n${body}`)
    } else if (entry.response_body_available === false) {
      parts.push('\nResponse Preview: [not available in referenced context]')
    }
  })

  return parts.join('\n')
}

export const buildAssetContext = (assets: SubmissionReferencedAsset[]): string => {
  const parts: string[] = ['Referenced assets:\n']

  assets.forEach((asset, index) => {
    parts.push(`\n--- Asset #${index + 1} ---`)
    parts.push(`ID: ${asset.id}`)
    parts.push(`Name: ${asset.name}`)
    parts.push(`Type: ${asset.asset_type}`)
    parts.push(`Value: ${asset.value}`)
    if (asset.risk_level) parts.push(`Risk: ${asset.risk_level}`)
    if (asset.status) parts.push(`Status: ${asset.status}`)
    if (asset.description) parts.push(`Description: ${asset.description}`)
    if (Array.isArray(asset.tags) && asset.tags.length > 0) {
      parts.push(`Tags: ${asset.tags.join(', ')}`)
    }
    if (asset.metadata && Object.keys(asset.metadata).length > 0) {
      parts.push(`Metadata: ${JSON.stringify(asset.metadata)}`)
    }
  })

  return parts.join('\n')
}

export const buildFileContext = (files: SubmissionReferencedFile[]): string => {
  const parts: string[] = ['Referenced files:\n']

  files.forEach((file, index) => {
    parts.push(`\n--- File #${index + 1} ---`)
    parts.push(`Path: ${file.relativePath}`)
    parts.push(`Absolute Path: ${file.path}`)
    parts.push(`Size: ${file.size} bytes`)
    if (file.truncated) {
      parts.push('Preview: [truncated]')
    }
    parts.push(`\nContent Preview:\n${file.preview}`)
  })

  return parts.join('\n')
}

export const buildMessageContext = (messages: SubmissionReferencedMessage[]): string => {
  const parts: string[] = ['Referenced conversation messages:\n']

  messages.forEach((message, index) => {
    parts.push(`\n--- Message #${index + 1} ---`)
    parts.push(`Role: ${message.roleLabel}`)
    parts.push(`Type: ${message.type}`)
    parts.push(`Timestamp: ${new Date(message.timestamp).toISOString()}`)
    parts.push(`Content:\n${message.content}`)
  })

  return parts.join('\n')
}

export const buildSubmissionTask = (params: {
  assets: SubmissionReferencedAsset[]
  files: SubmissionReferencedFile[]
  messages: SubmissionReferencedMessage[]
  task: string
  traffic: SubmissionReferencedTraffic[]
}): { displayContent?: string; fullTask: string } => {
  let fullTask = params.task
  let displayContent: string | undefined

  if (params.traffic.length > 0) {
    fullTask = `${buildTrafficContext(params.traffic)}\n\nUser task: ${params.task}`
    displayContent = params.task
  }

  if (params.assets.length > 0) {
    const assetContext = buildAssetContext(params.assets)
    fullTask = fullTask === params.task
      ? `${assetContext}\n\nUser task: ${params.task}`
      : `${assetContext}\n\n${fullTask}`
    displayContent = params.task
  }

  if (params.files.length > 0) {
    const fileContext = buildFileContext(params.files)
    fullTask = fullTask === params.task
      ? `${fileContext}\n\nUser task: ${params.task}`
      : `${fileContext}\n\n${fullTask}`
    displayContent = params.task
  }

  if (params.messages.length > 0) {
    const messageContext = buildMessageContext(params.messages)
    fullTask = fullTask === params.task
      ? `${messageContext}\n\nUser task: ${params.task}`
      : `${messageContext}\n\n${fullTask}`
    displayContent = params.task
  }

  return { displayContent, fullTask }
}

export const prepareSubmission = <TAttachment, TTraffic, TAsset, TFile, TMessage>(params: {
  clearDraftState: () => void
  pendingAttachments: TAttachment[]
  processedDocuments: ProcessedDocumentResult[]
  referencedAssets: TAsset[]
  referencedFiles: TFile[]
  referencedMessages: TMessage[]
  referencedTraffic: TTraffic[]
  setPendingDocumentAttachments: (documents: ProcessedDocumentResult[]) => void
  task: string
  toAssetContextItems: (assets: TAsset[]) => SubmissionReferencedAsset[]
  toFileContextItems: (files: TFile[]) => SubmissionReferencedFile[]
  toMessageContextItems: (messages: TMessage[]) => SubmissionReferencedMessage[]
  toTrafficContextItems: (traffic: TTraffic[]) => SubmissionReferencedTraffic[]
}): PreparedSubmission<TAttachment, TTraffic, TAsset, TFile, TMessage> => {
  const usedResources = collectSubmissionResourceSnapshot({
    pendingAttachments: params.pendingAttachments,
    processedDocuments: params.processedDocuments,
    referencedAssets: params.referencedAssets,
    referencedFiles: params.referencedFiles,
    referencedMessages: params.referencedMessages,
    referencedTraffic: params.referencedTraffic,
  })
  const { displayContent, fullTask } = buildSubmissionTask({
    assets: params.toAssetContextItems(params.referencedAssets),
    files: params.toFileContextItems(params.referencedFiles),
    messages: params.toMessageContextItems(params.referencedMessages),
    task: params.task,
    traffic: params.toTrafficContextItems(params.referencedTraffic),
  })

  params.clearDraftState()
  if (usedResources.usedDocuments.length > 0) {
    params.setPendingDocumentAttachments(usedResources.usedDocuments)
  }

  return {
    ...usedResources,
    displayContent,
    fullTask,
  }
}
