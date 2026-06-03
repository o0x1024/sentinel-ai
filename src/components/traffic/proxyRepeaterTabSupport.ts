import { formatRepeaterPrettyRequest } from './trafficRepeaterPrettyRequestSupport'
import type { RepeaterTab, RepeaterTabCreationOptions } from './proxyRepeaterTypes'

function buildDefaultRawRequest() {
  return 'GET / HTTP/1.1\r\nHost: example.com\r\nUser-Agent: Sentinel-AI/1.0\r\nAccept: */*\r\n\r\n'
}

export function createRepeaterTab(options: RepeaterTabCreationOptions): RepeaterTab {
  const {
    request,
    fallbackNameIndex,
    generateId,
    defaultRequestTab,
    defaultResponseTab,
  } = options
  let targetHost = ''
  let targetPort = 443
  let useTls = true
  let rawRequest = ''

  if (request?.absoluteUrl) {
    try {
      const urlObj = new URL(request.absoluteUrl)
      targetHost = urlObj.hostname

      const parsedPort = urlObj.port ? parseInt(urlObj.port, 10) : null
      if (parsedPort !== null && !Number.isNaN(parsedPort) && parsedPort > 0 && parsedPort <= 65535) {
        targetPort = parsedPort
      } else {
        targetPort = urlObj.protocol === 'https:' ? 443 : 80
      }

      useTls = urlObj.protocol === 'https:'

      const path = request.request.target || (urlObj.pathname + urlObj.search)
      rawRequest = `${request.request.method || 'GET'} ${path} ${request.request.versionPreference === 'AUTO' ? 'HTTP/1.1' : request.request.versionPreference}\r\n`
      rawRequest += `Host: ${urlObj.host}\r\n`

      for (const header of request.request.headers || []) {
        if (header.name.toLowerCase() !== 'host') {
          rawRequest += `${header.name}: ${header.value}\r\n`
        }
      }
      rawRequest += '\r\n'
      if (request.request.bodyText) {
        rawRequest += request.request.bodyText
      }
    } catch (error) {
      console.error('Failed to parse URL:', error)
      rawRequest = buildDefaultRawRequest()
    }
  } else {
    rawRequest = buildDefaultRawRequest()
  }

  const prettyRequest = formatRepeaterPrettyRequest(rawRequest)
  const requestTab = request?.preferredRequestView ?? defaultRequestTab
  const previewResponse = request?.previewResponse ?? null
  const previewRawResponse = previewResponse?.rawText ?? ''

  return {
    id: generateId(),
    draftId: null,
    mode: 'draft',
    name: targetHost || `Request ${fallbackNameIndex}`,
    sourceRequestId: request?.sourceRequestId ?? null,
    targetHost,
    targetPort,
    useTls,
    overrideSni: false,
    sniHost: '',
    initialRawRequest: rawRequest,
    rawRequest,
    prettyRequest,
    lastCompletedRawResponse: previewRawResponse,
    previousRawResponse: '',
    rawResponse: previewRawResponse,
    requestTab,
    responseTab: defaultResponseTab,
    response: previewResponse,
    isSending: false,
    modified: false,
    userEdited: false,
  }
}
