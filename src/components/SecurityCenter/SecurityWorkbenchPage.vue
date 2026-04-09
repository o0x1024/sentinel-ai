<template>
  <div class="">
    <div
      v-if="!selectedCaseId"
      class="rounded-lg border border-base-300 bg-base-100 p-4"
    >
      <div class="flex flex-wrap items-center gap-3">
        <label class="form-control flex-1 min-w-[16rem]">
          <input
            v-model="search"
            class="input input-bordered input-sm"
            :placeholder="wb('page.searchPlaceholder')"
            @keyup.enter="reloadList"
          />
        </label>
        <label class="form-control">
          <select v-model="statusFilter" class="select select-bordered select-sm" @change="applyListFilters">
            <option value="">{{ wb('page.allStatus') }}</option>
            <option value="new">{{ wb('status.new') }}</option>
            <option value="investigating">{{ wb('status.investigating') }}</option>
            <option value="awaiting_verification">{{ wb('status.awaiting_verification') }}</option>
            <option value="verified">{{ wb('status.verified') }}</option>
            <option value="false_positive">{{ wb('status.false_positive') }}</option>
            <option value="archived">{{ wb('status.archived') }}</option>
          </select>
        </label>
        <label class="form-control">
          <select v-model.number="pageSize" class="select select-bordered select-sm" @change="applyListFilters">
            <option :value="10">10</option>
            <option :value="20">20</option>
            <option :value="50">50</option>
          </select>
        </label>
        <div class="flex items-end gap-2">
          <button class="btn btn-sm btn-outline" @click="reloadList">{{ wb('page.refresh') }}</button>
          <button class="btn btn-sm btn-ghost" @click="resetListFilters">{{ wb('page.reset') }}</button>
        </div>
      </div>
    </div>

    <SecurityWorkbenchCaseDetail
      v-if="selectedCaseId"
      :case-item="selectedCase"
      :activities="selectedActivities"
      :notes="selectedNotes"
      :syncing-finding="syncingFinding"
      :execution-drafts="selectedExecutionDrafts"
      :execution-runs="selectedExecutionRuns"
      :verifier-runs="selectedVerifierRuns"
      :assessment-suggestion="selectedAssessmentSuggestion"
      :executing-draft-id="executingDraftId"
      :initial-tab="selectedWorkbenchTab"
      :selected-evidence-id="selectedEvidenceId"
      :selected-draft-id="selectedDraftId"
      :selected-run-id="selectedRunId"
      :selected-timeline-item-id="selectedTimelineItemId"
      :initial-timeline-search="selectedTimelineSearch"
      :initial-timeline-filter="selectedTimelineFilter"
      @back="openCase(null)"
      @copy-location-link="copyCurrentLocationLink"
      @change-tab="changeCaseTab"
      @save-conclusion="saveConclusion"
      @save-metadata="saveMetadata"
      @set-baseline="setBaseline"
      @add-note="addNote"
      @sync-finding="syncFinding"
      @sync-finding-with-suggestion="syncFindingWithSuggestion"
      @create-draft="createDraft"
      @update-draft-status="updateDraftStatus"
      @execute-draft="executeDraft"
      @open-target="openTimelineTarget"
      @copy-target-link="copyTimelineTargetLink"
      @copy-timeline-item-link="copyTimelineItemLink"
      @change-timeline-state="changeTimelineState"
      @delete-case="deleteCase"
    />

    <SecurityWorkbenchCaseList
      v-else
      :cases="caseItems"
      :total="totalCount"
      :page="page"
      :page-size="pageSize"
      :loading="isLoadingList"
      :selectedIds="selectedCaseIds"
      @change-page="setPage"
      @change-page-size="setPageSize"
      @toggle-selection="toggleCaseSelection"
      @toggle-select-current-page="toggleSelectCurrentPage"
      @clear-selection="clearCaseSelection"
      @delete-case="deleteCase"
      @delete-selected="deleteSelectedCases"
      @open-case="openCase"
    />
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { dialog } from '@/composables/useDialog'
import SecurityWorkbenchCaseDetail from './SecurityWorkbenchCaseDetail.vue'
import SecurityWorkbenchCaseList from './SecurityWorkbenchCaseList.vue'
import {
  addWorkbenchNote,
  createWorkbenchExecutionDraft,
  deleteWorkbenchCases,
  executeWorkbenchExecutionDraft,
  getWorkbenchAutoModeEnabled,
  getWorkbenchCaseDetail,
  listWorkbenchCases,
  syncWorkbenchCaseToFinding,
  updateWorkbenchExecutionDraftStatus,
  updateWorkbenchCase,
} from './securityWorkbenchCaseSupport'
import { wb } from './securityWorkbenchLocale'
import { buildWorkbenchObjectAnalysis } from './securityWorkbenchObjectAnalysis'
import { getWorkbenchStatusLabel } from './securityWorkbenchPresentation'
import { buildWorkbenchReplayPlans } from './securityWorkbenchReplayPlan'
import type {
  WorkbenchActivity,
  WorkbenchCaseDetailResult,
  WorkbenchCaseListItem,
  WorkbenchCaseStatus,
  WorkbenchExecutionDraftStatus,
  WorkbenchExecutionRun,
  WorkbenchNote,
  WorkbenchNoteKind,
  WorkbenchReplayPlan,
  WorkbenchVerifierRun,
} from './securityWorkbenchTypes'

