import type { ProxyRequest } from '../../proxyHistoryTypes'
import type {
  TrafficWorkbenchBasketItem,
  TrafficWorkbenchSource,
  TrafficWorkbenchToolSession,
} from '../../trafficWorkbenchTypes'

type WorkbenchTool = TrafficWorkbenchToolSession['tool']

export function buildTrafficWorkbenchSource(
  kind: TrafficWorkbenchSource['kind'],
  label: string,
  requestId?: number,
): TrafficWorkbenchSource {
  return { kind, label, requestId }
}

export function getHistorySource(requestOrId?: ProxyRequest | number | null) {
  if (typeof requestOrId === 'number') {
    return buildTrafficWorkbenchSource('history', `历史记录 #${requestOrId}`, requestOrId)
  }

  if (requestOrId) {
    const requestId = requestOrId.db_request_id ?? undefined
    const labelId = requestOrId.db_request_id ?? requestOrId.id
    return buildTrafficWorkbenchSource('history', `历史记录 #${labelId}`, requestId)
  }

  return buildTrafficWorkbenchSource('history', '历史记录')
}

export function getInterceptSource() {
  return buildTrafficWorkbenchSource('intercept', '拦截队列')
}

export function getBasketSource(item?: TrafficWorkbenchBasketItem) {
  return buildTrafficWorkbenchSource(
    'basket',
    item?.requestId ? `请求篮子 -> #${item.requestId}` : '请求篮子',
    item?.requestId,
  )
}

export function getToolSource(tool: WorkbenchTool, title: string) {
  return buildTrafficWorkbenchSource(tool, title)
}

export function formatHistoryOriginLabel(originKind?: string | null) {
  switch (originKind) {
    case 'draft':
      return '重放器历史执行'
    case 'attack-workspace':
      return '爆破器历史'
    default:
      return originKind || ''
  }
}

export function formatHistoryOriginSummary(request: ProxyRequest) {
  if (!request.origin_kind) {
    return ''
  }

  const parts = [formatHistoryOriginLabel(request.origin_kind)]
  if (request.origin_ref_id) {
    parts.push(request.origin_ref_id)
  }
  if (request.parent_request_id) {
    parts.push(`父请求 #${request.parent_request_id}`)
  }
  if (request.source_draft_revision_id) {
    parts.push(`修订 ${request.source_draft_revision_id}`)
  }
  return parts.join(' · ')
}
