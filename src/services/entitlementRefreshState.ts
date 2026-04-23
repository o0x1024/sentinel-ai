import { readonly, ref } from 'vue'

const STORAGE_KEY = 'sentinel.entitlement-refresh-runtime'

export interface EntitlementRefreshRuntimeState {
  last_attempt_at: number | null
  last_success_at: number | null
  last_failure_at: number | null
  next_retry_at: number | null
  consecutive_failures: number
  last_error: string | null
  last_error_code: string | null
}

const defaultState = (): EntitlementRefreshRuntimeState => ({
  last_attempt_at: null,
  last_success_at: null,
  last_failure_at: null,
  next_retry_at: null,
  consecutive_failures: 0,
  last_error: null,
  last_error_code: null,
})

const entitlementRefreshRuntimeState = ref<EntitlementRefreshRuntimeState>(loadInitialState())

function loadInitialState(): EntitlementRefreshRuntimeState {
  if (typeof window === 'undefined') {
    return defaultState()
  }

  try {
    const raw = window.localStorage.getItem(STORAGE_KEY)
    if (!raw) {
      return defaultState()
    }

    return {
      ...defaultState(),
      ...(JSON.parse(raw) as Partial<EntitlementRefreshRuntimeState>),
    }
  } catch (error) {
    console.error('Failed to load entitlement refresh runtime state:', error)
    return defaultState()
  }
}

function persistState() {
  if (typeof window === 'undefined') {
    return
  }

  try {
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify(entitlementRefreshRuntimeState.value))
  } catch (error) {
    console.error('Failed to persist entitlement refresh runtime state:', error)
  }
}

function updateState(patch: Partial<EntitlementRefreshRuntimeState>) {
  entitlementRefreshRuntimeState.value = {
    ...entitlementRefreshRuntimeState.value,
    ...patch,
  }
  persistState()
  return entitlementRefreshRuntimeState.value
}

function calculateBackoffSeconds(consecutiveFailures: number) {
  const steps = [300, 900, 1800, 3600]
  return steps[Math.min(Math.max(consecutiveFailures - 1, 0), steps.length - 1)]
}

export function markEntitlementRefreshAttempt(at = currentUnixTimestamp()) {
  return updateState({
    last_attempt_at: at,
  })
}

export function markEntitlementRefreshSuccess(at = currentUnixTimestamp()) {
  return updateState({
    last_attempt_at: at,
    last_success_at: at,
    consecutive_failures: 0,
    last_failure_at: null,
    next_retry_at: null,
    last_error: null,
    last_error_code: null,
  })
}

export function markEntitlementRefreshFailure(params: {
  message: string
  errorCode?: string | null
  retryAfterSecs?: number | null
  at?: number
}) {
  const at = params.at ?? currentUnixTimestamp()
  const consecutiveFailures = entitlementRefreshRuntimeState.value.consecutive_failures + 1
  const retryAfterSecs = normalizeRetryAfter(
    params.retryAfterSecs ?? calculateBackoffSeconds(consecutiveFailures),
  )

  return updateState({
    last_attempt_at: at,
    last_failure_at: at,
    consecutive_failures: consecutiveFailures,
    next_retry_at: at + retryAfterSecs,
    last_error: params.message,
    last_error_code: params.errorCode ?? null,
  })
}

export function resetEntitlementRefreshRuntimeState() {
  entitlementRefreshRuntimeState.value = defaultState()
  persistState()
}

export function canAttemptEntitlementAutoRefresh(at = currentUnixTimestamp()) {
  const nextRetryAt = entitlementRefreshRuntimeState.value.next_retry_at
  return !nextRetryAt || at >= nextRetryAt
}

export function getEntitlementRefreshCooldownSeconds(at = currentUnixTimestamp()) {
  const nextRetryAt = entitlementRefreshRuntimeState.value.next_retry_at
  if (!nextRetryAt || at >= nextRetryAt) {
    return 0
  }

  return nextRetryAt - at
}

export function formatDurationLabel(totalSeconds: number) {
  if (totalSeconds <= 0) {
    return '现在'
  }

  if (totalSeconds < 60) {
    return `${totalSeconds} 秒后`
  }

  if (totalSeconds < 3600) {
    return `${Math.ceil(totalSeconds / 60)} 分钟后`
  }

  return `${Math.ceil(totalSeconds / 3600)} 小时后`
}

export const useEntitlementRefreshRuntimeState = () =>
  readonly(entitlementRefreshRuntimeState)

function normalizeRetryAfter(value: number) {
  return Math.min(Math.max(Math.round(value), 60), 4 * 3600)
}

function currentUnixTimestamp() {
  return Math.floor(Date.now() / 1000)
}
