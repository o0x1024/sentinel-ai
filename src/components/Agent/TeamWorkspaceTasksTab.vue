<template>
  <div class="flex-1 overflow-auto p-3">
    <div ref="taskListRef" class="space-y-3">
      <div class="rounded-lg border border-info/20 bg-info/5 px-3 py-2 text-[11px] text-base-content/70">
        {{ t('agent.teamWorkspaceActionGuide') }}
      </div>

      <TeamTaskCreateForm
        :pending="pendingCreateTask"
        :team-members="teamMembers"
        :dependency-candidates="dependencyCandidates"
        @create-task="emit('create-task', $event)"
      />

      <div class="flex items-center justify-between rounded-lg border border-base-300 bg-base-100 px-2 py-1.5 text-[11px] text-base-content/65">
        <span>{{ t('agent.teamWorkspaceMessageScope') }}: {{ selectedTaskTitle || t('agent.teamWorkspaceGlobalView') }}</span>
        <button
          v-if="selectedTaskId"
          class="btn btn-ghost btn-xs"
          @click="emit('clear-selected-task')"
        >
          {{ t('agent.teamWorkspaceClearTaskSelection') }}
        </button>
      </div>

      <div v-if="tasks.length === 0" class="rounded-lg border border-dashed border-base-300 bg-base-100/70 px-3 py-3 text-sm text-base-content/50">
        {{ t('agent.teamWorkspaceNoTasks') }}
      </div>

      <section
        v-for="section in taskSections"
        :key="section.key"
        class="space-y-2"
      >
        <div class="flex items-center justify-between px-1">
          <div>
            <div class="text-[11px] font-semibold uppercase tracking-wide text-base-content/55">
              {{ t(section.titleKey) }}
            </div>
            <div class="text-[11px] text-base-content/45">
              {{ t(section.descriptionKey) }}
            </div>
          </div>
          <span class="badge badge-xs badge-ghost">{{ section.tasks.length }}</span>
        </div>

        <div v-if="section.tasks.length === 0" class="rounded-lg border border-dashed border-base-300 bg-base-100/70 px-3 py-2">
          <div class="text-[11px] text-base-content/55">
            {{ t(section.emptyKey) }}
          </div>
          <div class="mt-1 text-[11px] text-base-content/45">
            {{ t(section.actionKey) }}
          </div>
        </div>

        <template v-else>
          <TeamTaskCard
            v-for="task in section.tasks"
            :key="task.id"
            :task="task"
            :all-tasks="tasks"
            :selected="selectedTaskId === task.id"
            :pending-action-kind="pendingTaskActionKind || null"
            :pending-task-id="pendingTaskActionTaskId || null"
            :resolve-agent-name="resolveAgentName"
            @toggle-selected-task="emit('toggle-selected-task', $event)"
            @claim-task="emit('claim-task', $event)"
            @release-task="emit('release-task', $event)"
            @complete-task="emit('complete-task', $event)"
            @fail-task="openStatusReasonDialog('fail', $event)"
            @block-task="openStatusReasonDialog('block', $event)"
          />
        </template>
      </section>
    </div>

    <div
      v-if="statusReasonDialogOpen"
      class="fixed inset-0 z-20 flex items-center justify-center bg-black/40 p-4"
      @click.self="closeStatusReasonDialog"
    >
      <div class="w-full max-w-lg rounded-xl border border-base-300 bg-base-100 shadow-2xl">
        <div class="flex items-center justify-between border-b border-base-300 px-4 py-3">
          <div>
            <div class="text-sm font-semibold text-base-content">
              {{ statusReasonDialogTitle }}
            </div>
            <div class="text-[11px] text-base-content/55">
              {{ statusReasonDialogTaskLabel }}
            </div>
          </div>
          <button class="btn btn-ghost btn-xs" @click="closeStatusReasonDialog">
            {{ t('agent.teamTaskStatusReasonCancel') }}
          </button>
        </div>
        <div class="space-y-3 px-4 py-3">
          <div class="text-[11px] text-base-content/60">
            {{ statusReasonDialogHint }}
          </div>
          <label class="block">
            <div class="mb-1 text-[11px] font-medium text-base-content/70">
              {{ t('agent.teamTaskStatusReasonLabel') }}
            </div>
            <textarea
              v-model.trim="statusReasonText"
              class="textarea textarea-bordered textarea-sm min-h-[112px] w-full"
              :placeholder="statusReasonDialogPlaceholder"
            ></textarea>
          </label>
          <div v-if="statusReasonValidationMessage" class="text-[11px] text-warning">
            {{ statusReasonValidationMessage }}
          </div>
        </div>
        <div class="flex items-center justify-end gap-2 border-t border-base-300 px-4 py-3">
          <button class="btn btn-ghost btn-sm" @click="closeStatusReasonDialog">
            {{ t('agent.teamTaskStatusReasonCancel') }}
          </button>
          <button
            class="btn btn-primary btn-sm"
            :class="{ 'btn-disabled pointer-events-none opacity-70': statusReasonDialogPending }"
            @click="submitStatusReasonDialog"
          >
            <i v-if="statusReasonDialogPending" class="fas fa-spinner fa-spin mr-1"></i>
            {{ t('agent.teamTaskStatusReasonSubmit') }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import type { AgentTeamMember, TeamTask, TeamTaskCreateInput, TeamTaskReasonActionInput } from '@/types/agentTeam'
import TeamTaskCard from './TeamTaskCard.vue'
import TeamTaskCreateForm, { type TeamTaskDependencyCandidate } from './TeamTaskCreateForm.vue'
import { buildTeamTaskSections, teamTaskStatusI18nKey } from './teamWorkspacePresentation'
import { isTeamTaskActionPending } from './teamTaskActionsSupport'

const props = defineProps<{
  pendingCreateTask?: boolean
  tasks: TeamTask[]
  selectedTaskId: string | null
  selectedTaskTitle: string | null
  pendingTaskActionKind?: 'claim' | 'release' | 'complete' | 'fail' | 'block' | null
  pendingTaskActionTaskId?: string | null
  teamMembers: AgentTeamMember[]
  resolveAgentName: (agentId?: string | null) => string
}>()

const emit = defineEmits<{
  (e: 'clear-selected-task'): void
  (e: 'claim-task', task: TeamTask): void
  (e: 'complete-task', task: TeamTask): void
  (e: 'create-task', input: TeamTaskCreateInput): void
  (e: 'fail-task', input: TeamTaskReasonActionInput): void
  (e: 'block-task', input: TeamTaskReasonActionInput): void
  (e: 'release-task', task: TeamTask): void
  (e: 'toggle-selected-task', task: TeamTask): void
}>()

const { t } = useI18n()

const taskSections = computed(() => buildTeamTaskSections(props.tasks))
const taskListRef = ref<HTMLElement | null>(null)
const statusReasonAction = ref<'fail' | 'block' | null>(null)
const statusReasonTask = ref<TeamTask | null>(null)
const statusReasonText = ref('')

const dependencyCandidates = computed<TeamTaskDependencyCandidate[]>(() =>
  props.tasks.map((task) => ({
    value: String(task.task_id || task.id || '').trim(),
    label: String(task.title || task.task_id || task.id || '').trim(),
    meta: [
      props.resolveAgentName(task.owner_agent_id),
      t(teamTaskStatusI18nKey(task.status)),
    ].filter(Boolean).join(' · '),
  })).filter((item) => item.value && item.label),
)

const openStatusReasonDialog = (action: 'fail' | 'block', task: TeamTask) => {
  statusReasonAction.value = action
  statusReasonTask.value = task
  statusReasonText.value = String(task.last_error || '').trim()
}

const closeStatusReasonDialog = () => {
  if (statusReasonDialogPending.value) return
  statusReasonAction.value = null
  statusReasonTask.value = null
  statusReasonText.value = ''
}

const statusReasonDialogOpen = computed(() => Boolean(statusReasonAction.value && statusReasonTask.value))
const statusReasonDialogPending = computed(() => {
  if (!statusReasonTask.value || !statusReasonAction.value) return false
  return isTeamTaskActionPending(
    statusReasonTask.value,
    statusReasonAction.value,
    props.pendingTaskActionTaskId,
    props.pendingTaskActionKind,
  )
})
const statusReasonDialogTitle = computed(() => {
  if (statusReasonAction.value === 'fail') return t('agent.teamTaskStatusReasonTitleFail')
  if (statusReasonAction.value === 'block') return t('agent.teamTaskStatusReasonTitleBlock')
  return ''
})
const statusReasonDialogTaskLabel = computed(() => {
  const task = statusReasonTask.value
  if (!task) return ''
  return String(task.title || task.task_id || task.id || '').trim()
})
const statusReasonDialogHint = computed(() => {
  if (statusReasonAction.value === 'fail') return t('agent.teamTaskStatusReasonHintFail')
  if (statusReasonAction.value === 'block') return t('agent.teamTaskStatusReasonHintBlock')
  return ''
})
const statusReasonDialogPlaceholder = computed(() => {
  if (statusReasonAction.value === 'fail') return t('agent.teamTaskStatusReasonPlaceholderFail')
  if (statusReasonAction.value === 'block') return t('agent.teamTaskStatusReasonPlaceholderBlock')
  return ''
})
const statusReasonValidationMessage = computed(() => {
  if (!statusReasonDialogOpen.value) return ''
  if (!statusReasonText.value.trim()) return t('agent.teamTaskStatusReasonValidation')
  return ''
})

const submitStatusReasonDialog = () => {
  const action = statusReasonAction.value
  const task = statusReasonTask.value
  const reason = statusReasonText.value.trim()
  if (!action || !task || !reason) return
  const payload = { task, reason }
  if (action === 'fail') {
    emit('fail-task', payload)
  } else {
    emit('block-task', payload)
  }
  closeStatusReasonDialog()
}

const scrollSelectedTaskIntoView = async () => {
  const taskId = String(props.selectedTaskId || '').trim()
  if (!taskId) return
  await nextTick()
  const selector = `[data-team-task-id="${taskId}"]`
  const element = taskListRef.value?.querySelector<HTMLElement>(selector)
  element?.scrollIntoView({ block: 'nearest', behavior: 'smooth' })
}

watch(
  () => [props.selectedTaskId, props.tasks.length],
  () => {
    void scrollSelectedTaskIntoView()
  },
)
</script>
