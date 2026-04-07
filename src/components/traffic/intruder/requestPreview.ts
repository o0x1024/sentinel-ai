export type RequestPreviewChangeKind = 'added' | 'removed' | 'changed'

export interface RequestPreviewHeaderChange {
  id: string
  label: string
  kind: RequestPreviewChangeKind
  before: string
  after: string
}

export interface RequestPreviewParameterChange {
  id: string
  label: string
  kind: RequestPreviewChangeKind
  before: string
  after: string
}

export interface RequestPreviewParameterDiff {
  changes: RequestPreviewParameterChange[]
  unchangedCount: number
}

export interface RequestPreviewDiff {
  requestLineChanged: boolean
  requestLineBefore: string
  requestLineAfter: string
  headerChanges: RequestPreviewHeaderChange[]
  unchangedHeaderCount: number
  queryParameterDiff: RequestPreviewParameterDiff
  formParameterDiff: RequestPreviewParameterDiff
  bodyChanged: boolean
  bodyBefore: string
  bodyAfter: string
}

export interface RequestPreviewTraceDiff {
  index: number
  diff: RequestPreviewDiff
}

interface ParsedRequestHeader {
  label: string
  normalizedName: string
  line: string
}

interface ParsedRequestPreview {
  requestLine: string
  headers: ParsedRequestHeader[]
  body: string
  requestTarget: string
  contentType: string
}

function normalizeRequestPreviewText(rawRequest: string): string {
  return rawRequest.replace(/\r\n/g, '\n').replace(/\r/g, '\n')
}

function parseRequestPreview(rawRequest: string): ParsedRequestPreview {
  const normalized = normalizeRequestPreviewText(rawRequest)
  const separatorIndex = normalized.indexOf('\n\n')
  const headerPart = separatorIndex === -1 ? normalized : normalized.slice(0, separatorIndex)
  const body = separatorIndex === -1 ? '' : normalized.slice(separatorIndex + 2)
  const lines = headerPart ? headerPart.split('\n') : []
  const requestLine = lines[0] ?? ''
  const requestTarget = requestLine.split(/\s+/)[1] ?? ''
  const headers = lines.slice(1).map((line, index) => {
    const colonIndex = line.indexOf(':')
    const label = colonIndex > 0 ? line.slice(0, colonIndex).trim() : `Line ${index + 1}`
    return {
      label,
      normalizedName: colonIndex > 0 ? label.toLowerCase() : `__line_${index}`,
      line,
    }
  })

  return {
    requestLine,
    headers,
    body,
    requestTarget,
    contentType: headers.find((header) => header.normalizedName === 'content-type')?.line.split(':').slice(1).join(':').trim().toLowerCase() ?? '',
  }
}

function collectHeaderOrder(
  originalHeaders: ParsedRequestHeader[],
  finalHeaders: ParsedRequestHeader[],
): string[] {
  const orderedNames: string[] = []

  for (const header of [...originalHeaders, ...finalHeaders]) {
    if (!orderedNames.includes(header.normalizedName)) {
      orderedNames.push(header.normalizedName)
    }
  }

  return orderedNames
}

function parseQueryParameters(requestTarget: string): Array<[string, string]> {
  if (!requestTarget) return []

  try {
    const url = requestTarget.startsWith('http://') || requestTarget.startsWith('https://')
      ? new URL(requestTarget)
      : new URL(requestTarget.startsWith('/') ? requestTarget : `/${requestTarget}`, 'https://intruder.local')

    return Array.from(url.searchParams.entries())
  } catch {
    return []
  }
}

function isFormUrlEncoded(contentType: string): boolean {
  return contentType.includes('application/x-www-form-urlencoded')
}

function parseFormParameters(body: string, contentType: string): Array<[string, string]> {
  if (!body || !isFormUrlEncoded(contentType)) {
    return []
  }

  try {
    return Array.from(new URLSearchParams(body).entries())
  } catch {
    return []
  }
}

function collectParameterOrder(
  beforeEntries: Array<[string, string]>,
  afterEntries: Array<[string, string]>,
): string[] {
  const orderedNames: string[] = []

  for (const [name] of [...beforeEntries, ...afterEntries]) {
    if (!orderedNames.includes(name)) {
      orderedNames.push(name)
    }
  }

  return orderedNames
}

