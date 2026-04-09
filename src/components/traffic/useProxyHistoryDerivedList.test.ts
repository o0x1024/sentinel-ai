import { computed, nextTick, ref } from 'vue'
import { describe, expect, it } from 'vitest'
import { buildDefaultProxyHistoryFilterConfig, buildProxyHistoryFilterCache } from './proxyHistoryFilterSupport'
import { useProxyHistoryDerivedList } from './useProxyHistoryDerivedList'
import type { ProxyHistorySortState, ProxyRequest } from './proxyHistoryTypes'

const createRequest = (id: number, overrides: Partial<ProxyRequest> = {}): ProxyRequest => ({
  id,
  url: `https://example.com/${id}`,
  host: 'example.com',
  protocol: 'https',
  method: 'GET',
  status_code: 200,
  response_size: 128,
  response_time: 25,
  timestamp: `2026-04-09T00:00:${String(id).padStart(2, '0')}Z`,
  ...overrides,
})

describe('useProxyHistoryDerivedList', () => {
  it('does not duplicate prepended requests when the source list changes', async () => {
    const requests = ref<ProxyRequest[]>([
      createRequest(24),
      createRequest(23),
      createRequest(22),
    ])
    const sortState = ref<ProxyHistorySortState>({
      columnId: 'id',
      direction: 'desc',
    })
    const filterConfig = computed(() => buildDefaultProxyHistoryFilterConfig())
    const filterCache = computed(() => buildProxyHistoryFilterCache(filterConfig.value))

    const { filteredRequests, sortedRequests } = useProxyHistoryDerivedList({
      requests,
      shouldBypassFrontendFilters: computed(() => true),
      effectiveFilterConfig: filterConfig,
      filterCache,
      sortState,
    })

    await nextTick()

    requests.value = [
      createRequest(26),
      createRequest(25),
      ...requests.value,
    ]

    await nextTick()

    expect(filteredRequests.value.map((request) => request.id)).toEqual([26, 25, 24, 23, 22])
    expect(sortedRequests.value.map((request) => request.id)).toEqual([26, 25, 24, 23, 22])
  })

  it('recomputes rows when request details change without changing ids', async () => {
    const requests = ref<ProxyRequest[]>([
      createRequest(10, { title: 'before' }),
    ])
    const sortState = ref<ProxyHistorySortState>({
      columnId: 'id',
      direction: 'desc',
    })
    const filterConfig = computed(() => buildDefaultProxyHistoryFilterConfig())
    const filterCache = computed(() => buildProxyHistoryFilterCache(filterConfig.value))

    const { filteredRequests } = useProxyHistoryDerivedList({
      requests,
      shouldBypassFrontendFilters: computed(() => true),
      effectiveFilterConfig: filterConfig,
      filterCache,
      sortState,
    })

    await nextTick()

    requests.value = [
      createRequest(10, { title: 'after', has_full_details: true }),
    ]

    await nextTick()

    expect(filteredRequests.value).toHaveLength(1)
    expect(filteredRequests.value[0]).toMatchObject({
      id: 10,
      title: 'after',
      has_full_details: true,
    })
  })
})
