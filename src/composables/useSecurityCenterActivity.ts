import { computed, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type { Finding } from '@/components/SecurityCenter/vulnerabilityFindingTypes'
import type { WorkbenchCaseListItem, WorkbenchCaseListResult } from '@/components/SecurityCenter/securityWorkbenchTypes'

const STORAGE_KEY = 'sentinel-security-center-activity'
const SECURITY_ACTIVITY_LIMIT = 500
const MAX_READ_RECORD_IDS = 2000

interface SecurityCenterActivityState {
  baselineSeeded: boolean
  legacyMigrated: boolean
  lastViewedAt: string | null
  readFindingIds: string[]
  readWorkbenchCaseIds: string[]
}

const findings = ref<Finding[]>([])
const workbenchCases = ref<WorkbenchCaseListItem[]>([])
const initialized = ref(false)
const isLoading = ref(false)
const unlisteners: UnlistenFn[] = []
const storedState = loadStoredState()
const baselineSeeded = ref(storedState.baselineSeeded)
const legacyMigrated = ref(storedState.legacyMigrated)
const lastViewedAt = ref<string | null>(storedState.lastViewedAt)
const readFindingIds = ref<string[]>(storedState.readFindingIds)
const readWorkbenchCaseIds = ref<string[]>(storedState.readWorkbenchCaseIds)

function loadStoredState(): SecurityCenterActivityState {
  if (typeof window === 'undefined') {
    return {
      baselineSeeded: false,
      legacyMigrated: false,
      lastViewedAt: null,
      readFindingIds: [],
      readWorkbenchCaseIds: [],
    }
  }

  try {
    const raw = window.localStorage.getItem(STORAGE_KEY)
    if (!raw) {
      return {
        baselineSeeded: false,
        legacyMigrated: false,
        lastViewedAt: null,
        readFindingIds: [],
        readWorkbenchCaseIds: [],
      }
    }

    const parsed = JSON.parse(raw)
    const nextReadFindingIds = Array.isArray(parsed?.readFindingIds)
      ? parsed.readFindingIds.filter((item: unknown): item is string => typeof item === 'string' && item.trim().length > 0)
      : []
    const nextReadWorkbenchCaseIds = Array.isArray(parsed?.readWorkbenchCaseIds)
      ? parsed.readWorkbenchCaseIds.filter((item: unknown): item is string => typeof item === 'string' && item.trim().length > 0)
      : []

    return {
      baselineSeeded: Boolean(parsed?.baselineSeeded)
        || nextReadFindingIds.length > 0
        || nextReadWorkbenchCaseIds.length > 0,
      legacyMigrated: Boolean(parsed?.legacyMigrated),
      lastViewedAt:
        typeof parsed?.lastViewedAt === 'string' && parsed.lastViewedAt.trim()
          ? parsed.lastViewedAt
          : null,
      readFindingIds: nextReadFindingIds,
      readWorkbenchCaseIds: nextReadWorkbenchCaseIds,
    }
  } catch (error) {
    console.warn('[useSecurityCenterActivity] Failed to load stored state:', error)
    return {
      baselineSeeded: false,
      legacyMigrated: false,
      lastViewedAt: null,
      readFindingIds: [],
      readWorkbenchCaseIds: [],
    }
  }
}

function persistState() {
  if (typeof window === 'undefined') {
    return
  }

  try {
    window.localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({
        baselineSeeded: baselineSeeded.value,
        legacyMigrated: legacyMigrated.value,
        lastViewedAt: lastViewedAt.value,
        readFindingIds: readFindingIds.value,
        readWorkbenchCaseIds: readWorkbenchCaseIds.value,
      } satisfies SecurityCenterActivityState),
    )
  } catch (error) {
    console.warn('[useSecurityCenterActivity] Failed to persist state:', error)
  }
}

function toTimestamp(value: string | null | undefined) {
  if (!value) {
    return Number.NaN
  }

  return new Date(value).getTime()
}

function normalizeFinding(item: Finding): Finding | null {
  if (!item?.id || !item?.title) {
    return null
  }

  return {
    ...item,
    evidence: item.evidence || [],
  }
}

