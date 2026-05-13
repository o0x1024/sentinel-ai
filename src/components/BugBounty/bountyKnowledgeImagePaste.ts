export interface TextInsertionResult {
  value: string
  cursor: number
}

export interface BountyKnowledgeImageReference {
  id: string
  dataUrl: string
}

export interface CollapsedImageContent {
  content: string
  references: BountyKnowledgeImageReference[]
}

const IMAGE_REFERENCE_ID_PREFIX = 'bounty-image'
const IMAGE_DATA_URL_PATTERN = 'data:image\\/(?:png|jpeg|jpg|gif|webp);base64,[A-Za-z0-9+/=]+'
const INLINE_IMAGE_DATA_URL_RE = new RegExp(`!\\[([^\\]]*)\\]\\((${IMAGE_DATA_URL_PATTERN})\\)`, 'g')
const IMAGE_REFERENCE_LINE_RE = new RegExp(`^\\[([^\\]]+)\\]:\\s*(${IMAGE_DATA_URL_PATTERN})\\s*$`, 'gm')

export function findFirstClipboardImageFile(clipboardData: DataTransfer | null): File | null {
  if (!clipboardData) return null

  for (const item of Array.from(clipboardData.items)) {
    if (item.kind !== 'file' || !item.type.startsWith('image/')) continue
    const file = item.getAsFile()
    if (file) return file
  }

  return null
}

export function buildPastedImageReferenceMarkdown(id: string, label: string) {
  return `![${label}][${id}]`
}

function createReferenceId(existingIds: Set<string>, index: number) {
  let candidate = `${IMAGE_REFERENCE_ID_PREFIX}-${index}`
  while (existingIds.has(candidate)) {
    index += 1
    candidate = `${IMAGE_REFERENCE_ID_PREFIX}-${index}`
  }
  existingIds.add(candidate)
  return candidate
}

export function insertTextAtSelection(
  value: string,
  text: string,
  selectionStart: number,
  selectionEnd: number,
): TextInsertionResult {
  const start = Math.max(0, Math.min(selectionStart, value.length))
  const end = Math.max(start, Math.min(selectionEnd, value.length))
  const before = value.slice(0, start)
  const after = value.slice(end)
  const prefix = before && !before.endsWith('\n') ? '\n\n' : ''
  const suffix = after && !after.startsWith('\n') ? '\n\n' : ''
  const inserted = `${prefix}${text}${suffix}`

  return {
    value: `${before}${inserted}${after}`,
    cursor: before.length + inserted.length,
  }
}

export function readImageFileAsDataUrl(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader()
    reader.onload = () => {
      if (typeof reader.result === 'string') {
        resolve(reader.result)
        return
      }
      reject(new Error('Pasted image could not be read as text data'))
    }
    reader.onerror = () => reject(reader.error || new Error('Failed to read pasted image'))
    reader.readAsDataURL(file)
  })
}

export function collapseBountyKnowledgeImageContent(content: string): CollapsedImageContent {
  const references = new Map<string, string>()
  const existingIds = new Set<string>()
  let nextImageIndex = 1

  const withoutReferenceLines = content.replace(IMAGE_REFERENCE_LINE_RE, (_match, id: string, dataUrl: string) => {
    references.set(id, dataUrl)
    existingIds.add(id)
    const suffixMatch = new RegExp(`^${IMAGE_REFERENCE_ID_PREFIX}-(\\d+)$`).exec(id)
    if (suffixMatch) {
      nextImageIndex = Math.max(nextImageIndex, Number(suffixMatch[1]) + 1)
    }
    return ''
  })

  const visibleContent = withoutReferenceLines
    .replace(INLINE_IMAGE_DATA_URL_RE, (_match, label: string, dataUrl: string) => {
      const id = createReferenceId(existingIds, nextImageIndex)
      nextImageIndex += 1
      references.set(id, dataUrl)
      return buildPastedImageReferenceMarkdown(id, label || id)
    })
    .replace(/\n{3,}/g, '\n\n')
    .trim()

  return {
    content: visibleContent,
    references: Array.from(references.entries()).map(([id, dataUrl]) => ({ id, dataUrl })),
  }
}

export function expandBountyKnowledgeImageContent(
  content: string,
  references: BountyKnowledgeImageReference[],
): string {
  const referenceMap = new Map(references.map(reference => [reference.id, reference.dataUrl]))
  const usedIds = new Set<string>()
  const referenceUsageRe = /!\[[^\]]*]\[([^\]]+)]/g
  let match: RegExpExecArray | null

  while ((match = referenceUsageRe.exec(content)) !== null) {
    if (referenceMap.has(match[1])) {
      usedIds.add(match[1])
    }
  }

  if (usedIds.size === 0) {
    return content.trim()
  }

  const definitions = Array.from(usedIds)
    .map(id => `[${id}]: ${referenceMap.get(id)}`)
    .join('\n')

  return `${content.trim()}\n\n${definitions}`
}
