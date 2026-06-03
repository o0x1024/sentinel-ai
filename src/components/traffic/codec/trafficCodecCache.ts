interface CacheEntry {
  result: string
  timestamp: number
}

const MAX_CACHE_SIZE = 128

export class TrafficCodecCache {
  private cache = new Map<string, CacheEntry>()

  get(key: string): string | null {
    const entry = this.cache.get(key)
    if (!entry) return null
    return entry.result
  }

  set(key: string, result: string) {
    if (this.cache.size >= MAX_CACHE_SIZE) {
      const oldestKey = this.cache.keys().next().value
      if (oldestKey) this.cache.delete(oldestKey)
    }
    this.cache.set(key, { result, timestamp: Date.now() })
  }

  invalidateByRuleId(ruleId: string) {
    for (const key of this.cache.keys()) {
      if (key.includes(ruleId)) {
        this.cache.delete(key)
      }
    }
  }

  clear() {
    this.cache.clear()
  }
}
