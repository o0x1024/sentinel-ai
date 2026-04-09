<template>
  <div class="space-y-4">
    <div class="rounded-lg border border-base-300 bg-base-100 p-4 space-y-3">
      <div class="flex flex-wrap items-center justify-between gap-3">
        <div>
          <div class="font-medium text-sm">{{ wb('review.timelineTitle') }}</div>
          <div class="text-xs text-base-content/60">{{ wb('review.timelineSummary') }}</div>
        </div>
        <div class="flex flex-wrap gap-2 w-full sm:w-auto">
          <input
            v-model.trim="timelineSearch"
            class="input input-bordered input-sm w-full sm:w-[240px]"
            :placeholder="wb('review.searchPlaceholder')"
          />
          <select v-model="timelineFilter" class="select select-bordered select-sm w-full sm:w-[220px]">
            <option value="all">{{ wb('review.filterAll') }}</option>
            <option value="system">{{ wb('review.filterSystem') }}</option>
            <option value="notes">{{ wb('review.filterNotes') }}</option>
            <option value="draft_execution">{{ wb('review.filterDraftExecution') }}</option>
            <option value="finding_sync">{{ wb('review.filterFindingSync') }}</option>
            <option value="suggestion_sync">{{ wb('review.filterSuggestionSync') }}</option>
            <option value="observation">{{ wb('review.filterObservation') }}</option>
            <option value="conclusion">{{ wb('review.filterConclusion') }}</option>
            <option value="false_positive_reason">{{ wb('review.filterFalsePositive') }}</option>
            <option value="remediation_note">{{ wb('review.filterRemediation') }}</option>
            <option value="replay_note">{{ wb('review.filterReplay') }}</option>
          </select>
        </div>
      </div>

      <div v-if="timelineItems.length > 0" class="space-y-3">
        <div
          v-for="item in timelineItems"
          :key="item.id"
          :ref="setTimelineItemRef(item.id)"
          :id="`timeline-${item.id}`"
          :class="[
            item.kind === 'activity'
              ? 'rounded-lg border border-info/20 bg-info/5 p-4'
              : 'rounded-lg border border-base-300 bg-base-100 p-4',
            selectedTimelineItemId === item.id ? 'ring-1 ring-primary/30 border-primary' : '',
          ]"
        >
          <div class="flex flex-wrap items-center justify-between gap-2">
            <div class="flex flex-wrap items-center gap-2">
              <span
                :class="[
                  'badge badge-outline',
                  item.kind === 'activity' ? 'badge-info' : 'badge-ghost',
                ]"
              >
                {{ item.kind === 'activity' ? wb('review.systemActivity') : wb('review.manualNote') }}
              </span>
              <span
                :class="[
                  'badge',
                  item.kind === 'activity' ? 'badge-info badge-outline' : 'badge-outline',
                ]"
              >
                {{ item.kind === 'activity' ? getWorkbenchActivityKindLabel(item.entry.kind) : getWorkbenchNoteKindLabel(item.entry.kind) }}
              </span>
              <span class="font-medium text-sm">
                {{ item.kind === 'activity' ? item.entry.title : getNoteTitle(item.entry.kind) }}
              </span>
              <span class="text-xs text-base-content/60">
                {{ item.kind === 'activity' ? item.entry.actor : item.entry.author }}
              </span>
            </div>
            <span class="text-xs text-base-content/60">{{ formatWorkbenchTime(item.createdAt) }}</span>
          </div>

          <p class="mt-3 whitespace-pre-wrap break-words text-sm">
            {{ item.kind === 'activity' ? item.entry.summary : item.entry.body }}
          </p>

          <div
            v-if="item.kind === 'activity' && getTimelineActions(item.entry).length > 0"
            class="mt-3 flex flex-wrap gap-2"
          >
            <button
              v-for="action in getTimelineActions(item.entry)"
              :key="`${item.id}-${action.label}`"
              class="btn btn-xs btn-outline"
              @click="emit('open-target', action.target)"
            >
              {{ action.label }}
            </button>
            <button
              v-if="getTimelineActions(item.entry).length > 0"
              class="btn btn-xs btn-ghost"
              @click="emit('copy-target-link', getTimelineActions(item.entry)[0].target)"
            >
              {{ wb('review.copyLocationLink') }}
            </button>
            <button
              class="btn btn-xs btn-ghost"
              @click="emit('copy-timeline-item-link', item.id)"
            >
              {{ wb('review.copyNodeLink') }}
            </button>
          </div>

          <div
            v-else
            class="mt-3 flex flex-wrap gap-2"
          >
            <button
              class="btn btn-xs btn-ghost"
              @click="emit('copy-timeline-item-link', item.id)"
            >
              {{ wb('review.copyNodeLink') }}
            </button>
          </div>

          <div
            v-if="
              item.kind === 'activity'
                && (getWorkbenchSnapshotEntries(item.entry.before).length
                  || getWorkbenchSnapshotEntries(item.entry.after).length)
            "
            class="mt-3 grid gap-3 md:grid-cols-2"
          >
            <div
              v-if="getWorkbenchSnapshotEntries(item.entry.before).length"
              class="rounded-lg border border-base-300 bg-base-100 p-3"
            >
              <div class="mb-2 text-xs font-medium text-base-content/60">{{ wb('review.snapshotBefore') }}</div>
              <div class="space-y-2">
                <div
                  v-for="entry in getWorkbenchSnapshotEntries(item.entry.before)"
                  :key="`before-${item.id}-${entry.key}`"
                  class="grid gap-1 text-sm md:grid-cols-[120px,minmax(0,1fr)]"
                >
                  <div class="font-medium text-base-content/70 break-all">{{ entry.key }}</div>
                  <div class="break-words text-base-content/80">{{ entry.value }}</div>
                </div>
              </div>
            </div>

            <div
              v-if="getWorkbenchSnapshotEntries(item.entry.after).length"
              class="rounded-lg border border-success/20 bg-success/5 p-3"
            >
              <div class="mb-2 text-xs font-medium text-base-content/60">{{ wb('review.snapshotAfter') }}</div>
              <div class="space-y-2">
                <div
                  v-for="entry in getWorkbenchSnapshotEntries(item.entry.after)"
                  :key="`after-${item.id}-${entry.key}`"
                  class="grid gap-1 text-sm md:grid-cols-[120px,minmax(0,1fr)]"
                >
                  <div class="font-medium text-base-content/70 break-all">{{ entry.key }}</div>
                  <div class="break-words text-base-content/80">{{ entry.value }}</div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>

      <div
        v-else
        class="rounded-lg border border-dashed border-base-300 p-6 text-center text-sm text-base-content/60"
      >
        {{ wb('review.empty') }}
      </div>
    </div>

    <div class="rounded-lg border border-base-300 bg-base-100 p-4 space-y-3">
      <div class="grid gap-3 md:grid-cols-[180px,minmax(0,1fr)]">
        <select v-model="kind" class="select select-bordered">
          <option value="observation">{{ wb('review.noteTitle.observation') }}</option>
          <option value="conclusion">{{ wb('review.noteTitle.conclusion') }}</option>
          <option value="false_positive_reason">{{ wb('review.noteTitle.false_positive_reason') }}</option>
          <option value="remediation_note">{{ wb('review.noteTitle.remediation_note') }}</option>
          <option value="replay_note">{{ wb('review.noteTitle.replay_note') }}</option>
        </select>
        <textarea
          v-model="draft"
          class="textarea textarea-bordered min-h-[120px] w-full"
          :placeholder="wb('review.notePlaceholder')"
        />
      </div>
      <div class="flex items-center justify-between gap-3">
        <div class="text-xs text-base-content/60">
          {{ wb('review.noteSummary', { activityCount: activities.length, noteCount: notes.length }) }}
        </div>
        <button class="btn btn-sm btn-primary" @click="submitNote">{{ wb('review.addNote') }}</button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue'
