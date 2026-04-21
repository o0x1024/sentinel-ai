<template>
  <div
    v-if="actions.length > 0 || showMissingHint"
    class="mt-2 flex flex-wrap items-center gap-2"
  >
    <button
      v-for="action in actions"
      :key="action.kind"
      class="btn btn-xs"
      :class="[action.buttonClass, { 'btn-disabled pointer-events-none opacity-70': action.pending }]"
      @click.stop="emitAction(action.kind)"
    >
      <i v-if="action.pending" class="fas fa-spinner fa-spin mr-1"></i>
      {{ t(action.labelKey) }}
    </button>
    <div v-if="showMissingHint" class="text-[11px] text-base-content/45">
      {{ t(missingHintKey) }}
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import type { TeamTask } from '@/types/agentTeam'
import {
  getAvailableTeamTaskActions,
  getTeamTaskMissingActionHintKey,
  isTeamTaskActionPending,
  type TeamTaskActionKind,
  shouldShowTeamTaskMissingActionHint,
} from './teamTaskActionsSupport'

const props = defineProps<{
  task: TeamTask
  pendingActionKind?: TeamTaskActionKind | null
  pendingTaskId?: string | null
}>()

const emit = defineEmits<{
  claim: [task: TeamTask]
  release: [task: TeamTask]
  complete: [task: TeamTask]
  fail: [task: TeamTask]
  block: [task: TeamTask]
}>()

const { t } = useI18n()

interface ActionViewModel {
  kind: TeamTaskActionKind
  labelKey: string
  buttonClass: string
  pending: boolean
}

const emitAction = (kind: TeamTaskActionKind) => {
  switch (kind) {
    case 'claim':
      emit('claim', props.task)
      return
    case 'release':
      emit('release', props.task)
      return
    case 'complete':
      emit('complete', props.task)
      return
    case 'fail':
      emit('fail', props.task)
      return
    case 'block':
      emit('block', props.task)
  }
}

const actionLabelKey = (kind: TeamTaskActionKind): string => {
  switch (kind) {
    case 'claim':
      return 'agent.teamTaskActionClaim'
    case 'release':
      return 'agent.teamTaskActionRelease'
    case 'complete':
      return 'agent.teamTaskActionComplete'
    case 'fail':
      return 'agent.teamTaskActionFail'
    case 'block':
      return 'agent.teamTaskActionBlock'
  }
}

const actionButtonClass = (kind: TeamTaskActionKind): string => {
  switch (kind) {
    case 'claim':
      return 'btn-secondary'
    case 'release':
      return 'btn-ghost border border-base-300'
    case 'complete':
      return 'btn-success'
    case 'fail':
      return 'btn-error'
    case 'block':
      return 'btn-warning'
  }
}

const actions = computed<ActionViewModel[]>(() =>
  getAvailableTeamTaskActions(props.task).map((kind) => ({
    kind,
    labelKey: actionLabelKey(kind),
    buttonClass: actionButtonClass(kind),
    pending: isTeamTaskActionPending(
      props.task,
      kind,
      props.pendingTaskId,
      props.pendingActionKind,
    ),
  })),
)

const showMissingHint = computed(() => shouldShowTeamTaskMissingActionHint(props.task))
const missingHintKey = computed(() => getTeamTaskMissingActionHintKey(props.task))
</script>
