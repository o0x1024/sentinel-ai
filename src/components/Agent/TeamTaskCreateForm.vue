<template>
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
          :class="{ 'btn-disabled pointer-events-none opacity-70': pending }"
          @click="showCreateTaskForm ? submitCreateTask() : openCreateTaskForm()"
        >
          <i v-if="pending" class="fas fa-spinner fa-spin mr-1"></i>
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
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import type { AgentTeamMember, TeamTaskCreateInput } from '@/types/agentTeam'

export interface TeamTaskDependencyCandidate {
  value: string
  label: string
  meta: string
}

const props = defineProps<{
  pending?: boolean
  teamMembers: AgentTeamMember[]
  dependencyCandidates: TeamTaskDependencyCandidate[]
}>()

const emit = defineEmits<{
  (e: 'create-task', input: TeamTaskCreateInput): void
}>()

const { t } = useI18n()

const showCreateTaskForm = ref(false)
const createTaskTitle = ref('')
const createTaskInstruction = ref('')
const createTaskDependsOn = ref<string[]>([])
const createTaskOwnerAgentId = ref('')
const createTaskAcceptanceCriteria = ref('')

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

const submitCreateTask = () => {
  if (props.pending) return
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
</script>
