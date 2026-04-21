<template>
  <div
    :data-team-task-id="task.id"
    class="cursor-pointer rounded-lg border bg-base-100 p-2 transition-colors"
    :class="selected ? 'border-primary bg-primary/5 ring-1 ring-primary/30' : 'border-base-300 hover:border-primary/40 hover:bg-base-200/40'"
    @click="emit('toggle-selected-task', task)"
  >
    <div class="flex items-center justify-between gap-2">
      <div class="text-sm font-medium truncate">{{ task.title || task.task_id }}</div>
      <span class="badge badge-xs" :class="teamTaskStatusBadgeClass(task.status)">
        {{ t(teamTaskStatusI18nKey(task.status)) }}
      </span>
    </div>

    <div class="mt-1 text-xs text-base-content/55 line-clamp-2">{{ task.instruction || '—' }}</div>

    <TaskMetaList
      class="mt-2"
      :items="metaItems"
    />

    <div class="mt-1 text-[11px] text-primary/90">
      {{ selected ? t('agent.teamTaskSelectedHint') : t('agent.teamTaskSelectHint') }}
    </div>

    <TeamTaskActionBar
      :task="task"
      :pending-action-kind="pendingActionKind || null"
      :pending-task-id="pendingTaskId || null"
      @claim="emit('claim-task', $event)"
      @release="emit('release-task', $event)"
      @complete="emit('complete-task', $event)"
      @fail="emit('fail-task', $event)"
      @block="emit('block-task', $event)"
    />

    <div v-if="task.last_error" class="mt-1 text-[11px] text-error line-clamp-2">{{ task.last_error }}</div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import type { TeamTask } from '@/types/agentTeam'
import TaskMetaList from './TaskMetaList.vue'
import TeamTaskActionBar from './TeamTaskActionBar.vue'
import { buildTeamTaskMetaItems } from './taskMetaPresentation'
import {
  resolveSatisfiedTeamTaskDependencies,
  resolveTeamTaskDependencyTitles,
  resolveUnresolvedTeamTaskDependencies,
  teamTaskStatusBadgeClass,
  teamTaskStatusI18nKey,
} from './teamWorkspacePresentation'

const props = defineProps<{
  task: TeamTask
  allTasks: TeamTask[]
  selected: boolean
  pendingActionKind?: 'claim' | 'release' | 'complete' | 'fail' | 'block' | null
  pendingTaskId?: string | null
  resolveAgentName: (agentId?: string | null) => string
}>()

const emit = defineEmits<{
  (e: 'toggle-selected-task', task: TeamTask): void
  (e: 'claim-task', task: TeamTask): void
  (e: 'release-task', task: TeamTask): void
  (e: 'complete-task', task: TeamTask): void
  (e: 'fail-task', task: TeamTask): void
  (e: 'block-task', task: TeamTask): void
}>()

const { t } = useI18n()

const metaItems = computed(() =>
  buildTeamTaskMetaItems(props.task, {
    resolveAgentName: props.resolveAgentName,
    dependencyPreview: (task) => resolveTeamTaskDependencyTitles(task, props.allTasks).join('、'),
    dependencySatisfiedPreview: (task) => resolveSatisfiedTeamTaskDependencies(task, props.allTasks).join('、'),
    dependencyBlockerPreview: (task) => resolveUnresolvedTeamTaskDependencies(task, props.allTasks).join('、'),
  }),
)
</script>
