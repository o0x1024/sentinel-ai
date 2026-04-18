<template>
  <div class="h-full flex flex-col overflow-hidden">
    <div class="flex border-b border-base-300 overflow-x-auto">
      <button
        class="flex-1 py-2 text-xs font-medium transition-colors whitespace-nowrap px-2"
        :class="activeTab === 'tasks' ? 'text-secondary border-b-2 border-secondary bg-secondary/5' : 'text-base-content/50 hover:text-base-content'"
        @click="activeTab = 'tasks'"
      >
        <i class="fas fa-list-check mr-1"></i> {{ t('agent.teamWorkspaceTabTasks') }}
      </button>
      <button
        class="flex-1 py-2 text-xs font-medium transition-colors whitespace-nowrap px-2"
        :class="activeTab === 'inbox' ? 'text-accent border-b-2 border-accent bg-accent/5' : 'text-base-content/50 hover:text-base-content'"
        @click="activeTab = 'inbox'"
      >
        <i class="fas fa-inbox mr-1"></i> {{ t('agent.teamWorkspaceTabInbox') }}
      </button>
      <button
        class="flex-1 py-2 text-xs font-medium transition-colors whitespace-nowrap px-2"
        :class="activeTab === 'blackboard' ? 'text-warning border-b-2 border-warning bg-warning/5' : 'text-base-content/50 hover:text-base-content'"
        @click="activeTab = 'blackboard'"
      >
        <i class="fas fa-chalkboard mr-1"></i> {{ t('agent.teamWorkspaceTabBlackboard') }}
      </button>
      <button
        class="flex-1 py-2 text-xs font-medium transition-colors whitespace-nowrap px-2"
        :class="activeTab === 'agents' ? 'text-info border-b-2 border-info bg-info/5' : 'text-base-content/50 hover:text-base-content'"
        @click="activeTab = 'agents'"
      >
        <i class="fas fa-users mr-1"></i> {{ t('agent.teamWorkspaceTabAgents') }}
      </button>
    </div>

    <div v-if="loading" class="flex-1 flex items-center justify-center text-sm text-base-content/50">
      <i class="fas fa-spinner fa-spin mr-2"></i> {{ t('agent.teamWorkspaceLoading') }}
    </div>

    <div v-else-if="activeTab === 'tasks'" class="flex-1 overflow-auto p-3">
      <div ref="taskListRef" class="space-y-3">
        <div class="rounded-lg border border-info/20 bg-info/5 px-3 py-2 text-[11px] text-base-content/70">
          {{ t('agent.teamWorkspaceActionGuide') }}
        </div>
        <div class="rounded-lg border border-base-300 bg-base-100 px-3 py-2">
          <div class="flex items-center justify-between gap-2">
            <div>
              <div class="text-xs font-semibold text-base-content/80">
                {{ t('agent.teamTaskCreateTitle') }}
              </div>
              <div class="text-[11px] text-base-content/50">
                {{ t('agent.teamTaskCreateHint') }}
              </div>
            </div>
            <div class="flex items-center gap-2">
              <button
                v-if="showCreateTaskForm"
                class="btn btn-ghost btn-xs"
                @click="hideCreateTaskForm"
              >
                {{ t('agent.teamTaskCreateCancel') }}
              </button>
              <button
                class="btn btn-secondary btn-xs"
                :class="{ 'btn-disabled pointer-events-none opacity-70': pendingCreateTask }"
                @click="showCreateTaskForm ? submitCreateTask() : openCreateTaskForm()"
              >
                <i v-if="pendingCreateTask" class="fas fa-spinner fa-spin mr-1"></i>
                {{ showCreateTaskForm ? t('agent.teamTaskCreateSubmit') : t('agent.teamTaskCreateButton') }}
              </button>
            </div>
          </div>
          <div v-if="showCreateTaskForm" class="mt-3 space-y-2">
            <label class="block">
              <div class="mb-1 text-[11px] font-medium text-base-content/70">
                {{ t('agent.teamTaskCreateFieldTitle') }}
              </div>
              <input
                v-model.trim="createTaskTitle"
                type="text"
                class="input input-bordered input-sm w-full"
                :placeholder="t('agent.teamTaskCreateFieldTitlePlaceholder')"
              >
            </label>
            <label class="block">
              <div class="mb-1 text-[11px] font-medium text-base-content/70">
                {{ t('agent.teamTaskCreateFieldInstruction') }}
              </div>
              <textarea
                v-model.trim="createTaskInstruction"
                class="textarea textarea-bordered textarea-sm w-full min-h-[88px]"
                :placeholder="t('agent.teamTaskCreateFieldInstructionPlaceholder')"
              ></textarea>
            </label>
            <label class="block">
              <div class="mb-1 text-[11px] font-medium text-base-content/70">
                {{ t('agent.teamTaskCreateFieldOwner') }}
              </div>
              <select
                v-model="createTaskOwnerAgentId"
                class="select select-bordered select-sm w-full"
              >
                <option value="">{{ t('agent.teamTaskCreateFieldOwnerUnassigned') }}</option>
                <option
                  v-for="member in teamMembers"
                  :key="member.id"
                  :value="member.id"
                >
                  {{ member.name || member.id }}
                </option>
              </select>
            </label>
            <div class="block">
              <div class="mb-1 text-[11px] font-medium text-base-content/70">
                {{ t('agent.teamTaskCreateFieldDependencies') }}
              </div>
              <div
                v-if="dependencyCandidates.length === 0"
                class="rounded border border-dashed border-base-300 bg-base-100/70 px-3 py-2 text-[11px] text-base-content/45"
              >
                {{ t('agent.teamTaskCreateFieldDependenciesEmpty') }}
              </div>
              <div
                v-else
                class="max-h-32 space-y-1 overflow-auto rounded border border-base-300 bg-base-100 px-2 py-2"
              >
                <label
                  v-for="candidate in dependencyCandidates"
                  :key="candidate.value"
                  class="flex cursor-pointer items-start gap-2 rounded px-1 py-1 text-[11px] text-base-content/70 hover:bg-base-200/60"
                >
                  <input
                    :checked="createTaskDependsOn.includes(candidate.value)"
                    type="checkbox"
                    class="checkbox checkbox-xs mt-0.5"
                    @change="toggleCreateTaskDependency(candidate.value)"
                  >
                  <span class="min-w-0 flex-1">
                    <span class="block truncate font-medium text-base-content/80">{{ candidate.label }}</span>
                    <span class="block truncate text-base-content/45">{{ candidate.meta }}</span>
                  </span>
                </label>
              </div>
            </div>
            <label class="block">
              <div class="mb-1 text-[11px] font-medium text-base-content/70">
                {{ t('agent.teamTaskCreateFieldAcceptance') }}
              </div>
              <textarea
                v-model.trim="createTaskAcceptanceCriteria"
                class="textarea textarea-bordered textarea-sm w-full min-h-[72px]"
                :placeholder="t('agent.teamTaskCreateFieldAcceptancePlaceholder')"
              ></textarea>
            </label>
            <div v-if="createTaskValidationMessage" class="text-[11px] text-warning">
              {{ createTaskValidationMessage }}
            </div>
          </div>
        </div>
        <div class="flex items-center justify-between rounded-lg border border-base-300 bg-base-100 px-2 py-1.5 text-[11px] text-base-content/65">
          <span>{{ t('agent.teamWorkspaceMessageScope') }}: {{ selectedTaskTitle || t('agent.teamWorkspaceGlobalView') }}</span>
          <button
            v-if="selectedTaskId"
            class="btn btn-ghost btn-xs"
            @click="clearSelectedTask"
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
            <div
              v-for="task in section.tasks"
              :key="task.id"
              :data-team-task-id="task.id"
              class="cursor-pointer rounded-lg border bg-base-100 p-2 transition-colors"
              :class="selectedTaskId === task.id ? 'border-primary bg-primary/5 ring-1 ring-primary/30' : 'border-base-300 hover:border-primary/40 hover:bg-base-200/40'"
              @click="toggleSelectedTask(task)"
            >
              <div class="flex items-center justify-between gap-2">
                <div class="text-sm font-medium truncate">{{ task.title || task.task_id }}</div>
                <span class="badge badge-xs" :class="taskStatusBadgeClass(task.status)">
                  {{ t(taskStatusLabelKey(task.status)) }}
                </span>
              </div>
              <div class="mt-1 text-xs text-base-content/55 line-clamp-2">{{ task.instruction || '—' }}</div>
              <div class="mt-2 space-y-1 text-[11px] text-base-content/60">
                <div v-if="task.owner_agent_id">
                  <span class="font-medium">{{ t('agent.teamTaskOwnerLabel') }}:</span>
                  {{ resolveAgentName(task.owner_agent_id) }}
                </div>
                <div v-if="task.claimed_by_agent_id && task.claimed_by_agent_id !== task.owner_agent_id">
                  <span class="font-medium">{{ t('agent.teamTaskCurrentHandlerLabel') }}:</span>
                  {{ resolveAgentName(task.claimed_by_agent_id) }}
                </div>
                <div v-else-if="task.assignee_agent_id">
                  <span class="font-medium">{{ t('agent.teamTaskCurrentHandlerLabel') }}:</span>
                  {{ resolveAgentName(task.assignee_agent_id) }}
                </div>
                <div>
                  <span class="font-medium">{{ t('agent.teamTaskDependencyLabel') }}:</span>
                  {{ dependencyPreview(task) || t('agent.teamTaskNoDependencies') }}
                </div>
                <div v-if="dependencySatisfiedPreview(task)" class="text-success">
                  <span class="font-medium">{{ t('agent.teamTaskSatisfiedDependencyLabel') }}:</span>
                  {{ dependencySatisfiedPreview(task) }}
                </div>
                <div v-if="dependencyBlockerPreview(task)" class="text-warning">
                  <span class="font-medium">{{ t('agent.teamTaskBlockingDependencyLabel') }}:</span>
                  {{ dependencyBlockerPreview(task) }}
                </div>
                <div v-if="task.acceptance_criteria">
                  <span class="font-medium">{{ t('agent.teamTaskAcceptanceCriteriaLabel') }}:</span>
                  {{ task.acceptance_criteria }}
                </div>
                <div>
                  <span class="font-medium">{{ t('agent.teamTaskAttemptsLabel') }}:</span>
                  {{ task.attempt }}/{{ task.max_attempts }}
                </div>
                <div>
                  <span class="font-medium">{{ t('agent.teamTaskUpdatedAtLabel') }}:</span>
                  {{ formatTimestamp(task.updated_at) }}
                </div>
              </div>
            <div class="mt-1 text-[11px] text-primary/90">
              {{ selectedTaskId === task.id ? t('agent.teamTaskSelectedHint') : t('agent.teamTaskSelectHint') }}
            </div>
              <div
                v-if="canClaimTask(task) || canReleaseTask(task) || canCompleteTask(task) || canFailTask(task) || canBlockTask(task) || showMissingActionHint(task)"
                class="mt-2 flex flex-wrap items-center gap-2"
              >
                <button
                  v-if="canClaimTask(task)"
                  class="btn btn-xs btn-secondary"
                  :class="{ 'btn-disabled pointer-events-none opacity-70': isTaskActionPending(task, 'claim') }"
                  @click.stop="emitClaimTask(task)"
                >
                  <i v-if="isTaskActionPending(task, 'claim')" class="fas fa-spinner fa-spin mr-1"></i>
                  {{ t('agent.teamTaskActionClaim') }}
                </button>
                <button
                  v-if="canReleaseTask(task)"
                  class="btn btn-xs btn-ghost border border-base-300"
                  :class="{ 'btn-disabled pointer-events-none opacity-70': isTaskActionPending(task, 'release') }"
                  @click.stop="emitReleaseTask(task)"
                >
                  <i v-if="isTaskActionPending(task, 'release')" class="fas fa-spinner fa-spin mr-1"></i>
                  {{ t('agent.teamTaskActionRelease') }}
                </button>
                <button
                  v-if="canCompleteTask(task)"
                  class="btn btn-xs btn-success"
                  :class="{ 'btn-disabled pointer-events-none opacity-70': isTaskActionPending(task, 'complete') }"
                  @click.stop="emitCompleteTask(task)"
                >
                  <i v-if="isTaskActionPending(task, 'complete')" class="fas fa-spinner fa-spin mr-1"></i>
                  {{ t('agent.teamTaskActionComplete') }}
                </button>
                <button
                  v-if="canFailTask(task)"
                  class="btn btn-xs btn-error"
                  :class="{ 'btn-disabled pointer-events-none opacity-70': isTaskActionPending(task, 'fail') }"
                  @click.stop="emitFailTask(task)"
                >
                  <i v-if="isTaskActionPending(task, 'fail')" class="fas fa-spinner fa-spin mr-1"></i>
                  {{ t('agent.teamTaskActionFail') }}
                </button>
                <button
                  v-if="canBlockTask(task)"
                  class="btn btn-xs btn-warning"
                  :class="{ 'btn-disabled pointer-events-none opacity-70': isTaskActionPending(task, 'block') }"
                  @click.stop="emitBlockTask(task)"
                >
                  <i v-if="isTaskActionPending(task, 'block')" class="fas fa-spinner fa-spin mr-1"></i>
                  {{ t('agent.teamTaskActionBlock') }}
                </button>
                <div
                  v-if="showMissingActionHint(task)"
                  class="text-[11px] text-base-content/45"
                >
                  {{ t(missingActionHintKey(task)) }}
                </div>
              </div>
              <div v-if="task.last_error" class="mt-1 text-[11px] text-error line-clamp-2">{{ task.last_error }}</div>
            </div>
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

    <div v-else-if="activeTab === 'inbox'" class="flex-1 overflow-auto p-3">
      <div v-if="sessionMessages.length === 0" class="text-sm text-base-content/50">{{ t('agent.teamWorkspaceNoInboxMessages') }}</div>
      <div v-else class="space-y-2">
        <div
          v-for="msg in sessionMessages"
          :key="msg.id"
          class="rounded-lg border border-base-300 bg-base-100 p-2"
        >
          <div class="flex items-center justify-between gap-2">
            <div class="text-xs font-semibold">{{ msg.role }}</div>
            <span class="text-[11px] text-base-content/45">{{ formatTimestamp(msg.timestamp) }}</span>
          </div>
          <div class="mt-1 text-[11px] text-base-content/60">
            {{ msg.member_name || msg.member_id || t('agent.systemMessageLabel') }}
          </div>
          <div class="mt-1 text-[11px] bg-base-200/60 rounded p-1.5 whitespace-pre-wrap break-words">{{ msg.content || '—' }}</div>
        </div>
      </div>
    </div>

    <div v-else-if="activeTab === 'blackboard'" class="flex-1 overflow-auto p-3">
      <div v-if="blackboardEntries.length === 0" class="text-sm text-base-content/50">{{ t('agent.teamWorkspaceNoBlackboardEntries') }}</div>
      <div v-else class="space-y-2">
        <div
          v-for="entry in blackboardEntries"
          :key="entry.id"
          class="rounded-lg border border-base-300 bg-base-100 p-2"
        >
          <div class="flex items-center justify-between gap-2">
            <span class="badge badge-xs" :class="teamBlackboardEntryBadgeClass(entry.entry_type)">{{ t(blackboardEntryLabelKey(entry.entry_type)) }}</span>
            <span class="text-[11px] text-base-content/45">{{ formatTimestamp(entry.created_at) }}</span>
          </div>
          <div class="mt-1 text-[11px] text-base-content/60">
            {{ t('agent.teamBlackboardAgentLabel') }}: {{ resolveAgentName(entry.agent_id) }} · {{ t('agent.teamBlackboardTaskLabel') }}: {{ entry.task_id || '-' }}
          </div>
          <template v-if="isArtifactRefEntry(entry)">
            <div class="mt-1 text-[11px] bg-base-200/60 rounded p-1.5 whitespace-pre-wrap break-words">
              {{ artifactSummary(entry) || entry.content || '—' }}
            </div>
            <div v-if="artifactPath(entry)" class="mt-1 text-[11px] text-base-content/65 break-all">
              {{ t('agent.teamArtifactFileLabel') }}: {{ artifactPath(entry) }}
            </div>
            <div v-if="artifactContainerPath(entry)" class="mt-0.5 text-[11px] text-base-content/55 break-all">
              {{ t('agent.teamArtifactContainerLabel') }}: {{ artifactContainerPath(entry) }}
            </div>
            <div v-if="artifactHostPath(entry)" class="mt-0.5 text-[11px] text-base-content/55 break-all">
              {{ t('agent.teamArtifactHostLabel') }}: {{ artifactHostPath(entry) }}
            </div>
            <div v-if="artifactBytes(entry) !== null" class="mt-0.5 text-[11px] text-base-content/55">
              {{ t('agent.teamArtifactSizeLabel') }}: {{ formatBytes(artifactBytes(entry) ?? 0) }}
            </div>
          </template>
          <div v-else class="mt-1 text-[11px] bg-base-200/60 rounded p-1.5 whitespace-pre-wrap break-words">{{ entry.content || '—' }}</div>
        </div>
      </div>
    </div>

    <div v-else-if="activeTab === 'agents'" class="flex-1 overflow-auto p-3">
      <div v-if="(sessionDetail?.members || []).length === 0" class="text-sm text-base-content/50">{{ t('agent.teamWorkspaceNoAgents') }}</div>
      <div v-else class="space-y-2">
        <div
          v-for="member in (sessionDetail?.members || [])"
          :key="member.id"
          class="rounded-lg border border-base-300 bg-base-100 p-2"
        >
          <div class="flex items-center justify-between">
            <div class="text-sm font-medium">{{ member.name }}</div>
            <span class="badge badge-xs" :class="agentStatusBadgeClass(member.id, member.name, member.is_active)">
              {{ t(agentStatusI18nKey(member.id, member.name, member.is_active)) }}
            </span>
          </div>
          <div class="mt-1 text-[11px] text-base-content/55">{{ member.responsibility || t('agent.teamWorkspaceNoResponsibility') }}</div>
          <div class="mt-1 text-[11px] text-base-content/50">
            {{ t('agent.teamAgentTokensLabel') }}: {{ member.token_usage }} · {{ t('agent.teamAgentToolsLabel') }}: {{ member.tool_calls_count }}
          </div>
        </div>
      </div>
    </div>
    
    <div v-else class="flex-1 overflow-auto p-4">
      <div class="rounded-xl border border-base-300 bg-base-100 p-4 text-sm text-base-content/70">
        {{ t('agent.teamWorkspaceEnabledMessage') }}
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import type {
  AgentTeamMessage,
  AgentTeamSession,
  TeamBlackboardEntry,
  TeamTaskCreateInput,
  TeamTaskReasonActionInput,
  TeamTask,
} from '@/types/agentTeam'
import {
  buildTeamTaskSections,
  resolveTeamTaskDependencyTitles,
  resolveSatisfiedTeamTaskDependencies,
  resolveUnresolvedTeamTaskDependencies,
  teamBlackboardEntryTypeI18nKey,
  teamTaskStatusBadgeClass as getTeamTaskStatusBadgeClass,
  teamTaskStatusI18nKey,
  teamWorkspaceAgentStatusI18nKey,
} from './teamWorkspacePresentation'
import {
  canTeamTaskBlock,
  canTeamTaskClaim,
  canTeamTaskComplete,
  canTeamTaskFail,
  canTeamTaskRelease,
  getTeamTaskClaimAgentId,
  getTeamTaskReleaseAgentId,
} from './teamTaskActionsSupport'