function normalizeWorkbenchCase(item: WorkbenchCaseListItem): WorkbenchCaseListItem | null {
  if (!item?.id || !item?.title || !item.finding?.id) {
    return null
  }

  return item
}

function getLatestFindingCreatedAt(items: Finding[]) {
  for (const item of items) {
    if (!Number.isNaN(toTimestamp(item.created_at))) {
      return item.created_at
    }
  }

  return null
}

function appendUniqueIds(currentIds: string[], nextIds: string[]) {
  const merged = new Set(currentIds)
  nextIds.forEach((id) => {
    if (typeof id === 'string' && id.trim()) {
      merged.add(id)
    }
  })

  const deduped = Array.from(merged)
  if (deduped.length <= MAX_READ_RECORD_IDS) {
    return deduped
  }

  return deduped.slice(deduped.length - MAX_READ_RECORD_IDS)
}

function markLocalFindingsRead(ids: string[], viewedAt = new Date().toISOString()) {
  const idSet = new Set(ids)
  findings.value = findings.value.map((item) =>
    idSet.has(item.id)
      ? {
          ...item,
          viewed_at: viewedAt,
          viewed_by: item.viewed_by || 'local',
        }
      : item,
  )
}

async function seedInitialReadStateIfNeeded() {
  if (!legacyMigrated.value && readFindingIds.value.length > 0) {
    await markFindingsAsRead(readFindingIds.value)
    legacyMigrated.value = true
    persistState()
  }

  if (baselineSeeded.value) {
    return
  }

  if (lastViewedAt.value) {
    const viewedTimestamp = toTimestamp(lastViewedAt.value)
    await markFindingsAsRead(
      findings.value
        .filter((item) => {
          const createdAt = toTimestamp(item.created_at)
          return !Number.isNaN(createdAt) && createdAt <= viewedTimestamp
        })
        .map((item) => item.id),
    )
    readWorkbenchCaseIds.value = appendUniqueIds(
      readWorkbenchCaseIds.value,
      workbenchCases.value
        .filter((item) => {
          const lastActivityAt = toTimestamp(item.lastActivityAt)
          return !Number.isNaN(lastActivityAt) && lastActivityAt <= viewedTimestamp
        })
        .map((item) => item.id),
    )
    legacyMigrated.value = true
    baselineSeeded.value = true
    persistState()
    return
  }

  await markFindingsAsRead(findings.value.map((item) => item.id))
  readWorkbenchCaseIds.value = appendUniqueIds(
    readWorkbenchCaseIds.value,
    workbenchCases.value.map((item) => item.id),
  )
  baselineSeeded.value = true
  persistState()
}

async function refreshSecurityCenterActivity() {
  if (isLoading.value) {
    return
  }

  isLoading.value = true
  try {
    const [findingsResult, workbenchCasesResult] = await Promise.allSettled([
      invoke<any>('list_findings', {
        limit: SECURITY_ACTIVITY_LIMIT,
        offset: 0,
        severityFilter: null,
        statusFilter: null,
        statusFilters: null,
        analysisStageFilters: null,
        search: null,
      }),
      invoke<any>('security_workbench_list_cases', {
        request: {
          search: null,
          status: null,
          page: 1,
          pageSize: SECURITY_ACTIVITY_LIMIT,
        },
      }),
    ])

    if (findingsResult.status === 'fulfilled' && findingsResult.value?.success && Array.isArray(findingsResult.value.data)) {
      findings.value = findingsResult.value.data
        .map((item: Finding) => normalizeFinding(item))
        .filter((item: Finding | null): item is Finding => item !== null)
    } else {
      findings.value = []
    }

    if (
      workbenchCasesResult.status === 'fulfilled'
      && workbenchCasesResult.value?.success
      && workbenchCasesResult.value.data
    ) {
      const result = workbenchCasesResult.value.data as WorkbenchCaseListResult
      workbenchCases.value = Array.isArray(result.items)
        ? result.items
            .map((item: WorkbenchCaseListItem) => normalizeWorkbenchCase(item))
            .filter((item: WorkbenchCaseListItem | null): item is WorkbenchCaseListItem => item !== null)
        : []
    } else {
      workbenchCases.value = []
    }

    await seedInitialReadStateIfNeeded()
  } catch (error) {
    console.error('[useSecurityCenterActivity] Failed to refresh activity:', error)
    findings.value = []
    workbenchCases.value = []
  } finally {
    isLoading.value = false
  }
}