const route = useRoute()
const router = useRouter()

type CaseDetailTabId = 'overview' | 'evidence' | 'analysis' | 'plan' | 'drafts' | 'verification' | 'review'
type WorkbenchTimelineTarget = {
  tab: Extract<CaseDetailTabId, 'overview' | 'evidence' | 'drafts' | 'verification'>
  evidenceId?: string | null
  draftId?: string | null
  runId?: string | null
}
type WorkbenchListRouteState = {
  search: string
  status: WorkbenchCaseStatus | ''
  page: number
  pageSize: number
}

const isLoadingList = ref(false)
const isLoadingCase = ref(false)
const syncingFinding = ref(false)
const executingDraftId = ref<string | null>(null)
const caseItems = ref<WorkbenchCaseListItem[]>([])
const selectedCaseDetail = ref<WorkbenchCaseDetailResult | null>(null)
const totalCount = ref(0)
const page = ref(1)
const pageSize = ref(20)
const search = ref('')
const statusFilter = ref<WorkbenchCaseStatus | ''>('')
const selectedCaseIds = ref<string[]>([])
const workbenchAutoModeEnabled = ref(false)
const autoAttemptedCaseIds = new Set<string>()

const selectedCaseId = computed(() =>
  typeof route.params.caseId === 'string' && route.params.caseId.trim()
    ? route.params.caseId.trim()
    : typeof route.query.caseId === 'string' && route.query.caseId.trim()
      ? route.query.caseId.trim()
      : '',
)

const selectedCase = computed(() => selectedCaseDetail.value?.caseItem || null)
const selectedActivities = computed<WorkbenchActivity[]>(() => selectedCaseDetail.value?.activities || [])
const selectedNotes = computed<WorkbenchNote[]>(() => selectedCaseDetail.value?.notes || [])
const selectedExecutionDrafts = computed(() => selectedCaseDetail.value?.executionDrafts || [])
const selectedExecutionRuns = computed<WorkbenchExecutionRun[]>(() => selectedCaseDetail.value?.executionRuns || [])
const selectedVerifierRuns = computed<WorkbenchVerifierRun[]>(() => selectedCaseDetail.value?.verifierRuns || [])
const selectedAssessmentSuggestion = computed(() => selectedCaseDetail.value?.assessmentSuggestion || null)
const selectedWorkbenchTab = computed<CaseDetailTabId>(() => {
  const value = typeof route.query.workbenchTab === 'string' ? route.query.workbenchTab : ''
  return ['overview', 'evidence', 'analysis', 'plan', 'drafts', 'verification', 'review'].includes(value)
    ? (value as CaseDetailTabId)
    : 'overview'
})
const selectedEvidenceId = computed(() =>
  typeof route.query.evidenceId === 'string' && route.query.evidenceId.trim()
    ? route.query.evidenceId.trim()
    : null,
)
const selectedDraftId = computed(() =>
  typeof route.query.draftId === 'string' && route.query.draftId.trim()
    ? route.query.draftId.trim()
    : null,
)
const selectedRunId = computed(() =>
  typeof route.query.runId === 'string' && route.query.runId.trim()
    ? route.query.runId.trim()
    : null,
)
const selectedTimelineItemId = computed(() =>
  typeof route.query.timelineId === 'string' && route.query.timelineId.trim()
    ? route.query.timelineId.trim()
    : null,
)
const selectedTimelineSearch = computed(() =>
  typeof route.query.timelineSearch === 'string' ? route.query.timelineSearch.trim() : '',
)
const selectedTimelineFilter = computed<
  'all' | 'system' | 'notes' | 'draft_execution' | 'finding_sync' | 'suggestion_sync' | WorkbenchNoteKind