import type { WorkbenchActivity, WorkbenchNote, WorkbenchNoteKind } from './securityWorkbenchTypes'
import {
  formatWorkbenchTime,
  getWorkbenchActivityKindLabel,
  getWorkbenchNoteKindLabel,
  getWorkbenchSnapshotEntries,
} from './securityWorkbenchPresentation'
import { wb } from './securityWorkbenchLocale'

const props = defineProps<{
  activities: WorkbenchActivity[]
  notes: WorkbenchNote[]
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
  'add-note': [kind: WorkbenchNoteKind, body: string]
  'open-target': [target: {
    tab: 'overview' | 'evidence' | 'drafts' | 'verification'
    evidenceId?: string | null
    draftId?: string | null
    runId?: string | null
  }]
  'copy-target-link': [target: {
    tab: 'overview' | 'evidence' | 'drafts' | 'verification'
    evidenceId?: string | null
    draftId?: string | null
    runId?: string | null
  }]
  'copy-timeline-item-link': [itemId: string]
  'change-timeline-state': [state: {
    search: string
    filter:
      | 'all'
      | 'system'
      | 'notes'
      | 'draft_execution'
      | 'finding_sync'
      | 'suggestion_sync'
      | WorkbenchNoteKind
  }]
}>()

const kind = ref<WorkbenchNoteKind>('observation')
const draft = ref('')
const timelineSearch = ref(props.initialTimelineSearch || '')
const timelineFilter = ref<
  | 'all'
  | 'system'
  | 'notes'
  | 'draft_execution'
  | 'finding_sync'
  | 'suggestion_sync'
  | WorkbenchNoteKind
>(props.initialTimelineFilter || 'all')
const timelineItemRefs = ref<Record<string, HTMLElement | null>>({})

type WorkbenchTimelineItem =
  | {
      id: string
      kind: 'activity'
      createdAt: string
      entry: WorkbenchActivity
    }
  | {
      id: string
      kind: 'note'
      createdAt: string
      entry: WorkbenchNote
    }

type WorkbenchTimelineTarget = {
  tab: 'overview' | 'evidence' | 'drafts' | 'verification'
  evidenceId?: string | null
  draftId?: string | null
  runId?: string | null
}

