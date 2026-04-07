import {
  buildSourceRequestFromRawRequest,
  extractTargetFromRequest,
} from '@/components/traffic/intruder/http'
import type { IntruderAttackResult, IntruderSourceRequest } from '@/components/traffic/intruder/types'

function escapeSingleQuotes(value: string): string {
  return value.replace(/'/g, "'\\''")
}

export function buildIntruderResultRequest(result: IntruderAttackResult): IntruderSourceRequest | null {
  if (!result.rawRequest.trim()) {
    return null
  }

  const target = extractTargetFromRequest(result.rawRequest, result.finalUrl)
  return buildSourceRequestFromRawRequest(result.rawRequest, target)
}

export function resolveIntruderResultRequestUrl(result: IntruderAttackResult): string {
  const request = buildIntruderResultRequest(result)
  return request?.url ?? result.finalUrl ?? ''
}

export function buildIntruderResultCurlCommand(result: IntruderAttackResult): string | null {
  const request = buildIntruderResultRequest(result)
  if (!request) {
    return null
  }

  let curl = `curl -X ${request.method} '${escapeSingleQuotes(request.url)}'`

  for (const [key, value] of Object.entries(request.headers)) {
    curl += ` \\\n  -H '${escapeSingleQuotes(`${key}: ${value}`)}'`
  }

  if (request.body) {
    curl += ` \\\n  -d '${escapeSingleQuotes(request.body)}'`
  }

  return curl
}