>(() => {
  const value = typeof route.query.timelineFilter === 'string' ? route.query.timelineFilter : ''
  return ['all', 'system', 'notes', 'draft_execution', 'finding_sync', 'suggestion_sync', 'observation', 'conclusion', 'false_positive_reason', 'remediation_note', 'replay_note'].includes(value)
    ? (value as 'all' | 'system' | 'notes' | 'draft_execution' | 'finding_sync' | 'suggestion_sync' | WorkbenchNoteKind)
    : 'all'
})

const normalizeListPage = (value: unknown, fallback: number) => {
  const nextValue = Number(value)
  return Number.isFinite(nextValue) && nextValue > 0 ? Math.floor(nextValue) : fallback
}

const readListRouteState = (): WorkbenchListRouteState => ({
  search: typeof route.query.search === 'string' ? route.query.search.trim() : '',
  status:
    typeof route.query.status === 'string'
      && ['new', 'investigating', 'awaiting_verification', 'verified', 'false_positive', 'archived'].includes(route.query.status)
      ? (route.query.status as WorkbenchCaseStatus)
      : '',
  page: normalizeListPage(route.query.page, 1),
  pageSize: [10, 20, 50].includes(Number(route.query.pageSize))
    ? Number(route.query.pageSize)
    : 20,
})

const applyListRouteState = () => {
  const nextState = readListRouteState()
  search.value = nextState.search
  statusFilter.value = nextState.status
  page.value = nextState.page
  pageSize.value = nextState.pageSize
}

const buildListRouteQuery = () => {
  const nextQuery: Record<string, string> = {}
  if (search.value.trim()) nextQuery.search = search.value.trim()
  if (statusFilter.value) nextQuery.status = statusFilter.value
  if (page.value > 1) nextQuery.page = String(page.value)
  if (pageSize.value !== 20) nextQuery.pageSize = String(pageSize.value)
  return nextQuery
}

const syncCurrentListRoute = async () => {
  const nextQuery = {
    ...buildListRouteQuery(),
    ...(selectedCaseId.value ? { workbenchTab: selectedWorkbenchTab.value } : {}),
    ...(selectedCaseId.value && selectedEvidenceId.value ? { evidenceId: selectedEvidenceId.value } : {}),
    ...(selectedCaseId.value && selectedDraftId.value ? { draftId: selectedDraftId.value } : {}),
    ...(selectedCaseId.value && selectedRunId.value ? { runId: selectedRunId.value } : {}),
    ...(selectedCaseId.value && selectedTimelineItemId.value ? { timelineId: selectedTimelineItemId.value } : {}),
    ...(selectedCaseId.value && selectedTimelineSearch.value ? { timelineSearch: selectedTimelineSearch.value } : {}),
    ...(selectedCaseId.value && selectedTimelineFilter.value !== 'all' ? { timelineFilter: selectedTimelineFilter.value } : {}),
  }
  await router.replace({
    path: selectedCaseId.value ? `/security-center/workbench/${selectedCaseId.value}` : '/security-center/workbench',
    query: nextQuery,
  })
}

