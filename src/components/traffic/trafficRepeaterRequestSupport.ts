export interface RepeaterRequestSeed {
  name: string
  rawRequest: string
  targetHost: string
  targetPort: number
  useTls: boolean
}

function sanitizeFilenameSegment(value: string): string {
  const sanitized = value.replace(/[^a-zA-Z0-9.-]+/g, '-').replace(/-+/g, '-').replace(/^-|-$/g, '')
  return sanitized || 'request'
}

export function replaceRawRequestMethod(rawRequest: string, method: string): string {
  const trimmedMethod = method.trim().toUpperCase()
  if (!trimmedMethod) return rawRequest

  const normalized = rawRequest.replace(/\r\n/g, '\n').replace(/\r/g, '\n')
  const lines = normalized.split('\n')
  if (!lines.length || !lines[0]?.trim()) {
    return `${trimmedMethod} / HTTP/1.1\r\nHost: example.com\r\n\r\n`
  }

  const firstLineParts = lines[0].trim().split(/\s+/)
  const target = firstLineParts[1] || '/'
  const protocol = firstLineParts[2] || 'HTTP/1.1'
  lines[0] = `${trimmedMethod} ${target} ${protocol}`
  return lines.join('\r\n')
}

export function buildRepeaterRequestSeedFromUrlInput(input: string): RepeaterRequestSeed | null {
  const trimmed = input.trim()
  if (!trimmed) return null

  const normalized = /^[a-zA-Z][a-zA-Z\d+\-.]*:\/\//.test(trimmed) ? trimmed : `https://${trimmed}`

  try {
    const url = new URL(normalized)
    const path = `${url.pathname || '/'}${url.search}`
    const defaultPort = url.protocol === 'https:' ? 443 : 80
    const port = url.port ? Number.parseInt(url.port, 10) : defaultPort
    const hostHeader = url.port ? url.host : url.hostname

    return {
      name: url.hostname,
      targetHost: url.hostname,
      targetPort: port,
      useTls: url.protocol === 'https:',
      rawRequest: `GET ${path || '/'} HTTP/1.1\r\nHost: ${hostHeader}\r\nUser-Agent: Sentinel-AI/1.0\r\nAccept: */*\r\n\r\n`,
    }
  } catch {
    return null
  }
}

export function buildRepeaterRequestFilename(host: string, method: string): string {
  const safeHost = sanitizeFilenameSegment(host || 'request')
  const safeMethod = sanitizeFilenameSegment(method || 'request').toUpperCase()
  return `${safeMethod}-${safeHost}.http`
}
