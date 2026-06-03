import { serializeHeaderEntries } from './http/headers'
import type { HttpHeaderEntry } from './http/model'
import { buildRepeaterFullUrl } from './proxyRepeaterRequestSupport'
import type { RepeaterTab } from './proxyRepeaterTypes'

export interface RepeaterAssistantTrafficData {
  id: number
  url: string
  method: string
  host: string
  status_code: number
  request_headers?: string
  request_body?: string
  response_headers?: string
  response_body?: string
}

export function buildRepeaterAssistantTrafficData(tab: RepeaterTab): RepeaterAssistantTrafficData {
  const lines = tab.rawRequest.split(/\r\n|\r|\n/)
  const firstLine = lines[0]
  const methodMatch = firstLine.match(/^(\w+)\s+(\S+)/)
  const method = methodMatch ? methodMatch[1] : 'GET'
  const requestHeaders: HttpHeaderEntry[] = []
  const bodyLines: string[] = []
  let inBody = false

  for (let i = 1; i < lines.length; i++) {
    const line = lines[i]
    if (!inBody && line === '') {
      inBody = true
      continue
    }
    if (!inBody) {
      const colonIndex = line.indexOf(':')
      if (colonIndex > 0) {
        requestHeaders.push({
          name: line.substring(0, colonIndex).trim(),
          value: line.substring(colonIndex + 1).trim(),
        })
      }
    } else {
      bodyLines.push(line)
    }
  }

  const requestBody = bodyLines.join('\n').trim()

  return {
    id: Date.now(),
    url: buildRepeaterFullUrl(tab),
    method,
    host: tab.targetHost,
    status_code: tab.response?.statusCode || 0,
    request_headers: serializeHeaderEntries(requestHeaders),
    request_body: requestBody || undefined,
    response_headers: tab.response ? serializeHeaderEntries(tab.response.headers) : undefined,
    response_body: tab.response?.bodyText || undefined,
  }
}