const loadCaseList = async () => {
  isLoadingList.value = true
  try {
    const result = await listWorkbenchCases({
      search: search.value,
      status: statusFilter.value,
      page: page.value,
      pageSize: pageSize.value,
    })
    caseItems.value = result.items
    totalCount.value = result.total
    page.value = result.page
    pageSize.value = result.pageSize
  } catch (error) {
    console.error('Failed to load workbench cases', error)
    dialog.toast.error(wb('page.loadListFailed'))
  } finally {
    isLoadingList.value = false
  }
}

const loadCaseDetail = async (caseId: string) => {
  isLoadingCase.value = true
  try {
    const detail = await getWorkbenchCaseDetail(caseId)
    if (!detail) {
      selectedCaseDetail.value = null
      await router.replace({
        path: '/security-center/workbench',
        query: buildListRouteQuery(),
      })
      return
    }
    selectedCaseDetail.value = detail
  } catch (error) {
    console.error('Failed to load workbench case detail', error)
    selectedCaseDetail.value = null
    dialog.toast.error(wb('page.loadDetailFailed'))
  } finally {
    isLoadingCase.value = false
  }
}

const openCase = async (caseId: string | null) => {
  const listQuery = buildListRouteQuery()
  if (caseId) {
    await router.replace({
      path: `/security-center/workbench/${caseId}`,
      query: {
        ...listQuery,
        workbenchTab: 'overview',
      },
    })
    return
  }
  await router.replace({ path: '/security-center/workbench', query: listQuery })
}

const updateCaseLocation = async (
  caseId: string,
  target: {
    tab?: CaseDetailTabId
    evidenceId?: string | null
    draftId?: string | null
    runId?: string | null
  } = {},
) => {
  const nextQuery: Record<string, string> = {
    ...buildListRouteQuery(),
    workbenchTab: target.tab || selectedWorkbenchTab.value,
  }
  if (target.evidenceId) nextQuery.evidenceId = target.evidenceId
  if (target.draftId) nextQuery.draftId = target.draftId
  if (target.runId) nextQuery.runId = target.runId
  await router.replace({
    path: `/security-center/workbench/${caseId}`,
    query: nextQuery,
  })
}

const changeCaseTab = async (tab: CaseDetailTabId) => {
  if (!selectedCaseId.value) return
  await updateCaseLocation(selectedCaseId.value, { tab })
}

const openTimelineTarget = async (target: WorkbenchTimelineTarget) => {
  if (!selectedCaseId.value) return
  await updateCaseLocation(selectedCaseId.value, target)
}

const copyCurrentLocationLink = async () => {
  if (!selectedCaseId.value) return

  const resolved = router.resolve({
    path: `/security-center/workbench/${selectedCaseId.value}`,
    query: {
      ...buildListRouteQuery(),
      workbenchTab: selectedWorkbenchTab.value,
      ...(selectedEvidenceId.value ? { evidenceId: selectedEvidenceId.value } : {}),
      ...(selectedDraftId.value ? { draftId: selectedDraftId.value } : {}),
      ...(selectedRunId.value ? { runId: selectedRunId.value } : {}),
      ...(selectedTimelineItemId.value ? { timelineId: selectedTimelineItemId.value } : {}),
      ...(selectedTimelineSearch.value ? { timelineSearch: selectedTimelineSearch.value } : {}),
      ...(selectedTimelineFilter.value !== 'all' ? { timelineFilter: selectedTimelineFilter.value } : {}),
    },
  })
  const url = new URL(resolved.href, window.location.origin).toString()

  try {
    await navigator.clipboard.writeText(url)
    dialog.toast.success(wb('page.currentLocationCopied'))
  } catch (error) {
    console.error('Failed to copy workbench location link', error)
    dialog.toast.error(wb('page.copyCurrentLocationFailed'))
  }
}

