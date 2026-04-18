import { ref } from 'vue'
import type { HttpExchangeRequest } from './http/model'
import type {
  TrafficWorkbenchBasketItem,
  TrafficWorkbenchSource,
} from './trafficWorkbenchTypes'

const BASKET_STORAGE_KEY = 'sentinel:traffic-workbench-basket:v1'

const buildBasketId = (request: HttpExchangeRequest, source: TrafficWorkbenchSource) =>
  `${source.kind}:${request.absoluteUrl}:${request.request.method}:${source.requestId ?? 'none'}`

export const useTrafficWorkbenchBasket = () => {
  const basketItems = ref<TrafficWorkbenchBasketItem[]>(loadInitialItems())

  function loadInitialItems(): TrafficWorkbenchBasketItem[] {
    if (typeof window === 'undefined') {
      return []
    }

    try {
      const parsed = JSON.parse(window.localStorage.getItem(BASKET_STORAGE_KEY) || '[]')
      if (!Array.isArray(parsed)) {
        return []
      }

      return parsed.filter(isValidBasketItem)
    } catch {
      return []
    }
  }

  function isValidBasketItem(item: unknown): item is TrafficWorkbenchBasketItem {
    if (!item || typeof item !== 'object') {
      return false
    }

    const candidate = item as Partial<TrafficWorkbenchBasketItem>
    return Boolean(
      typeof candidate.id === 'string'
      && typeof candidate.name === 'string'
      && typeof candidate.host === 'string'
      && typeof candidate.createdAt === 'number'
      && candidate.request
      && typeof candidate.request.absoluteUrl === 'string'
      && typeof candidate.request.request?.method === 'string'
      && candidate.source
      && typeof candidate.source.label === 'string',
    )
  }

  function persist() {
    window.localStorage.setItem(BASKET_STORAGE_KEY, JSON.stringify(basketItems.value))
  }

  function addRequest(
    request: HttpExchangeRequest,
    source: TrafficWorkbenchSource,
    options?: { requestId?: number; title?: string },
  ) {
    const id = buildBasketId(request, source)
    if (basketItems.value.some((item) => item.id === id)) {
      return false
    }

    basketItems.value = [
      {
        id,
        name: options?.title || request.absoluteUrl,
        host: request.endpoint.host || '',
        request,
        requestId: options?.requestId,
        source,
        createdAt: Date.now(),
      },
      ...basketItems.value,
    ]
    persist()
    return true
  }

  function removeItem(id: string) {
    basketItems.value = basketItems.value.filter((item) => item.id !== id)
    persist()
  }

  function clear() {
    basketItems.value = []
    persist()
  }

  return {
    basketItems,
    addRequest,
    removeItem,
    clear,
  }
}