const props = defineProps<{
  tab: 'tasks' | 'inbox' | 'blackboard' | 'agents'
  loading: boolean
  pendingCreateTask?: boolean
  tasks: TeamTask[]
  selectedTaskId: string | null
  selectedTaskTitle: string | null
  pendingTaskActionKind?: 'claim' | 'release' | 'complete' | 'fail' | 'block' | null
  pendingTaskActionTaskId?: string | null
  sessionMessages: AgentTeamMessage[]
  blackboardEntries: TeamBlackboardEntry[]
  sessionDetail: AgentTeamSession | null
  resolveAgentName: (agentId?: string | null) => string
}>()

const emit = defineEmits<{
  (e: 'update:tab', tab: 'tasks' | 'inbox' | 'blackboard' | 'agents'): void
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

const activeTab = computed({
  get: () => props.tab,
  set: (val) => emit('update:tab', val)
})

const taskSections = computed(() => buildTeamTaskSections(props.tasks))
const teamMembers = computed(() => props.sessionDetail?.members || [])
const taskListRef = ref<HTMLElement | null>(null)
const showCreateTaskForm = ref(false)
const createTaskTitle = ref('')
const createTaskInstruction = ref('')
const createTaskDependsOn = ref<string[]>([])
const createTaskOwnerAgentId = ref('')
const createTaskAcceptanceCriteria = ref('')
const statusReasonAction = ref<'fail' | 'block' | null>(null)
const statusReasonTask = ref<TeamTask | null>(null)
const statusReasonText = ref('')
const dependencyCandidates = computed(() =>
  props.tasks.map((task) => ({
    value: String(task.task_id || task.id || '').trim(),
    label: String(task.title || task.task_id || task.id || '').trim(),
    meta: [
      props.resolveAgentName(task.owner_agent_id),
      t(taskStatusLabelKey(task.status)),
    ].filter(Boolean).join(' · '),
  })).filter((item) => item.value && item.label),
)
const createTaskValidationMessage = computed(() => {
  if (!showCreateTaskForm.value) return ''
  if (!createTaskTitle.value.trim()) return t('agent.teamTaskCreateValidationTitle')
  if (!createTaskInstruction.value.trim()) return t('agent.teamTaskCreateValidationInstruction')
  return ''
})

const resetCreateTaskForm = () => {
  createTaskTitle.value = ''
  createTaskInstruction.value = ''
  createTaskDependsOn.value = []
  createTaskOwnerAgentId.value = ''
  createTaskAcceptanceCriteria.value = ''
}

const openCreateTaskForm = () => {
  showCreateTaskForm.value = true
}

const hideCreateTaskForm = () => {
  showCreateTaskForm.value = false
  resetCreateTaskForm()
}

const clearSelectedTask = () => {
  emit('clear-selected-task')
}

const submitCreateTask = () => {
  if (props.pendingCreateTask) return
  const title = createTaskTitle.value.trim()
  const instruction = createTaskInstruction.value.trim()
  if (!title || !instruction) return
  emit('create-task', {
    title,
    instruction,
    depends_on: [...createTaskDependsOn.value],
    owner_agent_id: createTaskOwnerAgentId.value.trim() || null,
    acceptance_criteria: createTaskAcceptanceCriteria.value.trim() || null,
  })
  hideCreateTaskForm()
}

const toggleCreateTaskDependency = (taskId: string) => {
  const normalized = taskId.trim()
  if (!normalized) return
  if (createTaskDependsOn.value.includes(normalized)) {
    createTaskDependsOn.value = createTaskDependsOn.value.filter((item) => item !== normalized)
    return
  }
  createTaskDependsOn.value = [...createTaskDependsOn.value, normalized]
}

const emitClaimTask = (task: TeamTask) => {
  emit('claim-task', task)
}

const emitReleaseTask = (task: TeamTask) => {
  emit('release-task', task)
}

const emitCompleteTask = (task: TeamTask) => {
  emit('complete-task', task)
}

const emitFailTask = (task: TeamTask) => {
  openStatusReasonDialog('fail', task)
}

const emitBlockTask = (task: TeamTask) => {
  openStatusReasonDialog('block', task)
}

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
  return isTaskActionPending(statusReasonTask.value, statusReasonAction.value)
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

const toggleSelectedTask = (task: TeamTask) => {
  emit('toggle-selected-task', task)
}

const scrollSelectedTaskIntoView = async () => {
  if (props.tab !== 'tasks') return
  const taskId = String(props.selectedTaskId || '').trim()
  if (!taskId) return
  await nextTick()
  const selector = `[data-team-task-id="${taskId}"]`
  const element = taskListRef.value?.querySelector<HTMLElement>(selector)
  element?.scrollIntoView({ block: 'nearest', behavior: 'smooth' })
}

watch(
  () => [props.selectedTaskId, props.tab, props.tasks.length],
  () => {
    void scrollSelectedTaskIntoView()
  },
)

const taskStatusBadgeClass = (status: string) => getTeamTaskStatusBadgeClass(status)
const taskStatusLabelKey = (status: string) => teamTaskStatusI18nKey(status)

const AGENT_STATUS_PRIORITY: Record<string, number> = {
  idle: 0,
  pending: 1,
  completed: 2,
  running: 3,
  blocked: 4,
  failed: 5,
}

const normalizeAgentRuntimeStatus = (value: unknown): string => {
  const normalized = String(value || '').trim().toLowerCase()
  if (!normalized) return 'idle'
  if (normalized.includes('fail') || normalized.includes('error')) return 'failed'
  if (normalized.includes('block')) return 'blocked'
  if (
    normalized.includes('run') ||
    normalized.includes('execut') ||
    normalized.includes('claim') ||
    normalized.includes('wait')
  ) {
    return 'running'
  }
  if (normalized.includes('done') || normalized.includes('complete')) return 'completed'
  if (normalized.includes('queue') || normalized.includes('pending') || normalized.includes('ready')) {
    return 'pending'
  }
  return 'idle'
}

const mergeAgentStatus = (current: string | undefined, candidate: string): string => {
  const currentRank = AGENT_STATUS_PRIORITY[current || 'idle'] ?? 0
  const candidateRank = AGENT_STATUS_PRIORITY[candidate] ?? 0
  return candidateRank >= currentRank ? candidate : (current || 'idle')
}

const memberStatusById = computed(() => {
  const out = new Map<string, string>()

  for (const member of props.sessionDetail?.members || []) {
    if (!member?.id) continue
    const base = member.is_active ? 'running' : 'idle'
    out.set(member.id, mergeAgentStatus(out.get(member.id), base))
  }

  for (const task of props.tasks || []) {
    const assignee = typeof task.assignee_agent_id === 'string' ? task.assignee_agent_id.trim() : ''
    if (!assignee) continue
    const candidate = normalizeAgentRuntimeStatus(task.status)
    out.set(assignee, mergeAgentStatus(out.get(assignee), candidate))
  }

  return out
})

const agentRuntimeStatus = (memberId?: string, _memberName?: string, isActive?: boolean): string => {
  const key = typeof memberId === 'string' ? memberId.trim() : ''
  if (key && memberStatusById.value.has(key)) {
    return memberStatusById.value.get(key) || 'idle'
  }
  return isActive ? 'running' : 'idle'
}

const agentStatusI18nKey = (memberId?: string, memberName?: string, isActive?: boolean): string => {
  const status = agentRuntimeStatus(memberId, memberName, isActive)
  return teamWorkspaceAgentStatusI18nKey(status)
}

const agentStatusBadgeClass = (memberId?: string, memberName?: string, isActive?: boolean): string => {
  const status = agentRuntimeStatus(memberId, memberName, isActive)
  if (status === 'running') return 'badge-info'
  if (status === 'failed') return 'badge-error'
  if (status === 'blocked') return 'badge-warning'
  if (status === 'completed') return 'badge-success'
  if (status === 'pending') return 'badge-secondary'
  return 'badge-ghost'
}

const teamBlackboardEntryBadgeClass = (entryType?: string | null) => {
  const normalized = String(entryType || '').toLowerCase()
  if (normalized === 'task_output') return 'badge-success'
  if (normalized === 'artifact_ref') return 'badge-neutral'
  if (normalized === 'task_error') return 'badge-error'
  if (normalized === 'task_start') return 'badge-info'
  if (normalized === 'plan') return 'badge-secondary'
  if (normalized === 'plan_fallback') return 'badge-warning'
  if (normalized === 'goal') return 'badge-accent'
  return 'badge-ghost'
}

const blackboardEntryLabelKey = (entryType?: string | null) => {
  return teamBlackboardEntryTypeI18nKey(entryType)
}

const dependencyPreview = (task: TeamTask) => {
  return resolveTeamTaskDependencyTitles(task, props.tasks).join('、')
}

const dependencyBlockerPreview = (task: TeamTask) => {
  return resolveUnresolvedTeamTaskDependencies(task, props.tasks).join('、')
}

const dependencySatisfiedPreview = (task: TeamTask) => {
  return resolveSatisfiedTeamTaskDependencies(task, props.tasks).join('、')
}

const canClaimTask = (task: TeamTask) => canTeamTaskClaim(task)
const canReleaseTask = (task: TeamTask) => canTeamTaskRelease(task)
const canCompleteTask = (task: TeamTask) => canTeamTaskComplete(task)
const canFailTask = (task: TeamTask) => canTeamTaskFail(task)
const canBlockTask = (task: TeamTask) => canTeamTaskBlock(task)

const isTaskActionPending = (
  task: TeamTask,
  action: 'claim' | 'release' | 'complete' | 'fail' | 'block',
) => {
  return props.pendingTaskActionTaskId === task.id && props.pendingTaskActionKind === action
}

const missingActionHintKey = (task: TeamTask) => {
  const status = String(task.status || '').trim().toLowerCase()
  if (['pending', 'ready_for_claim'].includes(status) && !getTeamTaskClaimAgentId(task)) {
    return 'agent.teamTaskActionClaimMissingActor'
  }
  if (['claimed', 'running'].includes(status) && !getTeamTaskReleaseAgentId(task)) {
    return 'agent.teamTaskActionReleaseMissingActor'
  }
  return 'agent.teamTaskActionUnavailable'
}

const showMissingActionHint = (task: TeamTask) => {
  const status = String(task.status || '').trim().toLowerCase()
  if (['pending', 'ready_for_claim'].includes(status)) return !getTeamTaskClaimAgentId(task)
  if (['claimed', 'running'].includes(status)) return !getTeamTaskReleaseAgentId(task)
  return false
}

const isArtifactRefEntry = (entry: TeamBlackboardEntry) => {
  return String(entry.entry_type || '').toLowerCase() === 'artifact_ref'
}

const artifactSummary = (entry: TeamBlackboardEntry) => {
  const raw = entry.metadata?.summary
  return typeof raw === 'string' ? raw : ''
}

const artifactPath = (entry: TeamBlackboardEntry) => {
  const raw = entry.metadata?.artifact?.path
  return typeof raw === 'string' ? raw : ''
}

const artifactBytes = (entry: TeamBlackboardEntry) => {
  const raw = entry.metadata?.artifact?.bytes
  const bytes = Number(raw)
  return Number.isFinite(bytes) && bytes >= 0 ? bytes : null
}

const artifactContainerPath = (entry: TeamBlackboardEntry) => {
  const raw = entry.metadata?.artifact?.container_path
  return typeof raw === 'string' ? raw : ''
}

const artifactHostPath = (entry: TeamBlackboardEntry) => {
  const raw = entry.metadata?.artifact?.host_path
  return typeof raw === 'string' ? raw : ''
}

const formatBytes = (bytes: number) => {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
}

const formatTimestamp = (value?: string | null) => {
  if (!value) return '—'
  const time = new Date(value).getTime()
  if (!Number.isFinite(time)) return value
  return new Date(time).toLocaleString()
}
</script>
