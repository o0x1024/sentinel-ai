import type { HttpReplayResponse } from './http/model'
import { findHeaderValue } from './http/headers'
import type { TrafficDisplaySettings } from './trafficDisplaySettings'
import { formatTrafficJsonBody } from './trafficJsonFormattingSupport'
import { resolveTrafficResponseBodyText } from './trafficResponseDecodingSupport'

export function formatRepeaterPrettyResponse(
  response: HttpReplayResponse | null | undefined,
  settings: TrafficDisplaySettings,
): string {
  if (!response) {
    return ''
  }

  const contentType = findHeaderValue(response.headers, 'content-type') || ''
  const displayBody = resolveTrafficResponseBodyText(
    response.bodyText || '',
    contentType,
    settings,
    response.bodyBytesBase64,
  )
  const body = contentType.includes('json')
    ? formatTrafficJsonBody(displayBody)
    : displayBody

  return [
    `${response.versionObserved || 'HTTP/1.1'} ${response.statusCode} ${response.statusText || ''}`.trimEnd(),
    ...response.headers.map(header => `${header.name}: ${header.value}`),
    '',
    body,
  ].join('\r\n')
}
