import type { RepeaterTab } from './proxyRepeaterTypes'

type TranslateFn = (key: string, params?: Record<string, unknown>) => string

export function parseRepeaterErrorMessage(error: unknown, t: TranslateFn): string {
  if (!error) return t('trafficAnalysis.repeater.messages.unknownError')

  const errorStr = String(error).toLowerCase()

  if (errorStr.includes('timeout')) {
    return t('trafficAnalysis.repeater.messages.timeout')
  }
  if (errorStr.includes('connection refused') || errorStr.includes('econnrefused')) {
    return t('trafficAnalysis.repeater.messages.connectionRefused')
  }
  if (errorStr.includes('network')) {
    return t('trafficAnalysis.repeater.messages.networkError')
  }

  const message = String(error)
  return message.length > 100 ? `${message.substring(0, 100)}...` : message
}

export function repeaterTextToHex(value: string): string {
  if (!value) return ''

  const lines: string[] = []
  let hex = ''
  let ascii = ''
  let lineCount = 0

  for (let index = 0; index < value.length; index += 1) {
    const char = value.charCodeAt(index) & 0xff
    hex += `${char.toString(16).padStart(2, '0')} `
    ascii += char >= 32 && char < 127 ? String.fromCharCode(char) : '.'
    lineCount += 1

    if (lineCount === 16 || index === value.length - 1) {
      if (lineCount < 16) {
        hex += '   '.repeat(16 - lineCount)
      }
      lines.push(`${hex} ${ascii}`)
      hex = ''
      ascii = ''
      lineCount = 0
    }
  }

  return lines.join('\n')
}

export function buildRepeaterFullUrl(tab: RepeaterTab | null): string {
  if (!tab) return ''

  try {
    const protocol = tab.useTls ? 'https' : 'http'
    const host = tab.targetHost
    const port = tab.targetPort
    const defaultPort = tab.useTls ? 443 : 80
    const portStr = port !== defaultPort ? `:${port}` : ''
    const firstLine = tab.rawRequest.split(/\r\n|\r|\n/)[0]
    const match = firstLine.match(/^\w+\s+(\S+)/)
    let path = match ? match[1] : '/'

    if (!path.startsWith('/') && !path.startsWith('http')) {
      path = `/${path}`
    }

    if (path.startsWith('http://') || path.startsWith('https://')) {
      return path
    }

    const url = `${protocol}://${host}${portStr}${path}`
    try {
      new URL(url)
      return url
    } catch {
      console.warn('Invalid URL constructed:', url)
      return url
    }
  } catch (error) {
    console.error('Error building URL:', error)
    return ''
  }
}

export function buildRepeaterCurlCommand(tab: RepeaterTab | null): string {
  if (!tab) return ''

  const lines = tab.rawRequest.split(/\r\n|\r|\n/)
  const firstLine = lines[0]
  const methodMatch = firstLine.match(/^(\w+)\s+(\S+)/)
  const method = methodMatch ? methodMatch[1] : 'GET'
  const url = buildRepeaterFullUrl(tab)
  let curl = `curl -X ${method} '${url}'`
  let inBody = false
  const bodyLines: string[] = []

  for (let index = 1; index < lines.length; index += 1) {
    const line = lines[index]
    if (!inBody && line === '') {
      inBody = true
      continue
    }
    if (!inBody) {
      const colonIndex = line.indexOf(':')
      if (colonIndex > 0) {
        const key = line.substring(0, colonIndex).trim()
        const value = line.substring(colonIndex + 1).trim()
        if (key.toLowerCase() !== 'host' && key.toLowerCase() !== 'content-length') {
          curl += ` \\\n  -H '${key}: ${value}'`
        }
      }
    } else {
      bodyLines.push(line)
    }
  }

  const body = bodyLines.join('\n').trim()
  if (body) {
    curl += ` \\\n  -d '${body.replace(/'/g, "'\\''")}'`
  }

  return curl
}

export function detectRepeaterTargetFromRequest(requestText: string) {
  const lines = requestText.split(/\r\n|\r|\n/)
  let hostValue = ''

  for (const line of lines) {
    const trimmedLine = line.trim()
    if (trimmedLine === '') break

    const colonIndex = line.indexOf(':')
    if (colonIndex > 0) {
      const key = line.substring(0, colonIndex).trim()
      const value = line.substring(colonIndex + 1).trim()

      if (key.toLowerCase() === 'host') {
        hostValue = value
        break
      }
    }
  }

  if (!hostValue) return null

  const hostPortMatch = hostValue.match(/^([^:]+)(?::(\d+))?$/)
  if (!hostPortMatch) return null

  const hostname = hostPortMatch[1]
  const port = hostPortMatch[2] ? parseInt(hostPortMatch[2], 10) : null

  return {
    hostname,
    targetPort: port ?? 443,
    useTls: port === null ? true : port === 443,
  }
}

export function syncRepeaterTabTargetFromRequest(tab: RepeaterTab, requestText: string) {
  const detected = detectRepeaterTargetFromRequest(requestText)
  if (!detected) return false

  const needsUpdate = !tab.targetHost
    || tab.targetHost === 'example.com'
    || tab.targetHost !== detected.hostname

  if (!needsUpdate) return false

  tab.targetHost = detected.hostname
  tab.targetPort = detected.targetPort
  tab.useTls = detected.useTls
  tab.name = detected.hostname
  return true
}
