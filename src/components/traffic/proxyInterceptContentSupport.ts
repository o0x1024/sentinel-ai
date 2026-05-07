import { normalizeHttpVersionToken } from './http/version'
import type {
  InterceptedItem,
  InterceptedRequest,
  InterceptedResponse,
  InterceptedWebSocketMessage,
} from './proxyInterceptSupport'

export function formatInterceptPrettyContent(value: string, formatBody: (body: string) => string) {
  if (!value) return ''

  const lines = value.split('\n')
  const result: string[] = []
  let inBody = false
  const bodyLines: string[] = []

  for (const line of lines) {
    if (!inBody) {
      result.push(line)
      if (line.trim() === '') {
        inBody = true
      }
    } else {
      bodyLines.push(line)
    }
  }

  if (bodyLines.length > 0) {
    result.push(formatBody(bodyLines.join('\n')))
  }

  return result.join('\n')
}

export function buildInterceptHexView(value: string) {
  if (!value) return ''

  const bytes = new TextEncoder().encode(value)
  let hex = ''
  for (let index = 0; index < bytes.length; index += 16) {
    const offset = index.toString(16).padStart(8, '0')
    const chunk = bytes.slice(index, index + 16)
    const hexPart = Array.from(chunk)
      .map((byte) => byte.toString(16).padStart(2, '0'))
      .join(' ')
    const asciiPart = Array.from(chunk)
      .map((byte) => (byte >= 32 && byte < 127 ? String.fromCharCode(byte) : '.'))
      .join('')
    hex += `${offset}  ${hexPart.padEnd(48, ' ')}  ${asciiPart}\n`
  }
  return hex
}

export function buildInterceptRequestText(request: InterceptedRequest) {
  let content = `${request.method} ${request.path} ${request.protocol}\n`
  for (const [key, value] of Object.entries(request.headers)) {
    content += `${key}: ${value}\n`
  }
  if (request.body) {
    content += `\n${request.body}`
  }
  return content
}

export function getInterceptResponseProtocol(
  response: InterceptedResponse,
  interceptedRequests: InterceptedRequest[],
) {
  const matchingRequest = interceptedRequests.find((request) => request.id === response.request_id)
  return normalizeHttpVersionToken(matchingRequest?.protocol)
}

export function buildInterceptResponseText(
  response: InterceptedResponse,
  interceptedRequests: InterceptedRequest[],
) {
  let content = `${getInterceptResponseProtocol(response, interceptedRequests)} ${response.status}\n`
  for (const [key, value] of Object.entries(response.headers)) {
    content += `${key}: ${value}\n`
  }
  if (response.body) {
    content += `\n${response.body}`
  }
  return content
}

export function buildInterceptWebSocketText(message: InterceptedWebSocketMessage) {
  return message.content || ''
}

export function getInterceptItemKey(item: InterceptedItem) {
  return `${item.type}:${item.data.id}`
}

export function resolveInterceptForwardModifiedContent(currentContent: string, originalContent: string) {
  return currentContent === originalContent ? undefined : currentContent
}

export function buildInterceptItemText(item: InterceptedItem, interceptedRequests: InterceptedRequest[]) {
  if (item.type === 'request') {
    return buildInterceptRequestText(item.data as InterceptedRequest)
  }
  if (item.type === 'response') {
    return buildInterceptResponseText(item.data as InterceptedResponse, interceptedRequests)
  }
  return buildInterceptWebSocketText(item.data as InterceptedWebSocketMessage)
}
