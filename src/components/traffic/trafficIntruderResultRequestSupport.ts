import {
  buildSourceRequestFromRawRequest,
  extractTargetFromRequest,
} from '@/components/traffic/intruder/http'
import type { HttpExchangeRequest } from '@/components/traffic/http/model'
import type { IntruderAttackResult } from '@/components/traffic/intruder/types'

function escapeSingleQuotes(value: string): string {
  return value.replace(/'/g, "'\\''")
}

export function buildIntruderResultRequest(result: IntruderAttackResult): HttpExchangeRequest | null {
  if (!result.rawRequest.trim()) {
    return null
  }

  const target = extractTargetFromRequest(result.rawRequest, result.finalUrl)
  return buildSourceRequestFromRawRequest(result.rawRequest, target)
}

export function resolveIntruderResultRequestUrl(result: IntruderAttackResult): string {
  const request = buildIntruderResultRequest(result)
  return request?.absoluteUrl ?? result.finalUrl ?? ''
}

export function buildIntruderResultCurlCommand(result: IntruderAttackResult): string | null {
  const request = buildIntruderResultRequest(result)
  if (!request) {
    return null
  }

  let curl = `curl -X ${request.request.method} '${escapeSingleQuotes(request.absoluteUrl)}'`

  for (const header of request.request.headers) {
    curl += ` \\\n  -H '${escapeSingleQuotes(`${header.name}: ${header.value}`)}'`
  }

  if (request.request.bodyText) {
    curl += ` \\\n  -d '${escapeSingleQuotes(request.request.bodyText)}'`
  }

  return curl
}
