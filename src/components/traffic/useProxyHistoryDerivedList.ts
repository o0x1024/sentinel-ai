import { computed, ref, watch, type ComputedRef, type Ref } from 'vue'
import {
  filterProxyRequests,
  matchesProxyHistoryRequest,
} from './proxyHistoryFilterSupport'
import {
  compareProxyHistoryRequests,
  isProxyHistoryDefaultSort,
  sortProxyHistoryRequests,
} from './proxyHistoryTableSupport'
import type {
  ProxyHistoryFilterCache,
  ProxyHistoryFilterConfig,
  ProxyHistorySortState,
  ProxyRequest,
} from './proxyHistoryTypes'

type Params = {
  requests: Ref<ProxyRequest[]>
  shouldBypassFrontendFilters: ComputedRef<boolean>
  effectiveFilterConfig: ComputedRef<ProxyHistoryFilterConfig>
  filterCache: ComputedRef<ProxyHistoryFilterCache>
  sortState: Ref<ProxyHistorySortState>
}

type DerivedStep = {
  type: 'stable' | 'prepend' | 'append' | 'full'
  addedFrontCount: number
  addedTailStart: number
  preservedCount: number
}

const areIdsEqual = (left: number[], right: number[]) =>
  left.length === right.length && left.every((id, index) => id === right[index])

const classifyRequestIdStep = (previousIds: number[], nextIds: number[]): DerivedStep => {
  if (areIdsEqual(previousIds, nextIds)) {
    return {
      type: 'stable',
      addedFrontCount: 0,
      addedTailStart: nextIds.length,
      preservedCount: nextIds.length,
    }
  }

  for (let addedFrontCount = 1; addedFrontCount <= nextIds.length; addedFrontCount += 1) {
    const preservedCount = nextIds.length - addedFrontCount
    if (preservedCount > previousIds.length) continue

    let matches = true
    for (let index = 0; index < preservedCount; index += 1) {
      if (nextIds[addedFrontCount + index] !== previousIds[index]) {
        matches = false
        break
      }
    }

    if (matches) {
      return {
        type: 'prepend',
        addedFrontCount,
        addedTailStart: nextIds.length,
        preservedCount,
      }
    }
  }

  if (nextIds.length >= previousIds.length) {
    let matchesPrefix = true
    for (let index = 0; index < previousIds.length; index += 1) {
      if (nextIds[index] !== previousIds[index]) {
        matchesPrefix = false
        break
      }
    }

    if (matchesPrefix) {
      return {
        type: 'append',
        addedFrontCount: 0,
        addedTailStart: previousIds.length,
        preservedCount: previousIds.length,
      }
    }
  }

  return {
    type: 'full',
    addedFrontCount: 0,
    addedTailStart: nextIds.length,
    preservedCount: 0,
  }
}

const insertSortedRequest = (
  sortedRequests: ProxyRequest[],
  request: ProxyRequest,
  sortState: ProxyHistorySortState,
) => {
  let low = 0
  let high = sortedRequests.length

  while (low < high) {
    const middle = Math.floor((low + high) / 2)
    if (compareProxyHistoryRequests(request, sortedRequests[middle], sortState) < 0) {
      high = middle
    } else {
      low = middle + 1
    }
  }

  sortedRequests.splice(low, 0, request)
}

const insertSortedRequests = (
  baseRequests: ProxyRequest[],
  additions: ProxyRequest[],
  sortState: ProxyHistorySortState,
) => {
  const nextSorted = baseRequests.slice()
  additions.forEach((request) => insertSortedRequest(nextSorted, request, sortState))
  return nextSorted
}

const buildFilterSignature = (
  bypassFilters: boolean,
  filterConfig: ProxyHistoryFilterConfig,
) => JSON.stringify([bypassFilters, filterConfig])

const buildSortSignature = (sortState: ProxyHistorySortState) =>
  `${sortState.columnId}:${sortState.direction}`

export const useProxyHistoryDerivedList = (params: Params) => {
  const filteredRequests = ref<ProxyRequest[]>([])
  const sortedRequests = ref<ProxyRequest[]>([])

  let lastFilterSignature = ''
  let lastSortSignature = ''

  const requestIds = computed(() => params.requests.value.map((request) => request.id))
  const filterSignature = computed(() =>
    buildFilterSignature(
      params.shouldBypassFrontendFilters.value,
      params.effectiveFilterConfig.value,
    ),
  )
  const sortSignature = computed(() => buildSortSignature(params.sortState.value))

  const recomputeAll = () => {
    const nextFiltered = params.shouldBypassFrontendFilters.value
      ? params.requests.value
      : filterProxyRequests(
        params.requests.value,
        params.effectiveFilterConfig.value,
        params.filterCache.value,
      )

    filteredRequests.value = nextFiltered
    sortedRequests.value = sortProxyHistoryRequests(nextFiltered, params.sortState.value)
    lastFilterSignature = filterSignature.value
    lastSortSignature = sortSignature.value
  }

  watch([filterSignature, sortSignature], recomputeAll, { immediate: true })

  watch(
    () => params.requests.value,
    (nextRequests, previousRequests) => {
      if (nextRequests === previousRequests) {
        return
      }

      const nextIds = nextRequests.map((request) => request.id)
      const previousIds = (previousRequests || []).map((request) => request.id)

      if (areIdsEqual(nextIds, previousIds)) {
        recomputeAll()
      }
    },
  )

  watch(requestIds, (nextIds, previousIds = []) => {
    if (
      lastFilterSignature !== filterSignature.value
      || lastSortSignature !== sortSignature.value
    ) {
      recomputeAll()
      return
    }

    const step = classifyRequestIdStep(previousIds, nextIds)
    if (step.type === 'stable') {
      return
    }
    if (step.type === 'full') {
      recomputeAll()
      return
    }

    const nextRequests = params.requests.value
    const removedIds = new Set(previousIds.slice(step.preservedCount))
    const baseFiltered = removedIds.size === 0
      ? filteredRequests.value
      : filteredRequests.value.filter((request) => !removedIds.has(request.id))

    let addedRequests: ProxyRequest[] = []
    if (step.type === 'prepend') {
      addedRequests = nextRequests.slice(0, step.addedFrontCount)
    } else if (step.type === 'append') {
      addedRequests = nextRequests.slice(step.addedTailStart)
    }

    const matchedAddedRequests = params.shouldBypassFrontendFilters.value
      ? addedRequests
      : addedRequests.filter((request) =>
        matchesProxyHistoryRequest(
          request,
          params.effectiveFilterConfig.value,
          params.filterCache.value,
        ))

    filteredRequests.value = step.type === 'prepend'
      ? [...matchedAddedRequests, ...baseFiltered]
      : [...baseFiltered, ...matchedAddedRequests]

    if (isProxyHistoryDefaultSort(params.sortState.value)) {
      sortedRequests.value = filteredRequests.value
      return
    }

    const baseSorted = removedIds.size === 0
      ? sortedRequests.value
      : sortedRequests.value.filter((request) => !removedIds.has(request.id))
    sortedRequests.value = matchedAddedRequests.length === 0
      ? baseSorted
      : insertSortedRequests(baseSorted, matchedAddedRequests, params.sortState.value)
  })

  return {
    filteredRequests,
    sortedRequests,
  }
}
