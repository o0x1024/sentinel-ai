import { hasEditedResponse } from './proxyHistoryFormattingSupport'
import type { ProxyHistoryViewMode, ProxyRequest } from './proxyHistoryTypes'

export type ProxyHistoryResponseBodyVariant = 'original' | 'edited'

export function resolveProxyHistoryResponseBodyLoadVariant(
  request: ProxyRequest | null,
  viewMode: ProxyHistoryViewMode,
): ProxyHistoryResponseBodyVariant | null {
  if (!request || request.has_full_details) {
    return null
  }

  if (viewMode === 'edited' && hasEditedResponse(request)) {
    if (request.edited_response_body_loaded === false) {
      return 'edited'
    }

    return request.response_body_loaded === false ? 'original' : null
  }

  return request.response_body_loaded === false ? 'original' : null
}
