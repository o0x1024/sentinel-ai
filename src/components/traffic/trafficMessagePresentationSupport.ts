import type { RepeaterRequestTab, RepeaterResponseTab } from './proxyRepeaterTypes'
import type { ProxyHistoryRequestTab, ProxyHistoryResponseTab, ProxyHistoryViewMode } from './proxyHistoryTypes'

export type TrafficTextDisplayMode = 'pretty' | 'raw'

export function resolveTrafficTextDisplayMode(
  tab: 'pretty' | 'raw' | 'hex' | 'render' | 'diff' | 'plain',
): TrafficTextDisplayMode {
  return tab === 'pretty' ? 'pretty' : 'raw'
}

export function buildHistoryRequestStateKey(
  requestId: number | null | undefined,
  tab: ProxyHistoryRequestTab,
  viewMode: ProxyHistoryViewMode,
) {
  if (!requestId) return ''
  return `history:request:${requestId}:${tab}:${viewMode}`
}

export function buildHistoryResponseStateKey(
  requestId: number | null | undefined,
  tab: ProxyHistoryResponseTab,
  viewMode: ProxyHistoryViewMode,
) {
  if (!requestId) return ''
  return `history:response:${requestId}:${tab}:${viewMode}`
}

export function buildRepeaterRequestStateKey(tabId: string, tab: RepeaterRequestTab) {
  return `repeater:${tabId}:request:${tab}`
}

export function buildRepeaterResponseStateKey(tabId: string, tab: RepeaterResponseTab) {
  return `repeater:${tabId}:response:${tab}`
}

export function buildInterceptStateKey(
  itemType: 'request' | 'response' | 'websocket',
  itemIndex: number,
  tab: 'pretty' | 'raw' | 'hex',
) {
  return `intercept:${itemType}:${itemIndex}:${tab}`
}

export function buildComparerDraftStateKey(side: 'left' | 'right') {
  return `comparer:draft:${side}`
}

export function buildComparerPlainStateKey(side: 'left' | 'right', viewMode: TrafficTextDisplayMode) {
  return `comparer:plain:${side}:${viewMode}`
}
