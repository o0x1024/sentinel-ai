const ACTIVE_PROBE_DEFAULTS = {
  jitterRange: [300, 1000],
  minHostCooldownMs: 1000,
  maxConcurrentPerHost: 2,
  timeoutMs: 8000,
}

const FAST_PROBE_CLASS = 'fast'
const SLOW_PROBE_CLASS = 'slow'

function clampPositiveInteger(value, fallback) {
  return typeof value === 'number' && Number.isFinite(value) && value >= 0
    ? Math.trunc(value)
    : fallback
}

function resolveActiveProbeDefaults() {
  try {
    const settings = Sentinel.Runtime.getSettings()
    const activeProbe = settings && typeof settings === 'object'
      ? settings.activeProbe
      : null

    if (!activeProbe || typeof activeProbe !== 'object') {
      return { ...ACTIVE_PROBE_DEFAULTS }
    }

    const jitterRange = Array.isArray(activeProbe.jitterRange) && activeProbe.jitterRange.length === 2
      ? [
          clampPositiveInteger(activeProbe.jitterRange[0], ACTIVE_PROBE_DEFAULTS.jitterRange[0]),
          clampPositiveInteger(activeProbe.jitterRange[1], ACTIVE_PROBE_DEFAULTS.jitterRange[1]),
        ]
      : [...ACTIVE_PROBE_DEFAULTS.jitterRange]

    return {
      jitterRange,
      minHostCooldownMs: clampPositiveInteger(
        activeProbe.minHostCooldownMs,
        ACTIVE_PROBE_DEFAULTS.minHostCooldownMs,
      ),
      maxConcurrentPerHost: clampPositiveInteger(
        activeProbe.maxConcurrentPerHost,
        ACTIVE_PROBE_DEFAULTS.maxConcurrentPerHost,
      ),
      timeoutMs: clampPositiveInteger(activeProbe.timeoutMs, ACTIVE_PROBE_DEFAULTS.timeoutMs),
    }
  } catch {
    return { ...ACTIVE_PROBE_DEFAULTS }
  }
}

function buildDefaultActiveProbeKey(url) {
  try {
    const parsedUrl = new URL(String(url))
    return `${parsedUrl.host}${parsedUrl.pathname || '/'}`
  } catch {
    return 'global'
  }
}

function getHeaderValue(headers, targetName) {
  const normalizedTarget = String(targetName || '').toLowerCase()
  if (!normalizedTarget) {
    return null
  }

  if (headers instanceof Headers) {
    return headers.get(targetName)
  }

  if (Array.isArray(headers)) {
    for (const entry of headers) {
      if (!Array.isArray(entry) || entry.length < 2) {
        continue
      }
      if (String(entry[0]).toLowerCase() === normalizedTarget) {
        return String(entry[1])
      }
    }
    return null
  }

  if (headers && typeof headers === 'object') {
    for (const [key, value] of Object.entries(headers)) {
      if (String(key).toLowerCase() === normalizedTarget) {
        return String(value)
      }
    }
  }

  return null
}

export function buildActiveProbeMetadata(url, method, headers, requestId, init = {}) {
  const activeProbe = init.activeProbe && typeof init.activeProbe === 'object'
    ? init.activeProbe
    : null

  return {
    request_id: requestId,
    method: String(method || 'GET').toUpperCase(),
    url: String(url),
    probe_label: getHeaderValue(headers, 'x-sentinel-active-probe'),
    target_name: typeof activeProbe?.target_name === 'string' ? activeProbe.target_name : null,
    target_path: typeof activeProbe?.target_path === 'string' ? activeProbe.target_path : null,
    target_location: typeof activeProbe?.target_location === 'string' ? activeProbe.target_location : null,
    probe_value: typeof activeProbe?.probe_value === 'string' ? activeProbe.probe_value : null,
    technique: typeof activeProbe?.technique === 'string' ? activeProbe.technique : null,
    probe_class: typeof activeProbe?.probeClass === 'string' ? activeProbe.probeClass : FAST_PROBE_CLASS,
    probe_priority:
      typeof activeProbe?.priority === 'number' && Number.isFinite(activeProbe.priority)
        ? Math.trunc(activeProbe.priority)
        : 0,
  }
}

export function normalizeActiveProbeOptions(url, init = {}) {
  const defaults = resolveActiveProbeDefaults()
  const source =
    init.activeProbe && typeof init.activeProbe === 'object'
      ? init.activeProbe
      : init.activeProbe
        ? {}
        : null

  if (!source) {
    return null
  }

  const rawRange = source.jitterRange || init.jitterRange || defaults.jitterRange
  const normalizedRange = Array.isArray(rawRange) && rawRange.length === 2
    ? [
        clampPositiveInteger(rawRange[0], defaults.jitterRange[0]),
        clampPositiveInteger(rawRange[1], defaults.jitterRange[1]),
      ]
    : [...defaults.jitterRange]
  const jitterMin = Math.min(normalizedRange[0], normalizedRange[1])
  const jitterMax = Math.max(normalizedRange[0], normalizedRange[1])
  const probeClass = source.probeClass === SLOW_PROBE_CLASS ? SLOW_PROBE_CLASS : FAST_PROBE_CLASS

  return {
    probe_label: typeof source.probeLabel === 'string' ? source.probeLabel : null,
    cooldown_key: source.cooldownKey || init.cooldownKey || buildDefaultActiveProbeKey(url),
    jitter_range: [jitterMin, jitterMax],
    min_host_cooldown_ms: clampPositiveInteger(
      source.minHostCooldownMs ?? init.minHostCooldownMs,
      defaults.minHostCooldownMs,
    ),
    max_concurrent_per_host: Math.max(
      1,
      clampPositiveInteger(
        source.maxConcurrentPerHost ?? init.maxConcurrentPerHost,
        defaults.maxConcurrentPerHost,
      ),
    ),
    timeoutMs: defaults.timeoutMs,
    probe_priority:
      typeof source.priority === 'number' && Number.isFinite(source.priority)
        ? Math.trunc(source.priority)
        : 0,
    probe_class: probeClass,
  }
}
