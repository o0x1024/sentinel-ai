import { computed, ref } from 'vue'

const SEARCH_ANALYTICS_STORAGE_KEY = 'sentinel-search-analytics-events'
const MAX_SEARCH_ANALYTICS_EVENTS = 120

export type SearchAnalyticsEventType = 'search-submit' | 'result-open' | 'command-execute'

export interface SearchAnalyticsEvent {
  id: string
  type: SearchAnalyticsEventType
  query: string
  entryId?: string
  entryTitle?: string
  entryCategory?: string
  createdAt: number
}

export interface SearchAnalyticsSummaryItem {
  value: string
  count: number
}

export interface SearchAnalyticsSummary {
  totalSearches: number
  totalCommands: number
  totalOpens: number
  topQueries: SearchAnalyticsSummaryItem[]
  topCommands: SearchAnalyticsSummaryItem[]
  topOpenedEntries: SearchAnalyticsSummaryItem[]
}

function isBrowser() {
  return typeof window !== 'undefined'
}

function normalizeSearchAnalyticsEvent(event: SearchAnalyticsEvent) {
  return {
    id: String(event.id || '').trim(),
    type: event.type,
    query: String(event.query || '').trim(),
    entryId: String(event.entryId || '').trim() || undefined,
    entryTitle: String(event.entryTitle || '').trim() || undefined,
    entryCategory: String(event.entryCategory || '').trim() || undefined,
    createdAt: Number(event.createdAt || Date.now()),
  }
}

function createSearchAnalyticsId() {
  return `${Date.now()}-${Math.random().toString(36).slice(2, 10)}`
}

function loadSearchAnalyticsEvents() {
  if (!isBrowser()) {
    return []
  }

  try {
    const raw = window.localStorage.getItem(SEARCH_ANALYTICS_STORAGE_KEY)
    if (!raw) {
      return []
    }

    const parsed = JSON.parse(raw)
    if (!Array.isArray(parsed)) {
      return []
    }

    return parsed
      .map(item => normalizeSearchAnalyticsEvent(item as SearchAnalyticsEvent))
      .filter(item => item.id && item.query)
      .slice(0, MAX_SEARCH_ANALYTICS_EVENTS)
  } catch (error) {
    console.warn('[searchAnalytics] Failed to load analytics events:', error)
    return []
  }
}

function saveSearchAnalyticsEvents(events: SearchAnalyticsEvent[]) {
  if (!isBrowser()) {
    return
  }

  window.localStorage.setItem(
    SEARCH_ANALYTICS_STORAGE_KEY,
    JSON.stringify(events.slice(0, MAX_SEARCH_ANALYTICS_EVENTS)),
  )
}

function summarizeTopValues(values: string[]) {
  const counts = new Map<string, number>()
  values.forEach((value) => {
    const normalizedValue = String(value || '').trim()
    if (!normalizedValue) {
      return
    }

    counts.set(normalizedValue, (counts.get(normalizedValue) || 0) + 1)
  })

  return Array.from(counts.entries())
    .map(([value, count]) => ({ value, count }))
    .sort((left, right) => right.count - left.count || left.value.localeCompare(right.value, 'zh-CN'))
    .slice(0, 5)
}

const searchAnalyticsEventsState = ref<SearchAnalyticsEvent[]>(loadSearchAnalyticsEvents())

export function recordSearchAnalyticsEvent(
  type: SearchAnalyticsEventType,
  payload: Omit<SearchAnalyticsEvent, 'id' | 'type' | 'createdAt'> & { createdAt?: number },
) {
  const event = normalizeSearchAnalyticsEvent({
    id: createSearchAnalyticsId(),
    type,
    createdAt: payload.createdAt || Date.now(),
    ...payload,
  })

  searchAnalyticsEventsState.value = [
    event,
    ...searchAnalyticsEventsState.value,
  ].slice(0, MAX_SEARCH_ANALYTICS_EVENTS)
  saveSearchAnalyticsEvents(searchAnalyticsEventsState.value)
  return event
}

export function clearSearchAnalyticsEvents() {
  searchAnalyticsEventsState.value = []
  saveSearchAnalyticsEvents([])
  return searchAnalyticsEventsState.value
}

export function buildSearchAnalyticsSummary(events: SearchAnalyticsEvent[]) {
  const summary: SearchAnalyticsSummary = {
    totalSearches: events.filter(event => event.type === 'search-submit').length,
    totalCommands: events.filter(event => event.type === 'command-execute').length,
    totalOpens: events.filter(event => event.type === 'result-open').length,
    topQueries: summarizeTopValues(
      events
        .filter(event => event.type === 'search-submit')
        .map(event => event.query),
    ),
    topCommands: summarizeTopValues(
      events
        .filter(event => event.type === 'command-execute')
        .map(event => event.query),
    ),
    topOpenedEntries: summarizeTopValues(
      events
        .filter(event => event.type === 'result-open')
        .map(event => event.entryTitle || event.entryId || ''),
    ),
  }

  return summary
}

export function useSearchAnalytics() {
  return {
    events: computed(() => searchAnalyticsEventsState.value),
    summary: computed(() => buildSearchAnalyticsSummary(searchAnalyticsEventsState.value)),
    recordEvent: recordSearchAnalyticsEvent,
    clearEvents: clearSearchAnalyticsEvents,
  }
}
