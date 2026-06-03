const BASE64_PREFIX = '[BASE64]'

export const normalizeResponseContentType = (contentType: string): string =>
  contentType.split(';')[0]?.trim().toLowerCase() || ''

export const isImageResponseContentType = (contentType: string): boolean =>
  normalizeResponseContentType(contentType).startsWith('image/')

const isLikelyBase64Payload = (value: string): boolean => {
  const normalized = value.replace(/\s+/g, '')
  if (!normalized || normalized.length < 16 || normalized.length % 4 !== 0) {
    return false
  }

  return /^[A-Za-z0-9+/]+=*$/.test(normalized)
}

export const extractBase64BodyPayload = (body: string): string | null => {
  const trimmed = body.trim()
  if (!trimmed) {
    return null
  }

  if (trimmed.startsWith('data:')) {
    const marker = ';base64,'
    const markerIndex = trimmed.indexOf(marker)
    if (markerIndex >= 0) {
      return trimmed.slice(markerIndex + marker.length).replace(/\s+/g, '')
    }
    return null
  }

  if (trimmed.startsWith(BASE64_PREFIX)) {
    return trimmed.slice(BASE64_PREFIX.length).trim().replace(/\s+/g, '')
  }

  if (isLikelyBase64Payload(trimmed)) {
    return trimmed.replace(/\s+/g, '')
  }

  return null
}

const estimateBase64DecodedSize = (payload: string): number => {
  const normalized = payload.replace(/\s+/g, '')
  const padding = normalized.endsWith('==') ? 2 : normalized.endsWith('=') ? 1 : 0
  return Math.max(0, Math.floor((normalized.length * 3) / 4) - padding)
}

export const estimateResponseBodySize = (body: string): number => {
  const base64Payload = extractBase64BodyPayload(body)
  if (base64Payload) {
    return estimateBase64DecodedSize(base64Payload)
  }

  return new Blob([body]).size
}

export const decodeBase64BodyToBinaryString = (body: string): string | null => {
  const base64Payload = extractBase64BodyPayload(body)
  if (!base64Payload) {
    return null
  }

  try {
    return atob(base64Payload)
  } catch {
    return null
  }
}

export const getDisplayResponseBody = (
  body: string,
  contentType: string,
): string => {
  if (!body) {
    return ''
  }

  if (!isImageResponseContentType(contentType)) {
    return body
  }

  return decodeBase64BodyToBinaryString(body) || body
}

export const buildImagePreviewSrc = (
  body: string,
  contentType: string,
): string | null => {
  const normalizedType = normalizeResponseContentType(contentType)
  if (!normalizedType || !normalizedType.startsWith('image/')) {
    return null
  }

  const trimmed = body.trim()
  if (!trimmed) {
    return null
  }

  if (trimmed.startsWith('data:')) {
    return trimmed
  }

  if (normalizedType === 'image/svg+xml' && !extractBase64BodyPayload(trimmed)) {
    return `data:${normalizedType};charset=utf-8,${encodeURIComponent(body)}`
  }

  const base64Payload = extractBase64BodyPayload(trimmed)
  if (base64Payload) {
    return `data:${normalizedType};base64,${base64Payload}`
  }

  try {
    return `data:${normalizedType};base64,${btoa(body)}`
  } catch {
    return null
  }
}