const copyTimelineTargetLink = async (target: WorkbenchTimelineTarget) => {
  if (!selectedCaseId.value) return

  const resolved = router.resolve({
    path: `/security-center/workbench/${selectedCaseId.value}`,
    query: {
      ...buildListRouteQuery(),
      workbenchTab: target.tab,
      ...(target.evidenceId ? { evidenceId: target.evidenceId } : {}),
      ...(target.draftId ? { draftId: target.draftId } : {}),
      ...(target.runId ? { runId: target.runId } : {}),
      ...(selectedTimelineSearch.value ? { timelineSearch: selectedTimelineSearch.value } : {}),
      ...(selectedTimelineFilter.value !== 'all' ? { timelineFilter: selectedTimelineFilter.value } : {}),
    },
  })
  const url = new URL(resolved.href, window.location.origin).toString()

  try {
    await navigator.clipboard.writeText(url)
    dialog.toast.success(wb('page.targetLocationCopied'))
  } catch (error) {
    console.error('Failed to copy timeline target link', error)
    dialog.toast.error(wb('page.copyTargetLocationFailed'))
  }
}

const copyTimelineItemLink = async (itemId: string) => {
  if (!selectedCaseId.value) return

  const resolved = router.resolve({
    path: `/security-center/workbench/${selectedCaseId.value}`,
    query: {
      ...buildListRouteQuery(),
      workbenchTab: 'review',
      timelineId: itemId,
      ...(selectedTimelineSearch.value ? { timelineSearch: selectedTimelineSearch.value } : {}),
      ...(selectedTimelineFilter.value !== 'all' ? { timelineFilter: selectedTimelineFilter.value } : {}),
    },
  })
  const url = new URL(resolved.href, window.location.origin).toString()

  try {
    await navigator.clipboard.writeText(url)
    dialog.toast.success(wb('page.timelineItemLinkCopied'))
  } catch (error) {
    console.error('Failed to copy timeline item link', error)
    dialog.toast.error(wb('page.copyTimelineItemLinkFailed'))
  }
}

const reloadList = async () => {
  const before = JSON.stringify({
    path: route.path,
    query: route.query,
  })
  await syncCurrentListRoute()
  const after = JSON.stringify({
    path: route.path,
    query: route.query,
  })
  if (before === after) {
    await refreshWorkbenchData()
  }
}

const changeTimelineState = async (state: {
  search: string
  filter:
    | 'all'
    | 'system'
    | 'notes'
    | 'draft_execution'
    | 'finding_sync'
    | 'suggestion_sync'
    | WorkbenchNoteKind
}) => {
  if (!selectedCaseId.value || selectedWorkbenchTab.value !== 'review') return
  const nextQuery: Record<string, string> = {
    ...buildListRouteQuery(),
    workbenchTab: 'review',
  }
  if (selectedTimelineItemId.value) nextQuery.timelineId = selectedTimelineItemId.value
  if (state.search) nextQuery.timelineSearch = state.search
  if (state.filter !== 'all') nextQuery.timelineFilter = state.filter
  await router.replace({
    path: `/security-center/workbench/${selectedCaseId.value}`,
    query: nextQuery,
  })
}

const reloadSelectedCase = async () => {
  if (!selectedCaseId.value) return
  await loadCaseDetail(selectedCaseId.value)
}

const refreshWorkbenchData = async () => {
  if (selectedCaseId.value) {
    await reloadSelectedCase()
  }
  await loadCaseList()
}

const handleSecurityCenterRefresh = () => {
  void refreshWorkbenchData()
}

const applyListFilters = async () => {
  page.value = 1
  await syncCurrentListRoute()
}

const resetListFilters = async () => {
  search.value = ''
  statusFilter.value = ''
  pageSize.value = 20
  page.value = 1
  await syncCurrentListRoute()
}

const setPage = async (nextPage: number) => {
  if (nextPage < 1 || nextPage === page.value) return
  page.value = nextPage
  await syncCurrentListRoute()
}

