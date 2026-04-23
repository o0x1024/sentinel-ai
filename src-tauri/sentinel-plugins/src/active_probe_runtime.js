const ACTIVE_PROBE_DEFAULTS = {
  jitterRange: [300, 1000],
  minHostCooldownMs: 1000,
  maxConcurrentPerHost: 2,
  timeoutMs: 8000,
}

const FAST_PROBE_CLASS = 'fast'
const SLOW_PROBE_CLASS = 'slow'
const SLOW_PROBE_EXTRA_COOLDOWN_MS = 1500
const ADAPTIVE_PENALTY_MAX_MS = 15000

const activeProbeKeyStates = new Map()
const activeProbeKeyNextDispatchAt = new Map()
const activeProbeAdaptiveStates = new Map()

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

export function emitActiveProbeEvent(payload) {
  try {
    Deno.core.ops.op_emit_active_probe_event(payload)
  } catch (error) {
    Sentinel.log('debug', `Failed to emit active probe event: ${String(error)}`)
  }
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
    key: source.cooldownKey || init.cooldownKey || buildDefaultActiveProbeKey(url),
    jitterRange: [jitterMin, jitterMax],
    minHostCooldownMs: clampPositiveInteger(
      source.minHostCooldownMs ?? init.minHostCooldownMs,
      defaults.minHostCooldownMs,
    ),
    maxConcurrentPerHost: Math.max(
      1,
      clampPositiveInteger(
        source.maxConcurrentPerHost ?? init.maxConcurrentPerHost,
        defaults.maxConcurrentPerHost,
      ),
    ),
    timeoutMs: defaults.timeoutMs,
    priority:
      typeof source.priority === 'number' && Number.isFinite(source.priority)
        ? Math.trunc(source.priority)
        : 0,
    probeClass,
  }
}

function randomIntInclusive(min, max) {
  if (max <= min) {
    return min
  }
  return Math.floor(Math.random() * (max - min + 1)) + min
}

function getOrCreateActiveProbeKeyState(key) {
  let state = activeProbeKeyStates.get(key)
  if (!state) {
    state = {
      running: 0,
      waiters: [],
      sequence: 0,
    }
    activeProbeKeyStates.set(key, state)
  }
  return state
}

function getOrCreateAdaptiveState(key) {
  let state = activeProbeAdaptiveStates.get(key)
  if (!state) {
    state = {
      penaltyMs: 0,
    }
    activeProbeAdaptiveStates.set(key, state)
  }
  return state
}

function computeAdaptivePenalty(key) {
  return getOrCreateAdaptiveState(key).penaltyMs
}

function chooseNextWaiter(state) {
  if (state.waiters.length === 0) {
    return null
  }

  state.waiters.sort((left, right) => {
    if (right.priority !== left.priority) {
      return right.priority - left.priority
    }
    return left.sequence - right.sequence
  })

  return state.waiters.shift() || null
}

async function acquireActiveProbeSlot(key, maxConcurrentPerHost, priority) {
  const state = getOrCreateActiveProbeKeyState(key)
  const queuedAhead = state.running >= maxConcurrentPerHost ? state.waiters.length + 1 : 0
  if (state.running >= maxConcurrentPerHost) {
    await new Promise(resolve => {
      state.waiters.push({
        resolve,
        priority,
        sequence: state.sequence++,
      })
    })
  }

  state.running += 1

  return {
    queuedAhead,
    runningAfterAcquire: state.running,
    release() {
      state.running = Math.max(0, state.running - 1)
      const nextWaiter = chooseNextWaiter(state)
      if (nextWaiter) {
        nextWaiter.resolve()
        return
      }

      if (state.running === 0) {
        activeProbeKeyStates.delete(key)
      }
    },
  }
}

function buildEffectiveSchedulingPolicy(options) {
  const adaptivePenaltyMs = computeAdaptivePenalty(options.key)
  const classPenaltyMs = options.probeClass === SLOW_PROBE_CLASS ? SLOW_PROBE_EXTRA_COOLDOWN_MS : 0

  return {
    effectiveCooldownMs: options.minHostCooldownMs + adaptivePenaltyMs + classPenaltyMs,
    effectiveConcurrency: options.probeClass === SLOW_PROBE_CLASS
      ? 1
      : options.maxConcurrentPerHost,
    adaptivePenaltyMs,
  }
}

