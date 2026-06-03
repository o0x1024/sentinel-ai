import { computed, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { CodecRequestMeta, CodecResult, CodecStep } from './trafficCodecTypes'
import { useTrafficCodecRuleStore } from './trafficCodecRuleStore'
import { TrafficCodecCache } from './trafficCodecCache'

const codecViewEnabled = ref(true)
const decodeCache = new TrafficCodecCache()
const encodeCache = new TrafficCodecCache()

export function useTrafficCodec() {
  const store = useTrafficCodecRuleStore()
  store.ensureLoaded()

  const enabledRules = computed(() => store.rules.value.filter(r => r.enabled))

  async function decode(content: string, meta: CodecRequestMeta): Promise<CodecResult> {
    if (!codecViewEnabled.value || enabledRules.value.length === 0) {
      return { success: true, content, appliedRuleIds: [] }
    }
    const cacheKey = buildCacheKey(content, meta, 'decode')
    const cached = decodeCache.get(cacheKey)
    if (cached !== null) {
      return { success: true, content: cached, appliedRuleIds: [] }
    }
    const response = await invoke<{ data: CodecResult }>('codec_decode', { content, meta })
    const result = response.data
    if (result.success) {
      decodeCache.set(cacheKey, result.content)
    }
    return result
  }

  async function encode(content: string, meta: CodecRequestMeta): Promise<CodecResult> {
    if (!codecViewEnabled.value || enabledRules.value.length === 0) {
      return { success: true, content, appliedRuleIds: [] }
    }
    const cacheKey = buildCacheKey(content, meta, 'encode')
    const cached = encodeCache.get(cacheKey)
    if (cached !== null) {
      return { success: true, content: cached, appliedRuleIds: [] }
    }
    const response = await invoke<{ data: CodecResult }>('codec_encode', { content, meta })
    const result = response.data
    if (result.success) {
      encodeCache.set(cacheKey, result.content)
    }
    return result
  }

  async function batchEncode(contents: string[], meta: CodecRequestMeta): Promise<CodecResult[]> {
    if (!codecViewEnabled.value || enabledRules.value.length === 0) {
      return contents.map(c => ({ success: true, content: c, appliedRuleIds: [] }))
    }
    const response = await invoke<{ data: CodecResult[] }>('codec_batch_encode', { contents, meta })
    return response.data
  }

  async function testPipeline(content: string, steps: CodecStep[], direction: 'decode' | 'encode'): Promise<CodecResult> {
    const response = await invoke<{ data: CodecResult }>('codec_test_pipeline', { content, steps, direction })
    return response.data
  }

  function hasActiveCodec(meta: CodecRequestMeta): boolean {
    return enabledRules.value.some(rule => {
      const hostOk = rule.match.hosts.length === 0 || rule.match.hosts.some(h => globMatch(h, meta.host))
      const pathOk = rule.match.paths.length === 0 || rule.match.paths.some(p => globMatch(p, meta.path))
      return hostOk && pathOk
    })
  }

  function invalidateCache() {
    decodeCache.clear()
    encodeCache.clear()
  }

  return {
    decode,
    encode,
    batchEncode,
    testPipeline,
    hasActiveCodec,
    codecViewEnabled,
    invalidateCache,
    rules: store,
  }
}

function buildCacheKey(content: string, meta: CodecRequestMeta, direction: string): string {
  const hash = simpleHash(content)
  return `${direction}:${meta.host}:${meta.path}:${hash}`
}

function simpleHash(str: string): string {
  let hash = 0
  for (let i = 0; i < Math.min(str.length, 1000); i++) {
    const char = str.charCodeAt(i)
    hash = ((hash << 5) - hash) + char
    hash = hash & hash
  }
  return hash.toString(36)
}

function globMatch(pattern: string, value: string): boolean {
  if (pattern === '*' || pattern === '**') return true
  const p = pattern.toLowerCase()
  const v = value.toLowerCase()
  if (p.startsWith('*.')) {
    return v.endsWith(p.slice(1)) || v === p.slice(2)
  }
  if (p.endsWith('/**')) {
    return v.startsWith(p.slice(0, -3))
  }
  const regex = new RegExp('^' + p.replace(/[.+^${}()|[\]\\]/g, '\\$&').replace(/\*/g, '.*').replace(/\?/g, '.') + '$')
  return regex.test(v)
}
