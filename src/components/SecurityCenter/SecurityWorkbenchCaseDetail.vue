<template>
  <div v-if="caseItem" class="flex h-full min-h-0 flex-col">
    <div class="shrink-0 border-b border-base-300 bg-base-100 px-6 py-5">
      <div class="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h2 class="text-xl font-semibold break-words">
            {{ getWorkbenchFindingTitle(caseItem.finding) }}
          </h2>
        </div>
        <div class="flex flex-wrap items-center gap-2">
          <SecurityEvidenceTransferActions
            :evidence="primaryTransferableEvidence"
            size="sm"
            direction="down"
            :messages="transferMessages"
          />
          <button
            class="btn btn-sm btn-error btn-outline"
            @click="$emit('delete-case', caseItem.id)"
          >
            {{ wb('caseDetail.deleteCase') }}
          </button>
          <button class="btn btn-sm btn-outline" @click="$emit('copy-location-link')">
            {{ wb('page.copyLocationLink') }}
          </button>
          <button class="btn btn-sm btn-outline" @click="$emit('back')">
            {{ wb('caseDetail.backToList') }}
          </button>
        </div>
      </div>
      <div class="mt-4 overflow-x-auto">
        <div class="tabs tabs-boxed w-max min-w-full">
          <button
            v-for="tab in tabs"
            :key="tab.id"
            class="tab"
            :class="{ 'tab-active': activeTab === tab.id }"
            @click="activeTab = tab.id"
          >
            {{ tab.label }}
          </button>
        </div>
      </div>
    </div>

    <div class="min-h-0 flex-1 overflow-y-auto px-6 py-5">
      <WorkbenchCaseOverviewPanel
        v-if="activeTab === 'overview'"
        :case-item="caseItem"
        :syncing-finding="syncingFinding"
        :execution-runs="executionRuns"
        :assessment-suggestion="assessmentSuggestion"
        :system-agent-statuses="systemAgentStatuses"
        @save-conclusion="value => $emit('save-conclusion', caseItem.id, value)"
        @save-metadata="patch => $emit('save-metadata', caseItem.id, patch)"
        @sync-finding="$emit('sync-finding', caseItem.id)"
        @sync-finding-with-suggestion="$emit('sync-finding-with-suggestion', caseItem.id)"
      />
      <WorkbenchEvidenceChainPanel
        v-else-if="activeTab === 'evidence'"
        :case-item="caseItem"
        :selected-evidence-id="selectedEvidenceId"
        @set-baseline="evidenceId => $emit('set-baseline', caseItem.id, evidenceId)"
      />
      <WorkbenchObjectAnalysisPanel v-else-if="activeTab === 'analysis'" :case-item="caseItem" />
      <WorkbenchReplayPlanPanel
        v-else-if="activeTab === 'plan'"
        :case-item="caseItem"
        @create-draft="plan => $emit('create-draft', caseItem.id, plan)"
      />
      <WorkbenchExecutionDraftPanel
        v-else-if="activeTab === 'drafts'"
        :drafts="executionDrafts"
        :executing-draft-id="executingDraftId"
        :selected-draft-id="selectedDraftId"
        @update-status="
          (draftId, status) => $emit('update-draft-status', caseItem.id, draftId, status)
        "
        @execute-draft="
          (draftId, readOnly) => $emit('execute-draft', caseItem.id, draftId, readOnly)
        "
      />
      <WorkbenchVerificationPanel
        v-else-if="activeTab === 'verification'"
        :case-item="caseItem"
        :execution-runs="executionRuns"
        :verifier-runs="verifierRuns"
        :assessment-suggestion="assessmentSuggestion"
        :selected-run-id="selectedRunId"
      />
      <WorkbenchReviewNotesPanel
        v-else
        :activities="activities"
        :notes="notes"
        :selected-timeline-item-id="selectedTimelineItemId"
        :initial-timeline-search="initialTimelineSearch"
        :initial-timeline-filter="initialTimelineFilter"
        @add-note="(kind, body) => $emit('add-note', caseItem.id, kind, body)"
        @open-target="target => $emit('open-target', target)"
        @copy-target-link="target => $emit('copy-target-link', target)"
        @copy-timeline-item-link="itemId => $emit('copy-timeline-item-link', itemId)"
        @change-timeline-state="state => $emit('change-timeline-state', state)"
      />
    </div>
  </div>

  <div
    v-else
    class="flex h-full min-h-[16rem] items-center justify-center px-6 text-center text-base-content/60"
  >
    {{ wb('caseDetail.notFound') }}
  </div>