const setPageSize = async (nextPageSize: number) => {
  if (!Number.isFinite(nextPageSize) || nextPageSize < 1 || nextPageSize === pageSize.value) return
  pageSize.value = nextPageSize
  page.value = 1
  await syncCurrentListRoute()
}

const toggleCaseSelection = (caseId: string) => {
  if (selectedCaseIds.value.includes(caseId)) {
    selectedCaseIds.value = selectedCaseIds.value.filter(item => item !== caseId)
    return
  }
  selectedCaseIds.value = [...selectedCaseIds.value, caseId]
}

const toggleSelectCurrentPage = () => {
  const currentPageIds = caseItems.value.map(item => item.id)
  const hasUnselected = currentPageIds.some(caseId => !selectedCaseIds.value.includes(caseId))
  if (!hasUnselected) {
    selectedCaseIds.value = selectedCaseIds.value.filter(caseId => !currentPageIds.includes(caseId))
    return
  }

  selectedCaseIds.value = Array.from(new Set([...selectedCaseIds.value, ...currentPageIds]))
}

const clearCaseSelection = () => {
  selectedCaseIds.value = []
}

const removeDeletedCaseIdsFromSelection = (caseIds: string[]) => {
  if (!caseIds.length) return
  selectedCaseIds.value = selectedCaseIds.value.filter(caseId => !caseIds.includes(caseId))
}

const normalizePageAfterDeletion = (deletedCaseCount: number) => {
  const remainingTotal = Math.max(0, totalCount.value - deletedCaseCount)
  const maxPage = Math.max(1, Math.ceil(remainingTotal / pageSize.value))
  if (page.value > maxPage) {
    page.value = maxPage
  }
}

const confirmDeleteCases = async (caseIds: string[]) => {
  if (!caseIds.length) return false
  return dialog.confirm({
    title: caseIds.length === 1 ? wb('confirm.deleteCaseTitle') : wb('confirm.deleteCasesTitle'),
    message: caseIds.length === 1
      ? wb('confirm.deleteCaseMessage')
      : wb('confirm.deleteCasesMessage', { count: caseIds.length }),
    confirmText: caseIds.length === 1 ? wb('confirm.confirmDeleteCase') : wb('confirm.confirmDeleteCases'),
    cancelText: wb('confirm.cancel'),
    variant: 'error',
  })
}

const deleteCase = async (caseId: string) => {
  const confirmed = await confirmDeleteCases([caseId])
  if (!confirmed) return

  try {
    const result = await deleteWorkbenchCases([caseId])
    removeDeletedCaseIdsFromSelection(result.deletedCaseIds)
    normalizePageAfterDeletion(result.deletedCaseCount)
    if (selectedCaseId.value && result.deletedCaseIds.includes(selectedCaseId.value)) {
      await openCase(null)
      await loadCaseList()
      selectedCaseDetail.value = null
    } else {
      await syncCurrentListRoute()
      await loadCaseList()
    }
    dialog.toast.success(wb('toast.deletedCases', { count: result.deletedCaseCount }))
  } catch (error) {
    console.error('Failed to delete workbench case', error)
    dialog.toast.error(wb('toast.deleteCaseFailed'))
  }
}

const deleteSelectedCases = async () => {
  const caseIds = selectedCaseIds.value.slice()
  if (!caseIds.length) {
    dialog.toast.warning(wb('toast.selectCasesFirst'))
    return
  }

  const confirmed = await confirmDeleteCases(caseIds)
  if (!confirmed) return

  try {
    const result = await deleteWorkbenchCases(caseIds)
    removeDeletedCaseIdsFromSelection(result.deletedCaseIds)
    normalizePageAfterDeletion(result.deletedCaseCount)
    await syncCurrentListRoute()
    await loadCaseList()
    dialog.toast.success(wb('toast.deletedCases', { count: result.deletedCaseCount }))
  } catch (error) {
    console.error('Failed to bulk delete workbench cases', error)
    dialog.toast.error(wb('toast.deleteCasesFailed'))
  }
}