type WorkbenchTimelineAction = {
  label: string
  target: WorkbenchTimelineTarget
}

const setTimelineItemRef = (itemId: string) => (element: Element | null) => {
  timelineItemRefs.value[itemId] = element instanceof HTMLElement ? element : null
}

const getNoteTitle = (noteKind: WorkbenchNoteKind) => {
  switch (noteKind) {
    case 'conclusion':
      return wb('review.noteTitle.conclusion')
    case 'false_positive_reason':
      return wb('review.noteTitle.false_positive_reason')
    case 'remediation_note':
      return wb('review.noteTitle.remediation_note')
    case 'replay_note':
      return wb('review.noteTitle.replay_note')
    default:
      return wb('review.noteTitle.observation')
  }
}

const readSnapshotString = (snapshot: WorkbenchActivity['before'] | WorkbenchActivity['after']) =>
  getWorkbenchSnapshotEntries(snapshot)
    .map((entry) => `${entry.key}:${entry.value}`)
    .join(' ')

const getTimelineActions = (activity: WorkbenchActivity): WorkbenchTimelineAction[] => {
  const actions: WorkbenchTimelineAction[] = []

  if (activity.kind === 'draft_execution') {
    const beforeEntries = activity.before ?? {}
    const afterEntries = activity.after ?? {}

    if (typeof beforeEntries.targetEvidenceId === 'string' && beforeEntries.targetEvidenceId) {
      actions.push({
        label: wb('review.timelineAction.viewEvidence'),
        target: { tab: 'evidence', evidenceId: beforeEntries.targetEvidenceId },
      })
    }
    if (typeof beforeEntries.draftId === 'string' && beforeEntries.draftId) {
      actions.push({
        label: wb('review.timelineAction.viewDraft'),
        target: { tab: 'drafts', draftId: beforeEntries.draftId },
      })
    }
    if (typeof afterEntries.runId === 'string' && afterEntries.runId) {
      actions.push({
        label: wb('review.timelineAction.viewExecution'),
        target: { tab: 'verification', runId: afterEntries.runId },
      })
    }
  }

  if (activity.kind === 'finding_sync' || activity.kind === 'suggestion_sync') {
    actions.push({
      label: wb('review.timelineAction.viewOverview'),
      target: { tab: 'overview' },
    })
  }

  return actions
}

const timelineItems = computed<WorkbenchTimelineItem[]>(() => {
  const activityItems: WorkbenchTimelineItem[] = props.activities.map((entry) => ({
    id: `activity-${entry.id}`,
    kind: 'activity',
    createdAt: entry.createdAt,
    entry,
  }))
  const noteItems: WorkbenchTimelineItem[] = props.notes.map((entry) => ({
    id: `note-${entry.id}`,
    kind: 'note',
    createdAt: entry.createdAt,
    entry,
  }))
  const combined = [...activityItems, ...noteItems].sort(
    (left, right) => new Date(right.createdAt).getTime() - new Date(left.createdAt).getTime(),
  )

  return combined.filter((item) => {
    const filterMatched = (() => {
      if (timelineFilter.value === 'all') return true
      if (timelineFilter.value === 'system') return item.kind === 'activity'
      if (timelineFilter.value === 'notes') return item.kind === 'note'
      if (item.kind === 'activity') return item.entry.kind === timelineFilter.value
      return item.entry.kind === timelineFilter.value
    })()

    if (!filterMatched) return false
    if (!timelineSearch.value) return true

    const keyword = timelineSearch.value.toLowerCase()
    const haystack = item.kind === 'activity'
      ? [
          item.entry.title,
          item.entry.summary,
          item.entry.actor,
          getWorkbenchActivityKindLabel(item.entry.kind),
          readSnapshotString(item.entry.before),
          readSnapshotString(item.entry.after),
        ]
      : [
          item.entry.body,
          item.entry.author,
          getWorkbenchNoteKindLabel(item.entry.kind),
          getNoteTitle(item.entry.kind),
        ]

    return haystack
      .join(' ')
      .toLowerCase()
      .includes(keyword)
  })
})

watch(
  () => props.selectedTimelineItemId,
  async (itemId) => {
    if (!itemId) return
    await nextTick()
    timelineItemRefs.value[itemId]?.scrollIntoView({
      behavior: 'smooth',
      block: 'center',
    })
  },
  { immediate: true },
)

watch(
  () => props.initialTimelineSearch,
  (value) => {
    if (typeof value === 'string' && value !== timelineSearch.value) {
      timelineSearch.value = value
    }
  },
)

watch(
  () => props.initialTimelineFilter,
  (value) => {
    if (value && value !== timelineFilter.value) {
      timelineFilter.value = value
    }
  },
)

watch([timelineSearch, timelineFilter], ([searchValue, filterValue]) => {
  emit('change-timeline-state', {
    search: searchValue.trim(),
    filter: filterValue,
  })
})

const submitNote = () => {
  if (!draft.value.trim()) return
  emit('add-note', kind.value, draft.value)
  draft.value = ''
  kind.value = 'observation'
}
</script>