</template>

<script setup lang="ts">
import { computed, onActivated, onDeactivated, onMounted, onUnmounted, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import { dialog } from '@/composables/useDialog'
import type {
  WorkbenchActivity,
  WorkbenchAssessmentSuggestion,
  WorkbenchCase,
  WorkbenchExecutionDraft,
  WorkbenchExecutionDraftStatus,
  WorkbenchExecutionRun,
  WorkbenchVerifierRun,
  WorkbenchNote,
  WorkbenchNoteKind,
  WorkbenchReplayPlan,
  WorkbenchSystemAgentStatus,
} from './securityWorkbenchTypes'
import SecurityEvidenceTransferActions from './SecurityEvidenceTransferActions.vue'
import WorkbenchCaseOverviewPanel from './WorkbenchCaseOverviewPanel.vue'
import WorkbenchExecutionDraftPanel from './WorkbenchExecutionDraftPanel.vue'
import WorkbenchEvidenceChainPanel from './WorkbenchEvidenceChainPanel.vue'
import WorkbenchObjectAnalysisPanel from './WorkbenchObjectAnalysisPanel.vue'
import WorkbenchReplayPlanPanel from './WorkbenchReplayPlanPanel.vue'
import WorkbenchReviewNotesPanel from './WorkbenchReviewNotesPanel.vue'
import WorkbenchVerificationPanel from './WorkbenchVerificationPanel.vue'
import {
  findFirstTransferableSecurityEvidence,
  openSecurityEvidenceInTrafficWorkbench,
} from './securityEvidenceTransferSupport'
import { resolveSecurityEvidenceTransferShortcut } from './securityEvidenceTransferShortcut'
import { wb } from './securityWorkbenchLocale'
import { getWorkbenchFindingTitle } from './securityWorkbenchSystemAgentContent'

type CaseDetailTabId =
  | 'overview'
  | 'evidence'
  | 'analysis'
  | 'plan'
  | 'drafts'
  | 'verification'
  | 'review'

const props = defineProps<{
  caseItem: WorkbenchCase | null
  activities: WorkbenchActivity[]
  notes: WorkbenchNote[]
  syncingFinding: boolean
  executionDrafts: WorkbenchExecutionDraft[]
  executionRuns: WorkbenchExecutionRun[]
  verifierRuns: WorkbenchVerifierRun[]
  assessmentSuggestion: WorkbenchAssessmentSuggestion | null
  systemAgentStatuses: WorkbenchSystemAgentStatus[]
  executingDraftId: string | null
  initialTab?: CaseDetailTabId
  selectedEvidenceId?: string | null
  selectedDraftId?: string | null
  selectedRunId?: string | null
  selectedTimelineItemId?: string | null
  initialTimelineSearch?: string
  initialTimelineFilter?:
    | 'all'
    | 'system'
    | 'notes'
    | 'draft_execution'
    | 'finding_sync'
    | 'suggestion_sync'
    | WorkbenchNoteKind
}>()

const emit = defineEmits<{
  back: []
  'copy-location-link': []
  'change-tab': [tab: CaseDetailTabId]
  'save-conclusion': [caseId: string, value: string]
  'save-metadata': [
    caseId: string,
    patch: {
      status: WorkbenchCase['status']
      priority: WorkbenchCase['priority']
    },
  ]
  'set-baseline': [caseId: string, evidenceId: string]
  'add-note': [caseId: string, kind: WorkbenchNoteKind, body: string]
  'sync-finding': [caseId: string]
  'sync-finding-with-suggestion': [caseId: string]
  'create-draft': [caseId: string, plan: WorkbenchReplayPlan]
  'update-draft-status': [caseId: string, draftId: string, status: WorkbenchExecutionDraftStatus]
  'execute-draft': [caseId: string, draftId: string, readOnly: boolean]
  'open-target': [
    target: {
      tab: Extract<CaseDetailTabId, 'overview' | 'evidence' | 'drafts' | 'verification'>
      evidenceId?: string | null
      draftId?: string | null
      runId?: string | null
    },
  ]
  'copy-target-link': [
    target: {
      tab: Extract<CaseDetailTabId, 'overview' | 'evidence' | 'drafts' | 'verification'>
      evidenceId?: string | null
      draftId?: string | null
      runId?: string | null
    },
  ]
  'copy-timeline-item-link': [itemId: string]
  'change-timeline-state': [
    state: {
      search: string
      filter:
        | 'all'
        | 'system'
        | 'notes'
        | 'draft_execution'
        | 'finding_sync'
        | 'suggestion_sync'
        | WorkbenchNoteKind
    },
  ]
  'delete-case': [caseId: string]
}>()

const router = useRouter()
const activeTab = ref<CaseDetailTabId>(props.initialTab || 'overview')
const primaryTransferableEvidence = computed(() =>
  findFirstTransferableSecurityEvidence(props.caseItem?.finding.evidence)
)
const selectedTransferableEvidence = computed(() => {
  const selectedEvidenceId = props.selectedEvidenceId?.trim()
  if (!selectedEvidenceId) {
    return null
  }

  const selectedEvidence = props.caseItem?.finding.evidence?.find(
    evidence => evidence.id === selectedEvidenceId,
  )
  if (!selectedEvidence) {
    return null
  }

  return findFirstTransferableSecurityEvidence([selectedEvidence])
})
const shortcutTransferableEvidence = computed(
  () => selectedTransferableEvidence.value || primaryTransferableEvidence.value,
)
const transferMessages = computed(() => ({
  triggerLabel: wb('caseDetail.sendTo'),
  createDraft: wb('caseDetail.createDraft'),
  createAttackWorkspace: wb('caseDetail.createAttackWorkspace'),
  noTransferableRequest: wb('evidence.noCaseTransferableRequest'),
  draftCreated: wb('evidence.draftCreated'),
  attackWorkspaceCreated: wb('evidence.attackWorkspaceCreated'),
  transferFailed: wb('evidence.transferFailed', { error: '{error}' }),
}))

const tabs = computed<Array<{ id: CaseDetailTabId; label: string }>>(() => [
  { id: 'overview', label: wb('caseDetail.tabs.overview') },
  { id: 'evidence', label: wb('caseDetail.tabs.evidence') },
  { id: 'analysis', label: wb('caseDetail.tabs.analysis') },
  { id: 'plan', label: wb('caseDetail.tabs.plan') },
  { id: 'drafts', label: wb('caseDetail.tabs.drafts') },
  { id: 'verification', label: wb('caseDetail.tabs.verification') },
  { id: 'review', label: wb('caseDetail.tabs.review') },
])

const formatTransferError = (error: unknown) =>
  transferMessages.value.transferFailed.replace('{error}', String(error))

const transferCaseEvidenceRequest = async (target: 'draft' | 'attackWorkspace') => {
  const evidence = shortcutTransferableEvidence.value
  if (!evidence) {
    dialog.toast.warning(transferMessages.value.noTransferableRequest)
    return
  }

  try {
    const handled = await openSecurityEvidenceInTrafficWorkbench(router, evidence, target)
    if (!handled) {
      dialog.toast.warning(transferMessages.value.noTransferableRequest)
      return
    }

    dialog.toast.success(
      target === 'draft'
        ? transferMessages.value.draftCreated
        : transferMessages.value.attackWorkspaceCreated,
    )
  } catch (error) {
    console.error('Failed to send workbench evidence request from shortcut:', error)
    dialog.toast.error(formatTransferError(error))
  }
}

const handleWindowKeydown = (event: KeyboardEvent) => {
  if (!props.caseItem) {
    return
  }

  const target = resolveSecurityEvidenceTransferShortcut(event)
  if (!target) {
    return
  }

  if (!shortcutTransferableEvidence.value) {
    return
  }

  event.preventDefault()
  event.stopPropagation()
  void transferCaseEvidenceRequest(target)
}

watch(
  () => props.initialTab,
  nextTab => {
    if (nextTab && nextTab !== activeTab.value) {
      activeTab.value = nextTab
    }
  }
)

watch(activeTab, nextTab => {
  emit('change-tab', nextTab)
})

onMounted(() => {
  window.addEventListener('keydown', handleWindowKeydown)
})

onActivated(() => {
  window.addEventListener('keydown', handleWindowKeydown)
})

onDeactivated(() => {
  window.removeEventListener('keydown', handleWindowKeydown)
})

onUnmounted(() => {
  window.removeEventListener('keydown', handleWindowKeydown)
})
</script>