const saveConclusion = async (caseId: string, value: string) => {
  const nextStatus =
    selectedCase.value?.status === 'new' && value.trim() ? ('investigating' as const) : undefined
  try {
    await updateWorkbenchCase(caseId, {
      currentConclusion: value,
      ...(nextStatus ? { status: nextStatus } : {}),
    })
    await refreshWorkbenchData()
    dialog.toast.success(wb('toast.conclusionSaved'))
  } catch (error) {
    console.error('Failed to save workbench conclusion', error)
    dialog.toast.error(wb('toast.conclusionSaveFailed'))
  }
}

const saveMetadata = async (
  caseId: string,
  patch: {
    status: 'new' | 'investigating' | 'awaiting_verification' | 'verified' | 'false_positive' | 'archived'
    priority: 'low' | 'medium' | 'high'
  },
) => {
  try {
    await updateWorkbenchCase(caseId, patch)
    await refreshWorkbenchData()
    dialog.toast.success(wb('toast.metadataSaved'))
  } catch (error) {
    console.error('Failed to save workbench metadata', error)
    dialog.toast.error(wb('toast.metadataSaveFailed'))
  }
}

const setBaseline = async (caseId: string, evidenceId: string) => {
  try {
    await updateWorkbenchCase(caseId, { baselineEvidenceId: evidenceId })
    await refreshWorkbenchData()
    dialog.toast.success(wb('toast.baselineUpdated'))
  } catch (error) {
    console.error('Failed to update baseline evidence', error)
    dialog.toast.error(wb('toast.baselineUpdateFailed'))
  }
}

const addNote = async (caseId: string, kind: WorkbenchNoteKind, body: string) => {
  try {
    await addWorkbenchNote(caseId, kind, body)
    await refreshWorkbenchData()
    dialog.toast.success(wb('toast.noteAdded'))
  } catch (error) {
    console.error('Failed to add workbench note', error)
    dialog.toast.error(wb('toast.noteAddFailed'))
  }
}

const syncFinding = async (caseId: string) => {
  syncingFinding.value = true
  try {
    const result = await syncWorkbenchCaseToFinding(caseId)
    await refreshWorkbenchData()
    window.dispatchEvent(new CustomEvent('security-center-refresh'))
    if (result) {
      dialog.toast.success(
        wb('toast.findingSynced', {
          from: result.previousFindingStatus,
          to: result.nextFindingStatus,
        }),
      )
    } else {
      dialog.toast.warning(wb('toast.findingSyncMissing'))
    }
  } catch (error) {
    console.error('Failed to sync workbench case to finding', error)
    dialog.toast.error(wb('toast.findingSyncFailed'))
  } finally {
    syncingFinding.value = false
  }
}

const syncFindingWithSuggestion = async (caseId: string) => {
  const suggestion = selectedAssessmentSuggestion.value
  if (suggestion) {
    const confirmed = await dialog.confirm({
      title: wb('confirm.syncWithSuggestionTitle'),
      message: wb('confirm.syncWithSuggestionMessage', {
        status: getWorkbenchStatusLabel(suggestion.suggestedStatus),
        summary: suggestion.summary,
      }),
      confirmText: wb('confirm.confirmSyncWithSuggestion'),
      cancelText: wb('confirm.cancel'),
      variant: 'warning',
    })
    if (!confirmed) return
  }

  syncingFinding.value = true
  try {
    const result = await syncWorkbenchCaseToFinding(caseId, {
      applySuggestionToCase: true,
    })
    await refreshWorkbenchData()
    window.dispatchEvent(new CustomEvent('security-center-refresh'))
    if (result) {
      dialog.toast.success(
        wb('toast.findingSyncedWithSuggestion', {
          from: result.previousFindingStatus,
          to: result.nextFindingStatus,
        }),
      )
    } else {
      dialog.toast.warning(wb('toast.findingSyncMissing'))
    }
  } catch (error) {
    console.error('Failed to sync workbench case to finding with suggestion', error)
    dialog.toast.error(wb('toast.findingSyncWithSuggestionFailed'))
  } finally {
    syncingFinding.value = false
  }
}