function buildParameterDiff(
  beforeEntries: Array<[string, string]>,
  afterEntries: Array<[string, string]>,
): RequestPreviewParameterDiff {
  const beforeBuckets = new Map<string, string[]>()
  const afterBuckets = new Map<string, string[]>()

  for (const [name, value] of beforeEntries) {
    const bucket = beforeBuckets.get(name) ?? []
    bucket.push(value)
    beforeBuckets.set(name, bucket)
  }

  for (const [name, value] of afterEntries) {
    const bucket = afterBuckets.get(name) ?? []
    bucket.push(value)
    afterBuckets.set(name, bucket)
  }

  const changes: RequestPreviewParameterChange[] = []
  let unchangedCount = 0

  for (const name of collectParameterOrder(beforeEntries, afterEntries)) {
    const beforeValues = beforeBuckets.get(name) ?? []
    const afterValues = afterBuckets.get(name) ?? []
    const count = Math.max(beforeValues.length, afterValues.length)

    for (let index = 0; index < count; index += 1) {
      const before = beforeValues[index] ?? ''
      const after = afterValues[index] ?? ''
      const hasBefore = index < beforeValues.length
      const hasAfter = index < afterValues.length

      if (hasBefore && hasAfter && before === after) {
        unchangedCount += 1
        continue
      }

      let kind: RequestPreviewChangeKind
      if (!hasBefore) {
        kind = 'added'
      } else if (!hasAfter) {
        kind = 'removed'
      } else {
        kind = 'changed'
      }

      changes.push({
        id: `${name}-${index}`,
        label: name,
        kind,
        before: hasBefore ? before : '',
        after: hasAfter ? after : '',
      })
    }
  }

  return {
    changes,
    unchangedCount,
  }
}

export function buildRequestPreviewDiff(
  originalRequest: string,
  finalRequest: string,
): RequestPreviewDiff {
  const original = parseRequestPreview(originalRequest)
  const next = parseRequestPreview(finalRequest)
  const originalBuckets = new Map<string, ParsedRequestHeader[]>()
  const nextBuckets = new Map<string, ParsedRequestHeader[]>()

  for (const header of original.headers) {
    const bucket = originalBuckets.get(header.normalizedName) ?? []
    bucket.push(header)
    originalBuckets.set(header.normalizedName, bucket)
  }

  for (const header of next.headers) {
    const bucket = nextBuckets.get(header.normalizedName) ?? []
    bucket.push(header)
    nextBuckets.set(header.normalizedName, bucket)
  }

  const headerChanges: RequestPreviewHeaderChange[] = []
  let unchangedHeaderCount = 0

  for (const name of collectHeaderOrder(original.headers, next.headers)) {
    const beforeItems = originalBuckets.get(name) ?? []
    const afterItems = nextBuckets.get(name) ?? []
    const count = Math.max(beforeItems.length, afterItems.length)

    for (let index = 0; index < count; index += 1) {
      const before = beforeItems[index]
      const after = afterItems[index]

      if (before?.line === after?.line) {
        unchangedHeaderCount += 1
        continue
      }

      let kind: RequestPreviewChangeKind
      if (!before) {
        kind = 'added'
      } else if (!after) {
        kind = 'removed'
      } else {
        kind = 'changed'
      }

      headerChanges.push({
        id: `${name}-${index}`,
        label: before?.label || after?.label || 'Header',
        kind,
        before: before?.line ?? '',
        after: after?.line ?? '',
      })
    }
  }

  return {
    requestLineChanged: original.requestLine !== next.requestLine,
    requestLineBefore: original.requestLine,
    requestLineAfter: next.requestLine,
    headerChanges,
    unchangedHeaderCount,
    queryParameterDiff: buildParameterDiff(
      parseQueryParameters(original.requestTarget),
      parseQueryParameters(next.requestTarget),
    ),
    formParameterDiff: buildParameterDiff(
      parseFormParameters(original.body, original.contentType),
      parseFormParameters(next.body, next.contentType),
    ),
    bodyChanged: original.body !== next.body,
    bodyBefore: original.body,
    bodyAfter: next.body,
  }
}

export function buildRequestPreviewTraceDiffs(
  originalRequest: string,
  requestTexts: string[],
): RequestPreviewTraceDiff[] {
  const diffs: RequestPreviewTraceDiff[] = []
  let previousRequest = originalRequest

  requestTexts.forEach((requestText, index) => {
    diffs.push({
      index,
      diff: buildRequestPreviewDiff(previousRequest, requestText),
    })
    previousRequest = requestText
  })

  return diffs
}