async function markFindingsAsRead(ids: string[]) {
  const nextIds = ids.filter((id) => typeof id === 'string' && id.trim())
  if (!nextIds.length) {
    return
  }

  markLocalFindingsRead(nextIds)
  readFindingIds.value = appendUniqueIds(readFindingIds.value, nextIds)
  persistState()
  const response = await invoke<any>('mark_findings_read', { findingIds: nextIds })
  if (!response?.success) {
    throw new Error(response?.error || 'Failed to mark findings as read')
  }
}

function markWorkbenchCasesAsRead(ids: string[]) {
  const nextIds = ids.filter((id) => typeof id === 'string' && id.trim())
  if (!nextIds.length) {
    return
  }

  readWorkbenchCaseIds.value = appendUniqueIds(readWorkbenchCaseIds.value, nextIds)
  persistState()
}

async function markFindingAsRead(id: string) {
  await markFindingsAsRead([id])
}

function markWorkbenchCaseAsRead(id: string) {
  markWorkbenchCasesAsRead([id])
}

function markCurrentAsSeen() {
  void markFindingsAsRead(findings.value.map((item) => item.id))
  markWorkbenchCasesAsRead(workbenchCases.value.map((item) => item.id))
  lastViewedAt.value = getLatestFindingCreatedAt(findings.value) || new Date().toISOString()
  persistState()
}

async function initializeSecurityCenterActivity() {
  if (initialized.value) {
    return
  }

  initialized.value = true
  await refreshSecurityCenterActivity()

  const refreshFromEvent = () => {
    void refreshSecurityCenterActivity()
  }

  unlisteners.push(await listen('scan:finding', refreshFromEvent))
  unlisteners.push(await listen('scan:finding-updated', refreshFromEvent))
  unlisteners.push(await listen('system-agent:verification-complete', refreshFromEvent))
  unlisteners.push(await listen('security-workbench:changed', refreshFromEvent))
}

const readWorkbenchCaseIdSet = computed(() => new Set(readWorkbenchCaseIds.value))
const unreadFindings = computed(() =>
  findings.value.filter((item) => !item.viewed_at),
)
const unreadWorkbenchCases = computed(() =>
  workbenchCases.value.filter((item) => !readWorkbenchCaseIdSet.value.has(item.id)),
)
const unreadFindingCount = computed(() => unreadFindings.value.length)
const unreadWorkbenchCaseCount = computed(() => unreadWorkbenchCases.value.length)
const unreadSecurityCenterCount = computed(() => unreadFindingCount.value + unreadWorkbenchCaseCount.value)

export function useSecurityCenterActivity() {
  return {
    findings: computed(() => findings.value),
    workbenchCases: computed(() => workbenchCases.value),
    isLoading,
    lastViewedAt: computed(() => lastViewedAt.value),
    readFindingIds: computed(() => readFindingIds.value),
    readWorkbenchCaseIds: computed(() => readWorkbenchCaseIds.value),
    unreadFindings,
    unreadWorkbenchCases,
    unreadFindingCount,
    unreadWorkbenchCaseCount,
    unreadSecurityCenterCount,
    newFindingCount: unreadFindingCount,
    initializeSecurityCenterActivity,
    refreshSecurityCenterActivity,
    isFindingRead: (id: string) => Boolean(findings.value.find((item) => item.id === id)?.viewed_at),
    isWorkbenchCaseRead: (id: string) => readWorkbenchCaseIdSet.value.has(id),
    markFindingAsRead,
    markWorkbenchCaseAsRead,
    markFindingsAsRead,
    markWorkbenchCasesAsRead,
    markCurrentAsSeen,
  }
}
