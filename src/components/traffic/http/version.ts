import type { HttpVersion } from './model'

export type ConcreteHttpVersion = Exclude<HttpVersion, 'AUTO'>

const SUPPORTED_HTTP_VERSIONS = new Set<ConcreteHttpVersion>([
  'HTTP/1.0',
  'HTTP/1.1',
  'HTTP/2',
])

export const DEFAULT_HTTP_VERSION: ConcreteHttpVersion = 'HTTP/1.1'

export function normalizeHttpVersionToken(
  value?: string | null,
  fallback: ConcreteHttpVersion = DEFAULT_HTTP_VERSION,
): ConcreteHttpVersion {
  const normalized = value?.trim().toUpperCase()
  if (!normalized) {
    return fallback
  }

  if (normalized === 'HTTP/2.0') {
    return 'HTTP/2'
  }

  if (SUPPORTED_HTTP_VERSIONS.has(normalized as ConcreteHttpVersion)) {
    return normalized as ConcreteHttpVersion
  }

  return fallback
}
