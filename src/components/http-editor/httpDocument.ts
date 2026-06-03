export type HttpBodyLanguage = 'json' | 'javascript' | 'html' | 'xml' | 'css' | 'form' | 'text' | 'binary'

export interface ParsedHttpHeader {
  name: string
  value: string
  raw: string
  lineNumber: number
}

export type ParsedHttpStartLine =
  | {
      kind: 'request'
      method: string
      target: string
      protocol: string
      raw: string
    }
  | {
      kind: 'response'
      protocol: string
      statusCode: string
      statusText: string
      raw: string
    }
  | {
      kind: 'plain'
      raw: string
    }

export interface ParsedHttpMessageDocument {
  rawText: string
  normalizedText: string
  startLine: string
  parsedStartLine: ParsedHttpStartLine
  headers: ParsedHttpHeader[]
  body: string
  contentType: string
  bodyLanguage: HttpBodyLanguage
  lineCount: number
  bodyLineCount: number
  bodyStartLineNumber: number | null
  hasSeparatorLine: boolean
}

export const normalizeHttpText = (content: string): string => content.replace(/\r\n/g, '\n')

export const isLikelyTextBody = (body: string): boolean => {
  if (!body) return false
  const sample = body.slice(0, 512)
  let suspiciousCount = 0

  for (let index = 0; index < sample.length; index += 1) {
    const code = sample.charCodeAt(index)
    const isAllowedControl = code === 9 || code === 10 || code === 13
    const isPrintable = code >= 32 && code !== 127
    if (!isAllowedControl && !isPrintable) suspiciousCount += 1
  }

  return suspiciousCount === 0
}

const isLikelyJsonBody = (trimmed: string): boolean => {
  if (!trimmed) return false
  if (trimmed.startsWith('{') || trimmed.startsWith('[')) return true
  return false
}

export const detectHttpBodyLanguage = (body: string, contentType: string): HttpBodyLanguage => {
  const normalizedContentType = contentType.toLowerCase()
  const trimmed = body.trim()

  if (!trimmed) return 'text'

  if (normalizedContentType.includes('application/x-www-form-urlencoded')) return 'form'
  if (normalizedContentType.includes('json')) return 'json'
  if (normalizedContentType.includes('javascript') || normalizedContentType.includes('ecmascript')) return 'javascript'
  if (normalizedContentType.includes('html')) return 'html'
  if (normalizedContentType.includes('xml') || normalizedContentType.includes('svg')) return 'xml'
  if (normalizedContentType.includes('css')) return 'css'
  if (normalizedContentType.includes('text/')) return 'text'

  if (isLikelyJsonBody(trimmed)) {
    try {
      JSON.parse(trimmed)
      return 'json'
    } catch {
      // Keep JSON highlighting stable while the user is editing incomplete JSON.
      return 'json'
    }
  }

  if (/^<!doctype html/i.test(trimmed) || /^<html[\s>]/i.test(trimmed)) return 'html'
  if (/^<[\w!?/]/.test(trimmed)) return 'xml'
  if (/^(?:const|let|var|function|class|import|export)\b/.test(trimmed)) return 'javascript'
  if (/^[^{]+{[\s\S]*}$/.test(trimmed) && /:\s*[^,\n]+/.test(trimmed)) return 'css'

  return isLikelyTextBody(body) ? 'text' : 'binary'
}

export const parseHttpStartLine = (startLine: string): ParsedHttpStartLine => {
  const requestMatch = /^([A-Z]+)\s+(\S+)\s+(HTTP\/\d(?:\.\d+)?)$/.exec(startLine)
  if (requestMatch) {
    return {
      kind: 'request',
      method: requestMatch[1],
      target: requestMatch[2],
      protocol: requestMatch[3],
      raw: startLine,
    }
  }

  const responseMatch = /^(HTTP\/\d(?:\.\d+)?)\s+(\d+)\s*(.*)$/.exec(startLine)
  if (responseMatch) {
    return {
      kind: 'response',
      protocol: responseMatch[1],
      statusCode: responseMatch[2],
      statusText: responseMatch[3],
      raw: startLine,
    }
  }

  return {
    kind: 'plain',
    raw: startLine,
  }
}

export const parseHttpMessageDocument = (content: string): ParsedHttpMessageDocument => {
  const normalized = normalizeHttpText(content)
  const separatorIndex = normalized.indexOf('\n\n')
  const headerBlock = separatorIndex === -1 ? normalized : normalized.slice(0, separatorIndex)
  const body = separatorIndex === -1 ? '' : normalized.slice(separatorIndex + 2)
  const lines = headerBlock.split('\n')
  const startLine = lines.shift() || ''

  const headers = lines
    .filter((line) => line.length > 0)
    .map((line, index): ParsedHttpHeader => {
      const colonIndex = line.indexOf(':')
      if (colonIndex <= 0) {
        return {
          name: '',
          value: line,
          raw: line,
          lineNumber: index + 2,
        }
      }

      return {
        name: line.slice(0, colonIndex).trim(),
        value: line.slice(colonIndex + 1).trim(),
        raw: line,
        lineNumber: index + 2,
      }
    })

  const contentTypeHeader = headers.find((header) => header.name.toLowerCase() === 'content-type')
  const contentType = contentTypeHeader?.value || ''
  const bodyLineCount = body ? body.split('\n').length : 0
  const hasSeparatorLine = body.length > 0
  const bodyStartLineNumber = hasSeparatorLine ? headers.length + 3 : null

  return {
    rawText: content,
    normalizedText: normalized,
    startLine,
    parsedStartLine: parseHttpStartLine(startLine),
    headers,
    body,
    contentType,
    bodyLanguage: detectHttpBodyLanguage(body, contentType),
    lineCount: 1 + headers.length + (hasSeparatorLine ? 1 + bodyLineCount : 0),
    bodyLineCount,
    bodyStartLineNumber,
    hasSeparatorLine,
  }
}
