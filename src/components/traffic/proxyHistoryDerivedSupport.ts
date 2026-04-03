import type { ProxyRequest } from './proxyHistoryTypes'

type ProxyHistoryDerived = {
  hasParams: boolean
  extension: string
  mimeType: string
  mimeTypeCategory: string
  timestampMs: number
  formattedTime: string
  searchText: string
  searchTextLower: string
  listenerValue: string
}

type CachedProxyHistoryDerived = {
  signature: string
  value: ProxyHistoryDerived
}

const derivedCache = new Map<number, CachedProxyHistoryDerived>()

const extensionCategoryMap: Record<string, string> = {
  html: 'html',
  htm: 'html',
  js: 'script',
  mjs: 'script',
  xml: 'xml',
  css: 'css',
  png: 'image',
  jpg: 'image',
  jpeg: 'image',
  gif: 'image',
  webp: 'image',
  svg: 'image',
  ico: 'image',
  swf: 'flash',
  txt: 'text',
  json: 'text',
  woff: 'binary',
  woff2: 'binary',
  ttf: 'binary',
  eot: 'binary',
  pdf: 'binary',
  zip: 'binary',
}

const extensionMimeMap: Record<string, string> = {
  html: 'HTML',
  htm: 'HTML',
  json: 'JSON',
  xml: 'XML',
  js: 'JavaScript',
  css: 'CSS',
  png: 'image',
  jpg: 'image',
  jpeg: 'image',
  gif: 'image',
  svg: 'image',
  ico: 'image',
  pdf: 'application/pdf',
  txt: 'text',
}

const buildDerivedSignature = (request: ProxyRequest) => [
  request.url,
  request.host,
  request.response_headers || '',
  request.mime_type || '',
  request.extension || '',
  request.listener || '',
  request.timestamp,
].join('\u0000')

const parseUrlParts = (url: string) => {
  try {
    const parsedUrl = new URL(url)
    const pathname = parsedUrl.pathname || ''
    const parts = pathname.split('.')
    return {
      hasParams: parsedUrl.search.length > 0,
      extension: parts.length > 1
        ? parts[parts.length - 1].split(/[?#]/)[0].toLowerCase()
        : '',
    }
  } catch {
    const pathname = url.split('?')[0]
    const parts = pathname.split('.')
    return {
      hasParams: url.includes('?'),
      extension: parts.length > 1 ? parts[parts.length - 1].toLowerCase() : '',
    }
  }
}

const parseContentType = (responseHeaders?: string) => {
  if (!responseHeaders) return ''

  try {
    const parsed = JSON.parse(responseHeaders) as Record<string, unknown>
    const contentType = parsed['content-type'] || parsed['Content-Type']
    return typeof contentType === 'string' ? contentType.toLowerCase() : ''
  } catch {
    return ''
  }
}

const getMimeTypeCategoryFromContentType = (contentType: string): string => {
  if (!contentType) return ''
  if (contentType.includes('html')) return 'html'
  if (contentType.includes('javascript') || contentType.includes('ecmascript')) return 'script'
  if (contentType.includes('xml')) return 'xml'
  if (contentType.includes('css')) return 'css'
  if (contentType.includes('image')) return 'image'
  if (contentType.includes('flash') || contentType.includes('shockwave')) return 'flash'
  if (contentType.includes('text') || contentType.includes('json')) return 'text'
  if (
    contentType.includes('octet-stream') ||
    contentType.includes('binary') ||
    contentType.includes('font') ||
    contentType.includes('application')
  ) {
    return 'binary'
  }
  return 'unknown'
}

const getMimeTypeLabelFromContentType = (contentType: string) => {
  if (!contentType) return ''
  return contentType.split(';')[0].trim()
}

export const getProxyHistoryDerived = (request: ProxyRequest): ProxyHistoryDerived => {
  const signature = buildDerivedSignature(request)
  const cached = derivedCache.get(request.id)
  if (cached && cached.signature === signature) {
    return cached.value
  }

  const { hasParams, extension } = parseUrlParts(request.url)
  const contentType = parseContentType(request.response_headers)
  const mimeType = request.mime_type || getMimeTypeLabelFromContentType(contentType) || extensionMimeMap[extension] || ''
  const mimeTypeCategory = getMimeTypeCategoryFromContentType(contentType) || extensionCategoryMap[extension] || 'unknown'
  const searchText = `${request.url} ${request.host}`
  const timestampMs = new Date(request.timestamp).getTime()

  const value: ProxyHistoryDerived = {
    hasParams,
    extension: request.extension || extension,
    mimeType,
    mimeTypeCategory,
    timestampMs,
    formattedTime: Number.isNaN(timestampMs) ? request.timestamp : new Date(timestampMs).toLocaleString('zh-CN'),
    searchText,
    searchTextLower: searchText.toLowerCase(),
    listenerValue: request.listener || 'Proxy',
  }

  derivedCache.set(request.id, {
    signature,
    value,
  })

  return value
}

export const clearProxyHistoryDerivedCache = () => {
  derivedCache.clear()
}

export const pruneProxyHistoryDerivedCache = (requestIds: Iterable<number>) => {
  const liveIds = new Set(requestIds)
  for (const cachedId of derivedCache.keys()) {
    if (!liveIds.has(cachedId)) {
      derivedCache.delete(cachedId)
    }
  }
}