const createDraft = async (caseId: string, plan: WorkbenchReplayPlan) => {
  try {
    await createWorkbenchExecutionDraft(caseId, plan)
    await refreshWorkbenchData()
    dialog.toast.success(wb('toast.draftCreated'))
  } catch (error) {
    console.error('Failed to create execution draft', error)
    dialog.toast.error(wb('toast.draftCreateFailed'))
  }
}

const updateDraftStatus = async (
  _caseId: string,
  draftId: string,
  status: WorkbenchExecutionDraftStatus,
) => {
  try {
    await updateWorkbenchExecutionDraftStatus(draftId, status)
    await refreshWorkbenchData()
    dialog.toast.success(wb('toast.draftStatusUpdated'))
  } catch (error) {
    console.error('Failed to update execution draft status', error)
    dialog.toast.error(wb('toast.draftStatusUpdateFailed'))
  }
}

const executeDraft = async (_caseId: string, draftId: string, readOnly: boolean) => {
  if (!readOnly) {
    const confirmed = await dialog.confirm({
      title: wb('confirm.executeMutableTitle'),
      message: wb('confirm.executeMutableMessage'),
      confirmText: wb('confirm.confirmExecute'),
      cancelText: wb('confirm.cancel'),
      variant: 'warning',
    })
    if (!confirmed) return
  }

  executingDraftId.value = draftId
  try {
    const result = await executeWorkbenchExecutionDraft(draftId, {
      confirmNonReadonly: !readOnly,
    })
    await refreshWorkbenchData()
    if (result) {
      dialog.toast.success(readOnly ? wb('toast.readonlyDraftExecuted') : wb('toast.mutableDraftExecuted'))
    } else {
      dialog.toast.warning(wb('page.noExecutableDraft'))
    }
  } catch (error) {
    console.error('Failed to execute workbench draft', error)
    dialog.toast.error(readOnly ? wb('page.executeReadonlyDraftFailed') : wb('page.executeMutableDraftFailed'))
  } finally {
    executingDraftId.value = null
  }
}

const autoCreateAndExecuteReadonlyDraft = async (detail: WorkbenchCaseDetailResult) => {
  if (!workbenchAutoModeEnabled.value) return
  if (autoAttemptedCaseIds.has(detail.caseItem.id)) return
  if (detail.executionDrafts.length > 0 || detail.executionRuns.length > 0) return

  const plans = buildWorkbenchReplayPlans(
    detail.caseItem,
    buildWorkbenchObjectAnalysis(detail.caseItem),
  ).filter(plan => plan.readOnly)

  if (!plans.length) return

  autoAttemptedCaseIds.add(detail.caseItem.id)
  try {
    const draft = await createWorkbenchExecutionDraft(detail.caseItem.id, plans[0])
    if (!draft) return
    await executeWorkbenchExecutionDraft(draft.id, {
      confirmNonReadonly: false,
    })
    await refreshWorkbenchData()
  } catch (error) {
    console.error('Failed to auto create and execute readonly workbench draft', error)
  }
}

watch(
  selectedCaseId,
  async caseId => {
    if (caseId) {
      await loadCaseDetail(caseId)
      return
    }
    selectedCaseDetail.value = null
  },
  { immediate: true },
)

watch(
  selectedCaseDetail,
  async detail => {
    if (!detail) return
    await autoCreateAndExecuteReadonlyDraft(detail)
  },
)

watch(
  () => [route.query.search, route.query.status, route.query.page, route.query.pageSize],
  async () => {
    applyListRouteState()
    await loadCaseList()
  },
  { immediate: true },
)

onMounted(async () => {
  applyListRouteState()
  try {
    workbenchAutoModeEnabled.value = await getWorkbenchAutoModeEnabled()
  } catch (error) {
    console.error('Failed to load workbench auto mode setting', error)
  }
  window.addEventListener('security-center-refresh', handleSecurityCenterRefresh)
})

onUnmounted(() => {
  window.removeEventListener('security-center-refresh', handleSecurityCenterRefresh)
})
</script>
