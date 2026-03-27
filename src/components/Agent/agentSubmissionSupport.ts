import type { ProcessedDocumentResult } from '@/types/agent'

export interface SubmissionReferencedTraffic {
  host: string
  method: string
  request_body?: string
  request_headers?: string
  response_body?: string
  response_headers?: string
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

export interface SubmissionResourceSnapshot<TAttachment, TTraffic, TAsset> {
  usedAssets: TAsset[]
  usedAttachments: TAttachment[]
  usedDocuments: ProcessedDocumentResult[]
  usedTraffic: TTraffic[]
}

export interface PreparedSubmission<TAttachment, TTraffic, TAsset>
  extends SubmissionResourceSnapshot<TAttachment, TTraffic, TAsset> {
  displayContent?: string
  fullTask: string
}

export const collectSubmissionResourceSnapshot = <TAttachment, TTraffic, TAsset>(params: {
  pendingAttachments: TAttachment[]
  processedDocuments: ProcessedDocumentResult[]
  referencedAssets: TAsset[]
  referencedTraffic: TTraffic[]
}): SubmissionResourceSnapshot<TAttachment, TTraffic, TAsset> => ({
  usedAssets: [...params.referencedAssets],
  usedAttachments: [...params.pendingAttachments],
  usedDocuments: params.processedDocuments.filter((doc) => doc.status === 'ready'),
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
    parts.push(`Method: ${entry.method}`)
    parts.push(`Host: ${entry.host}`)

    const showRequest = sendType === 'request' || sendType === 'both'
    const showResponse = sendType === 'response' || sendType === 'both'

    if (showResponse) {
      parts.push(`Status: ${entry.status_code || 'N/A'}`)
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

    if (showResponse && entry.response_headers) {
      try {
        const headers = JSON.parse(entry.response_headers)
        const headerStr = Object.entries(headers)
          .map(([key, value]) => `  ${key}: ${value}`)
          .join('\n')
        parts.push(`\nResponse Headers:\n${headerStr}`)
      } catch {
        parts.push(`\nResponse Headers: ${entry.response_headers}`)
      }
    }

    if (showResponse && entry.response_body) {
      const body = entry.response_body.length > 3000
        ? `${entry.response_body.substring(0, 3000)}... [truncated]`
        : entry.response_body
      parts.push(`\nResponse Body:\n${body}`)
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

export const buildSubmissionTask = (params: {
  assets: SubmissionReferencedAsset[]
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

  return { displayContent, fullTask }
}

export const prepareSubmission = <TAttachment, TTraffic, TAsset>(params: {
  clearDraftState: () => void
  pendingAttachments: TAttachment[]
  processedDocuments: ProcessedDocumentResult[]
  referencedAssets: TAsset[]
  referencedTraffic: TTraffic[]
  setPendingDocumentAttachments: (documents: ProcessedDocumentResult[]) => void
  task: string
  toAssetContextItems: (assets: TAsset[]) => SubmissionReferencedAsset[]
  toTrafficContextItems: (traffic: TTraffic[]) => SubmissionReferencedTraffic[]
}): PreparedSubmission<TAttachment, TTraffic, TAsset> => {
  const usedResources = collectSubmissionResourceSnapshot({
    pendingAttachments: params.pendingAttachments,
    processedDocuments: params.processedDocuments,
    referencedAssets: params.referencedAssets,
    referencedTraffic: params.referencedTraffic,
  })
  const { displayContent, fullTask } = buildSubmissionTask({
    assets: params.toAssetContextItems(params.referencedAssets),
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