function buildSchedulingState(options, slot, policy) {
  return {
    cooldown_key: options.key,
    active_slots: slot.runningAfterAcquire,
    max_concurrent_per_host: policy.effectiveConcurrency,
    queue_depth: slot.queuedAhead,
    adaptive_penalty_ms: policy.adaptivePenaltyMs,
    probe_class: options.probeClass,
    probe_priority: options.priority,
  }
}

export function reportActiveProbeOutcome(options, outcome = {}) {
  if (!options?.key) {
    return
  }

  const state = getOrCreateAdaptiveState(options.key)
  const status = typeof outcome.status === 'number' ? outcome.status : 0
  const errorText = String(outcome.error || '').toLowerCase()
  const responseElapsedMs =
    typeof outcome.responseElapsedMs === 'number' && Number.isFinite(outcome.responseElapsedMs)
      ? outcome.responseElapsedMs
      : 0

  const isHardFailure =
    status === 429
    || status >= 500
    || errorText.includes('timeout')
    || errorText.includes('aborted')
    || Boolean(errorText)
    || responseElapsedMs >= Math.max(options.timeoutMs - 500, options.timeoutMs * 0.85)

  if (isHardFailure) {
    state.penaltyMs = Math.min(
      ADAPTIVE_PENALTY_MAX_MS,
      Math.max(400, state.penaltyMs > 0 ? state.penaltyMs * 2 : 400),
    )
    return
  }

  state.penaltyMs = Math.max(0, Math.floor(state.penaltyMs * 0.5))
  if (state.penaltyMs === 0) {
    activeProbeAdaptiveStates.delete(options.key)
  }
}

export async function scheduleActiveProbe(url, init, task, metadata = null) {
  const options = normalizeActiveProbeOptions(url, init)
  if (!options) {
    return await task()
  }

  const initialState = getOrCreateActiveProbeKeyState(options.key)
  if (metadata) {
    emitActiveProbeEvent({
      ...metadata,
      phase: 'queued',
      cooldown_key: options.key,
      active_slots: initialState.running,
      max_concurrent_per_host: options.maxConcurrentPerHost,
      queue_depth: initialState.waiters.length,
      adaptive_penalty_ms: computeAdaptivePenalty(options.key),
      probe_class: options.probeClass,
      probe_priority: options.priority,
    })
  }

  const policy = buildEffectiveSchedulingPolicy(options)
  const slot = await acquireActiveProbeSlot(options.key, policy.effectiveConcurrency, options.priority)

  try {
    const schedulingState = buildSchedulingState(options, slot, policy)
    const jitterWait = randomIntInclusive(
      options.jitterRange[0],
      options.jitterRange[1],
    )
    const now = Date.now()
    const earliestDispatchAt = Math.max(
      now,
      activeProbeKeyNextDispatchAt.get(options.key) || 0,
    )
    const cooldownWait = Math.max(0, earliestDispatchAt - now)
    const dispatchAt = now + cooldownWait + jitterWait
    activeProbeKeyNextDispatchAt.set(
      options.key,
      dispatchAt + policy.effectiveCooldownMs,
    )

    if (metadata) {
      emitActiveProbeEvent({
        ...metadata,
        phase: 'scheduled',
        ...schedulingState,
        cooldown_wait_ms: cooldownWait,
        jitter_wait_ms: jitterWait,
        total_wait_ms: cooldownWait + jitterWait,
      })
    }

    if (cooldownWait + jitterWait > 0) {
      await globalThis.sleep(cooldownWait + jitterWait)
    }

    if (metadata) {
      emitActiveProbeEvent({
        ...metadata,
        phase: 'dispatching',
        ...schedulingState,
      })
    }

    return await task({
      ...schedulingState,
      total_wait_ms: cooldownWait + jitterWait,
    })
  } finally {
    slot.release()
  }
}
